use serde::Deserialize;
use anyhow::Context;
use crate::models::Metadata;
use crate::metadata_provider::MetadataProvider;

pub struct ItunesProvider;

// DTO for making Metadata domain model later
#[derive(Deserialize)]
struct ItunesTrack {
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

impl From<ItunesTrack> for Metadata {
    fn from(track: ItunesTrack) -> Self {
        Metadata {
            artist_name: track.artist_name,
            collection_name: track.collection_name,
            track_name: track.track_name,
            artwork_url: track.artwork_url100,
            primary_genre: track.primary_genre,
        }
    }
}

// the iTunes API wraps the array of songs inside an outer object so
// we use this struct to deserialize that outer object then we can
// extract the 'results' array inside.
#[derive(Deserialize)]
struct ItunesResponse {
    results: Vec<ItunesTrack>,
}

impl MetadataProvider for ItunesProvider {
    fn search(&self, query: &str) -> anyhow::Result<Vec<Metadata>> {
        let itunes_endpoint =
            format!("https://itunes.apple.com/search?media=music&entity=song&limit=5&term={}", query);

        let response = reqwest::blocking
            ::get(&itunes_endpoint)
            .context("Failed to connect to iTunes API")?
            .json::<ItunesResponse>()
            .context("Failed to parse iTunes JSON response")?.results;

        // converts Vec<ItunesTrack> into Vec<Metadata>
        let results = response
            .into_iter()
            .map(|track| track.into())
            .collect();

        Ok(results)
    }
}
