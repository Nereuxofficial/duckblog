use crate::post::PostMetadata;
use axum::body::{boxed, Body, BoxBody};
use axum::http::{Request, Response, StatusCode, Uri};
use liquid::{object, Template};
use tokio::fs::read_to_string;
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::{debug, info_span, instrument};
#[instrument]
pub(crate) async fn build_header(post: Option<PostMetadata>) -> String {
    let template = liquid_parse("header.liquid").await;
    let metadata = {
        match post {
            None => PostMetadata::default(),
            Some(m) => m,
        }
    };
    if metadata.images.is_some() {
        debug!("Images: {:#?}", metadata.images);
    }
    let globals = object!({ "metadata": metadata });
    template.render(&globals).unwrap()
}
#[instrument(skip(file))]
pub(crate) async fn liquid_parse(file: impl ToString) -> Template {
    let compiler = liquid::ParserBuilder::with_stdlib()
        .build()
        .expect("Could not build liquid compiler");
    let file = info_span!("Reading file")
        .in_scope(|| async move {
            read_to_string(format!("liquid/{}", file.to_string()))
                .await
                .unwrap()
        })
        .await;
    compiler.parse(&file).unwrap()
}
#[instrument(err(Debug))]
pub(crate) async fn static_file_handler(
    uri: Uri,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    debug!("Requested {}", uri);
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    match ServeDir::new("static").oneshot(req).await {
        Ok(response) => Ok(response.map(boxed)),
        Err(_) => Err((StatusCode::NOT_FOUND, "File not found".to_string())),
    }
}

#[instrument]
pub(crate) fn get_reading_time(text: &str) -> usize {
    // We estimate with about 200 WPM and round up.
    let words = text.split_whitespace().count();
    (words as f64 / 200.0).ceil() as usize
}
