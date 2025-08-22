#![allow(dead_code)]
use capabilities::{config, logger};
use poem::listener::TcpListener;
mod capabilities;
// mod app;
mod base;
mod bootstrap;
mod test_utils;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    config::init();
    let _ = logger::init();
    // Base environment variables
    let host: String = config::get_env("HOST");
    let port: String = config::get_env("PORT");
    let server_url = format!("{host}:{port}");

    let app = bootstrap::build_app().await;
    logger::info(&format!("Starting server on {server_url}"));
    poem::Server::new(TcpListener::bind(server_url))
        .run(app)
        .await
}
