mod post;
mod utils;

use crate::post::Post;
use crate::utils::{build_header, liquid_parse, static_file_handler};
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{routing::get, Router};
use color_eyre::Result;
use liquid::{object, Object};
use new_mime_guess::MimeGuess;
use std::fs::read_to_string;
use std::net::SocketAddr;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::*;
// TODO: Tables don't get processed properly
// TODO: Estimated reading time
// TODO: Make logo larger and remove Home Text
// TODO: Time formatting
// TODO: Feedback button in navbar with link to github issues
#[tokio::main]
async fn main() -> Result<()> {
    // initialize color_eyre
    color_eyre::install()?;

    // initialize tracing
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/posts/*path", get(get_post))
        .route("/posts", get(list_posts))
        .route("/", get(list_posts))
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
        .fallback(handler_404);

    // run our app with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 8010));
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

async fn get_post(Path(path): Path<String>) -> impl IntoResponse {
    // Dumb workaround for images in posts
    if path.contains("images") {
        return get_image(path).await.into_response();
    }
    debug!("Post `{}` requested", path);
    let loaded_post = Post::load(format!("content/posts/{}", path)).await;
    if let Ok(post) = loaded_post {
        let template = liquid_parse("post.html.liquid");
        let title = post.metadata.title.clone();
        let description = post.metadata.description.clone();
        let content = post.content.clone();
        let header = build_header(Some(post.metadata));
        let navbar = read_to_string("src/navbar.liquid").unwrap();
        let footer = read_to_string("src/footer.liquid").unwrap();
        let globals: Object = object!({
            "title": title,
            "description": description,
            "content": content,
            "header": header,
            "navbar": navbar,
            "footer": footer,
        });
        let markup = template.render(&globals).unwrap();
        Html(markup).into_response()
    } else {
        debug!("Post not found because: {:#?}", loaded_post);
        handler_404().await.into_response()
    }
}
async fn list_posts() -> impl IntoResponse {
    info!("Listing posts");
    let posts = Post::parse_all_posts().await.unwrap();
    let navbar = read_to_string("src/navbar.liquid").unwrap();
    let footer = read_to_string("src/footer.liquid").unwrap();
    let template = liquid_parse("index.html.liquid");
    let globals: Object = object!({ "posts": posts,
            "navbar": navbar,
            "footer": footer });
    let markup = template.render(&globals).unwrap();
    Html(markup).into_response()
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
