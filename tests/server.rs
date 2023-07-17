use crate::utils::panic_after;
use http_rs::http_version::HttpVersion;
use http_rs::request::Request;
use http_rs::request_method::RequestMethod;
use http_rs::response::Response;
use http_rs::response_status_code::ResponseStatusCode;
use http_rs::server::*;
use http_rs::server_config::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Result, Write};
use std::net::TcpStream;
use std::sync::Arc;

mod utils;

/**
  - tests:
    - 400 on incomplete request
    - 408 on timeout
*/

static DEFAULT_RESPONSE: &str = "Ok";

fn default_server_config() -> ServerConfig {
    ServerConfig {
        root: "test_files".to_string(),
        keep_alive: KeepAliveConfig::On {
            timeout: 1,
            max_requests: 1,
            include_header: true,
        },
        ..Default::default()
    }
}

fn setup(config: Option<ServerConfig>) -> Server {
    let config = config.unwrap_or(default_server_config());

    let server = Server::new(Some(config));

    server.listener(|request| {
        if request.url != "/" {
            return None;
        }

        let mut response_builder = Response::builder().status_code(ResponseStatusCode::Ok);

        if request.method == RequestMethod::Get {
            response_builder = response_builder.text_body(DEFAULT_RESPONSE);
        } else {
            response_builder = response_builder.body(request.body.clone());
        }

        Some(response_builder.get())
    })
}

fn run_test(test: impl Fn()) {
    let handle = std::thread::spawn(|| {
        let mut server = setup(None);

        server.run(Arc::new(true)).expect("Server runs");
    });

    test();

    handle.join().unwrap();
}

fn run_test_with_config(config: ServerConfig, test: impl Fn()) {
    let handle = std::thread::spawn(|| {
        let mut server = setup(Some(config));

        server.run(Arc::new(true)).expect("Server runs");
    });

    test();

    handle.join().unwrap();
}

fn default_get(url: &str) -> Request {
    Request {
        method: RequestMethod::Get,
        url: url.to_string(),
        version: HttpVersion::Http1_1,
        headers: Default::default(),
        body: vec![],
    }
}

fn default_post(url: &str, body: &[u8]) -> Request {
    let mut headers: HashMap<String, String> = HashMap::new();

    headers.insert("Content-Length".to_string(), body.len().to_string());

    Request {
        method: RequestMethod::Post,
        url: url.to_string(),
        version: HttpVersion::Http1_1,
        headers,
        body: Vec::from(body),
    }
}

fn issue_request(
    request_bytes_segments: &[&[u8]],
    segment_sleep_time: std::time::Duration,
) -> Result<Response> {
    let mut tcp = TcpStream::connect("127.0.0.1:80")?;

    for segment in request_bytes_segments {
        tcp.write_all(segment)?;
        std::thread::sleep(segment_sleep_time);
    }

    let mut response_bytes: Vec<u8> = vec![];
    tcp.read_to_end(&mut response_bytes)?;

    let response_str = std::str::from_utf8(&response_bytes).unwrap();

    let mut status_code: Option<ResponseStatusCode> = None;
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut body: Vec<u8> = vec![];

    let mut empty_line_found = false;

    for (index, line) in response_str.split("\r\n").enumerate() {
        if line.is_empty() && !empty_line_found {
            empty_line_found = true;
            continue;
        }

        if index == 0 {
            let parts = line.split(' ').collect::<Vec<&str>>();
            let code_int = parts[1].parse::<u16>().unwrap();
            status_code = Some(unsafe { std::mem::transmute(code_int) });
        } else if empty_line_found {
            body.extend_from_slice(line.as_bytes());
        } else {
            let parts = line.split(": ").collect::<Vec<&str>>();
            headers.insert(parts[0].to_string(), parts[1].to_string());
        }
    }

    let mut response_builder = Response::builder()
        .status_code(status_code.unwrap())
        .body(body);

    for (name, value) in headers {
        response_builder = response_builder.header(&name, &value);
    }

    Ok(response_builder.get())
}

fn issue_req_request(request: &Request) -> Result<Response> {
    issue_request(&[&request.as_bytes()], std::time::Duration::from_secs(0))
}

fn issue_str_request(str_request: &str) -> Result<Response> {
    issue_request(&[str_request.as_bytes()], std::time::Duration::from_secs(0))
}

fn issue_segmented_str_request(segments: &[&str]) -> Result<Response> {
    let byte_segments = segments
        .iter()
        .map(|v| v.as_bytes())
        .collect::<Vec<&[u8]>>();
    issue_request(&byte_segments, std::time::Duration::from_millis(50))
}

#[test]
fn get_request() {
    run_test(|| {
        let request = default_get("/");

        let response = issue_req_request(&request).unwrap();

        assert_eq!(response.body(), "Ok".as_bytes());
        assert_eq!(
            response.headers().get("Connection"),
            Some(&"close".to_string())
        );
    });
}

#[test]
fn get_request_for_content() {
    run_test(|| {
        let request = default_get("/file.txt");

        let response = issue_req_request(&request).unwrap();

        let mut file = File::open("test_files/file.txt").unwrap();
        let mut file_contents: Vec<u8> = vec![];
        file.read_to_end(&mut file_contents).unwrap();

        assert_eq!(response.body(), &file_contents);
        assert_eq!(
            response.headers().get("Content-Length"),
            Some(&file_contents.len().to_string())
        );
    });
}

#[test]
fn post_request() {
    run_test(|| {
        let body = vec![1, 2, 3];
        let request = default_post("/", &body);

        let response = issue_req_request(&request).unwrap();

        assert_eq!(response.body(), &body);
        assert_eq!(
            response.headers().get("Content-Length"),
            Some(&body.len().to_string())
        );
    });
}

#[test]
fn malformed_request_400() {
    run_test(|| {
        let request = "GET / HTTP/1.1Host: localhost\r\n\r\n";

        let response = issue_str_request(request).unwrap();

        assert_eq!(response.status_code(), &ResponseStatusCode::BadRequest);
        assert!(response.body().is_empty());
    });
}

#[test]
fn incomplete_request_timeout_408() {
    let closure = || {
        panic_after(std::time::Duration::from_millis(1200), || {
            let request = "GET / HTTP/1.1";

            let response = issue_str_request(request).unwrap();

            assert_eq!(response.status_code(), &ResponseStatusCode::RequestTimeout);
            assert!(response.body().is_empty());
        });
    };

    let mut config = default_server_config();
    config.keep_alive = KeepAliveConfig::Off;
    config.timeout = 1;

    run_test(closure);
    run_test_with_config(config, closure);
}

#[test]
fn handles_transfer_encoding_chunked() {
    run_test(|| {
        let request = "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n123\r\n5\r\n45678\r\n0\r\n\r\n";

        let response = issue_str_request(request).unwrap();

        assert_eq!(response.status_code(), &ResponseStatusCode::Ok);
        assert_eq!(std::str::from_utf8(response.body()).unwrap(), "12345678");
    });
}

#[test]
fn handles_segmented_transfer_encoding_chunked() {
    run_test(|| {
        let segments = [
            "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n123\r\n",
            "5\r\n45678\r\n",
            "1\r\n9\r\n",
            "0\r\n\r\n",
        ];

        let response = issue_segmented_str_request(&segments).unwrap();

        assert_eq!(response.status_code(), &ResponseStatusCode::Ok);
        assert_eq!(std::str::from_utf8(response.body()).unwrap(), "123456789");
    });
}
