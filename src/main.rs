mod post;
mod rss;
mod sponsors;
mod ssg;
mod utils;

use crate::post::Post;
use crate::rss::serve_rss_feed;
use crate::sponsors::{get_sponsors, noncached_get_sponsors, Sponsor};
use crate::ssg::generate_static_site;
use crate::utils::{build_header, liquid_parse, static_file_handler};
use axum::body::Body;
use axum::extract::Path;
use axum::http::{StatusCode, Uri};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{routing::get, Router};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use liquid::{object, Object};
use moka::future::Cache;
use new_mime_guess::MimeGuess;
use opentelemetry_otlp::WithExportConfig;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::fs::read_to_string;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tower::limit::ConcurrencyLimit;
use tower::timeout::Timeout;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::*;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

// Use Jemalloc only for musl-64 bits platforms
#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

pub static POST_CACHE: OnceLock<Cache<String, Post>> = OnceLock::new();
pub static IMAGE_CACHE: OnceLock<Cache<String, Vec<u8>>> = OnceLock::new();
pub static SPONSORS: OnceLock<Arc<RwLock<Vec<Sponsor>>>> = OnceLock::new();

// TODO: Think about blue/green deployment
// TODO: Wrapping Code blocks
// TODO: Create sitemap.xml
// TODO: add tower-livereload
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
        env::var("HONEYCOMB_API_KEY")?,
    );
    metadata.insert("x-honeycomb-dataset".to_string(), "duckblog".to_string());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint("https://api.honeycomb.io/v1/traces")
                .with_timeout(Duration::from_secs(2))
                .with_headers(metadata),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    // filter printed-out log statements according to the RUST_LOG env var
    let rust_log_var = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let log_filter = Targets::from_str(&rust_log_var)?;
    // different filter for traces sent to honeycomb
    Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_filter(log_filter),
        )
        .with(telemetry)
        .with(sentry_tracing::layer())
        .init();

    // Trace executed code
    tracing::subscriber::with_default(Registry::default(), || {
        // Spans will be sent to the configured OpenTelemetry exporter
        let root = span!(tracing::Level::TRACE, "app_start", work_units = 2);
        let _enter = root.enter();
    });
    // Load Sponsors
    SPONSORS
        .set(Arc::new(RwLock::new(noncached_get_sponsors().await?)))
        .unwrap();
    // Spawn a task to refresh them every hour
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            info!("Refreshing Sponsors");
            let new_sponsors = noncached_get_sponsors().await;
            if let Ok(sponsors) = new_sponsors {
                let mut sponsor_lock = SPONSORS.get().unwrap().write().await;
                sponsor_lock.clear();
                sponsor_lock.extend(sponsors);
            }
        }
    });
    init_caches().await;
    start_server().await;
    Ok(())
}

/// Initializes the cache for Posts as well as the cache for images
#[instrument]
async fn init_caches() {
    // Initiate Caches
    POST_CACHE
        .set(
            Cache::builder()
                .initial_capacity(100)
                .max_capacity(10000)
                .time_to_live(Duration::from_secs(60 * 30))
                .time_to_idle(Duration::from_secs(60 * 10))
                .build(),
        )
        .unwrap();
    IMAGE_CACHE
        .set(
            Cache::builder()
                .initial_capacity(50)
                .max_capacity(10000)
                .time_to_live(Duration::from_secs(60 * 60))
                .time_to_idle(Duration::from_secs(60 * 30))
                .build(),
        )
        .unwrap();
}

#[instrument]
async fn start_server() {
    // Define Routes
    let app = Router::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .route(
            "/security.txt",
            get(|| async { read_to_string("./security.txt").await.unwrap() }),
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
        .route("/about", get(get_about))
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
        .route("/feed.xml", get(serve_rss_feed))
        // include trace context as header into the response
        .layer(OtelInResponseLayer::default())
        //start OpenTelemetry trace on incoming request
        .layer(OtelAxumLayer::default())
        .fallback(handler_404);

    // run our app with hyper
    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .unwrap_or("80".to_string())
            .parse()
            .unwrap(),
    ));
    info!(
        "listening on http://{} in folder {}",
        addr,
        env::current_dir().unwrap().display()
    );
    if env::args().any(|arg| arg == "--ssg") {
        if cfg!(debug_assertions) {
            warn!("You are running the SSG in debug mode. This is not recommended.");
        }
        tokio::spawn(generate_static_site());
    }
    let concurrency_limit = ConcurrencyLimit::new(app.into_make_service(), 2500);
    let timeout = Timeout::new(concurrency_limit, std::time::Duration::from_secs(5));
    axum::Server::bind(&addr).serve(timeout).await.unwrap();
}

#[instrument]
async fn get_image(path: String) -> impl IntoResponse {
    debug!("Image `{}` requested", path);
    // Check if in cache otherwise load from disk
    if let Some(image) = IMAGE_CACHE.get().unwrap().get(&path).await {
        debug!("Image `{}` loaded from cache", path);
        return Response::builder()
            .header(
                "Content-Type",
                MimeGuess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string(),
            )
            .body(Body::from(image))
            .unwrap()
            .into_response();
    }
    if let Ok(mut file) = File::open(format!("content/posts/{path}")).await {
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)
            .await
            .expect("Could not read image");
        IMAGE_CACHE
            .get()
            .unwrap()
            .insert(path.clone(), buffer.clone())
            .await;

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
async fn get_about() -> impl IntoResponse {
    let template = liquid_parse("post.html.liquid");
    let about = Post::load("content/about".to_string()).await.unwrap();
    let sponsors: Vec<Sponsor> = vec![];
    let header = build_header(Some(about.clone().metadata)).await;
    let navbar = read_to_string("./liquid/navbar.liquid").await.unwrap();
    let footer = read_to_string("./liquid/footer.liquid").await.unwrap();
    let globals: Object = object!({
        "post": about,
        "header": header,
        "navbar": navbar,
        "footer": footer,
        "sponsors": sponsors,
    });
    let markup = template.await.render(&globals).unwrap();
    Html(markup).into_response()
}

#[instrument]
async fn get_post(Path(path): Path<String>) -> impl IntoResponse {
    // FIXME: Dumb workaround for images in posts
    if path.contains("images") {
        return get_image(path).await.into_response();
    }
    if !path.ends_with('/') {
        // Workaround for wrong image paths, breaks /about
        return Redirect::to(format!("/posts/{}/", path).as_str()).into_response();
    }
    // Remove trailing slash
    let path = path.trim_end_matches('/');
    debug!("Post `{}` requested", path);
    let loaded_post = Post::load(format!("content/posts/{}", path)).await;
    if let Ok(post) = loaded_post {
        let cloned_post = post.clone();
        let template = liquid_parse("post.html.liquid");
        let sponsors = get_sponsors().await.unwrap();
        let header = build_header(Some(post.metadata)).await;
        let navbar = read_to_string("./liquid/navbar.liquid").await.unwrap();
        let footer = read_to_string("./liquid/footer.liquid").await.unwrap();
        let globals: Object = object!({
            "post": cloned_post,
            "header": header,
            "navbar": navbar,
            "footer": footer,
            "sponsors": sponsors,
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

/// Handler for 404 Not found. Note that we include the file at compile time since it's not gonna change
#[instrument(name = "404")]
async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html::from(include_str!("../liquid/404.html")),
    )
}
