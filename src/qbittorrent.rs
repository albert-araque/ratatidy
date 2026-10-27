use crate::config::QBittorrentConfig;
use crate::scanner::FileNode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Same name heuristic the TUI uses: a path that contains the torrent name takes its hash and seed state.
/// The last matching torrent wins.
pub fn match_torrents(nodes: &mut [FileNode], torrents: &[TorrentInfo]) {
    for node in nodes {
        for path in &node.paths {
            let path_str = path.to_string_lossy();
            for torrent in torrents {
                if path_str.contains(&torrent.name) {
                    node.torrent_hash = Some(torrent.hash.clone());
                    node.is_seeding =
                        torrent.state.contains("UP") || torrent.state.contains("uploading");
                }
            }
        }
    }
}

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

/// Real client when its API answers, mock otherwise (offline mode).
/// Credentials are optional: qBittorrent can skip auth for localhost.
pub async fn connect(config: &QBittorrentConfig, use_mock: bool) -> Box<dyn QbitClient> {
    if use_mock {
        return Box::new(MockQbitClient);
    }
    match RealQbitClient::new(
        &config.url,
        config.username.clone(),
        config.password.clone(),
    )
    .await
    {
        Ok(client) if client.get_torrents().await.is_ok() => Box::new(client),
        _ => Box::new(MockQbitClient),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::FileKey;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn node(path: &str) -> FileNode {
        FileNode {
            key: FileKey { dev: 1, inode: 1 },
            size: 10,
            nlink: 1,
            paths: vec![PathBuf::from(path)],
            has_downloads: true,
            has_media: false,
            torrent_hash: None,
            is_seeding: false,
            modified: Some(SystemTime::UNIX_EPOCH),
        }
    }

    #[test]
    fn match_torrents_uses_the_path_and_the_last_name_hit() {
        let mut nodes = vec![node("/downloads/Inception.2010.1080p/movie.mkv")];
        let torrents = vec![
            TorrentInfo {
                hash: "first".into(),
                name: "Inception.2010".into(),
                state: "downloading".into(),
                progress: 0.5,
                ratio: 0.0,
            },
            TorrentInfo {
                hash: "second".into(),
                name: "Inception.2010.1080p".into(),
                state: "stalledUP".into(),
                progress: 1.0,
                ratio: 1.0,
            },
        ];
        match_torrents(&mut nodes, &torrents);
        assert_eq!(nodes[0].torrent_hash.as_deref(), Some("second"));
        assert!(nodes[0].is_seeding);

        let mut missed = vec![node("/downloads/Other/movie.mkv")];
        match_torrents(&mut missed, &torrents);
        assert!(missed[0].torrent_hash.is_none());
        assert!(!missed[0].is_seeding);
    }
}
