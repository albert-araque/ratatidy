//! `ratatidy agent <command>`: what the TUI runs over SSH on a Remote.
//! Each command is one-shot and answers in JSON on stdout.

use crate::config::Config;
use crate::host::{delete_local, write_json_line};
use crate::qbittorrent::{self, TorrentInfo};
use crate::scanner::{ScanEvent, Scanner};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Bump on any incompatible change to the messages below
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentInfo {
    pub protocol_version: u32,
    pub download_dir: PathBuf,
    pub media_dirs: Vec<PathBuf>,
    pub torrents: Vec<TorrentInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub paths: Vec<PathBuf>,
    pub hashes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub errors: Vec<String>,
}

pub async fn run(command: Option<&str>) -> Result<()> {
    let config = Config::load_from_file().unwrap_or_else(Config::from_env);
    let download_dir = config
        .download_dir
        .clone()
        .ok_or_else(|| anyhow!("download_dir is not configured on this host"))?;
    // Buffered: one write per scanned file would be a syscall each
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());

    match command {
        Some("info") => {
            let qbit = qbittorrent::connect(&config.qbittorrent, false).await;
            let info = AgentInfo {
                protocol_version: PROTOCOL_VERSION,
                download_dir,
                media_dirs: config.media_dirs,
                torrents: qbit.get_torrents().await.unwrap_or_default(),
            };
            write_json_line(&mut out, &info)?;
        }
        Some("scan") => {
            let (tx, rx) = std::sync::mpsc::channel();
            Scanner::new(download_dir, config.media_dirs).scan_async(tx);
            for event in rx {
                let done = !matches!(event, ScanEvent::FileScanned);
                write_json_line(&mut out, &event)?;
                if done {
                    break;
                }
            }
        }
        Some("delete") => {
            let mut body = Vec::new();
            std::io::stdin().read_to_end(&mut body)?;
            let request: DeleteRequest = serde_json::from_slice(&body)?;
            let qbit = qbittorrent::connect(&config.qbittorrent, false).await;
            let response = delete_local(&request, qbit.as_ref()).await;
            write_json_line(&mut out, &response)?;
        }
        other => {
            return Err(anyhow!(
                "Unknown agent command {:?}; expected info, scan or delete",
                other.unwrap_or("")
            ));
        }
    }
    out.flush()?;
    Ok(())
}
