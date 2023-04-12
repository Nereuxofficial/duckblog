use crate::utils::get_reading_time;
use color_eyre::Result;
use pulldown_cmark::{html, Parser};
use std::env;
use tokio::fs::{read_to_string, File};
use tracing::*;

// TODO: Cache Posts

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Post {
    pub content: String,
    pub path: String,
    pub metadata: PostMetadata,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PostMetadata {
    pub title: String,
    /// Data in RFC3339 format (2021-08-23T22:19:48+02:00)
    pub date: String,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub draft: Option<bool>,
    pub description: String,
    pub time_to_read: Option<usize>,
}

impl Post {
    pub async fn load(path: String) -> Result<Self> {
        if File::open(path.clone()).await.is_ok() {
            Self::parse_file(format!("{path}/index")).await
        } else {
            Self::parse_file(path).await
        }
    }
    async fn parse_file(path: String) -> Result<Self> {
        debug!("Parsing post `{}`", path);
        let file = read_to_string(format!("{path}.md")).await?;
        // Cut metadata from the markdown file and parse it
        let file_metadata = file.split("+++").nth(1).unwrap();
        let mut metadata: PostMetadata = toml::from_str(file_metadata.trim())?;
        let markdown = file.split("+++").nth(2).unwrap();
        metadata.time_to_read = Some(get_reading_time(markdown));
        metadata.date = chrono::DateTime::parse_from_rfc3339(&metadata.date)?
            .with_timezone(&chrono::Utc)
            .date_naive()
            .to_string();
        let parser = Parser::new(markdown);
        let mut html = String::new();
        html::push_html(&mut html, parser);
        Ok(Post {
            content: html,
            path: path.replace("index", "").replace("content/", ""),
            metadata,
        })
    }
    pub async fn parse_all_posts() -> Result<Vec<Self>> {
        // List all files in content/posts
        let posts = std::fs::read_dir("content/posts").unwrap();
        let mut posts_list = Vec::new();
        for post in posts {
            let post = post.unwrap();
            // Remove the .md extension
            let post_title = post.file_name().into_string().unwrap().replace(".md", "");
            posts_list.push(Post::load(format!("content/posts/{}", post_title)).await?);
        }
        if env::var("DEBUG").is_err() {
            posts_list.retain(|post| post.metadata.draft != Some(true));
        }
        posts_list.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));
        Ok(posts_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_post() {
        let post = Post::load("Lichess-Elite-Analysis".to_string())
            .await
            .unwrap();
        assert_eq!(post.metadata.title, "Lichess Elite Analysis");
        assert_eq!(post.metadata.date, "2021-09-12T21:31:55+02:00");
    }

    #[tokio::test]
    async fn test_load_post_with_folder() {
        let post = Post::load("bitboard-rust".to_string()).await.unwrap();
        assert_eq!(
            post.metadata.title,
            "Writing a BitBoard in Rust Pt. 1: The Basics"
        );
        assert_eq!(post.metadata.draft, None);
    }

    #[tokio::test]
    async fn test_load_all_posts() {
        let posts = Post::parse_all_posts().await.unwrap();
        assert!(!posts.is_empty());
    }
}
