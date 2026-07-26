use anyhow::Context;
use std::path::Path;
use tokio::io::{ AsyncBufReadExt, AsyncWriteExt, BufReader };
use crate::models::Metadata;

pub async fn append_to_history(meta: &Metadata) -> anyhow::Result<()> {
    let path = Path::new("./history.json");

    if let Ok(file) = tokio::fs::File::open(path).await {
        let mut reader = BufReader::new(file).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(existing) = serde_json::from_str::<Metadata>(&line) {
                if
                    existing.track_name == meta.track_name &&
                    existing.artist_name == meta.artist_name
                {
                    return Ok(());
                }
            }
        }
    }

    let json_line = serde_json::to_string(meta).context("Failed to serialize metadata")?;

    let mut file = tokio::fs::OpenOptions
        ::new()
        .create(true)
        .append(true)
        .open(path).await
        .context("Failed to open history.json")?;

    file.write_all(json_line.as_bytes()).await?;
    file.write_all(b"\n").await?;

    Ok(())
}
