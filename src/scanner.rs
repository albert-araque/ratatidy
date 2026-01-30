use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileKey {
    pub dev: u64,
    pub inode: u64,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub key: FileKey,
    pub size: u64,
    pub nlink: u32,
    pub paths: Vec<PathBuf>,

    pub has_downloads: bool,
    pub has_media: bool,

    // qBittorrent enrichment (Phase 3)
    pub torrent_hash: Option<String>,
    pub is_seeding: bool,
}

pub struct Scanner {
    pub download_dir: PathBuf,
    pub media_dirs: Vec<PathBuf>,
}

impl Scanner {
    pub fn new(download_dir: PathBuf, media_dirs: Vec<PathBuf>) -> Self {
        Self {
            download_dir,
            media_dirs,
        }
    }

    pub fn scan(&self) -> Result<Vec<FileNode>> {
        let mut nodes: HashMap<FileKey, FileNode> = HashMap::new();

        // Scan downloads
        self.scan_dir(&self.download_dir, true, &mut nodes)?;

        // Scan media
        for media_dir in &self.media_dirs {
            self.scan_dir(media_dir, false, &mut nodes)?;
        }

        Ok(nodes.into_values().collect())
    }

    fn scan_dir(
        &self,
        path: &Path,
        is_download: bool,
        nodes: &mut HashMap<FileKey, FileNode>,
    ) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path();

            if metadata.is_dir() {
                self.scan_dir(&path, is_download, nodes)?;
            } else if metadata.is_file() {
                // Stable way to get file index on Windows using windows-sys
                let file = fs::File::open(&path)?;
                let handle = file.as_raw_handle();

                let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
                let success = unsafe { GetFileInformationByHandle(handle as _, &mut info) };

                if success == 0 {
                    continue;
                }

                let key = FileKey {
                    dev: info.dwVolumeSerialNumber as u64,
                    inode: ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64),
                };

                let node = nodes.entry(key).or_insert_with(|| FileNode {
                    key,
                    size: metadata.len(),
                    nlink: info.nNumberOfLinks,
                    paths: Vec::new(),
                    has_downloads: false,
                    has_media: false,
                    torrent_hash: None,
                    is_seeding: false,
                });

                node.paths.push(path);
                if is_download {
                    node.has_downloads = true;
                } else {
                    node.has_media = true;
                }
            }
        }
        Ok(())
    }
}
