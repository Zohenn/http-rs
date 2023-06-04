use std::fs;
use std::path::Path;
use crate::request::Request;
use crate::response::Response;
use crate::response_status_code::ResponseStatusCode;

fn get_content(request: &Request) -> std::io::Result<Vec<u8>> {
    let path = Path::new("web").join(request.url.strip_prefix("/").unwrap());

    fs::read(path)
}

pub fn serve_content(request: &Request) -> Response {
    let content = get_content(request);

    if let Ok(content_bytes) = content {
        Response::builder()
            .status_code(ResponseStatusCode::Ok)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(content_bytes)
            .get()
    } else {
        Response::error_response(ResponseStatusCode::NotFound)
    }
}