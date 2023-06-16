use http_rs::response::Response;
use http_rs::response_status_code::ResponseStatusCode;
use http_rs::server::Server;
use http_rs::server_config::{KeepAliveConfig, ServerConfigBuilder};
use log::LevelFilter;
use pretty_env_logger::env_logger::Target;
use std::io::Result;

fn main() -> Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Debug)
        .target(Target::Stdout)
        .init();

    let config = ServerConfigBuilder::new()
        .root("root")
        .keep_alive(KeepAliveConfig::On {
            timeout: 10,
            max_requests: 4,
            include_header: true,
        })
        .get();

    Server::new(Some(config))
        .listener(|request| {
            if request.url == "/post" {
                return Some(
                    Response::builder()
                        .status_code(ResponseStatusCode::Ok)
                        .header(
                            "Content-Type",
                            request
                                .headers
                                .get("Content-Type")
                                .unwrap_or(&"text/html".to_string()),
                        )
                        .header("Content-Length", &request.body.len().to_string())
                        .body(request.body.clone())
                        .get(),
                );
            } else if request.url != "/long.html" {
                return None;
            }

            std::thread::sleep(std::time::Duration::from_secs(10));

            Some(
                Response::builder()
                    .status_code(ResponseStatusCode::Ok)
                    .header("Content-Type", "text/html")
                    .body("Long request".as_bytes().to_vec())
                    .get(),
            )
        })
        .run()
}
