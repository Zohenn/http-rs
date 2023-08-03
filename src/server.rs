use crate::connection::{Connection, ReadStrategy};
use crate::request::{parse_chunked_body, parse_request, Request, RequestBodyType};
use crate::request_method::RequestMethod;
use crate::response::{Response, ResponseBuilder};
use crate::response_status_code::ResponseStatusCode;
use crate::rules::{parse_file, Rule, RuleAction, RuleEvaluationResult};
use crate::server_config::{KeepAliveConfig, ServerConfig};
use crate::types::IoResult;
use log::{debug, error, info};
use std::fs;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

type RequestListener = dyn Fn(&Request) -> Option<Response> + Send + Sync;

#[derive(Clone)]
pub struct Server {
    config: Arc<ServerConfig>,
    rules: Arc<Vec<Rule>>,
    https_config: Option<Arc<rustls::ServerConfig>>,
    listener: Option<Arc<RequestListener>>,
}

impl Server {
    pub fn new(config: Option<ServerConfig>) -> Self {
        let rules = match &config {
            Some(config) if config.rules_path.is_some() => {
                match parse_file(config.rules_path.as_ref().unwrap()) {
                    Ok(rules) => rules,
                    Err(e) => {
                        error!("Error parsing rules file: {e}");
                        vec![]
                    }
                }
            }
            _ => vec![],
        };

        Server {
            config: Arc::new(config.unwrap_or(ServerConfig::default())),
            rules: Arc::new(rules),
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

    pub fn run(&mut self, stop: Arc<bool>) -> IoResult<()> {
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
            let stop = stop.clone();
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

                    if *stop {
                        debug!("Stopping listening for connections");
                        break;
                    }
                }

                tx.send(index).unwrap();
            });
        }

        rx.recv().unwrap();

        Ok(())
    }

    fn handle_connection(&self, stream: &mut TcpStream) -> IoResult<()> {
        let (persistent, max_requests) = match self.config.keep_alive {
            KeepAliveConfig::On {
                timeout,
                max_requests,
                ..
            } => {
                stream.set_read_timeout(Some(std::time::Duration::from_secs(timeout as u64)))?;
                (true, max_requests)
            }
            _ => {
                stream.set_read_timeout(Some(std::time::Duration::from_secs(
                    self.config.timeout as u64,
                )))?;
                (false, 0)
            }
        };

        let mut connection = Connection::new(stream, self.https_config.clone(), persistent);

        let mut state = HandleConnectionState::New;
        let mut state_machine =
            HandleConnectionStateMachine::new(self, &mut connection, persistent, max_requests);

        loop {
            state = state_machine.next(state);
            match state {
                HandleConnectionState::Close => return Ok(()),
                HandleConnectionState::Error(err) => return Err(err.into()),
                _ => {}
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
                let mut response =
                    error_response(Some(request), ResponseStatusCode::MethodNotAllowed);
                response.set_header("Allow", &RequestMethod::safe_methods_str());
                return response;
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

enum HandleConnectionState {
    New,
    Read(Option<Request>),
    SendResponse(Option<Request>, Response),
    ClientError(Option<Request>, ResponseStatusCode),
    Close,
    Error(ErrorKind),
}

struct HandleConnectionStateMachine<'server, 'connection, 'stream> {
    server: &'server Server,
    connection: &'connection mut Connection<'stream>,
    persistent: bool,
    max_requests: u8,
    served_requests_count: u8,
}

impl<'server, 'connection, 'stream> HandleConnectionStateMachine<'server, 'connection, 'stream> {
    fn new(
        server: &'server Server,
        connection: &'connection mut Connection<'stream>,
        persistent: bool,
        max_requests: u8,
    ) -> Self {
        HandleConnectionStateMachine {
            server,
            connection,
            persistent,
            max_requests,
            served_requests_count: 0u8,
        }
    }

    fn next(&mut self, state: HandleConnectionState) -> HandleConnectionState {
        let new_state: HandleConnectionState = match state {
            HandleConnectionState::New => HandleConnectionState::Read(None),
            HandleConnectionState::Read(current_request) => self.read(current_request),
            HandleConnectionState::SendResponse(request, response) => {
                self.send_response(request, response)
            }
            HandleConnectionState::ClientError(request, status_code) => {
                self.client_error(request, status_code)
            }
            HandleConnectionState::Close | HandleConnectionState::Error(_) => state,
        };

        new_state
    }

    fn read(&mut self, current_request: Option<Request>) -> HandleConnectionState {
        let read_strategy = if let Some(request) = &current_request {
            match request.body_type() {
                RequestBodyType::ContentLength => ReadStrategy::UntilNoBytesRead(
                    request.content_length().unwrap() - request.body.len(),
                ),
                RequestBodyType::TransferEncodingChunked => ReadStrategy::UntilDoubleCrlfAtEnd,
                RequestBodyType::None => unreachable!(),
            }
        } else {
            ReadStrategy::UntilDoubleCrlf
        };

        let request_bytes = match self.connection.read(read_strategy) {
            Ok(bytes) if bytes.is_empty() && current_request.is_some() => {
                return HandleConnectionState::ClientError(
                    current_request,
                    ResponseStatusCode::BadRequest,
                );
            }
            Ok(bytes) if bytes.is_empty() => {
                debug!("Got empty message (TCP FIN, probably)");
                return HandleConnectionState::Close;
            }
            Ok(bytes) => bytes,
            Err(err) => {
                return match err.kind() {
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
                        HandleConnectionState::Close
                    }
                    ErrorKind::TimedOut => {
                        HandleConnectionState::ClientError(None, ResponseStatusCode::RequestTimeout)
                    }
                    _ => HandleConnectionState::Error(err.kind()),
                }
            }
        };

        return match current_request {
            None => {
                let request = parse_request(request_bytes.as_slice());
                match request {
                    Ok((request, is_request_complete)) => {
                        let has_body = match request.body_type() {
                            RequestBodyType::ContentLength => {
                                matches!(request.content_length(), Some(length) if !(request.body.len() == length || length == 0))
                            }
                            RequestBodyType::TransferEncodingChunked => !is_request_complete,
                            RequestBodyType::None => false,
                        };

                        // todo: this probably can be changed to is_request_complete
                        if !has_body {
                            let response = self.server.prepare_response(&request);
                            HandleConnectionState::SendResponse(Some(request), response)
                        } else {
                            HandleConnectionState::Read(Some(request))
                        }
                    }
                    Err(err) => {
                        debug!("Parse request error: {err:?}");
                        HandleConnectionState::ClientError(None, ResponseStatusCode::BadRequest)
                    }
                }
            }
            Some(mut request) => {
                if matches!(request.body_type(), RequestBodyType::ContentLength)
                    && request_bytes.len() > request.content_length().unwrap()
                {
                    return HandleConnectionState::ClientError(
                        Some(request),
                        ResponseStatusCode::BadRequest,
                    );
                }

                let mut request_bytes = request_bytes;

                if matches!(
                    request.body_type(),
                    RequestBodyType::TransferEncodingChunked
                ) {
                    let Ok((body, is_complete)) = parse_chunked_body(request_bytes) else {
                        return HandleConnectionState::ClientError(Some(request), ResponseStatusCode::BadRequest);
                    };

                    // not sure if there will ever be a case when is_complete is false
                    if !is_complete {
                        return HandleConnectionState::Read(Some(request));
                    }

                    request_bytes = body;
                }

                request.body.extend(request_bytes);

                let response = self.server.serve_content(&request);
                HandleConnectionState::SendResponse(Some(request), response)
            }
        };
    }

    fn send_response(
        &mut self,
        request: Option<Request>,
        response: Response,
    ) -> HandleConnectionState {
        let mut response = match &request {
            Some(request) => apply_rules(&self.server.rules, request, response),
            None => response,
        };

        let should_close = !self.persistent
            || self.served_requests_count == self.max_requests - 1
            || request
                .as_ref()
                .is_some_and(|request| request.has_header("Connection", Some("close")));

        if should_close {
            response.set_header("Connection", "close");
        }

        match self.connection.write(&response.as_bytes()) {
            Ok(_) => {}
            Err(err) => return HandleConnectionState::Error(err.kind()),
        }

        self.served_requests_count += 1;

        if should_close {
            HandleConnectionState::Close
        } else {
            HandleConnectionState::Read(None)
        }
    }

    fn client_error(
        &mut self,
        request: Option<Request>,
        status_code: ResponseStatusCode,
    ) -> HandleConnectionState {
        let response = error_response(request.as_ref(), status_code);
        HandleConnectionState::SendResponse(request, response)
    }
}

fn apply_rules(rules: &[Rule], request: &Request, response: Response) -> Response {
    let mut out_response = response;

    for rule in rules {
        if !rule.matches(&request.url) {
            continue;
        }

        match rule.evaluate(request, out_response) {
            RuleEvaluationResult::Continue(response) => out_response = response,
            RuleEvaluationResult::Finish(response) => return response,
        }
    }

    out_response
}

fn get_content(root: &str, content_path: &str) -> IoResult<Vec<u8>> {
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

        // Next 3 tests are most certainly not unit tests, but I'm not going to mock fs

        #[test]
        #[should_panic(expected = "private")]
        fn panic_if_could_not_load_key() {
            let config = ServerConfig {
                https: true,
                cert_path: Some("./test_files/keys/server.crt".to_string()),
                ..Default::default()
            };
            init_https(&config);
        }

        #[test]
        fn returns_config() {
            let config = ServerConfig {
                https: true,
                cert_path: Some("./test_files/keys/server.crt".to_string()),
                key_path: Some("./test_files/keys/server.key".to_string()),
                ..Default::default()
            };

            assert!(init_https(&config).is_some());
        }

        #[test]
        fn returns_none_if_https_is_disabled() {
            let config = ServerConfig {
                https: false,
                cert_path: Some("./test_files/keys/server.crt".to_string()),
                key_path: Some("./test_files/keys/server.key".to_string()),
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
            assert!(get_content("test_files", "file.txt").is_ok());
        }

        #[test]
        fn ok_if_file_does_not_exist() {
            assert!(get_content("test_files", "0qhwe0t9h.txt").is_err());
        }

        #[test]
        fn err_if_file_is_outside_root() {
            assert!(
                matches!(get_content("test_files/dir", "/../file.txt"), Err(e) if e.kind() == ErrorKind::PermissionDenied)
            );
        }
    }

    mod content_response {
        use crate::header::Headers;
        use crate::http_version::HttpVersion;
        use crate::request::Request;
        use crate::request_method::RequestMethod;
        use crate::server::content_response;
        use crate::server_config::KeepAliveConfig;

        fn get_request(method: RequestMethod, url: &str) -> Request {
            Request {
                method,
                url: url.to_string(),
                version: HttpVersion::Http1_1,
                headers: Headers::new(),
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
        use crate::header::Headers;
        use crate::http_version::HttpVersion;
        use crate::request::Request;
        use crate::request_method::RequestMethod;
        use crate::response_status_code::ResponseStatusCode;
        use crate::server::error_response;

        fn get_request(accept: &str) -> Request {
            Request {
                method: RequestMethod::Get,
                url: "/".to_string(),
                version: HttpVersion::Http1_1,
                headers: Headers::from([("Accept".to_string(), accept.to_string())]),
                body: vec![],
            }
        }

        #[test]
        fn empty_body_with_no_request() {
            let response = error_response(None, ResponseStatusCode::NotFound);

            assert!(response.body().is_empty());
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

                assert!(response.body().is_empty());
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
        use crate::header::Headers;
        use crate::http_version::HttpVersion;
        use crate::request::Request;
        use crate::request_method::RequestMethod;
        use crate::response_status_code::ResponseStatusCode;
        use crate::server::options_response;

        fn get_request(url: &str) -> Request {
            Request {
                method: RequestMethod::Options,
                url: url.to_string(),
                version: HttpVersion::Http1_1,
                headers: Headers::new(),
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
