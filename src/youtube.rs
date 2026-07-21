use std::path::PathBuf;
use anyhow::{ Context, bail };
use tokio::process::Command;
use tokio::sync::mpsc;
use std::process::Stdio;
use tokio::io::{ BufReader, AsyncBufReadExt };
use regex::Regex;
use crate::models::{ AudioDownload, DownloadEvent, DownloadEventStream };

pub async fn download_youtube_audio(
    url: String,
    cookies_path: Option<PathBuf>,
    proxy: Option<String>
) -> DownloadEventStream {
    let (tx, rx) = mpsc::channel(64);

    tokio::spawn(async move {
        if let Err(e) = download(url, cookies_path, proxy, tx.clone()).await {
            let _ = tx.send(DownloadEvent::Error(e)).await;
        }
    });

    DownloadEventStream { rx }
}

async fn download(
    url: String,
    cookies_path: Option<PathBuf>,
    proxy: Option<String>,
    tx: mpsc::Sender<DownloadEvent>
) -> anyhow::Result<()> {
    let mut ytdlp = Command::new("yt-dlp");

    ytdlp.env("PYTHONUNBUFFERED", "1");

    ytdlp.args([
        "-x",
        "--audio-format",
        "mp3",
        "--audio-quality",
        "0",
        "-o",
        "%(title)s.%(ext)s",
        "-q",
        "--progress",
        "--newline",
        "--force-ipv4",
        "--retries",
        "1",
        "--socket-timeout",
        "3",
        "--print",
        "%(channel)s",
        "--print",
        "%(title)s",
        "--print",
        "after_move:filepath",
    ]);

    if let Some(path) = cookies_path {
        ytdlp.arg("--cookies").arg(path);
    }

    if let Some(p) = proxy {
        ytdlp.arg("--proxy").arg(p);
    }

    ytdlp.arg(url);

    let mut child = ytdlp.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    let stderr = child.stderr.take().context("Couldn't take stderr from ytdlp")?;
    let stdout = child.stdout.take().context("Couldn't take stdout from ytdlp")?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let stderr_task = tokio::spawn(async move {
        let mut lines = Vec::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            lines.push(line);
        }
        lines.join("\n")
    });

    let progress_re = Regex::new(
        r"\[download\]\s+([\d.]+)%\s+of\s+~?([\d.]+\w+)\s+at\s+([\d.]+\w+/s)\s+ETA\s+(\d{2}:\d{2})"
    )?;

    let mut channel = String::new();
    let mut title = String::new();
    let mut file_path_str = String::new();

    while let Ok(Some(line)) = stdout_reader.next_line().await {
        if let Some(caps) = progress_re.captures(&line) {
            let percentage: f64 = caps[1].parse().unwrap_or(0.0);
            let total_bytes = parse_size(&caps[2]);
            let speed_bytes_per_sec = parse_speed(&caps[3]);
            let eta_seconds = parse_eta(&caps[4]);

            let downloaded_bytes = ((total_bytes as f64) * (percentage / 100.0)) as u64;

            tx.send(DownloadEvent::Progress {
                downloaded_bytes,
                total_bytes,
                speed_bytes_per_sec,
                eta_seconds,
            }).await?;
        } else {
            if channel.is_empty() {
                channel = line;
            } else if title.is_empty() {
                title = line;
            } else {
                file_path_str = line;
            }
        }
    }

    let status = child.wait().await?;

    if !status.success() {
        let stderr_output = stderr_task.await.unwrap();
        bail!("yt-dlp failed ({}): {stderr_output}", status.code().unwrap_or(-1));
    }

    // if file path is empty so the other two are too
    if file_path_str.is_empty() {
        bail!("yt-dlp did not print the file path");
    }

    let file_path = PathBuf::from(file_path_str);

    let clean_channel = channel.split(',').next().unwrap_or(&channel).trim();
    let leftovers = ['-', ':', '|', ' '];
    let clean_title = title
        .replace(clean_channel, "")
        .trim_start_matches(leftovers)
        .trim()
        .to_string();

    let result = AudioDownload {
        channel: clean_channel.to_string(),
        title: clean_title,
        file_path: file_path,
    };

    tx.send(DownloadEvent::Finished(result)).await?;

    Ok(())
}

fn parse_size(s: &str) -> u64 {
    let num_end = s
        .trim()
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let num_str = &s[..num_end];
    let unit_str = s[num_end..].trim();

    let num: f64 = num_str.parse().unwrap_or(0.0);
    let multiplier = match unit_str {
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    (num * multiplier) as u64
}

// 58.90KiB/s -> 60313 bytes/sec
fn parse_speed(s: &str) -> u64 {
    let s = s.trim_end_matches("/s");
    parse_size(s)
}

// 01:20 -> 80 seconds
fn parse_eta(s: &str) -> u64 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let mins: u64 = parts[0].parse().unwrap_or(0);
            let secs: u64 = parts[1].parse().unwrap_or(0);
            mins * 60 + secs
        }
        3 => {
            let hours: u64 = parts[0].parse().unwrap_or(0);
            let mins: u64 = parts[1].parse().unwrap_or(0);
            let secs: u64 = parts[2].parse().unwrap_or(0);
            hours * 3600 + mins * 60 + secs
        }
        _ => 0,
    }
}
