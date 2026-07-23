use axum::{ routing::{ get, post }, Router, extract::{ Path, State }, http::StatusCode };
use sqlx::sqlite::SqlitePool;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

#[tokio::main]
async fn main() {
    let db = SqlitePool::connect("sqlite:cache.db?mode=rwc").await.unwrap();

    sqlx::query("CREATE TABLE IF NOT EXISTS cache (id TEXT PRIMARY KEY, status TEXT)")
        .execute(&db).await
        .unwrap();

    let state = AppState { db };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/cache/{id}", get(get_id_status))
        .route("/cache/{id}/claim", post(claim_id))
        .route("/cache/{id}/upload", post(upload_mp3))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "connected!"
}

async fn get_id_status(State(state): State<AppState>, Path(id): Path<u32>) -> String {
    let query = "SELECT status FROM cache WHERE id = ?";

    // fetch_optional returns ok(some(string)) if found and ok(none) if not found
    let result: Option<String> = sqlx
        ::query_scalar(query)
        .bind(id.to_string())
        .fetch_optional(&state.db).await
        .unwrap();

    match result {
        Some(status) => format!("ID {} exists. status: {}", id, status),
        None => format!("ID {} not found in cache", id),
    }
}

async fn claim_id(State(state): State<AppState>, Path(id): Path<u32>) -> String {
    let query = "INSERT INTO cache (id, status) VALUES (?, 'pending')";

    let result = sqlx::query(query).bind(id.to_string()).execute(&state.db).await;

    match result {
        Ok(_) => format!("Successfully claimed ID {}", id),
        Err(e) => format!("Failed to claim ID {}: {}", id, e),
    }
}

async fn upload_mp3(State(state): State<AppState>, Path(id): Path<u32>) -> StatusCode {
    let query = "UPDATE cache SET status = 'ready' WHERE id = ?";

    let result = sqlx::query(query).bind(id.to_string()).execute(&state.db).await;

    match result {
        Ok(res) => {
            if res.rows_affected() > 0 { StatusCode::OK } else { StatusCode::NOT_FOUND }
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
