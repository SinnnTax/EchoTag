mod youtube;
mod itunes;
mod tagger;
mod cli;
mod metadata_provider;
mod proxy;
mod cache_client;
mod config;
mod history;
mod ai;

use std::io::Write;
use anyhow::Context;
use clap::Parser;
use tokio::task::JoinSet;
use tokio::io::AsyncBufReadExt;
use indicatif::{
    ProgressBar,
    ProgressStyle,
    MultiProgress,
    MultiProgressAlignment,
    ProgressDrawTarget,
};
use youtube::{ download_youtube_audio, extract_video_id };
use itunes::ItunesProvider;
use tagger::{ write_metadata, rename_audio_file };
use metadata_provider::MetadataProvider;
use shared::models::{ DownloadEvent, Metadata };
use cache_client::{ try_download_from_cache, claim_id, upload_to_cache, get_cached_metadata };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Download { urls, cookies, skip_metadata_verification } => {
            let mut set: JoinSet<anyhow::Result<()>> = JoinSet::new();

            let mp = MultiProgress::new();
            mp.set_alignment(MultiProgressAlignment::Top);

            let stdin = tokio::io::stdin();
            let mut stdin_reader = tokio::io::BufReader::new(stdin);
            let mut skip_line = String::new();

            for url in urls {
                let mut video_id_opt: Option<String> = None;

                if let Some(video_id) = extract_video_id(&url) {
                    mp.println(format!("Checking cache for ID: {}", video_id))?;
                    let save_dir = std::path::Path::new("./");

                    match try_download_from_cache(&video_id, save_dir).await {
                        Ok(Some(path)) => {
                            mp.println(format!("Already cached! Downloaded to: {:?}", path))?;
                            match get_cached_metadata(&video_id).await {
                                Ok(Some(meta)) => {
                                    if let Err(e) = history::append_to_history(&meta, None).await {
                                        mp.println(format!("Failed to save to history: {:?}", e))?;
                                    }
                                }
                                Ok(None) =>
                                    mp.println(
                                        format!("No metadata found for {} in cache", path.display())
                                    )?,
                                Err(e) =>
                                    mp.println(
                                        format!(
                                            "Failed to fetch metadata from cache server: {:?}",
                                            e
                                        )
                                    )?,
                            }
                            continue;
                        }
                        Ok(None) => {
                            match claim_id(&video_id).await {
                                Ok(true) => {
                                    mp.println(format!("Successfully claimed ID {}", video_id))?;
                                }
                                Ok(false) => {
                                    mp.println(
                                        format!("ID {} is being downloaded by another user. Waiting...", video_id)
                                    )?;

                                    let mut got_file = false;
                                    let mut attempts = 0;

                                    // wait max 1 minutes
                                    while attempts < 12 {
                                        attempts += 1;
                                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                                        match try_download_from_cache(&video_id, save_dir).await {
                                            Ok(Some(path)) => {
                                                mp.println(
                                                    format!(
                                                        "Cache is now ready! Downloaded to: {:?}",
                                                        path
                                                    )
                                                )?;
                                                got_file = true;
                                                break;
                                            }
                                            Ok(None) => {
                                                if attempts % 4 == 0 {
                                                    mp.println(
                                                        format!("Still waiting for other user...")
                                                    )?;
                                                }
                                            }
                                            Err(e) => {
                                                mp.println(
                                                    format!(
                                                        "Cache error while waiting: {:?}. Falling back.",
                                                        e
                                                    )
                                                )?;
                                                break;
                                            }
                                        }
                                    }

                                    if got_file {
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    mp.println(
                                        format!("Failed to claim ID: {:?}. Proceeding anyway...", e)
                                    )?;
                                }
                            }
                        }
                        Err(e) => {
                            mp.println(
                                format!(
                                    "Cache server returned error: {:?}. Falling back to YouTube.",
                                    e
                                )
                            )?;
                        }
                    }
                    video_id_opt = Some(video_id);
                } else {
                    mp.println(format!("Could not extract YouTube ID from URL. Skipping cache."))?;
                }

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
                let mut skipped = false;

                skip_line.clear();

                loop {
                    tokio::select! {
                        event = stream.rx.recv() => {
                            if let Some(event) = event {
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
                                        mp.remove(&bar);
                                        mp.println(format!("Failed to download {}: {:?}", url, e))?;
                                        break;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        input = stdin_reader.read_line(&mut skip_line) => {
                            if input.is_ok() {
                                if let Some(cancel) = stream.cancel.take() {
                                    let _ = cancel.send(());
                                    skipped = true;
                                    mp.remove(&bar);
                                    mp.println(format!("Skipped {}", url))?;
                                    break;
                                }
                            }
                        }

                    }
                }

                if skipped {
                    continue;
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

                    bar.disable_steady_tick();
                    bar.set_draw_target(ProgressDrawTarget::hidden());
                    println!();

                    let mut results = ItunesProvider.find_metadata(&download).await?;
                    let mut metadata = if results.is_empty() {
                        mp.println(
                            format!(
                                "iTunes returned 0 results for {}.\nGoing with default settings.",
                                download.title
                            )
                        )?;
                        Metadata {
                            artist_name: download.channel.clone(),
                            track_name: download.title.clone(),
                            collection_name: "404".to_string(),
                            primary_genre: "404".to_string(),
                            artwork_url: "default_cover_art.jpg".to_string(),
                        }
                    } else {
                        results.remove(0)
                    };

                    let mut metadata_verified = skip_metadata_verification;
                    let mut input;

                    while !metadata_verified {
                        println!("--------------------------------------------------");
                        println!("Proposed metadata for '{}'", download.title);
                        println!("\tArtist: {}", metadata.artist_name);
                        println!("\tTrack:  {}", metadata.track_name);
                        println!("\tAlbum:  {}", metadata.collection_name);
                        println!("\tGenre:  {}", metadata.primary_genre);
                        println!("--------------------------------------------------");

                        input = prompt_user(
                            &mut stdin_reader,
                            "Is this correct? [y]es / [n]ew search / [m]anual entry: ",
                            false
                        ).await?;

                        let answer = input.trim().to_lowercase();

                        if answer.starts_with("y") {
                            metadata_verified = true;
                        } else if answer.starts_with("n") {
                            input = prompt_user(
                                &mut stdin_reader,
                                "Enter a search query to find the correct metadata (e.g. Artist - Song): ",
                                false
                            ).await?;

                            let query = input.trim().to_string();
                            if !query.is_empty() {
                                bar.set_message("Searching for Metadata...");
                                bar.enable_steady_tick(std::time::Duration::from_millis(100));
                                bar.set_draw_target(ProgressDrawTarget::stderr());
                                match ItunesProvider.search(&query).await {
                                    Ok(mut new_results) if !new_results.is_empty() => {
                                        metadata = new_results.remove(0);

                                        bar.disable_steady_tick();
                                        bar.set_draw_target(ProgressDrawTarget::hidden());
                                        println!();

                                        mp.println("Found new metadata. Reviewing...")?;
                                    }
                                    Ok(_) => {
                                        bar.disable_steady_tick();
                                        bar.set_draw_target(ProgressDrawTarget::hidden());
                                        println!();
                                        println!("No results found for that query.");
                                    }
                                    Err(e) => {
                                        bar.disable_steady_tick();
                                        bar.set_draw_target(ProgressDrawTarget::hidden());
                                        println!();

                                        println!("Search failed: {:?}", e);
                                    }
                                }
                            }
                        } else if answer.starts_with("m") {
                            println!("--- Manual Metadata Entry ---");
                            let artist_name = prompt_user(
                                &mut stdin_reader,
                                "Artist: ",
                                false
                            ).await?;
                            let track_name = prompt_user(
                                &mut stdin_reader,
                                "Track: ",
                                false
                            ).await?;
                            let collection_name = prompt_user(
                                &mut stdin_reader,
                                "Album: ",
                                false
                            ).await?;
                            let primary_genre = prompt_user(
                                &mut stdin_reader,
                                "Genre: ",
                                false
                            ).await?;

                            let artwork = prompt_user(
                                &mut stdin_reader,
                                "Artwork URL (leave empty for default): ",
                                true
                            ).await?;

                            let artwork_url = if artwork.trim().is_empty() {
                                "default_cover_art.jpg".to_string()
                            } else {
                                artwork.trim().to_string()
                            };

                            metadata = Metadata {
                                artist_name,
                                track_name,
                                collection_name,
                                primary_genre,
                                artwork_url,
                            };

                            metadata_verified = true;
                        } else {
                            println!("Invalid input. Please enter 'y', 'n', or 'm'.");
                        }
                    }

                    mp.remove(&bar);

                    let mp_clone = mp.clone();
                    let task_video_id = video_id_opt.clone();

                    set.spawn(async move {
                        let taggin_start = std::time::Instant::now();

                        write_metadata(&metadata, &download.file_path).await.context(
                            "Failed to write metadata to the downloaded file"
                        )?;

                        let final_file_path = rename_audio_file(
                            &download.file_path,
                            &metadata
                        ).await.with_context(||
                            format!("Failed to rename {:?}", &download.file_path)
                        )?;

                        let elapsed = taggin_start.elapsed();
                        mp_clone.println(
                            format!("Tagged \"{}\" in {:.2?} seconds", download.title, elapsed)
                        )?;

                        if let Some(vid) = task_video_id {
                            mp_clone.println(format!("Uploading to cache server..."))?;
                            match upload_to_cache(&vid, &final_file_path, &metadata).await {
                                Ok(_) =>
                                    mp_clone.println(format!("Successfully uploaded to cache!"))?,
                                Err(e) =>
                                    mp_clone.println(
                                        format!("Failed to upload to cache: {:?}", e)
                                    )?,
                            }
                        }

                        if let Err(e) = history::append_to_history(&metadata, None).await {
                            mp_clone.println(format!("Failed to save to history: {:?}", e))?;
                        }

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

            let mut directory_files = match tokio::fs::read_dir("./").await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to read directory for cleanup: {}", e);
                    std::process::exit(0);
                }
            };

            while let Ok(Some(entry)) = directory_files.next_entry().await {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str());

                if ext == Some("part") || ext == Some("webm") || ext == Some("ytdl") {
                    if tokio::fs::remove_file(&path).await.is_ok() {
                        println!("Cleaned up partial file: {}", path.display());
                    }
                }
            }

            std::process::exit(0);
        }
        cli::Command::Update { paths } => {
            println!("Updating for {paths:?}");
        }
        cli::Command::Config { setup_model, setup_cache_server } => {
            if setup_model {
                config::setup_provider()?;
            } else if setup_cache_server {
                config::setup_cache_server()?;
            } else {
                println!(
                    "Use --setup-model flag to configure your AI provider.\nUse --setup-cache-server flag to configure cache server."
                );
            }
        }
        cli::Command::Chat => {
            ai::start_chat().await?;
        }
    }

    Ok(())
}

async fn prompt_user(
    reader: &mut tokio::io::BufReader<tokio::io::Stdin>,
    prompt: &str,
    allow_empty: bool
) -> anyhow::Result<String> {
    loop {
        print!("{}", prompt);
        std::io::stdout().flush()?;
        let mut input = String::new();
        reader.read_line(&mut input).await?;

        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() || allow_empty {
            return Ok(trimmed);
        }
        println!("Field cannot be empty. Please try again.");
    }
}
