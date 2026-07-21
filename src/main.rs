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
use indicatif::{ ProgressBar, ProgressStyle, MultiProgress };
use youtube::download_youtube_audio;
use itunes::ItunesProvider;
use tagger::{ write_metadata, rename_audio_file };
use metadata_provider::MetadataProvider;
use models::{ DownloadEvent };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Download { urls, cookies } => {
            let mut set: JoinSet<anyhow::Result<()>> = JoinSet::new();

            let mp = MultiProgress::new();

            // downloading sequentially to avoid youtube's anti-bot 429 error
            for url in urls {
                mp.println(format!("Starting download for: {}", url))?;

                let cookies = cookies.clone();
                let download_start = std::time::Instant::now();
                let mut stream = download_youtube_audio(url.to_string(), Some(cookies), None);

                let bar = mp.add(ProgressBar::new(1));
                bar.set_style(
                    ProgressStyle::with_template(
                        "{spinner:.green.bold} {msg:.bold} [{elapsed_precise}] {bar:50.green/black.dim} {bytes}/{total_bytes} ({percent}%) {bytes_per_sec} {eta:.dim}  "
                    )?
                        .progress_chars("█▛▌▖  ")
                        .tick_chars("/|\\- ")
                );
                bar.set_message("Downloading audio");
                bar.enable_steady_tick(std::time::Duration::from_millis(100));

                let mut downloaded_audio = None;

                let mut download_size = 0;
                while let Some(event) = stream.rx.recv().await {
                    match event {
                        DownloadEvent::Progress { downloaded_bytes, total_bytes, .. } => {
                            bar.set_length(total_bytes);
                            bar.set_position(downloaded_bytes);

                            if total_bytes == downloaded_bytes && total_bytes > 0 {
                                bar.disable_steady_tick();
                                bar.set_message("Processing audio…");
                                download_size = total_bytes;
                            }
                        }
                        DownloadEvent::Finished(audio) => {
                            downloaded_audio = Some(audio);
                            break;
                        }
                        DownloadEvent::Error(e) => {
                            bar.finish_and_clear();
                            mp.println(format!("Failed to download {}: {:?}", url, e))?;
                            break;
                        }
                    }
                }
                if let Some(download) = downloaded_audio {
                    let elapsed = download_start.elapsed();
                    let avg_speed = (download_size as f64) / elapsed.as_secs_f64();

                    let gb = 1024.0 * 1024.0 * 1024.0;
                    let mb = 1024.0 * 1024.0;

                    let speed_str = if avg_speed / gb >= 1.0 {
                        format!("{:.2}GB/s", avg_speed / gb)
                    } else {
                        format!("{:.2}MB/s", avg_speed / mb)
                    };

                    mp.println(
                        format!(
                            "Downloaded \"{}\" in {:.2?} (avg {})",
                            download.title,
                            elapsed,
                            speed_str
                        )
                    )?;

                    let mp_clone = mp.clone();
                    let bar_clone = bar.clone();

                    set.spawn(async move {
                        let taggin_start = std::time::Instant::now();
                        let results = ItunesProvider.find_metadata(&download).await?;

                        if results.is_empty() {
                            mp_clone.println(
                                format!("iTunes returned 0 results for {}.", download.title)
                            )?;
                            mp_clone.remove(&bar_clone);
                            return Ok(());
                        }

                        write_metadata(&results[0], &download.file_path).await.context(
                            "Failed to write metadata to the downloaded file"
                        )?;

                        rename_audio_file(&download.file_path, &results[0]).await.with_context(||
                            format!("Failed to rename {:?}", &download.file_path)
                        )?;

                        let elapsed = taggin_start.elapsed();
                        mp_clone.remove(&bar_clone);
                        mp_clone.println(
                            format!("Tagged \"{}\" in {:.2?} seconds", download.title, elapsed)
                        )?;

                        Ok(())
                    });
                }
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
