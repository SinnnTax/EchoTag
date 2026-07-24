use anyhow::{ bail, Context };
use std::path::{ Path, PathBuf };
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use reqwest::StatusCode;

const SERVER_URL: &'static str = "http://127.0.0.1:3000";

pub async fn try_download_from_cache(
    video_id: &str,
    save_dir: &Path
) -> anyhow::Result<Option<PathBuf>> {
    let url = format!("{}/cache/{}", SERVER_URL, video_id);

    let response = reqwest::get(&url).await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        bail!("Cache server returned error: {}", response.status());
    }

    let filename = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|val| val.to_str().ok())
        .and_then(|s| s.split("filename=\"").nth(1))
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or("cached_song.mp3")
        .to_string();

    let file_path = save_dir.join(&filename);

    let mut file = tokio::fs::File
        ::create(&file_path).await
        .with_context(|| format!("Failed to create file {:?}", file_path))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to read chunk from network")?;

        file.write_all(&chunk).await.context("Failed to write chunk to disk")?;
    }

    file.flush().await?;

    Ok(Some(file_path))
}
