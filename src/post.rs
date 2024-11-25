use std::{collections::HashMap, fmt::Display};

use crate::{
    utils::{get_reading_time, liquid_parse},
    POSTS,
};
use chrono::NaiveDate;
use color_eyre::Result;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use rss::{Category, CategoryBuilder, Item as RssItem, ItemBuilder};
use tokio::fs::read_to_string;
use tracing::*;

// TODO: Work out issues with pathing
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Post {
    pub content: String,
    pub path: String,
    pub metadata: PostMetadata,
}

#[allow(clippy::from_over_into)]
impl Into<RssItem> for Post {
    fn into(self) -> RssItem {
        ItemBuilder::default()
            .title(Some(self.metadata.title))
            .link(Some(format!("https://nereux.blog/posts/{}", self.path)))
            .description(Some(self.metadata.description))
            .pub_date(Some(self.metadata.date.to_string()))
            .categories(
                self.metadata
                    .tags
                    .iter()
                    .map(|x| {
                        CategoryBuilder::default()
                            .name(x.0.clone())
                            .domain(Some(x.get_url()))
                            .build()
                    })
                    .collect::<Vec<Category>>(),
            )
            .build()
    }
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Image(String);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Tag(pub String);
impl Tag {
    pub fn from_str(s: &str) -> Self {
        Tag(s.to_string())
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.clone().fmt(f)
    }
}

impl PartialEq<String> for Tag {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl Tag {
    pub fn get_url(&self) -> String {
        format!("https://nereux.blog/tags/{}", self.0)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PostMetadata {
    pub title: String,
    /// Data in RFC3339 format (2021-08-23T22:19:48+02:00)
    pub date: NaiveDate,
    pub tags: Vec<Tag>,
    pub draft: Option<bool>,
    pub description: String,
    pub url: String,
    pub time_to_read: Option<usize>,
    pub images: Vec<Image>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PostMetadataBuilder {
    pub title: String,
    /// Data in RFC3339 format (2021-08-23T22:19:48+02:00)
    pub date: NaiveDate,
    pub tags: Vec<Tag>,
    pub draft: Option<bool>,
    pub description: String,
    pub url: String,
}

impl Default for PostMetadata {
    fn default() -> Self {
        PostMetadata {
            title: "DuckBlog".to_string(),
            date: "2021-08-23".parse::<NaiveDate>().unwrap(),
            tags: vec![Tag::from_str("Duck"), Tag::from_str("Blog")],
            draft: Some(false),
            description: "Nereuxofficial's blog about mostly Rust".to_string(),
            time_to_read: Some(1337),
            url: "/".to_string(),
            // TODO: Customize this for the main page. Maybe the image of the latest post?
            images: vec![],
        }
    }
}

impl Post {
    #[instrument(err)]
    pub async fn load(path: String) -> Result<Self> {
        Self::non_cached_load(path).await
    }

    #[instrument(err)]
    async fn non_cached_load(path: String) -> Result<Self> {
        Self::parse_file(path).await
    }

    // Extract metadata from the markdown file. Expects metadata to be in Obsidian format
    #[instrument(err)]
    async fn extract_metadata(input: &str, text: &str) -> Result<PostMetadata> {
        let metadata_builder: PostMetadataBuilder = serde_yml::from_str(input)?;
        let ttr = Some(get_reading_time(text));
        Ok(PostMetadata {
            title: metadata_builder.title,
            date: metadata_builder.date,
            tags: metadata_builder.tags,
            draft: metadata_builder.draft,
            description: metadata_builder.description,
            url: metadata_builder.url,
            time_to_read: ttr,
            images: Self::load_images(text).await,
        })
    }
    #[instrument(err)]
    pub async fn parse_file(mut path: String) -> Result<Self> {
        if path.contains("//") {
            warn!("Path {} contains double slashes, this is not allowed", path);
            path = path.replace("//", "/");
        }
        debug!("Parsing post `{}`", path);
        let file = read_to_string(path).await?;
        // Split content from metadata
        let mut content_split_iterator = file.split("---");
        let metadata_part = content_split_iterator.nth(1).unwrap();
        let text = content_split_iterator.next().unwrap();
        let metadata = Self::extract_metadata(metadata_part, text).await?;
        // Before Parsing replace Cool duck sections
        let parsed_md = Self::cool_duck_replacement(&file).await;
        let parser = Parser::new_ext(parsed_md.as_str(), Options::all());
        let mut html = String::new();
        html::push_html(&mut html, parser);
        // TODO: There has to be some nicer way. Maybe this can be done in the markdown parser
        html = info_span!("Postprocessing").in_scope(|| {
            html.replace("<ul>", "<ul class=\"list-disc pl-5 pb-2\">")
                .replace("<a ", "<a class=\"text-green-500\"")
                .replace(
                    "<code class=\"",
                    "<code class=\"whitespace-pre-wrap scrollable overflow-x-auto pb-2 ",
                )
                // Make headers bigger, add padding below
                .replace("<h1", "<h1 class=\"text-4xl font-bold pb-2\"")
                .replace("<h2", "<h2 class=\"text-3xl font-bold pb-2\"")
                // Center Images using tailwindcss
                .replace("<img src", "<img class=\"mx-auto\" src")
        });
        Ok(Post {
            // TODO: This could probably be done better
            content: html,
            path: metadata.url.clone(),
            metadata,
        })
    }
    /// Extract images out of markdown text
    #[instrument]
    async fn load_images(text: &str) -> Vec<Image> {
        let img_regex = Regex::new(r#"!\[.*?]\((.*?)\)"#).unwrap();
        img_regex
            .captures_iter(text)
            .map(|x| Image(x[1].to_string()))
            .collect()
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
                    .render(&liquid::object!({ "text": caps.get(1).unwrap().as_str(),
                    }))
                    .unwrap()
            }
        });
        result.to_string()
    }
    #[instrument(err)]
    pub async fn parse_all_posts() -> Result<Vec<Self>> {
        // List all files in content/posts
        let files = std::fs::read_dir("content/posts").unwrap_or_else(|f| {
            panic!(
                "Could not read posts directory: {} in directory {}",
                f,
                std::env::current_dir().unwrap().display()
            )
        });
        let mut posts_list = Vec::new();
        for file in files {
            let post = file.unwrap();
            posts_list.push(
                Post::load(format!(
                    "content/posts/{}",
                    post.file_name().into_string().unwrap()
                ))
                .await?,
            );
        }
        if cfg!(not(debug_assertions)) {
            posts_list.retain(|post| post.metadata.draft != Some(true));
        }
        let _ = POSTS.set(HashMap::from_iter(
            posts_list
                .iter()
                .map(|post| (post.metadata.url.clone(), post.clone())),
        ));
        posts_list.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));
        Ok(posts_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_load_post() {
        let post = Post::load(
            "content/posts/Making a Dino Light with the ESP32 and WS2812.md".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(
            post.metadata.title,
            "Making a Dino Light with the ESP32 and WS2812"
        );
        assert_eq!(
            post.metadata.date,
            NaiveDate::from_str("2022-03-05").unwrap()
        );
    }

    #[tokio::test]
    async fn test_load_all_posts() {
        let posts = Post::parse_all_posts().await.unwrap();
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn test_has_images() {
        let post = Post::load(
            "content/posts/Making a Dino Light with the ESP32 and WS2812.md".to_string(),
        )
        .await
        .unwrap();
        assert!(!post.metadata.images.is_empty());
        assert!(!post
            .clone()
            .metadata
            .images
            .iter()
            .any(|image| image.0.is_empty()));
        assert!(!post
            .metadata
            .clone()
            .images
            .iter()
            .any(|image| image.0.contains("//")));
        assert!(post.metadata.clone().images.len() == 1);
        assert_eq!(post.metadata.clone().images[0].0, "/images/dino_light.avif");
        let posts = Post::parse_all_posts().await.unwrap();
        let mut with_images = posts.iter().filter(|post| !post.metadata.images.is_empty());
        // Check if we have broken paths
        assert!(!with_images.any(|post| post
            .metadata
            .images
            .iter()
            .any(|image| image.0.contains("//"))));
    }

    #[tokio::test]
    async fn test_serialize_post() {
        let post = Post::load(
            "content/posts/A getting started guide to ESP32 no-std Rust development.md".to_string(),
        )
        .await
        .unwrap();
        let serialized = serde_json::to_string(&post).unwrap();
        println!("{}", serialized);
    }
}
