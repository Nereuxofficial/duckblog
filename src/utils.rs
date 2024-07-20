use crate::post::PostMetadata;
use liquid::{object, Template};
use tokio::fs::read_to_string;
use tracing::{debug, info_span, instrument};
#[instrument]
pub(crate) async fn build_header(post: Option<PostMetadata>) -> String {
    let template = liquid_parse("header.liquid").await;
    let metadata = post.unwrap_or_default();
    if !metadata.images.is_empty() {
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

#[instrument]
pub(crate) fn get_reading_time(text: &str) -> usize {
    // We estimate with about 200 WPM and round up.
    let words = text.split_whitespace().count();
    (words as f64 / 200.0).ceil() as usize
}
