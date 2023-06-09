use crate::connection::Connection;
use crate::request::{parse_request, Request};
use crate::response::{Response, ResponseBuilder};
use crate::response_status_code::ResponseStatusCode;
use crate::server_config::ServerConfig;
use crate::utils::StringUtils;
use std::fs;
use std::io::Result;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

type RequestListener = dyn Fn(&Request) -> Option<Response> + Send + Sync;

#[derive(Clone)]
pub struct Server {
    config: Arc<ServerConfig>,
    https_config: Option<Arc<rustls::ServerConfig>>,
    listener: Option<Arc<RequestListener>>,
}

impl Server {
    pub fn new(config: Option<ServerConfig>) -> Self {
        Server {
            config: Arc::new(config.unwrap_or(ServerConfig::default())),
            https_config: None,
            listener: None,
        }
    }

    fn init_https(&mut self) {
        if !self.config.https {
            return;
        }

        let certs = self.config.load_certs();
        let key = self.config.load_key();

        if certs.is_empty() || key.is_none() {
            return;
        }

        self.https_config = Some(Arc::new(
            rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(certs, key.unwrap())
                .unwrap(),
        ));
    }

    pub fn listener(mut self, listener: impl Fn(&Request) -> Option<Response> + 'static + Send + Sync) -> Self {
        self.listener = Some(Arc::new(listener));

        self
    }

    pub fn run(&mut self) -> Result<()> {
        self.init_https();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port))?;

        for stream in listener.incoming() {
            let mut cloned_server = self.clone();
            std::thread::spawn(move || {
                match cloned_server.handle_connection(&mut stream.unwrap()) {
                    Ok(_) => println!("Connection closed"),
                    Err(err) => println!("Connection error: {err:?}"),
                }
            });
        }

        Ok(())
    }

    fn handle_connection(&mut self, stream: &mut TcpStream) -> Result<()> {
        let mut connection = Connection::new(stream, self.https_config.clone());

        loop {
            let request_bytes = match connection.read() {
                Ok(None) => {
                    println!("Got none bytes");
                    return Ok(())
                },
                Ok(Some(bytes)) if bytes.is_empty() => {
                    println!("Got empty message (TCP FIN, probably)");
                    return Ok(())
                },
                Ok(Some(bytes)) => bytes,
                Err(err) => return Err(err),
            };

            let request = parse_request(request_bytes.as_slice());

            let mut response = if let Ok(request) = request {
                self.serve_content(&request)
            } else {
                self.error_response(None, ResponseStatusCode::BadRequest)
            };

            connection.write(&response.as_bytes())?;
        }
    }

    fn get_content(&self, request: &Request) -> Result<Vec<u8>> {
        let path = Path::new(&self.config.root).join(request.url.strip_prefix('/').unwrap());

        fs::read(path)
    }

    fn serve_content(&self, request: &Request) -> Response {
        let content = self.get_content(request);

        if let Ok(content_bytes) = content {
            return Response::builder()
                .status_code(ResponseStatusCode::Ok)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Content-Length", &content_bytes.len().to_string())
                .body(content_bytes)
                .get();
        }

        if let Some(listener) = &self.listener {
            if let Some(response) = listener(request) {
                return response;
            }
        }

        self.error_response(Some(request), ResponseStatusCode::NotFound)
    }

    fn error_response(
        &self,
        request: Option<&Request>,
        status_code: ResponseStatusCode,
    ) -> Response {
        let mut response_builder = ResponseBuilder::new().status_code(status_code);

        let accepts_html = {
            if let Some(request) = request {
                let accept_header = request.headers.get("Accept");
                accept_header.is_some() && accept_header.unwrap().contains("text/html")
            } else {
                false
            }
        };

        if accepts_html {
            response_builder = response_builder
                .header("Content-Type", "text/html; charset=utf-8")
                .body(
                    format!(
                        "<html><body><h1 style='text-align: center'>{} {}</h1></body></html>",
                        status_code as u16, status_code
                    )
                    .as_bytes_vec(),
                )
        }

        response_builder.get()
    }
}
