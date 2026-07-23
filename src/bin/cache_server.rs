use axum::{ routing::get, Router, extract::Path };

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health_check)).route("/cache/{id}", get(get_id));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "connected!"
}

async fn get_id(Path(id): Path<u32>) -> String {
    format!("id is {id}")
}
