use crate::utils::{get_reading_time, liquid_parse};
use color_eyre::Result;
use pulldown_cmark::{html, Parser};
use regex::Regex;
use tokio::fs;
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
pub struct Image(String);

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
    pub images: Option<Vec<Image>>,
}

impl Default for PostMetadata {
    fn default() -> Self {
        PostMetadata {
            title: "DuckBlog".to_string(),
            date: "2021-08-23T22:19:48+02:00".to_string(),
            tags: vec!["Duck".to_string(), "Blog".to_string()],
            keywords: vec!["Duck".to_string(), "Blog".to_string()],
            draft: Some(false),
            description: "Nereuxofficial's blog about mostly Rust".to_string(),
            time_to_read: Some(1337),
            // TODO: Customize this for the main page
            images: None,
        }
    }
}

impl Post {
    #[instrument(err)]
    pub async fn load(path: String) -> Result<Self> {
        let path = path.trim_end_matches("/").to_string();
        if File::open(path.clone()).await.is_ok() {
            Self::parse_file(format!("{path}/index")).await
        } else {
            Self::parse_file(path).await
        }
    }
    #[instrument(err)]
    pub async fn parse_file(mut path: String) -> Result<Self> {
        if path.contains("//") {
            warn!("Path {} contains double slashes, this is not allowed", path);
            path = path.replace("//", "/");
        }
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
        metadata.images = Self::load_images(&path).await;
        // Before Parsing replace Cool duck sections
        let parsed_md = Self::cool_duck_replacement(markdown).await;
        let parser = Parser::new(parsed_md.as_str());
        let mut html = String::new();
        html::push_html(&mut html, parser);
        html = info_span!("Postprocessing").in_scope(|| {
            html.replace("<ul>", "<ul class=\"list-disc pl-5 pb-2\">")
                .replace("<a ", "<a class=\"text-green-500\"")
                // TODO: Add overflow-x-auto scrollable whitespace-pre-wrap to code blocks
                .replace(
                    "<code class=\"",
                    "<code class=\"whitespace-pre-wrap scrollable overflow-x-auto pb-2 ",
                )
                // Make headers bigger, add padding below
                .replace("<h1", "<h1 class=\"text-4xl font-bold pb-2\"")
                .replace("<h2", "<h2 class=\"text-3xl font-bold pb-2\"")
        });
        Ok(Post {
            // TODO: This could probably be done better
            content: html,
            path: path.replace("index", "").replace("content/", ""),
            metadata,
        })
    }
    #[instrument]
    async fn load_images(path: &str) -> Option<Vec<Image>> {
        match fs::read_dir(format!("{}/images", path.trim_end_matches("/index"))).await {
            Ok(mut images) => {
                let mut images_list = Vec::new();
                while let Ok(entry) = images.next_entry().await {
                    if let Some(entry) = entry {
                        images_list.push(Image(
                            entry
                                .path()
                                .to_str()
                                .expect("Not a valid file path, should be unicode")
                                .trim_start_matches("content/")
                                .to_string(),
                        ));
                    } else {
                        break;
                    }
                }
                Some(images_list)
            }
            Err(_) => None,
        }
    }
    #[instrument]
    async fn cool_duck_replacement(text: &str) -> String {
        // Match with regex(only linux newline because im not insane) and then parse with liquid
        let re = Regex::new(r"%Coolduck says%\s*((.|\n)*?)\s*%coolduck%").unwrap();
        let template = liquid_parse("duck.liquid").await;
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
    #[instrument(err)]
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

    #[tokio::test]
    async fn test_has_images() {
        let post = Post::load("content/posts/esp32-ws2812-dino-light".to_string())
            .await
            .unwrap();
        assert!(post.metadata.images.is_some());
        assert!(!post
            .clone()
            .metadata
            .images
            .unwrap()
            .iter()
            .any(|image| image.0.is_empty()));
        assert!(!post
            .metadata
            .clone()
            .images
            .unwrap()
            .iter()
            .any(|image| image.0.contains("//")));
        assert!(post.metadata.clone().images.unwrap().len() == 1);
        assert_eq!(
            post.metadata.clone().images.unwrap()[0].0,
            "posts/esp32-ws2812-dino-light/images/dino_light.jpg"
        );
        let posts = Post::parse_all_posts().await.unwrap();
        let mut with_images = posts.iter().filter(|post| post.metadata.images.is_some());
        // Check if we have broken paths
        assert!(!with_images.any(|post| post
            .metadata
            .images
            .iter()
            .any(|images| images.iter().any(|image| image.0.contains("//")))));
    }
}
