use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub download_dir: PathBuf,
    pub media_dirs: Vec<PathBuf>,
    pub delete_mode: DeleteMode,
    pub trash_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub video_extensions: Vec<String>,
    pub qbittorrent: QBittorrentConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeleteMode {
    Container,
    File,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QBittorrentConfig {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("/downloads"),
            media_dirs: vec![
                PathBuf::from("/media/movies"),
                PathBuf::from("/media/tvshows"),
            ],
            delete_mode: DeleteMode::Container,
            trash_dir: None,
            dry_run: true,
            video_extensions: vec![
                "mkv".to_string(),
                "mp4".to_string(),
                "avi".to_string(),
                "mov".to_string(),
            ],
            qbittorrent: QBittorrentConfig {
                url: "http://localhost:8080".to_string(),
                username: None,
                password: None,
            },
        }
    }
}
