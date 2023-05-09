use crate::post::PostMetadata;
use axum::body::{boxed, Body, BoxBody};
use axum::http::{Request, Response, StatusCode, Uri};
use liquid::{object, Template};
use std::fs::read_to_string;
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::{debug, instrument};
#[instrument]
pub(crate) fn build_header(post: Option<PostMetadata>) -> String {
    let template = liquid_parse("header.liquid");
    let title = post
        .clone()
        .map(|p| p.title)
        .unwrap_or(String::from("Nereuxofficials Blog"));
    let description = post
        .map(|p| p.description)
        .unwrap_or(String::from("TODO: Description"));
    let extra_tags = String::new();
    let globals = object!({
        "title": title,
        "description": description,
        "extra_tags": extra_tags,
    });
    template.render(&globals).unwrap()
}
#[instrument(skip(file))]
pub(crate) fn liquid_parse(file: impl ToString) -> Template {
    let compiler = liquid::ParserBuilder::with_stdlib()
        .build()
        .expect("Could not build liquid compiler");
    compiler
        .parse(&read_to_string(format!("src/{}", file.to_string())).unwrap())
        .unwrap()
}
#[instrument]
pub(crate) async fn static_file_handler(
    uri: Uri,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    debug!("Requested {}", uri);
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    match ServeDir::new("static").oneshot(req).await {
        Ok(response) => Ok(response.map(boxed)),
        Err(_) => Err((StatusCode::NOT_FOUND, format!("File not found"))),
    }
}

pub(crate) fn get_reading_time(text: &str) -> usize {
    // We estimate with about 200 WPM and round up.
    let words = text.split_whitespace().count();
    (words as f64 / 200.0).ceil() as usize
}
