use clap::Parser;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Directory where torrents are downloaded
    #[arg(short, long, env = "RATATIDY_DOWNLOAD_DIR")]
    pub download_dir: Option<PathBuf>,

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

impl QBittorrentConfig {
    pub fn is_configured(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }
}

impl Config {
    pub fn load() -> Self {
        Self::parse()
    }

    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "ratatidy").map(|dirs| dirs.config_dir().join("config.toml"))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let toml_str = toml::to_string_pretty(self)?;
            std::fs::write(&path, toml_str)?;
            println!("Config saved to: {}", path.display());
        }
        Ok(())
    }

    pub fn load_from_file() -> Option<Self> {
        let path = Self::config_path()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: None,
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
