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

    pub fn listener(
        mut self,
        listener: impl Fn(&Request) -> Option<Response> + Send + Sync + 'static,
    ) -> Self {
        self.listener = Some(Arc::new(listener));

        self
    }

    pub fn run(&mut self) -> Result<()> {
        self.https_config = init_https(&self.config);

        let mut listeners = vec![TcpListener::bind(format!(
            "127.0.0.1:{}",
            self.config.port
        ))?];

        if self.https_config.is_some() {
            listeners.push(TcpListener::bind("127.0.0.1:443".to_string())?);
        }

        let (tx, rx) = std::sync::mpsc::channel();

        for (index, listener) in listeners.into_iter().enumerate() {
            let cloned_server = self.clone();
            let tx = tx.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    debug!("New connection");
                    let cloned_server = cloned_server.clone();
                    std::thread::spawn(move || {
                        match cloned_server.handle_connection(&mut stream.unwrap()) {
                            Ok(_) => debug!("Connection closed"),
                            Err(err) => info!("Connection error: {err:?}"),
                        }
                    });
                }

                tx.send(index).unwrap();
            });
        }

        rx.recv().unwrap();

        Ok(())
    }

    fn handle_connection(&self, stream: &mut TcpStream) -> Result<()> {
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
                        response = Some(error_response(None, ResponseStatusCode::RequestTimeout));
                        vec![]
                    }
                    _ => return Err(err),
                },
            };

            if response.is_none() {
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
                            response = Some(error_response(None, ResponseStatusCode::BadRequest));
                        }
                    }
                    Some(request) => {
                        let length = request.content_length().unwrap();
                        if request_bytes.len() > length {
                            response = Some(error_response(
                                Some(request),
                                ResponseStatusCode::BadRequest,
                            ));
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
            options_response(request)
        } else {
            self.serve_content(request)
        }
    }

    fn serve_content(&self, request: &Request) -> Response {
        let content = get_content(&self.config.root, &request.url);

        if let Ok(content_bytes) = content {
            if !request.method.is_safe() {
                return error_response(Some(request), ResponseStatusCode::MethodNotAllowed)
                    .add_header("Allow", &RequestMethod::safe_methods_str());
            } else if request.method == RequestMethod::Options {
                return options_response(request);
            }

            return content_response(request, content_bytes, self.config.keep_alive);
        }

        if let Some(listener) = &self.listener {
            if let Some(response) = listener(request) {
                return response;
            }
        }

        error_response(Some(request), ResponseStatusCode::NotFound)
    }
}

fn init_https(config: &ServerConfig) -> Option<Arc<rustls::ServerConfig>> {
    if !config.https {
        return None;
    }

    let certs = config.load_certs();
    let key = config.load_key();

    if certs.is_empty() {
        panic!("Specified file does not contain a valid certificate");
    }

    if key.is_none() {
        panic!("Specified file does not contain a valid private key");
    }

    Some(Arc::new(
        rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key.unwrap())
            .unwrap(),
    ))
}

fn get_content(root: &str, content_path: &str) -> Result<Vec<u8>> {
    let root_path = Path::new(root);
    let path = root_path.join(content_path.trim_start_matches('/'));
    let canonical_root_path = fs::canonicalize(root_path)?;
    let canonical_path = fs::canonicalize(path)?;

    // Do this check so no smarty-pants tries to access files
    // outside web root directory, e.g. with GET /../example_http.rs
    if !canonical_path.starts_with(canonical_root_path) {
        return Err(std::io::Error::from(ErrorKind::PermissionDenied));
    }

    fs::read(canonical_path)
}

fn content_response(
    request: &Request,
    content_bytes: Vec<u8>,
    keep_alive_config: KeepAliveConfig,
) -> Response {
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
    } = keep_alive_config
    {
        if include_header {
            builder = builder.header(
                "Keep-Alive",
                &format!("timeout={timeout}, max={max_requests}"),
            );
        }
    }

    if request.method == RequestMethod::Get {
        // todo: this is where content should actually be read
        builder = builder.body(content_bytes);
    }

    builder.get()
}

