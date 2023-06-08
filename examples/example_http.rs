use http_rs::server::Server;
use http_rs::server_config::ServerConfigBuilder;
use std::io::Result;

fn main() -> Result<()> {
    let config = ServerConfigBuilder::new().root("root").get();

    Server::new(Some(config)).run()
}
