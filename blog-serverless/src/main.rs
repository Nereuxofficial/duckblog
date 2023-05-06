use include_dir::{include_dir, Dir};
use tide::http::{Mime, Url};
use tide::log::info;
use tide::{Request, Response};

const PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../public");
#[shuttle_runtime::main]
async fn tide() -> shuttle_tide::ShuttleTide<()> {
    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());
    app.at("/*").get(serve_include_dir);
    app.at("/posts").get(|_| async move {
        let mut response = Response::new(200);
        response.set_body(
            PUBLIC_DIR
                .get_file("index.html")
                .unwrap()
                .contents()
                .to_vec(),
        );
        response.set_content_type(Mime::from("text/html"));
        Ok(response)
    });
    app.at("/").get(|_| async move {
        let mut response = Response::new(200);
        response.set_body(
            PUBLIC_DIR
                .get_file("index.html")
                .unwrap()
                .contents()
                .to_vec(),
        );
        response.set_content_type(Mime::from("text/html"));
        Ok(response)
    });

    Ok(app.into())
}

async fn serve_include_dir(req: Request<()>) -> tide::Result {
    // TODO: This could really be cleaner and more efficient
    info!("Request: {}", req.url());
    let path = req.url().path();
    let path = if path.ends_with('/') {
        format!("{}index.html", path)
    } else {
        path.to_string()
    };
    let path = path.trim_start_matches('/').trim_end_matches('/');
    info!("Looking for file: {}", path);
    let file = PUBLIC_DIR.get_file(path);
    if let Some(file) = file {
        info!("Found file: {}", file.path().display());
        let mut response = Response::new(200);
        response.set_body(file.contents().to_vec());
        let mime_type =
            new_mime_guess::from_ext(file.path().extension().unwrap().to_str().unwrap())
                .first_or_text_plain();
        response.set_content_type(Mime::from(mime_type.essence_str()));
        Ok(response)
    } else {
        match PUBLIC_DIR.get_file(format!("{}.html", path)) {
            Some(file) => {
                info!("Found file: {}", file.path().display());
                let mut response = Response::new(200);
                response.set_body(file.contents().to_vec());
                response.set_content_type(Mime::from("text/html"));
                Ok(response)
            }
            None => {
                let mut response = Response::new(404);
                response.set_body(PUBLIC_DIR.get_file("404.html").unwrap().contents().to_vec());
                response.set_content_type(Mime::from("text/html"));
                Ok(response)
            }
        }
    }
}
