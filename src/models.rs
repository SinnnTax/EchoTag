use std::path::PathBuf;

#[derive(Clone)]
pub struct Metadata {
    pub artist_name: String,
    pub collection_name: String,
    pub track_name: String,
    pub artwork_url: String,
    pub primary_genre: String,
}

#[derive(Clone)]
pub struct AudioDownload {
    pub channel: String,
    pub title: String,
    pub file_path: PathBuf,
}