fn error_response(request: Option<&Request>, status_code: ResponseStatusCode) -> Response {
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

fn options_response(request: &Request) -> Response {
    let mut response_builder = ResponseBuilder::new().status_code(ResponseStatusCode::NoContent);

    if request.url != "*" {
        response_builder = response_builder.header("Allow", &RequestMethod::safe_methods_str());
    }

    response_builder.get()
}

#[cfg(test)]
mod test {
    mod server_init_https {
        use crate::server::init_https;
        use crate::server_config::ServerConfig;

        #[test]
        #[should_panic(expected = "certificate")]
        fn panic_if_could_not_load_certs() {
            let config = ServerConfig {
                https: true,
                ..Default::default()
            };
            init_https(&config);
        }

        #[test]
        #[should_panic(expected = "private")]
        fn panic_if_could_not_load_key() {
            let config = ServerConfig {
                https: true,
                // well, yeah
                cert_path: Some("./examples/keys/server.crt".to_string()),
                ..Default::default()
            };
            init_https(&config);
        }

        #[test]
        fn returns_config() {
            let config = ServerConfig {
                https: true,
                // well, yeah x2
                cert_path: Some("./examples/keys/server.crt".to_string()),
                key_path: Some("./examples/keys/server.key".to_string()),
                ..Default::default()
            };

            assert!(init_https(&config).is_some());
        }

        #[test]
        fn returns_none_if_https_is_disabled() {
            let config = ServerConfig {
                https: false,
                // well, yeah x3
                cert_path: Some("./examples/keys/server.crt".to_string()),
                key_path: Some("./examples/keys/server.key".to_string()),
                ..Default::default()
            };

            assert!(init_https(&config).is_none());
        }
    }

    mod get_content {
        // These tests are dumb but I'm not going to mock fs
        use crate::server::get_content;
        use std::io::ErrorKind;

        #[test]
        fn ok_if_file_exists() {
            assert!(get_content(".", "Cargo.toml").is_ok());
        }

        #[test]
        fn ok_if_file_does_not_exist() {
            assert!(get_content(".", "Cargo.tomlllll").is_err());
        }

        #[test]
        fn err_if_file_is_outside_root() {
            assert!(
                matches!(get_content("src", "/../Cargo.toml"), Err(e) if e.kind() == ErrorKind::PermissionDenied)
            );
        }
    }

    mod content_response {
        use crate::http_version::HttpVersion;
        use crate::request::Request;
        use crate::request_method::RequestMethod;
        use crate::server::content_response;
        use crate::server_config::KeepAliveConfig;
        use std::collections::HashMap;

        fn get_request(method: RequestMethod, url: &str) -> Request {
            Request {
                method,
                url: url.to_string(),
                version: HttpVersion::Http1_1,
                headers: HashMap::new(),
                body: vec![],
            }
        }

        fn get_default_request(method: RequestMethod) -> Request {
            get_request(method, "/index.html")
        }

        #[test]
        fn adds_content_type_header() {
            for (url, content_type) in [
                ("/index.html", "text/html; charset=utf-8"),
                ("/123", "application/octet-stream"),
            ] {
                let request = get_request(RequestMethod::Get, url);
                let response = content_response(&request, vec![], KeepAliveConfig::Off);

                assert_eq!(
                    response.headers().get("Content-Type"),
                    Some(&content_type.to_string())
                );
            }
        }

        #[test]
        fn adds_content_length_header() {
            let request = get_default_request(RequestMethod::Head);
            let content_bytes = vec![b'1', b'2', b'3'];
            let response = content_response(&request, content_bytes.clone(), KeepAliveConfig::Off);

            assert_eq!(
                response.headers().get("Content-Length"),
                Some(&content_bytes.len().to_string())
            );
        }

        #[test]
        fn does_not_add_keep_alive_header_with_keep_alive_disabled() {
            let request = get_default_request(RequestMethod::Get);
            let response = content_response(&request, vec![], KeepAliveConfig::Off);

            assert!(response.headers().get("Keep-Alive").is_none());
        }

        #[test]
        fn does_not_add_keep_alive_header_with_keep_alive_include_header_false() {
            let request = get_default_request(RequestMethod::Get);
            let timeout = 123;
            let max_requests = 231;
            let response = content_response(
                &request,
                vec![],
                KeepAliveConfig::On {
                    timeout,
                    max_requests,
                    include_header: false,
                },
            );

            assert!(response.headers().get("Keep-Alive").is_none());
        }

        #[test]
        fn adds_keep_alive_header_with_keep_alive_include_header_true() {
            let request = get_default_request(RequestMethod::Get);
            let timeout = 123;
            let max_requests = 231;
            let response = content_response(
                &request,
                vec![],
                KeepAliveConfig::On {
                    timeout,
                    max_requests,
                    include_header: true,
                },
            );

            assert_eq!(
                response.headers().get("Keep-Alive").unwrap(),
                &format!("timeout={timeout}, max={max_requests}")
            );
        }

        #[test]
        fn has_body_for_get_request() {
            let request = get_default_request(RequestMethod::Get);
            let response = content_response(&request, vec![b'1', b'2', b'3'], KeepAliveConfig::Off);

            assert!(!response.body().is_empty());
        }

        #[test]
        fn has_no_body_for_non_get_request() {
            let request = get_default_request(RequestMethod::Post);
            let response = content_response(&request, vec![b'1', b'2', b'3'], KeepAliveConfig::Off);

            assert!(response.body().is_empty());
        }
    }

    mod error_response {
        use crate::http_version::HttpVersion;
        use crate::request::Request;
        use crate::request_method::RequestMethod;
        use crate::response_status_code::ResponseStatusCode;
        use crate::server::error_response;
        use std::collections::HashMap;

        fn get_request(accept: &str) -> Request {
            Request {
                method: RequestMethod::Get,
                url: "/".to_string(),
                version: HttpVersion::Http1_1,
                headers: HashMap::from([("Accept".to_string(), accept.to_string())]),
                body: vec![],
            }
        }

        #[test]
        fn empty_body_with_no_request() {
            let response = error_response(None, ResponseStatusCode::NotFound);

            assert_eq!(response.body().len(), 0);
            assert_eq!(response.headers().get("Content-Length"), None);
        }

        #[test]
        fn empty_body_if_does_not_accept_html() {
            for accept in [
                "text/javascript",
                "image/webp",
                "application/json, application/xml",
            ] {
                let response =
                    error_response(Some(&get_request(accept)), ResponseStatusCode::NotFound);

                assert_eq!(response.body().len(), 0);
                assert_eq!(response.headers().get("Content-Length"), None);
            }
        }

        #[test]
        fn default_html_in_body_if_accepts_html() {
            for accept in ["*/*", "text/html", "application/json, text/*"] {
                let response =
                    error_response(Some(&get_request(accept)), ResponseStatusCode::NotFound);

                assert!(!response.body().is_empty());
                assert!(response.headers().get("Content-Length").is_some());
            }
        }
    }

    mod options_response {
        use crate::http_version::HttpVersion;
        use crate::request::Request;
        use crate::request_method::RequestMethod;
        use crate::response_status_code::ResponseStatusCode;
        use crate::server::options_response;
        use std::collections::HashMap;

        fn get_request(url: &str) -> Request {
            Request {
                method: RequestMethod::Options,
                url: url.to_string(),
                version: HttpVersion::Http1_1,
                headers: HashMap::new(),
                body: vec![],
            }
        }

        #[test]
        fn has_204_status_code() {
            let response = options_response(&get_request("/"));

            assert_eq!(response.status_code(), &ResponseStatusCode::NoContent);
        }

        #[test]
        fn has_empty_body() {
            let response = options_response(&get_request("/"));

            assert_eq!(response.body().len(), 0);
        }

        #[test]
        fn sets_allow_header_for_non_star_url() {
            let response = options_response(&get_request("/a/b/index.html"));

            assert_eq!(
                response.headers().get("Allow"),
                Some(&RequestMethod::safe_methods_str())
            );
        }

        #[test]
        fn does_not_set_allow_header_for_star_url() {
            let response = options_response(&get_request("*"));

            assert_eq!(response.headers().get("Allow"), None);
        }
    }
}
