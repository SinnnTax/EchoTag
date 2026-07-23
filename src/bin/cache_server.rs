use axum::{ routing::get, Router };

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health_check)).route("/actuallyworks", get(works));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "connected!"
}

async fn works() -> String {
    "yes".to_string()
}
