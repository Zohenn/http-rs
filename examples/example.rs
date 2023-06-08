use http_rs::server::Server;
use http_rs::server_config::{ServerConfigBuilder};
use std::io::Result;

fn main() -> Result<()> {
    let config = ServerConfigBuilder::new()
        .root("root")
        .port(443)
        .https(true)
        .cert_path("keys/server.crt")
        .key_path("keys/server.key")
        .get();
    Server::new(Some(config)).run()
}
