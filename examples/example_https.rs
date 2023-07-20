use http_rs::response::Response;
use http_rs::response_status_code::ResponseStatusCode;
use http_rs::server::Server;
use http_rs::server_config::ServerConfigBuilder;
use log::LevelFilter;
use pretty_env_logger::env_logger::Target;
use std::io::Result;
use std::sync::Arc;

fn main() -> Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Debug)
        .target(Target::Stdout)
        .init();

    let config = ServerConfigBuilder::new()
        .root("root")
        .https(true)
        .cert_path("keys/server.crt")
        .key_path("keys/server.key")
        .get();

    Server::new(Some(config))
        .listener(|request| {
            if request.url.starts_with("/public") {
                return None;
            } else if request.url == "/post" {
                return Some(
                    Response::builder()
                        .status_code(ResponseStatusCode::Ok)
                        .header(
                            "Content-Type",
                            &request
                                .get_header("Content-Type")
                                .unwrap_or("text/html".to_string()),
                        )
                        .header("Content-Length", &request.body.len().to_string())
                        .body(request.body.clone())
                        .get(),
                );
            }

            Some(
                Response::builder()
                    .status_code(ResponseStatusCode::Ok)
                    .header("Content-Type", "text/html")
                    .body(format!("Listener: {}", request.url).as_bytes().to_vec())
                    .get(),
            )
        })
        .run(Arc::new(false))
}
