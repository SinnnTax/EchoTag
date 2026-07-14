use std::process::Command;

fn main() {
    let mut ytdlp = Command::new("yt-dlp");
    let ytdlp = ytdlp
        .args([
            "-x",
            "--audio-format",
            "mp3",
            "--audio-quality",
            "0",
            "-o",
            "%(title)s.%(ext)s",
            "-q",
            "--force-ipv4",
            "--cookies",
            "D:\\rust.etc\\EchoTag\\cookies.txt",
            "--print",
            "%(channel)s",
            "--print",
            "%(title)s",
            "--print",
            "after_move:filepath",
            "https://youtu.be/eZtlb9eegj0?si=gZ6SI0-qecfiTJLY",
        ])
        .output()
        .unwrap();
    println!("{}", String::from_utf8(ytdlp.stdout).unwrap());
}
