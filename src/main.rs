mod youtube;
mod itunes;
mod tagger;
mod cli;
mod metadata_provider;
mod models;
mod proxy;

use std::path::Path;
use anyhow::Context;
use clap::Parser;
use tokio::task::JoinSet;
use youtube::download_youtube_audio;
use itunes::ItunesProvider;
use tagger::{ write_metadata, rename_audio_file };
use metadata_provider::MetadataProvider;
use proxy::{ filter_proxy, get_proxy };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Download { urls, cookies } => {
            let mut set: JoinSet<anyhow::Result<()>> = JoinSet::new();

            for url in urls {
                let cookies = cookies.clone();

                set.spawn(async move {
                    println!("Starting download for: {}", url);

                    let download = download_youtube_audio(&url, Some(&cookies)).await?;

                    println!("Channel: {}", download.channel);
                    println!("Title: {}", download.title);

                    let results = ItunesProvider.find_metadata(&download).await?;

                    if results.is_empty() {
                        println!("iTunes returned 0 results for {}", url);
                        return Ok(());
                    }

                    write_metadata(&results[0], &download.file_path).await.context(
                        "Failed to write metadata to the downloaded file"
                    )?;

                    rename_audio_file(&download.file_path, &results[0]).await.with_context(||
                        format!("Failed to rename {:?}", &download.file_path)
                    )?;

                    println!("Successfully tagged: {}", download.title);

                    Ok(())
                });
            }

            // this loop ensures main doesn't exit until all downloads are done.
            while let Some(res) = set.join_next().await {
                res??;
            }
        }
        cli::Command::Update { paths } => {
            println!("Updating for {paths:?}");
        }
    }

    get_proxy(
        "https://raw.githubusercontent.com/iplocate/free-proxy-list/refs/heads/main/protocols/https.txt",
        Path::new("proxy.txt")
    ).await?;

    filter_proxy(Path::new("proxy.txt"), Path::new("filtered_proxy.txt")).await?;

    Ok(())
}
