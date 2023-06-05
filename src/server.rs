use std::fs;
use crate::request::{parse_request, Request};
use crate::response::Response;
use crate::response_status_code::ResponseStatusCode;
use crate::utils::StringUtils;
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use std::path::Path;

pub struct ServerConfig {
    pub root: String,
    pub port: u32,
}

impl ServerConfig {
    pub fn default() -> Self {
        ServerConfig {
            root: String::from("web"),
            port: 80
        }
    }
}

pub struct Server {
    config: ServerConfig,
}

impl Server {
    pub fn new(config: Option<ServerConfig>) -> Self {
        Server { config: config.unwrap_or(ServerConfig::default()) }
    }

    pub fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port))?;

        for stream in listener.incoming() {
            self.handle_connection(&mut stream?)?;
        }

        Ok(())
    }

    fn handle_connection(&self, stream: &mut TcpStream) -> Result<()> {
        // let mut raw_request = String::new();
        let mut stream_buf: [u8; 255] = [0; 255];
        let mut request_bytes: Vec<u8> = Vec::new();

        loop {
            let read_result = stream.read(stream_buf.as_mut_slice());
            match read_result {
                Ok(n) => {
                    request_bytes.extend_from_slice(stream_buf.take(n as u64).into_inner());
                    // raw_request = raw_request.add(std::str::from_utf8(&stream_buf).unwrap());
                    if n < stream_buf.len() {
                        break;
                    }
                    stream_buf.fill(0);
                }
                Err(e) => panic!("{}", e),
            }
        }

        // println!("Received message:\n{raw_request}\n");

        let request = parse_request(request_bytes.as_slice());

        // println!("{request:#?}\n");

        let mut response = if let Ok(request) = request {
            self.serve_content(&request)
        } else {
            Response::error_response(None, ResponseStatusCode::BadRequest)
        };

        stream.write_all(&response.as_bytes())?;

        Ok(())
    }

    fn get_content(&self, request: &Request) -> Result<Vec<u8>> {
        let path = Path::new(&self.config.root).join(request.url.strip_prefix("/").unwrap());

        fs::read(path)
    }

    fn serve_content(&self, request: &Request) -> Response {
        let content = self.get_content(request);

        if let Ok(content_bytes) = content {
            Response::builder()
                .status_code(ResponseStatusCode::Ok)
                .header("Content-Type", "text/html; charset=utf-8")
                .body(content_bytes)
                .get()
        } else {
            Response::error_response(Some(request), ResponseStatusCode::NotFound)
        }
    }
}
