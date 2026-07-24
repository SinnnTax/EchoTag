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

pub async fn claim_id(video_id: &str) -> anyhow::Result<bool> {
    let url = format!("{}/cache/{}/claim", SERVER_URL, video_id);

    let client = reqwest::Client::new();
    let response = client.post(&url).send().await?;

    Ok(response.status().is_success())
}

pub async fn upload_to_cache(video_id: &str, file_path: &Path) -> anyhow::Result<()> {
    let url = format!("{}/cache/{}/upload", SERVER_URL, video_id);

    let file_bytes = tokio::fs::read(file_path).await?;

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.mp3")
        .to_string();

    let form = reqwest::multipart::Form
        ::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_bytes).file_name(filename).mime_str("audio/mpeg")?
        );

    let client = reqwest::Client::new();
    let response = client.post(&url).multipart(form).send().await?;

    if !response.status().is_success() {
        bail!("Failed to upload to cache: {}", response.status());
    }

    Ok(())
}
