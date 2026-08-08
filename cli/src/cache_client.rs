use anyhow::{ bail, Context };
use std::path::{ Path, PathBuf };
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use reqwest::StatusCode;
use shared::models::Metadata;

fn build_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().expect("Failed to build HTTP client")
}

fn get_server_url() -> anyhow::Result<String> {
    dotenvy
        ::dotenv()
        .context(
            "Failed to load .env file. Please run 'echotag config --setup-cache-server' first."
        )?;
    let ip = std::env
        ::var("CACHE_SERVER_IP")
        .context(
            "CACHE_SERVER_IP not found in .env, Please run 'echotag config --setup-cache-server' first."
        )?;
    Ok(format!("http://{}:3000", ip))
}

pub async fn try_download_from_cache(
    video_id: &str,
    save_dir: &Path
) -> anyhow::Result<Option<PathBuf>> {
    let url = format!("{}/cache/{}", get_server_url()?, video_id);

    let response = build_client().get(&url).send().await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        bail!("{}", response.status());
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
    let url = format!("{}/cache/{}/claim", get_server_url()?, video_id);

    let client = reqwest::Client::new();
    let response = client.post(&url).send().await?;

    Ok(response.status().is_success())
}

pub async fn upload_to_cache(
    video_id: &str,
    file_path: &Path,
    metadata: &Metadata
) -> anyhow::Result<()> {
    let url = format!("{}/cache/{}/upload", get_server_url()?, video_id);

    let file_bytes = tokio::fs::read(file_path).await?;

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| format!("Couldn't extract file name from {}", file_path.display()))?
        .to_string();

    let metadata_json = serde_json::to_string(metadata)?;

    let form = reqwest::multipart::Form
        ::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_bytes).file_name(filename).mime_str("audio/mpeg")?
        )
        .part(
            "metadata",
            reqwest::multipart::Part::text(metadata_json).mime_str("application/json")?
        );

    let response = build_client().post(&url).multipart(form).send().await?;

    if !response.status().is_success() {
        bail!("{}", response.status());
    }

    Ok(())
}

pub async fn get_cached_metadata(video_id: &str) -> anyhow::Result<Option<Metadata>> {
    let url = format!("{}/cache/{}/metadata", get_server_url()?, video_id);

    let response = build_client().get(&url).send().await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        bail!("{}", response.status());
    }

    let metadata = response.json::<Metadata>().await?;

    Ok(Some(metadata))
}
