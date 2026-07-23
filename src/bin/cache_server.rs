use axum::{ routing::get, Router, extract::{ Path, State } };

#[derive(Clone)]
struct AppState {
    name: String,
}

#[tokio::main]
async fn main() {
    let state = AppState { name: "EchoTag Cache Server".to_string() };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/cache/{id}", get(get_id))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "connected!"
}

async fn get_id(State(state): State<AppState>, Path(id): Path<u32>) -> String {
    format!("message from {}: id is {id}", state.name)
}
