use crate::connection::{Connection, ReadUntil};
use crate::request::{parse_request, Request};
use crate::request_method::RequestMethod;
use crate::response::{Response, ResponseBuilder};
use crate::response_status_code::ResponseStatusCode;
use crate::server_config::{KeepAliveConfig, ServerConfig};
use log::{debug, info};
use std::fs;
use std::io::{ErrorKind, Result};
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
            // todo: either panic or log error here
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

    pub fn listener(
        mut self,
        listener: impl Fn(&Request) -> Option<Response> + Send + Sync + 'static,
    ) -> Self {
        self.listener = Some(Arc::new(listener));

        self
    }

    pub fn run(&mut self) -> Result<()> {
        self.init_https();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port))?;

        for stream in listener.incoming() {
            debug!("New connection");
            let mut cloned_server = self.clone();
            std::thread::spawn(move || {
                match cloned_server.handle_connection(&mut stream.unwrap()) {
                    Ok(_) => debug!("Connection closed"),
                    Err(err) => info!("Connection error: {err:?}"),
                }
            });
        }

        Ok(())
    }

    fn handle_connection(&mut self, stream: &mut TcpStream) -> Result<()> {
        let (persistent, max_requests) = match self.config.keep_alive {
            KeepAliveConfig::On {
                timeout,
                max_requests,
                ..
            } => {
                stream.set_read_timeout(Some(std::time::Duration::from_secs(timeout as u64)))?;
                (true, max_requests)
            }
            _ => (false, 0),
        };

        let mut connection = Connection::new(stream, self.https_config.clone(), persistent);
        let mut served_requests_count = 0u8;
        let mut current_request: Option<Request> = None;

        loop {
            let mut response: Option<Response> = None;

            let read_until = if let Some(request) = &current_request {
                ReadUntil::NoBytes(request.content_length().unwrap() - request.body.len())
            } else {
                ReadUntil::DoubleCrLf
            };
            let request_bytes = match connection.read(read_until) {
                Ok(None) => {
                    debug!("Got none bytes");
                    return Ok(());
                }
                Ok(Some(bytes)) if bytes.is_empty() => {
                    debug!("Got empty message (TCP FIN, probably)");
                    return Ok(());
                }
                Ok(Some(bytes)) => bytes,
                Err(err) => match err.kind() {
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => return Ok(()),
                    ErrorKind::TimedOut => {
                        response =
                            Some(self.error_response(None, ResponseStatusCode::RequestTimeout));
                        vec![]
                    }
                    _ => return Err(err),
                },
            };

            if !response.is_some() {
                match &mut current_request {
                    None => {
                        let request = parse_request(request_bytes.as_slice());
                        if let Ok(request) = request {
                            let has_body = matches!(request.content_length(), Some(length) if !(request.body.len() == length || length == 0));

                            if !has_body {
                                response = Some(self.prepare_response(&request));
                            } else {
                                current_request = Some(request);
                            }
                        } else {
                            response =
                                Some(self.error_response(None, ResponseStatusCode::BadRequest));
                        }
                    }
                    Some(request) => {
                        let length = request.content_length().unwrap();
                        if request_bytes.len() > length {
                            response = Some(
                                self.error_response(Some(request), ResponseStatusCode::BadRequest),
                            );
                        } else {
                            request.body.extend(request_bytes);
                            response = Some(self.serve_content(request));
                        }
                    }
                }
            }

            if let Some(response) = response {
                let mut response = response;
                let should_close = !persistent
                    || served_requests_count == max_requests - 1
                    || current_request
                        .as_ref()
                        .is_some_and(|request| request.has_header("Connection", Some("close")));

                if should_close {
                    response = response.add_header("Connection", "close");
                }

                connection.write(&response.as_bytes())?;

                current_request = None;
                served_requests_count += 1;

                if should_close {
                    return Ok(());
                }
            }
        }
    }

    fn prepare_response(&self, request: &Request) -> Response {
        if request.method == RequestMethod::Options && request.url == "*" {
            self.options_response(request)
        } else {
            self.serve_content(request)
        }
    }

    fn get_content(&self, request: &Request) -> Result<Vec<u8>> {
        let root_path = Path::new(&self.config.root);
        let path = root_path.join(request.url.strip_prefix('/').unwrap());
        let canonical_root_path = fs::canonicalize(root_path)?;
        let canonical_path = fs::canonicalize(path)?;

        // Do this check so no smarty-pants tries to access files
        // outside web root directory, e.g. with GET /../example_http.rs
        if !canonical_path.starts_with(canonical_root_path) {
            return Err(std::io::Error::from(ErrorKind::PermissionDenied));
        }

        fs::read(canonical_path)
    }

    fn serve_content(&self, request: &Request) -> Response {
        let content = self.get_content(request);

        if let Ok(content_bytes) = content {
            if !request.method.is_safe() {
                return self
                    .error_response(Some(request), ResponseStatusCode::MethodNotAllowed)
                    .add_header("Allow", &RequestMethod::safe_methods_str());
            } else if request.method == RequestMethod::Options {
                return self.options_response(request);
            }

            let mime_type = mime_guess::from_path(&request.url).first();
            let content_type = if let Some(mime) = mime_type {
                let charset = if mime.type_() == "text" {
                    "; charset=utf-8"
                } else {
                    ""
                };
                mime.essence_str().to_string() + charset
            } else {
                "application/octet-stream".to_string()
            };

            let mut builder = Response::builder()
                .status_code(ResponseStatusCode::Ok)
                .header("Content-Type", &content_type)
                .header("Content-Length", &content_bytes.len().to_string());

            if let KeepAliveConfig::On {
                timeout,
                max_requests,
                include_header,
            } = self.config.keep_alive
            {
                if include_header {
                    builder = builder.header(
                        "Keep-Alive",
                        &format!("timeout={timeout}, max={max_requests}"),
                    );
                }
            }

            if request.method == RequestMethod::Get {
                builder = builder.body(content_bytes);
            }

            return builder.get();
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

        let accepts_html = if let Some(request) = request {
            let accept_header = request.headers.get("Accept");
            matches!(accept_header, Some(v) if v.contains("text/html") || v.contains("text/*") || v.contains("*/*"))
        } else {
            false
        };

        if accepts_html {
            let text_body = format!(
                "<html><body><h1 style='text-align: center'>{} {}</h1></body></html>",
                status_code as u16, status_code
            );
            response_builder = response_builder
                .header("Content-Type", "text/html; charset=utf-8")
                .text_body(&text_body)
        }

        response_builder.get()
    }

    fn options_response(&self, request: &Request) -> Response {
        let mut response_builder =
            ResponseBuilder::new().status_code(ResponseStatusCode::NoContent);

        if request.url != "*" {
            response_builder = response_builder.header("Allow", &RequestMethod::safe_methods_str());
        }

        response_builder.get()
    }
}
