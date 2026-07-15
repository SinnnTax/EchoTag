use std::io;
use std::path::{ Path, PathBuf };
use std::process::Command;

struct AudioDownload {
    channel: String,
    title: String,
    file_path: PathBuf,
}

struct Metadata {}

fn download_youtube_audio(url: &str, cookies_path: Option<&Path>) -> io::Result<AudioDownload> {
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
        return Err(
            io::Error::new(
                io::ErrorKind::TimedOut,
                format!("yt-dlp failed ({}): {stderr}", output.status.code().unwrap_or(-1))
            )
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 3 {
        return Err(
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected 3 lines of output, got {}", lines.len())
            )
        );
    }
    let channel = lines[0];
    let title = lines[1];

    let clean_channel = channel.split(',').next().unwrap_or(channel).trim();
    let leftovers = ['-', ':', '|'];
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

fn itunes_search(music: AudioDownload) -> Result<String, reqwest::Error> {
    let itunes_endpoint = format!(
        "https://itunes.apple.com/search?media=music&entity=song&limit=5&term={} {}",
        music.channel,
        music.title
    );
    Ok(reqwest::blocking::get(itunes_endpoint)?.text()?)
}

fn main() {
    let url = "https://youtu.be/eZtlb9eegj0";
    let cookies = Some(Path::new("D:\\rust.etc\\EchoTag\\cookies.txt"));

    match download_youtube_audio(url, cookies) {
        Ok(download) => {
            println!("Channel: {}", download.channel);
            println!("Title: {}", download.title);
            println!("Saved to: {}", download.file_path.display());

            println!("iTunes Result:{}", itunes_search(download).unwrap());
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
