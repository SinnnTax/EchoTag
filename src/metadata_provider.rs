use crate::models::{ Metadata, AudioDownload };

pub trait MetadataProvider {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<Metadata>>;

    async fn find_metadata(&self, music: &AudioDownload) -> anyhow::Result<Vec<Metadata>> {
        let mut query = format!("{} {}", music.channel, music.title);

        for _ in 0..7 {
            let results = self.search(&query).await?;

            // if we got results return them immediately
            if !results.is_empty() {
                return Ok(results);
            }

            // if no results then try to chop off the last word
            match query.rfind(' ') {
                Some(index) => {
                    query.truncate(index);
                    query = query.trim().to_string();
                }
                None => {
                    // no spaces left to shorten anymore
                    break;
                }
            }
        }

        Ok(Vec::new())
    }
}
