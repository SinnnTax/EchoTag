use std::process::Command;
use std::process::Output;
use std::io::Result;

fn download_youtube(url: String, cookies: Option<String>) -> Result<Output> {
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

    if let Some(cookie) = cookies {
        ytdlp.args(["--cookies", &cookie]);
    }

    ytdlp.arg(url);

    ytdlp.output()
}

fn main() {
    let result = download_youtube(
        "https://youtu.be/eZtlb9eegj0".to_string(),
        Some("D:\\rust.etc\\EchoTag\\cookies.txt".to_string())
    );

    match result {
        Ok(output) => println!("{}", String::from_utf8_lossy(&output.stdout)),
        Err(e) => eprintln!("Error: {}", e),
    }
}
