//! Since we didn't do it in the first place we need to build a static site from the running server
use crate::post::Post;
use axum::http::Uri;
use copy_dir::copy_dir;
use itertools::Itertools;
use std::process::exit;
use std::str::FromStr;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};
// TODO: Fix paths
// Also: Fuck paths
const SERVER_URL: &str = "127.0.0.1:8010";
const FOLDER: &str = "public/";
pub async fn generate_static_site() {
    // Delete previous files
    let _ = fs::remove_dir_all(FOLDER).await;
    // Create our folders
    let _ = fs::create_dir("public/").await;
    let _ = fs::create_dir("public/tags").await;
    let _ = fs::create_dir("public/posts").await;
    // Wait until the site is up
    while reqwest::get(format!("http://{}/", SERVER_URL))
        .await
        .is_err()
    {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
    generate_posts().await;
    copy_static_files().await;
    generate_404().await;
    generate_tags().await;
    save_page_to_path(Uri::from_str(format!("http://{}/index.html", SERVER_URL).as_str()).unwrap())
        .await;
    save_page_to_path(Uri::from_str(format!("http://{}/about", SERVER_URL).as_str()).unwrap())
        .await;
    save_page_to_path(Uri::from_str(format!("http://{}/posts", SERVER_URL).as_str()).unwrap())
        .await;
    // Rss feed
    save_page_to_path(Uri::from_str(format!("http://{}/feed.xml", SERVER_URL).as_str()).unwrap())
        .await;
    copy_post_images().await;
    info!("Static site generated");
    exit(0);
}
async fn copy_post_images() {
    info!("Copying post images");
    let posts = Post::parse_all_posts().await.unwrap();
    for post in posts {
        if Post::parse_file(format!("content/{}/index", post.path))
            .await
            .is_ok()
        {
            let src = format!("content/{}images", post.path);
            let dest = format!("{}/{}/images", FOLDER, post.path);
            let res = copy_dir(&src, &dest);
            if res.is_ok() {
                info!("Copied images from {} to {}", &src, &dest);
            } else {
                warn!("Couldn't copy images from {} to {}", src, dest);
            }
        }
    }
}
async fn generate_posts() {
    // Wait until the site is up
    while reqwest::get(format!("http://{}/", SERVER_URL))
        .await
        .is_err()
    {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    let posts = Post::parse_all_posts().await.unwrap();
    debug!(
        "Found posts: {}",
        posts.iter().map(|x| x.path.clone()).join(", ")
    );
    for post in posts {
        let uri = format!("http://{}{}", SERVER_URL, post.path);
        save_page_to_path(Uri::from_str(uri.as_str()).unwrap()).await;
    }
}
async fn generate_tags() {
    let posts = Post::parse_all_posts().await.unwrap();
    let mut tags: Vec<String> = Vec::new();
    posts
        .iter()
        .map(|post| post.metadata.tags.clone())
        .for_each(|tag| tags.extend(tag.iter().map(|x| x.to_string()).collect::<Vec<String>>()));
    for tag in tags.iter().unique() {
        let uri = format!("http://{}/tags/{}", SERVER_URL, tag);
        save_page_to_path(Uri::from_str(uri.as_str()).unwrap()).await;
    }
}
/// Save 404 page
async fn generate_404() {
    let uri = format!("http://{}/404", SERVER_URL);
    save_page_to_path(Uri::from_str(uri.as_str()).unwrap()).await;
}
/// Saves the html under the URL to the path in the URL
async fn save_page_to_path(uri: Uri) {
    // The url path with the first / removed
    let mut path = uri.path();
    if path.starts_with("/") {
        path = &path[1..path.len()];
    }
    let big_path = format!("{}{}", path, "index");
    if path.ends_with("/") {
        fs::create_dir_all(format!("{}/{}", FOLDER, path))
            .await
            .expect("Could not create dir");
        path = big_path.as_str();
    }
    let mut response = reqwest::get(uri.to_string().trim_end_matches('/'))
        .await
        .unwrap();
    if path.ends_with(".html") {
        path = &path[..path.len() - 5];
    }
    let mut file = tokio::fs::File::create(format!("{}{}.html", FOLDER, path))
        .await
        .unwrap();
    while let Some(chunk) = response.chunk().await.unwrap() {
        file.write_all(&chunk).await.unwrap();
    }
}
/// Copy our static resources
async fn copy_static_files() {
    copy_dir("static", format!("{}static", FOLDER)).expect("Copying files failed");
}
