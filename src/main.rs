mod youtube;
mod itunes;
mod tagger;
mod cli;
mod metadata_provider;
mod models;
mod proxy;

use anyhow::Context;
use clap::Parser;
use tokio::task::JoinSet;
use youtube::download_youtube_audio;
use itunes::ItunesProvider;
use tagger::{ write_metadata, rename_audio_file };
use metadata_provider::MetadataProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Download { urls, cookies } => {
            let mut set: JoinSet<anyhow::Result<()>> = JoinSet::new();

            // downloading sequentially to avoid youtube's anti-bot 429 error
            for url in urls {
                println!("Starting download for: {}", url);

                let download = match download_youtube_audio(&url, Some(&cookies), None).await {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Failed to download {}: {:?}", url, e);
                        continue;
                    }
                };

                println!("Downloaded: {} - {}", download.channel, download.title);

                set.spawn(async move {
                    let results = ItunesProvider.find_metadata(&download).await?;

                    if results.is_empty() {
                        println!("iTunes returned 0 results for {}.", download.title);
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

            // waiting for all the background tagging tasks to finish before the program exits
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => eprintln!("A tagging task failed: {:?}", e),
                    Err(join_err) => eprintln!("A tagging task panicked: {:?}", join_err),
                }
            }
        }
        cli::Command::Update { paths } => {
            println!("Updating for {paths:?}");
        }
    }

    Ok(())
}
