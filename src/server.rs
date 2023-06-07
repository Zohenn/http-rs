use crate::request::{parse_request, Request};
use crate::response::{Response, ResponseBuilder};
use crate::response_status_code::ResponseStatusCode;
use crate::utils::StringUtils;
use rustls_pemfile::Item;
use std::fs;
use std::io::{BufReader, Error, Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use std::path::Path;
use std::sync::Arc;

pub struct ServerConfig {
    pub root: String,
    pub port: u32,
    pub https: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

impl ServerConfig {
    pub fn default() -> Self {
        ServerConfig {
            root: String::from("web"),
            port: 80,
            https: false,
            cert_path: None,
            key_path: None,
        }
    }

    fn load_certs(&self) -> Vec<rustls::Certificate> {
        if let Some(cert_path) = &self.cert_path {
            let cert_file = fs::File::open(cert_path).expect("Could not open certificate file");
            let mut reader = BufReader::new(cert_file);
            rustls_pemfile::certs(&mut reader)
                .unwrap()
                .iter()
                .map(|v| rustls::Certificate(v.clone()))
                .collect()
        } else {
            vec![]
        }
    }

    fn load_key(&self) -> Option<rustls::PrivateKey> {
        if let Some(key_path) = &self.key_path {
            let key_file = fs::File::open(key_path).expect("Could not open key file");
            let mut reader = BufReader::new(key_file);
            if let Ok(Some(item)) = rustls_pemfile::read_one(&mut reader) {
                match item {
                    Item::RSAKey(key) | Item::PKCS8Key(key) | Item::ECKey(key) => {
                        Some(rustls::PrivateKey(key))
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct Server {
    config: ServerConfig,
    tls_connection: Option<rustls::ServerConnection>,
}

impl Server {
    pub fn new(config: Option<ServerConfig>) -> Self {
        Server {
            config: config.unwrap_or(ServerConfig::default()),
            tls_connection: None,
        }
    }

    fn init_https(&self) -> Option<Arc<rustls::ServerConfig>> {
        if !self.config.https {
            return None;
        }

        let certs = self.config.load_certs();
        let key = self.config.load_key();

        if certs.is_empty() || key.is_none() {
            return None;
        }

        Some(Arc::new(
            rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(certs, key.unwrap())
                .unwrap(),
        ))
    }

    pub fn run(&mut self) -> Result<()> {
        let https_server_config = self.init_https();

        if https_server_config.is_some() {
            self.tls_connection = Some(rustls::ServerConnection::new(https_server_config.unwrap()).unwrap());
        }

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port))?;

        for stream in listener.incoming() {
            self.handle_connection(&mut stream?)?;
        }

        Ok(())
    }

    fn handle_connection(&mut self, stream: &mut TcpStream) -> Result<()> {
        // let mut raw_request = String::new();
        let mut stream_buf: [u8; 255] = [0; 255];
        let mut request_bytes: Vec<u8> = Vec::new();

        let https_server_config = self.init_https();

        if https_server_config.is_some() {
            self.tls_connection = Some(rustls::ServerConnection::new(https_server_config.unwrap()).unwrap());
        }

        if let Some(tls_connection) = &mut self.tls_connection {
            while tls_connection.is_handshaking() {
                tls_connection.read_tls(stream)?;
                match tls_connection.process_new_packets() {
                    Err(err) => {
                        println!("{:?}", err);
                        return Ok(())
                    },
                    Ok(state) => {
                        println!("{state:?}");
                        match tls_connection.reader().read_to_end(&mut request_bytes) {
                            Ok(n) => {},
                            Err(err) => println!("{err:?}")
                        }
                        println!("{:?}", std::str::from_utf8(&request_bytes).unwrap());
                    },
                }
                tls_connection.write_tls(stream)?;
                println!("{} {} {}", tls_connection.is_handshaking(), tls_connection.wants_read(), tls_connection.wants_write());
            }
            tls_connection.read_tls(stream)?;
            match tls_connection.process_new_packets() {
                Err(err) => {
                    println!("{:?}", err);
                    tls_connection.send_close_notify();
                    return Ok(());
                },
                Ok(state) => {
                    // let mut buf: Vec<u8> = vec![];
                    println!("{state:?}");
                    match tls_connection.reader().read_to_end(&mut request_bytes) {
                        Ok(n) => {},
                        Err(err) => println!("{err:?}")
                    }
                    println!("{:?}", std::str::from_utf8(&request_bytes).unwrap());
                    if state.tls_bytes_to_write() > 0 {
                        tls_connection.write_tls(stream)?;
                    }
                },
            }
            println!("{} {} {}", tls_connection.is_handshaking(), tls_connection.wants_read(), tls_connection.wants_write());
        } else {
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
        }

        // println!("Received message:\n{raw_request}\n");

        let request = parse_request(request_bytes.as_slice());

        // println!("{request:#?}\n");

        let mut response = if let Ok(request) = request {
            self.serve_content(&request)
        } else {
            self.error_response(None, ResponseStatusCode::BadRequest)
        };

        if self.tls_connection.is_some() {
            let conn = self.tls_connection.as_mut().unwrap();
            conn.writer().write_all(&response.as_bytes()).unwrap();
            conn.write_tls(stream).unwrap();
            conn.send_close_notify();
        } else {
            stream.write_all(&response.as_bytes())?;
        }

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
            self.error_response(Some(request), ResponseStatusCode::NotFound)
        }
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
