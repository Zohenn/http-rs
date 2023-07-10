use rustls_pemfile::Item;
use std::fs;
use std::io::BufReader;

#[derive(Copy, Clone, PartialEq)]
pub enum KeepAliveConfig {
    Off,
    On {
        max_requests: u8,
        timeout: u8,
        include_header: bool,
    },
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        KeepAliveConfig::On {
            max_requests: 100,
            timeout: 10,
            include_header: true,
        }
    }
}

pub struct ServerConfig {
    pub root: String,
    pub port: u32,
    pub https: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub keep_alive: KeepAliveConfig,
    pub timeout: u8,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            root: String::from("web"),
            port: 80,
            https: false,
            cert_path: None,
            key_path: None,
            keep_alive: KeepAliveConfig::default(),
            timeout: 10,
        }
    }
}

impl ServerConfig {
    pub(crate) fn load_certs(&self) -> Vec<rustls::Certificate> {
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

    pub(crate) fn load_key(&self) -> Option<rustls::PrivateKey> {
        let Some(key_path) = &self.key_path else {
            return None
        };

        let key_file = fs::File::open(key_path).expect("Could not open key file");
        let mut reader = BufReader::new(key_file);
        let Ok(Some(item)) = rustls_pemfile::read_one(&mut reader) else {
            return None;
        };

        match item {
            Item::RSAKey(key) | Item::PKCS8Key(key) | Item::ECKey(key) => {
                Some(rustls::PrivateKey(key))
            }
            _ => None,
        }
    }
}

pub struct ServerConfigBuilder {
    server_config: ServerConfig,
}

#[allow(clippy::new_without_default)]
impl ServerConfigBuilder {
    pub fn new() -> Self {
        ServerConfigBuilder {
            server_config: ServerConfig::default(),
        }
    }

    pub fn root(mut self, root: &str) -> Self {
        self.server_config.root = root.to_string();

        self
    }

    pub fn port(mut self, port: u32) -> Self {
        self.server_config.port = port;

        self
    }

    pub fn https(mut self, https: bool) -> Self {
        self.server_config.https = https;

        self
    }

    pub fn cert_path(mut self, cert_path: &str) -> Self {
        self.server_config.cert_path = Some(cert_path.to_string());

        self
    }

    pub fn key_path(mut self, key_path: &str) -> Self {
        self.server_config.key_path = Some(key_path.to_string());

        self
    }

    pub fn keep_alive(mut self, keep_alive_config: KeepAliveConfig) -> Self {
        self.server_config.keep_alive = keep_alive_config;

        self
    }

    pub fn get(self) -> ServerConfig {
        self.server_config
    }
}
