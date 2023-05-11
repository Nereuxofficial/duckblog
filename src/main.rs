mod post;
mod ssg;
mod utils;

use crate::post::Post;
use crate::ssg::generate_static_site;
use crate::utils::{build_header, liquid_parse, static_file_handler};
use axum::body::Body;
use axum::extract::Path;
use axum::http::{StatusCode, Uri};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{routing::get, Router};
use liquid::{object, Object};
use new_mime_guess::MimeGuess;
use opentelemetry_otlp::WithExportConfig;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::fs::read_to_string;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::*;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

// TODO: Tables don't get processed properly
// TODO: Think about blue/green deployment
// TODO: Wrapping Code blocks
// TODO: Large cleanup
// TODO: Create sitemap.xml
// TODO: RSS Feed
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Read .env
    dotenvy::dotenv().ok();
    // this reports panics
    let _guard = sentry::init((
        env::var("SENTRY_DSN")?,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));
    // OpenTelemetry tracing
    let mut metadata = HashMap::new();
    metadata.insert(
        "x-honeycomb-team".to_string(),
        env::var("HONEYCOMB_API_KEY")?.to_string(),
    );
    metadata.insert("x-honeycomb-dataset".to_string(), "duckblog".to_string());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint("https://api.honeycomb.io/v1/traces")
                .with_http_client(reqwest::Client::new())
                .with_timeout(std::time::Duration::from_secs(2))
                .with_headers(metadata),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    // filter printed-out log statements according to the RUST_LOG env var
    let rust_log_var = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let log_filter = Targets::from_str(&rust_log_var)?;
    // different filter for traces sent to honeycomb
    let trace_filter = Targets::from_str("futile=info")?;
    Registry::default()
        .with(telemetry)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_filter(log_filter),
        )
        .init();
    // Define Routes
    let app = info_span!("route.definitions").in_scope(|| {
        Router::new()
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
            .route("/posts/*path", get(get_post))
            .route(
                "/posts",
                get(|| async { list_posts(Path(String::new())).await }),
            )
            .route("/", get(|| async { list_posts(Path(String::new())).await }))
            .route(
                "/index.html",
                get(|| async { list_posts(Path(String::new())).await }),
            )
            .route("/tags/:tag", get(list_posts))
            .route(
                "/about",
                get(|| async { get_post(Path("../about".to_string())).await }),
            )
            .route(
                "/donate",
                get(|| async { get_post(Path("../donate".to_string())).await }),
            )
            .nest(
                "/static",
                Router::new().route("/*uri", get(static_file_handler)),
            )
            .route(
                "/favicon.ico",
                get(|| async {
                    static_file_handler(Uri::from_static("https://nereux.blog/favicon.ico")).await
                }),
            )
            .fallback(handler_404)
    });

    // run our app with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 8010));
    info!("listening on http://{}", addr);
    if std::env::args().any(|arg| arg == "--ssg") {
        if cfg!(debug_assertions) {
            warn!("You are running the SSG in debug mode. This is not recommended.");
        }
        tokio::spawn(generate_static_site());
    }
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn get_image(path: String) -> impl IntoResponse {
    debug!("Image `{}` requested", path);
    if let Ok(mut file) = File::open(format!("content/posts/{path}")).await {
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)
            .await
            .expect("Could not read image");

        // Return the image with the right mime type
        Response::builder()
            .header(
                "Content-Type",
                MimeGuess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string(),
            )
            .body(Body::from(buffer))
            .unwrap()
            .into_response()
    } else {
        handler_404().await.into_response()
    }
}

#[instrument]
async fn get_post(Path(path): Path<String>) -> impl IntoResponse {
    // FIXME: Dumb workaround for images in posts
    if path.contains("images") {
        return get_image(path).await.into_response();
    }
    if !path.ends_with("/") {
        // FIXME: Workaround for wrong image paths
        return Redirect::to(format!("/posts/{}/", path).as_str()).into_response();
    }
    // Remove trailing slash
    let path = path.trim_end_matches('/');
    debug!("Post `{}` requested", path);
    let loaded_post = Post::load(format!("content/posts/{}", path)).await;
    if let Ok(post) = loaded_post {
        let cloned_post = post.clone();
        let template = liquid_parse("post.html.liquid");
        let header = build_header(Some(post.metadata)).await;
        let navbar = read_to_string("./liquid/navbar.liquid").await.unwrap();
        let footer = read_to_string("./liquid/footer.liquid").await.unwrap();
        // TODO: Cleanup, don't pass in data which is already in metadata
        let globals: Object = object!({
            "post": cloned_post,
            "header": header,
            "navbar": navbar,
            "footer": footer,
        });
        let markup = template.await.render(&globals).unwrap();
        Html(markup).into_response()
    } else {
        debug!("Post not found because: {:#?}", loaded_post);
        handler_404().await.into_response()
    }
}
#[instrument]
async fn list_posts(Path(path): Path<String>) -> impl IntoResponse {
    info!("Listing posts with filter: {:#?}", path);
    let mut posts = Post::parse_all_posts().await.unwrap();
    if !path.is_empty() {
        posts.retain(|post| post.metadata.tags.iter().any(|tag| tag == &path));
    }
    let navbar = read_to_string("./liquid/navbar.liquid").await.unwrap();
    let footer = read_to_string("./liquid/footer.liquid").await.unwrap();
    let template = info_span!("liquid.parse").in_scope(|| liquid_parse("index.html.liquid"));
    let globals: Object = object!({ "posts": posts,
            "navbar": navbar,
            "footer": footer });
    let markup = info_span!("liquid.render")
        .in_scope(|| async { template.await.render(&globals).unwrap() })
        .await;
    Html(markup).into_response()
}
#[instrument(name = "404")]
async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html::from(include_str!("../liquid/404.html")),
    )
}
