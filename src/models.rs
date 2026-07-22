use std::path::PathBuf;
use tokio::sync::{ mpsc, oneshot };
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

#[allow(dead_code)]
pub enum DownloadEvent {
    Progress {
        downloaded_bytes: u64,
        total_bytes: u64,
        speed_bytes_per_sec: u64,
        eta_seconds: u64,
    },
    Finished(AudioDownload),
    Error(anyhow::Error),
}

pub struct DownloadEventStream {
    pub rx: mpsc::Receiver<DownloadEvent>,
    pub cancel: Option<oneshot::Sender<()>>,
}
