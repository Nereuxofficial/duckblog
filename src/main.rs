mod post;
mod rss;
mod sponsors;
mod ssg;
mod utils;

use crate::post::Post;
use crate::rss::serve_rss_feed;
use crate::sponsors::{get_sponsors, noncached_get_sponsors, Sponsor};
use crate::ssg::generate_static_site;
use crate::utils::{build_header, liquid_parse};
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{routing::get, Router};
use itertools::Itertools;
use liquid::{object, Object};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::fs::read_to_string;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing::*;

pub static POSTS: OnceLock<HashMap<String, Post>> = OnceLock::new();
pub static SPONSORS: OnceLock<Arc<RwLock<Vec<Sponsor>>>> = OnceLock::new();

// TODO: Think about blue/green deployment
// TODO: Wrapping Code blocks
// TODO: Create sitemap.xml
// TODO: add tower-livereload
fn main() -> Result<(), Box<dyn Error>> {
    // Read .env
    dotenvy::dotenv().ok();
    // According to the sentry docs this should be started before the runtime is started
    let _guard = sentry::init((
        env::var("SENTRY_DSN")?,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));
    tracing_subscriber::fmt().init();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // Load Sponsors
            SPONSORS
                .set(Arc::new(RwLock::new(
                    noncached_get_sponsors().await.unwrap_or_default(),
                )))
                .unwrap();
            let _ = Post::parse_all_posts().await.unwrap();
            // Spawn a task to refresh them
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(500)).await;
                    debug!("Refreshing Sponsors");
                    let new_sponsors = noncached_get_sponsors().await;
                    if let Ok(sponsors) = new_sponsors {
                        let mut sponsor_lock = SPONSORS.get().unwrap().write().await;
                        sponsor_lock.clear();
                        sponsor_lock.extend(sponsors);
                    }
                }
            });
            start_server().await;
        });
    Ok(())
}

#[instrument]
async fn start_server() {
    // Define Routes
    let app = Router::new()
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
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/images", ServeDir::new("content/images"))
        .route(
            "/favicon.ico",
            get(|| async {
                Response::builder()
                    .header("Content-Type", "image/x-icon")
                    .body(Body::from(include_bytes!("../static/favicon.ico").to_vec()))
                    .unwrap()
            }),
        )
        .route("/feed.xml", get(serve_rss_feed))
        .fallback(handler_404);

    // run our app with hyper
    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .unwrap_or("8000".to_string())
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
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[instrument]
async fn get_about() -> impl IntoResponse {
    let template = liquid_parse("post.html.liquid");
    let about = Post::load("content/about.md".to_string()).await.unwrap();
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
    if !path.ends_with('/') {
        // Workaround for wrong image paths, breaks /about
        return Redirect::to(format!("/posts/{}/", path).as_str()).into_response();
    }
    // Remove trailing slash
    let path = format!("/posts/{}", path.trim_end_matches('/'));
    debug!("Post `{}` requested", path);
    let posts_map = POSTS.get().unwrap();
    let loaded_post = posts_map.get(&path);
    if let Some(post) = loaded_post {
        let template = liquid_parse("post.html.liquid");
        let sponsors = get_sponsors().await.unwrap();
        let header = build_header(Some(post.metadata.clone())).await;
        let navbar = read_to_string("./liquid/navbar.liquid").await.unwrap();
        let footer = read_to_string("./liquid/footer.liquid").await.unwrap();
        let globals: Object = object!({
            "post": post,
            "header": header,
            "navbar": navbar,
            "footer": footer,
            "sponsors": sponsors,
        });
        let markup = template.await.render(&globals).unwrap();
        Html(markup).into_response()
    } else {
        debug!("Post not found because: {:#?}", loaded_post);
        handler_404(path).await.into_response()
    }
}
#[instrument]
async fn list_posts(Path(path): Path<String>) -> impl IntoResponse {
    info!("Listing posts with filter: {:#?}", path);
    let mut posts = POSTS
        .get()
        .unwrap()
        .values()
        .sorted_by_key(|p| p.metadata.date)
        .cloned()
        .collect::<Vec<Post>>();
    posts.reverse();
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
async fn handler_404(path: String) -> impl IntoResponse {
    error!("Path not found: {path}");
    (
        StatusCode::NOT_FOUND,
        Html::from(include_str!("../liquid/404.html")),
    )
}
