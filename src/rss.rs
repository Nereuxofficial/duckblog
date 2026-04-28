use crate::post::Post;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use rss::{ChannelBuilder, Item};
use tracing::instrument;

#[instrument]
async fn build_rss_feed() -> rss::Channel {
    let posts = Post::parse_all_posts().await.unwrap();
    ChannelBuilder::default()
        .title("Nereuxofficials Blog")
        .link("https://nereux.blog")
        .description("A blog about Rust, Linux, and other things.")
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
    feed.pretty_write_to(&mut buffer, b' ', 2).unwrap();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        buffer,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_rss_feed() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let posts = Post::parse_all_posts().await.unwrap();
        let feed = build_rss_feed().await;

        assert!(!feed.items().is_empty());
        if cfg!(not(debug_assertions)) {
            let non_draft_posts = posts.iter().filter(|p| !p.metadata.draft).count();
            assert_eq!(feed.items().len(), non_draft_posts);
        }

        for item in feed.items() {
            assert!(item.title().is_some());
            assert!(item.link().is_some());
            if let Some(link) = item.link() {
                assert!(link.starts_with("https://nereux.blog/posts/"));
                assert!(!link.contains("//posts/"));
            }
        }

        let app = Router::new().route("/feed.xml", get(serve_rss_feed));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/feed.xml", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            "application/rss+xml; charset=utf-8"
        );

        let body = response.text().await.unwrap();
        dbg!(&body);
        assert!(body.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(body.contains("<channel>"));
        assert!(body.contains("<title>Nereuxofficials Blog</title>"));
        assert!(body.contains("<link>https://nereux.blog</link>"));
        assert!(!body.contains("/posts//posts/"));
        assert!(body.contains("<pubDate>"));
        assert!(body.contains(", "));
        assert!(body.contains(" +0000</pubDate>"));
    }
}
