use std::path::{ Path, PathBuf };
use std::process::Command;
use anyhow::bail;
use crate::models::AudioDownload;

pub fn download_youtube_audio(
    url: &str,
    cookies_path: Option<&Path>
) -> anyhow::Result<AudioDownload> {
    let mut ytdlp = Command::new("yt-dlp");

    ytdlp.args([
        "-x",
        "--audio-format",
        "mp3",
        "--audio-quality",
        "0",
        "-o",
        "%(title)s.%(ext)s",
        "-q",
        "--force-ipv4",
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

    ytdlp.arg(url);

    let output = ytdlp.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let error_msg = format!("yt-dlp failed ({}): {stderr}", output.status.code().unwrap_or(-1));
        bail!(error_msg);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 3 {
        let error_msg = format!("Expected 3 lines of output, got {}", lines.len());
        bail!(error_msg);
    }

    let channel = lines[0];
    let title = lines[1];

    let clean_channel = channel.split(',').next().unwrap_or(channel).trim();
    let leftovers = ['-', ':', '|', ' '];
    let clean_title = title
        .replace(clean_channel, "")
        .trim_start_matches(leftovers)
        .trim()
        .to_string();

    Ok(AudioDownload {
        channel: clean_channel.to_string(),
        title: clean_title,
        file_path: PathBuf::from(lines[2]),
    })
}
