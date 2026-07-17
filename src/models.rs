use std::path::PathBuf;

pub struct Metadata {
    pub artist_name: String,
    pub collection_name: String,
    pub track_name: String,
    pub artwork_url: String,
    pub primary_genre: String,
}

pub struct AudioDownload {
    pub channel: String,
    pub title: String,
    pub file_path: PathBuf,
}
