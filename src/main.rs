mod youtube;
mod itunes;
mod tagger;
mod cli;
mod metadata_provider;
mod models;

use anyhow::Context;
use clap::Parser;
use youtube::download_youtube_audio;
use itunes::ItunesProvider;
use tagger::{ write_metadata, rename_audio_file };
use metadata_provider::MetadataProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Download { urls, cookies } => {
            for url in urls {
                println!("Starting download for: {}", url);

                let download = download_youtube_audio(&url, Some(&cookies)).await?;

                println!("Channel: {}", download.channel);
                println!("Title: {}", download.title);

                let results = ItunesProvider.find_metadata(&download).await?;

                if results.is_empty() {
                    println!("iTunes returned 0 results for {}.", url);
                    continue; // skip writing metadata
                }

                write_metadata(&results[0], &download.file_path).await.context(
                    "Failed to write metadata to the downloaded file"
                )?;

                rename_audio_file(&download.file_path, &results[0]).await.with_context(||
                    format!("Failed to rename {:?}", &download.file_path)
                )?;

                println!("Successfully tagged: {}", download.title);
            }
        }
        cli::Command::Update { paths } => {
            println!("Updating for {paths:?}");
        }
    }

    Ok(())
}
