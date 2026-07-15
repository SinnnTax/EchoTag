use std::io;
use std::path::{ Path, PathBuf };
use std::process::Command;
use serde::Deserialize;
use lofty::file::TaggedFileExt;
use lofty::tag::{ Accessor, Tag, TagExt };
use lofty::error::LoftyError;
use lofty::config::WriteOptions;

struct AudioDownload {
    channel: String,
    title: String,
    file_path: PathBuf,
}

#[derive(Deserialize)]
struct Metadata {
    #[serde(rename(deserialize = "artistName"))]
    artist_name: String,

    #[serde(rename(deserialize = "collectionName"))]
    collection_name: String,

    #[serde(rename(deserialize = "trackName"))]
    track_name: String,

    #[serde(rename(deserialize = "artworkUrl100"))]
    artwork_url100: String,

    #[serde(rename(deserialize = "primaryGenreName"))]
    primary_genre: String,
}

// the iTunes API wraps the array of songs inside an outer object so
// we use this struct to deserialize that outer object then we can
// extract the 'results' array inside.
#[derive(Deserialize)]
struct ItunesResponse {
    results: Vec<Metadata>,
}

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

fn itunes_search(music: &AudioDownload) -> Result<Vec<Metadata>, reqwest::Error> {
    let itunes_endpoint = format!(
        "https://itunes.apple.com/search?media=music&entity=song&limit=5&term={} {}",
        music.channel,
        music.title
    );

    Ok(reqwest::blocking::get(itunes_endpoint)?.json::<ItunesResponse>()?.results)
}

fn write_metadata(metadata: &Metadata, path: &Path) -> Result<(), LoftyError> {
    // read the file to determine its format to extract any existing tags
    let mut tagged_file = lofty::read_from_path(path)?;

    // get the primary tag for this specific file format
    let tag = match tagged_file.primary_tag_mut() {
        // if the file already has a primary tag (e.g., ID3v2 for MP3), use it
        Some(primary_tag) => primary_tag,

        None => {
            // If no primary tag exists, ask lofty what the best tag type
            // is for this file format, and create a new one
            let tag_type = tagged_file.primary_tag_type();

            tagged_file.insert_tag(Tag::new(tag_type));

            // now that the new empty tag is inserted retrieve it for editing
            tagged_file.primary_tag_mut().unwrap()
        }
    };

    tag.set_artist(metadata.artist_name.clone());
    tag.set_album(metadata.collection_name.clone());
    tag.set_title(metadata.track_name.clone());
    tag.set_genre(metadata.primary_genre.clone());

    tag.save_to_path(path, WriteOptions::default())?;

    Ok(())
}

fn main() {
    let url = "https://youtu.be/eZtlb9eegj0";
    let cookies = Some(Path::new("D:\\rust.etc\\EchoTag\\cookies.txt"));

    match download_youtube_audio(url, cookies) {
        Ok(download) => {
            println!("Channel: {}", download.channel);
            println!("Title: {}", download.title);
            println!("Saved to: {}", download.file_path.display());

            match itunes_search(&download) {
                Ok(results) => {
                    if results.is_empty() {
                        println!("iTunes returned 0 results.");
                    } else {
                        println!("iTunes found {} result(s):", results.len());

                        for (i, meta) in results.iter().enumerate() {
                            println!("\n--- Result #{} ---", i + 1);
                            println!("Track: {}", meta.track_name);
                            println!("Artist: {}", meta.artist_name);
                            println!("Album: {}", meta.collection_name);
                            println!("Genre: {}", meta.primary_genre);
                            println!("Artwork: {}", meta.artwork_url100);
                        }

                        match write_metadata(&results[0], &download.file_path) {
                            Ok(_) =>
                                println!("Metadata saved to {}.mp3 correctly.", download.title),
                            Err(e) =>
                                eprintln!(
                                    "Failed to write metadata into {}.mp3: {}",
                                    download.title,
                                    e
                                ),
                        }
                    }
                }
                Err(e) => eprintln!("iTunes request failed: {e}"),
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
