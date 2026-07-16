use serde::Deserialize;
use crate::youtube::AudioDownload;
use anyhow::Context;

#[derive(Deserialize)]
pub struct Metadata {
    #[serde(rename(deserialize = "artistName"))]
    pub artist_name: String,

    #[serde(rename(deserialize = "collectionName"))]
    pub collection_name: String,

    #[serde(rename(deserialize = "trackName"))]
    pub track_name: String,

    #[serde(rename(deserialize = "artworkUrl100"))]
    pub artwork_url100: String,

    #[serde(rename(deserialize = "primaryGenreName"))]
    pub primary_genre: String,
}

// the iTunes API wraps the array of songs inside an outer object so
// we use this struct to deserialize that outer object then we can
// extract the 'results' array inside.
#[derive(Deserialize)]
struct ItunesResponse {
    results: Vec<Metadata>,
}

pub fn itunes_search(music: &AudioDownload) -> anyhow::Result<Vec<Metadata>> {
    let itunes_endpoint = format!(
        "https://itunes.apple.com/search?media=music&entity=song&limit=5&term={} {}",
        music.channel,
        music.title
    );

    let results = reqwest::blocking
        ::get(&itunes_endpoint)
        .context("Failed to connect to iTunes API")?
        .json::<ItunesResponse>()
        .context("Failed to parse iTunes JSON response")?.results;

    Ok(results)
}
