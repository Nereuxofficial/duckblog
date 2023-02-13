mod post;

use crate::post::Post;
use axum::body::{Body, BoxBody};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{routing::get, Router};

use color_eyre::Result;
use maud::html;
use new_mime_guess::MimeGuess;
use std::net::SocketAddr;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::*;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    // initialize tracing
    tracing_subscriber::fmt::init();
    // TODO: Separate Router for posts
    // build our application with a route
    let app = Router::new()
        .route("/posts/*path", get(get_post))
        .route("/posts/", get(list_posts))
        .fallback(handler_404);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    debug!("listening on {}", addr);
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

async fn list_posts() -> impl IntoResponse {
    "Nope not yet"
}

async fn get_post(Path(path): Path<String>) -> impl IntoResponse {
    if path.contains("images") {
        return get_image(path).await.into_response();
    }
    debug!("Post `{}` requested", path);
    if let Ok(post) = Post::load(path).await {
        Html(format!(
            "{}{}",
            html! {
                h1 { (post.metadata.title)}
                h3 { (post.metadata.description)}
            }
            .into_string(),
            post.content
        ))
        .into_response()
    } else {
        handler_404().await.into_response()
    }
}
async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
