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
#[allow(dead_code)]
pub struct TorrentFile {
    pub name: String,
    pub size: u64,
}

#[async_trait]
pub trait QbitClient: Send + Sync {
    async fn get_torrents(&self) -> anyhow::Result<Vec<TorrentInfo>>;
    #[allow(dead_code)]
    async fn get_torrent_files(&self, hash: &str) -> anyhow::Result<Vec<TorrentFile>>;
    async fn delete_torrent(&self, hash: &str, delete_files: bool) -> anyhow::Result<()>;
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

    async fn delete_torrent(&self, _hash: &str, _delete_files: bool) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct RealQbitClient {
    client: reqwest::Client,
    base_url: String,
}

impl RealQbitClient {
    pub async fn new(
        url: &str,
        username: Option<String>,
        password: Option<String>,
    ) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().cookie_store(true).build()?;

        let base_url = url.trim_end_matches('/').to_string();

        if let (Some(u), Some(p)) = (username, password) {
            let login_url = format!("{}/api/v2/auth/login", base_url);
            client
                .post(&login_url)
                .form(&[("username", &u), ("password", &p)])
                .send()
                .await?;
        }

        Ok(Self { client, base_url })
    }
}

#[async_trait]
impl QbitClient for RealQbitClient {
    async fn get_torrents(&self) -> anyhow::Result<Vec<TorrentInfo>> {
        let url = format!("{}/api/v2/torrents/info", self.base_url);
        let torrents = self.client.get(&url).send().await?.json().await?;
        Ok(torrents)
    }

    async fn get_torrent_files(&self, hash: &str) -> anyhow::Result<Vec<TorrentFile>> {
        let url = format!("{}/api/v2/torrents/files?hash={}", self.base_url, hash);
        let files = self.client.get(&url).send().await?.json().await?;
        Ok(files)
    }

    async fn delete_torrent(&self, hash: &str, delete_files: bool) -> anyhow::Result<()> {
        let url = format!("{}/api/v2/torrents/delete", self.base_url);
        self.client
            .post(&url)
            .form(&[("hashes", hash), ("deleteFiles", &delete_files.to_string())])
            .send()
            .await?;
        Ok(())
    }
}
