use tokio::io::{ AsyncBufReadExt, BufReader };
use shared::models::Metadata;
use base64::{ engine::general_purpose, Engine };

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_project_info() -> String {
    format!("EchoTag Desktop - Built with Rust and Tauri!")
}

#[tauri::command]
async fn get_history() -> Result<Vec<Metadata>, String> {
    let file = tokio::fs::File
        ::open("history.ndjson").await
        .map_err(|e| format!("Failed to open history.ndjson: {}", e))?;

    let mut reader = BufReader::new(file).lines();
    let mut history_list = Vec::new();

    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(mut metadata) = serde_json::from_str::<Metadata>(&line) {
            let local_art_path = format!("{}_{}.jpg", metadata.artist_name, metadata.track_name);
            let is_default_url = metadata.artwork_url == "default_cover_art.jpg";

            let local_exists = tokio::fs::metadata(&local_art_path).await.is_ok();
            let default_exists =
                is_default_url && tokio::fs::metadata("default_cover_art.jpg").await.is_ok();

            if local_exists {
                if let Ok(bytes) = tokio::fs::read(&local_art_path).await {
                    let b64 = general_purpose::STANDARD.encode(&bytes);
                    metadata.artwork_url = format!("data:image/jpeg;base64,{}", b64);
                }
            } else if default_exists {
                if let Ok(bytes) = tokio::fs::read("default_cover_art.jpg").await {
                    let b64 = general_purpose::STANDARD.encode(&bytes);
                    metadata.artwork_url = format!("data:image/jpeg;base64,{}", b64);
                }
            } else {
                metadata.artwork_url = metadata.artwork_url.replace("100x100", "2000x2000");
            }

            history_list.push(metadata);
        }
    }

    Ok(history_list)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder
        ::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_project_info, get_history])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
