use http_rs::response::Response;
use http_rs::response_status_code::ResponseStatusCode;
use http_rs::server::Server;
use http_rs::server_config::ServerConfigBuilder;
use std::io::Result;

fn main() -> Result<()> {
    let config = ServerConfigBuilder::new()
        .root("root")
        .port(443)
        .https(true)
        .cert_path("keys/server.crt")
        .key_path("keys/server.key")
        .get();

    Server::new(Some(config))
        .listener(|request| {
            if request.url.starts_with("/public") {
                return None;
            }

            Some(
                Response::builder()
                    .status_code(ResponseStatusCode::Ok)
                    .header("Content-Type", "text/html")
                    .body(format!("Listener: {}", request.url).as_bytes().to_vec())
                    .get(),
            )
        })
        .run()
}
