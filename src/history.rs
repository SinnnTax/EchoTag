use anyhow::Context;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;
use crate::models::Metadata;

pub async fn append_to_history(meta: &Metadata, path: Option<&Path>) -> anyhow::Result<()> {
    let path = path.unwrap_or_else(|| Path::new("./history.ndjson"));

    if path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            for line in content.lines() {
                if let Ok(existing) = serde_json::from_str::<Metadata>(line) {
                    if
                        existing.track_name == meta.track_name &&
                        existing.artist_name == meta.artist_name
                    {
                        return Ok(());
                    }
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
        .context("Failed to open history.ndjson")?;

    file.write_all(json_line.as_bytes()).await?;
    file.write_all(b"\n").await?;

    Ok(())
}

pub async fn get_history_prompt(path: Option<&Path>) -> anyhow::Result<String> {
    let path = path.unwrap_or_else(|| Path::new("./history.ndjson"));

    if !path.exists() {
        return Ok(
            "You are a helpful music recommendation engine. \
            The user has no listening history yet. \
            Suggest three songs that recently became popular and are currently trending.".to_string()
        );
    }

    let content = tokio::fs::read_to_string(path).await?;

    let mut artist_counts: HashMap<String, u32> = HashMap::new();
    let mut all_tracks: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Ok(meta) = serde_json::from_str::<Metadata>(line) {
            *artist_counts.entry(meta.artist_name.clone()).or_insert(0) += 1;
            all_tracks.push(format!("{} - {}", meta.artist_name, meta.track_name));
        }
    }

    let mut top_artists: Vec<(String, u32)> = artist_counts.into_iter().collect();
    top_artists.sort_by(|a, b| b.1.cmp(&a.1)); // Sort descending by count

    let num_artists = top_artists.len();
    let top_artists_str = top_artists
        .iter()
        .take(15)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    let num_tracks = all_tracks.len();
    let last_tracks_str = all_tracks.iter().rev().take(10).cloned().collect::<Vec<_>>().join("\n");

    let artist_section = if num_artists == 1 {
        format!("Their most listened to artist is: {}.", top_artists_str)
    } else {
        format!(
            "Their top {} most listened to artists are: {}.",
            num_artists.min(15),
            top_artists_str
        )
    };

    let track_section = if num_tracks == 1 {
        format!("Here is the last track they downloaded:\n{}", last_tracks_str)
    } else {
        format!(
            "Here are the last {} tracks they downloaded:\n{}",
            num_tracks.min(10),
            last_tracks_str
        )
    };

    let prompt = format!(
        "You are a helpful music recommendation engine. \
        The user has downloaded {} song(s) total. \
        {} \
        {} \
        Based on their taste, suggest 3 new songs they might like.",
        num_tracks,
        artist_section,
        track_section
    );

    Ok(prompt)
}
