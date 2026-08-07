use anyhow::Context;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;
use shared::models::Metadata;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_metadata(artist: &str, track: &str) -> Metadata {
        Metadata {
            artist_name: artist.to_string(),
            track_name: track.to_string(),
            collection_name: "Test Album".to_string(),
            artwork_url: "http://example.com/art.jpg".to_string(),
            primary_genre: "Test Genre".to_string(),
        }
    }

    #[tokio::test]
    async fn append_to_history_writes_and_prevents_duplicates() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push("echotag_history_test.ndjson");

        let metadata = make_test_metadata("Yeat", "Nvr Again");

        append_to_history(&metadata, Some(&temp_path)).await.expect("First append failed");

        assert!(temp_path.exists(), "History file was not created");

        let content1 = fs::read_to_string(&temp_path).expect("Failed to read file");
        assert!(content1.contains("Nvr Again"), "Track is missing");

        append_to_history(&metadata, Some(&temp_path)).await.expect("Second append failed");

        let content2 = fs::read_to_string(&temp_path).expect("Failed to read file");
        assert_eq!(content1, content2, "A duplicate entry was written!");

        let _ = fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn get_history_prompt_returns_default_when_no_file() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push("echotag_history_nonexistent.ndjson");
        let _ = fs::remove_file(&temp_path);

        let prompt = get_history_prompt(Some(&temp_path)).await.expect(
            "get_history_prompt should succeed even if the file does not exist"
        );

        assert!(prompt.contains("no listening history yet"), "Default prompt was wrong");
    }

    #[tokio::test]
    async fn get_history_prompt_counts_artists_and_tracks_correctly() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push("echotag_history_logic_test.ndjson");
        let _ = fs::remove_file(&temp_path);

        let meta1 = make_test_metadata("Yeat", "2093");
        let meta2 = make_test_metadata("Yeat", "Pulled In First");
        let meta3 = make_test_metadata("Yeat", "Nvr Again");

        append_to_history(&meta1, Some(&temp_path)).await.unwrap();
        append_to_history(&meta2, Some(&temp_path)).await.unwrap();
        append_to_history(&meta3, Some(&temp_path)).await.unwrap();

        let prompt = get_history_prompt(Some(&temp_path)).await.unwrap();

        assert!(prompt.contains("3 song(s) total"), "Wrong total song count");
        assert!(prompt.contains("Yeat"), "Top artist missing from prompt");
        assert!(prompt.contains("2093"), "Track missing from prompt");

        let _ = fs::remove_file(&temp_path);
    }
}
