use axum::{
    routing::{ get, post },
    Router,
    extract::{ Path, State, Multipart, DefaultBodyLimit },
    http::{ StatusCode, header, HeaderMap, HeaderValue },
    response::{ IntoResponse, Response },
    body::Body,
};
use sqlx::sqlite::SqlitePool;
use tokio_util::io::ReaderStream;

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
        .route("/cache/{id}", get(download_mp3))
        .route("/cache/{id}/claim", post(claim_id))
        .route("/cache/{id}/upload", post(upload_mp3))
        .layer(DefaultBodyLimit::disable())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "connected!"
}

async fn download_mp3(State(_state): State<AppState>, Path(id): Path<String>) -> Response {
    let dir = format!("./cache/{}", id);

    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(_) => {
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    let mut filename = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".mp3") {
                filename = Some(name.to_string());
                break;
            }
        }
    }

    let filename = match filename {
        Some(f) => f,
        None => {
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    let file_path = format!("{}/{}", dir, filename);
    let file = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("audio/mpeg"));

    let disposition = format!("attachment; filename=\"{}\"", filename);
    if let Ok(val) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, val);
    }

    (StatusCode::OK, headers, body).into_response()
}

async fn claim_id(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    let query = "INSERT INTO cache (id, status) VALUES (?, 'pending')";

    let result = sqlx::query(query).bind(id.clone()).execute(&state.db).await;

    match result {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::CONFLICT,
    }
}

async fn upload_mp3(
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart
) -> StatusCode {
    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field.file_name().unwrap_or("unknown.mp3").to_string();

        let data = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => {
                return StatusCode::BAD_REQUEST;
            }
        };

        let dir = format!("./cache/{}", id);
        if tokio::fs::create_dir_all(&dir).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        let file_path = format!("{}/{}", dir, filename);
        if tokio::fs::write(&file_path, &data).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        let query = "UPDATE cache SET status = 'ready' WHERE id = ?";
        let result = sqlx::query(query).bind(id).execute(&state.db).await;

        return match result {
            Ok(res) if res.rows_affected() > 0 => StatusCode::OK,
            _ => StatusCode::NOT_FOUND,
        };
    }

    StatusCode::BAD_REQUEST
}
