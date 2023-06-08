use http_rs::server::Server;
use http_rs::server_config::ServerConfigBuilder;
use std::io::Result;
use http_rs::response::Response;
use http_rs::response_status_code::ResponseStatusCode;

fn main() -> Result<()> {
    let config = ServerConfigBuilder::new().root("root").get();

    Server::new(Some(config))
        .listener(|request| {
            if request.url != "/long.html" {
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
