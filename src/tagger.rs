use std::path::Path;
use lofty::file::TaggedFileExt;
use lofty::tag::{ Accessor, Tag, TagExt };
use lofty::config::WriteOptions;
use lofty::picture::{ MimeType, Picture, PictureType };
use anyhow::Context;
use crate::itunes::Metadata;

pub fn write_metadata(metadata: &Metadata, path: &Path) -> anyhow::Result<()> {
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

    let cover_at_path = format!("{}_{}.jpg", &metadata.artist_name, &metadata.track_name);
    let cover_art_path = Path::new(&cover_at_path);

    download_cover_art(Some(cover_art_path), &metadata.artwork_url100).context(
        "Failed to download cover art in write_metadata function."
    )?;

    let cover_art = std::fs
        ::read(cover_art_path)
        .with_context(|| format!("Failed to read {:?}", cover_art_path))?;

    let cover = Picture::unchecked(cover_art)
        .pic_type(PictureType::CoverFront)
        .mime_type(MimeType::Jpeg)
        .build();

    tag.set_picture(0, cover);

    tag.save_to_path(path, WriteOptions::default())?;

    Ok(())
}

fn download_cover_art(path: Option<&Path>, url: &str) -> anyhow::Result<u64> {
    let path = path.unwrap_or(Path::new("cover_art.jpg"));
    let mut file = std::fs::File
        ::create(path)
        .with_context(|| format!("Couldn't create {:?}", path))?;

    // change 100x100 to 2000x200 to get higher resolution picture
    let url = url.replace("100", "2000");

    Ok(
        reqwest::blocking
            ::get(&url)
            .with_context(|| format!("Couldn't connect to {}!", &url))?
            .copy_to(&mut file)?
    )
}

pub fn rename_audio_file(old_path: &Path, metadata: &Metadata) -> anyhow::Result<()> {
    let new_file_name = format!(
        "{} - {} - ({}).mp3",
        metadata.artist_name,
        metadata.track_name,
        metadata.collection_name
    );

    // if old_path is absolute, so the new_path should too
    let parent_dir = old_path
        .parent()
        .context("Could not determine parent directory of the downloaded file")?;

    let new_path = parent_dir.join(new_file_name);

    Ok(std::fs::rename(old_path, &new_path)?)
}
