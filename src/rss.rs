use crate::post::Post;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use rss::{ChannelBuilder, Item};
use tracing::instrument;

#[instrument]
async fn build_rss_feed() -> rss::Channel {
    let posts = Post::parse_all_posts().await.unwrap();
    ChannelBuilder::default()
        .title("Nereuxofficials Blog")
        .link("https://nereux.blog")
        .description("A blog with cool duck about Rust, Linux, and other things.")
        .language(Some("en-US".into()))
        .items(
            posts
                .iter()
                .map(|x| x.clone().into())
                .collect::<Vec<Item>>(),
        )
        .build()
}

pub async fn serve_rss_feed() -> impl IntoResponse {
    let feed = build_rss_feed().await;
    let mut buffer = Vec::new();
    feed.write_to(&mut buffer).unwrap();
    (StatusCode::OK, buffer)
}
