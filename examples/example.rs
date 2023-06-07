use http_rs::server::{Server, ServerConfig};
use std::io::Result;

fn main() -> Result<()> {
    // run()
    Server::new(Some(ServerConfig {
        root: String::from("root"),
        port: 443,
        https: true,
        cert_path: Some(String::from("keys/server.crt")),
        key_path: Some(String::from("keys/server.key")),
    }))
    .run()
}
