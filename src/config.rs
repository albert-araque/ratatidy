use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Directory where torrents are downloaded
    #[arg(short, long, env = "RATATIDY_DOWNLOAD_DIR")]
    pub download_dir: PathBuf,

    /// Directories containing your media library (comma separated)
    #[arg(short, long, env = "RATATIDY_MEDIA_DIRS", value_delimiter = ',')]
    pub media_dirs: Vec<PathBuf>,

    /// Deletion mode: container (folder) or file
    #[arg(long, default_value = "container", env = "RATATIDY_DELETE_MODE")]
    pub delete_mode: DeleteMode,

    /// Optional directory to move files to instead of deleting
    #[arg(long, env = "RATATIDY_TRASH_DIR")]
    pub trash_dir: Option<PathBuf>,

    /// Don't actually delete anything (dry run)
    #[arg(long, default_value_t = false, env = "RATATIDY_DRY_RUN")]
    pub dry_run: bool,

    /// Video file extensions to scan (comma separated)
    #[arg(long, value_delimiter = ',', default_value = "mkv,mp4,avi,mov")]
    pub video_extensions: Vec<String>,

    #[command(flatten)]
    pub qbittorrent: QBittorrentConfig,
}

#[derive(clap::ValueEnum, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeleteMode {
    Container,
    File,
}

#[derive(clap::Args, Debug, Serialize, Deserialize, Clone)]
pub struct QBittorrentConfig {
    /// qBittorrent Web UI URL
    #[arg(
        long = "qbit-url",
        default_value = "http://localhost:8080",
        env = "QBIT_URL"
    )]
    pub url: String,

    /// qBittorrent Username
    #[arg(long = "qbit-user", env = "QBIT_USER")]
    pub username: Option<String>,

    /// qBittorrent Password
    #[arg(long = "qbit-pass", env = "QBIT_PASS")]
    pub password: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        Self::parse()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("."),
            media_dirs: vec![],
            delete_mode: DeleteMode::Container,
            trash_dir: None,
            dry_run: false,
            video_extensions: vec!["mkv".into(), "mp4".into(), "avi".into(), "mov".into()],
            qbittorrent: QBittorrentConfig {
                url: "http://localhost:8080".into(),
                username: None,
                password: None,
            },
        }
    }
}
