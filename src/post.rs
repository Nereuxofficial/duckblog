use crate::utils::{get_reading_time, liquid_parse};
use color_eyre::Result;
use pulldown_cmark::{html, Parser};
use regex::Regex;
use tokio::fs::{read_to_string, File};
use tracing::*;

// TODO: Cache Posts
// TODO: Work out issues with pathing
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
    #[instrument]
    pub async fn load(path: String) -> Result<Self> {
        if File::open(path.clone()).await.is_ok() {
            Self::parse_file(format!("{path}/index")).await
        } else {
            Self::parse_file(path).await
        }
    }
    #[instrument]
    pub async fn parse_file(path: String) -> Result<Self> {
        debug!("Parsing post `{}`", path);
        let file = read_to_string(format!("{path}.md")).await?;
        // Cut metadata from the markdown file and parse it
        let file_metadata = file.split("+++").nth(1).unwrap();
        let mut metadata: PostMetadata = toml::from_str(file_metadata.trim())?;
        let mut markdown = file.split("+++").nth(2).unwrap();
        metadata.time_to_read = Some(get_reading_time(markdown));
        metadata.date = chrono::DateTime::parse_from_rfc3339(&metadata.date)?
            .with_timezone(&chrono::Utc)
            .date_naive()
            .to_string();
        // Before Parsing replace Cool duck sections
        let parsed_md = Self::cool_duck_replacement(markdown);
        markdown = parsed_md.as_str();
        let parser = Parser::new(markdown);
        let mut html = String::new();
        html::push_html(&mut html, parser);
        Ok(Post {
            content: html,
            path: path.replace("index", "").replace("content/", ""),
            metadata,
        })
    }
    #[instrument]
    fn cool_duck_replacement(text: &str) -> String {
        // Match with regex and then parse with liquid
        let re = Regex::new(r"%Coolduck says%\s*(.*?)\s*%coolduck%").unwrap();
        let template = liquid_parse("duck.liquid");
        let result = re.replace_all(text, {
            // Render template with $1
            |caps: &regex::Captures| {
                template
                    .render(&liquid::object!({ "text": caps.get(1).unwrap().as_str() }))
                    .unwrap()
            }
        });
        result.to_string()
    }
    #[instrument]
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
        if cfg!(not(debug_assertions)) {
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
        let post = Post::load("content/posts/Lichess-Elite-Analysis".to_string())
            .await
            .unwrap();
        assert_eq!(post.metadata.title, "Lichess Elite Analysis");
        assert_eq!(post.metadata.date, "2021-09-12");
    }

    #[tokio::test]
    async fn test_load_post_with_folder() {
        let post = Post::load("content/posts/bitboard-rust".to_string())
            .await
            .unwrap();
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
