use std::path::Path;
use anyhow::Context;

mod youtube;
mod itunes;
mod tagger;

use youtube::download_youtube_audio;
use itunes::itunes_search;
use tagger::write_metadata;

fn main() -> anyhow::Result<()> {
    let url = "https://youtu.be/eZtlb9eegj0";
    let cookies = Some(Path::new("D:\\rust.etc\\EchoTag\\cookies.txt"));

    let download = download_youtube_audio(url, cookies)?;

    println!("Channel: {}", download.channel);
    println!("Title: {}", download.title);
    println!("Saved to: {}", download.file_path.display());

    let results = itunes_search(&download)?;

    if results.is_empty() {
        println!("iTunes returned 0 results.");
        return Ok(());
    }

    println!("iTunes found {} result(s):", results.len());
    for (i, meta) in results.iter().enumerate() {
        println!("\n--- Result #{} ---", i + 1);
        println!("Track: {}", meta.track_name);
        println!("Artist: {}", meta.artist_name);
        println!("Album: {}", meta.collection_name);
        println!("Genre: {}", meta.primary_genre);
        println!("Artwork: {}", meta.artwork_url100);
    }

    write_metadata(&results[0], &download.file_path).context(
        "Failed to write metadata to the downloaded file"
    )?;

    println!("Metadata including cover art saved to {} correctly.", download.title);

    Ok(())
}
