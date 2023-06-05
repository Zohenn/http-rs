use http_rs::server::{Server, ServerConfig};
use std::io::Result;

fn main() -> Result<()> {
    // run()
    Server::new(Some(ServerConfig {
        root: String::from("root"),
        port: 81,
    }))
    .run()
}
