use axum::{ routing::get, Router, extract::{ Path, State } };
use sqlx::sqlite::SqlitePool;

#[derive(Clone)]
struct AppState {
    name: String,
    db: SqlitePool,
}

#[tokio::main]
async fn main() {
    let db = SqlitePool::connect("sqlite:cache.db?mode=rwc").await.unwrap();

    sqlx::query("CREATE TABLE IF NOT EXISTS cache (id TEXT PRIMARY KEY, status TEXT)")
        .execute(&db).await
        .unwrap();

    let state = AppState { name: "EchoTag Cache Server".to_string(), db };

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
    let db_time: i64 = sqlx
        ::query_scalar("SELECT CAST(strftime('%s', 'now') AS INTEGER)")
        .fetch_one(&state.db).await
        .unwrap();

    format!("[{}] db time: {}. asked for id: {}", state.name, db_time, id)
}
