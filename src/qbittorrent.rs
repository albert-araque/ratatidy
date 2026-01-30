use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentInfo {
    pub hash: String,
    pub name: String,
    pub state: String,
    pub progress: f32,
    pub ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentFile {
    pub name: String,
    pub size: u64,
}

#[async_trait]
pub trait QbitClient: Send + Sync {
    async fn get_torrents(&self) -> anyhow::Result<Vec<TorrentInfo>>;
    async fn get_torrent_files(&self, hash: &str) -> anyhow::Result<Vec<TorrentFile>>;
}

pub struct MockQbitClient;

#[async_trait]
impl QbitClient for MockQbitClient {
    async fn get_torrents(&self) -> anyhow::Result<Vec<TorrentInfo>> {
        Ok(vec![
            TorrentInfo {
                hash: "hash_inception".to_string(),
                name: "Inception.2010.1080p.mkv".to_string(),
                state: "stalledUP".to_string(),
                progress: 1.0,
                ratio: 2.5,
            },
            TorrentInfo {
                hash: "hash_the_bear".to_string(),
                name: "The.Bear.S01.1080p".to_string(),
                state: "uploading".to_string(),
                progress: 1.0,
                ratio: 1.2,
            },
        ])
    }

    async fn get_torrent_files(&self, hash: &str) -> anyhow::Result<Vec<TorrentFile>> {
        match hash {
            "hash_inception" => Ok(vec![TorrentFile {
                name: "Inception.2010.1080p.mkv".to_string(),
                size: 1000,
            }]),
            "hash_the_bear" => Ok(vec![
                TorrentFile {
                    name: "The.Bear.S01.1080p/The.Bear.S01E01.mkv".to_string(),
                    size: 500,
                },
                TorrentFile {
                    name: "The.Bear.S01.1080p/The.Bear.S01E02.mkv".to_string(),
                    size: 500,
                },
            ]),
            _ => Ok(vec![]),
        }
    }
}
