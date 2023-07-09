use http_rs::http_version::HttpVersion;
use http_rs::request::Request;
use http_rs::request_method::RequestMethod;
use http_rs::response::Response;
use http_rs::response_status_code::ResponseStatusCode;
use http_rs::server::*;
use http_rs::server_config::*;
use std::collections::HashMap;
use std::io::{Read, Result, Write};
use std::net::TcpStream;

/**
  - tests:
    - 400 on incomplete request
    - 408 on timeout
*/

static DEFAULT_RESPONSE: &str = "Ok";

fn setup() -> Server {
    let config = ServerConfig {
        keep_alive: KeepAliveConfig::Off,
        ..Default::default()
    };

    let server = Server::new(Some(config));

    server.listener(|request| {
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
    std::thread::spawn(|| {
        let mut server = setup();

        server.run().expect("Server runs");
    });

    test();
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

fn issue_request(request_bytes: &[u8]) -> Result<Response> {
    let mut tcp = TcpStream::connect("127.0.0.1:80")?;

    tcp.write_all(request_bytes)?;

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
    issue_request(&request.as_bytes())
}

fn issue_str_request(str_request: &str) -> Result<Response> {
    issue_request(str_request.as_bytes())
}

#[test]
fn get_request() {
    run_test(|| {
        let request = default_get("/");

        let response = issue_req_request(&request).unwrap();

        assert_eq!(response.body(), "Ok".as_bytes());
    });
}

#[test]
fn post_request() {
    run_test(|| {
        let body = vec![1, 2, 3];
        let request = default_post("/", &body);

        let response = issue_req_request(&request).unwrap();

        assert_eq!(response.body(), &body);
    });
}
