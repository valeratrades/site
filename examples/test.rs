use axum::{Router, response::Html, routing::get};
use site::dashboards::market_structure;

#[tokio::main]
async fn main() {
	let ms = market_structure::request_market_structure().await.unwrap();

	let app = Router::new().route("/", get(|| async { Html(ms.0) }));

	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
	axum::serve(listener, app).await.unwrap();
}
