use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileKey {
    pub dev: u64,
    pub inode: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanEvent {
    FileScanned,
    Finished(Vec<FileNode>),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub key: FileKey,
    pub size: u64,
    pub nlink: u32,
    pub paths: Vec<PathBuf>,
    pub has_downloads: bool,
    pub has_media: bool,
    pub torrent_hash: Option<String>,
    pub is_seeding: bool,
    pub modified: Option<SystemTime>,
}

#[derive(Serialize, Deserialize, Default)]
struct ScanCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    key: FileKey,
    nlink: u32,
    size: u64,
    modified: Option<SystemTime>,
}

impl ScanCache {
    fn load() -> Self {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "ratatidy", "ratatidy") {
            let cache_path = proj_dirs.cache_dir().join("scan_cache.json");
            if cache_path.exists() {
                if let Ok(content) = fs::read_to_string(cache_path) {
                    return serde_json::from_str(&content).unwrap_or_default();
                }
            }
        }
        Self::default()
    }

    fn save(&self) -> Result<()> {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "ratatidy", "ratatidy") {
            let cache_dir = proj_dirs.cache_dir();
            fs::create_dir_all(cache_dir)?;
            let cache_path = cache_dir.join("scan_cache.json");
            let content = serde_json::to_string(self)?;
            fs::write(cache_path, content)?;
        }
        Ok(())
    }
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

    pub fn scan_async(&self, sender: std::sync::mpsc::Sender<ScanEvent>) {
        let download_dir = self.download_dir.clone();
        let media_dirs = self.media_dirs.clone();
        let scanner_clone = Self::new(download_dir.clone(), media_dirs.clone());

        std::thread::spawn(move || {
            let mut nodes: HashMap<FileKey, FileNode> = HashMap::new();
            let cache = Arc::new(Mutex::new(ScanCache::load()));

            if let Err(e) =
                scanner_clone.scan_async_dir(&download_dir, true, &mut nodes, &sender, &cache)
            {
                let _ = sender.send(ScanEvent::Error(e.to_string()));
                return;
            }

            for media_dir in &media_dirs {
                if let Err(e) =
                    scanner_clone.scan_async_dir(media_dir, false, &mut nodes, &sender, &cache)
                {
                    let _ = sender.send(ScanEvent::Error(e.to_string()));
                    return;
                }
            }

            // Save cache
            if let Ok(cache) = cache.lock() {
                let _ = cache.save();
            }

            let _ = sender.send(ScanEvent::Finished(nodes.into_values().collect()));
        });
    }

    fn scan_async_dir(
        &self,
        path: &Path,
        is_download: bool,
        nodes: &mut HashMap<FileKey, FileNode>,
        sender: &std::sync::mpsc::Sender<ScanEvent>,
        cache: &Arc<Mutex<ScanCache>>,
    ) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path();

            if metadata.is_dir() {
                self.scan_async_dir(&path, is_download, nodes, sender, cache)?;
            } else if metadata.is_file() {
                let mtime = metadata.created().ok().or_else(|| metadata.modified().ok());
                let size = metadata.len();

                let (key, nlink) = {
                    let mut cache_lock = cache.lock().unwrap();
                    if let Some(entry) = cache_lock.entries.get(&path) {
                        if entry.size == size && entry.modified == mtime {
                            (entry.key, entry.nlink)
                        } else {
                            let (key, nlink) = self.get_file_info(&path, &metadata)?;
                            cache_lock.entries.insert(
                                path.clone(),
                                CacheEntry {
                                    key,
                                    nlink,
                                    size,
                                    modified: mtime,
                                },
                            );
                            (key, nlink)
                        }
                    } else {
                        let (key, nlink) = self.get_file_info(&path, &metadata)?;
                        cache_lock.entries.insert(
                            path.clone(),
                            CacheEntry {
                                key,
                                nlink,
                                size,
                                modified: mtime,
                            },
                        );
                        (key, nlink)
                    }
                };

                let node = nodes.entry(key).or_insert_with(|| FileNode {
                    key,
                    size,
                    nlink,
                    paths: Vec::new(),
                    has_downloads: false,
                    has_media: false,
                    torrent_hash: None,
                    is_seeding: false,
                    modified: mtime,
                });

                node.paths.push(path);
                if is_download {
                    node.has_downloads = true;
                } else {
                    node.has_media = true;
                }
                let _ = sender.send(ScanEvent::FileScanned);
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn get_file_info(&self, path: &Path, _metadata: &fs::Metadata) -> Result<(FileKey, u32)> {
        let file = fs::File::open(path)?;
        let handle = file.as_raw_handle();
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let success = unsafe { GetFileInformationByHandle(handle as _, &mut info) };

        if success == 0 {
            return Err(anyhow::anyhow!("Failed to get file info"));
        }

        let key = FileKey {
            dev: info.dwVolumeSerialNumber as u64,
            inode: ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64),
        };
        Ok((key, info.nNumberOfLinks))
    }

    #[cfg(unix)]
    fn get_file_info(&self, _path: &Path, metadata: &fs::Metadata) -> Result<(FileKey, u32)> {
        let key = FileKey {
            dev: metadata.dev(),
            inode: metadata.ino(),
        };
        Ok((key, metadata.nlink() as u32))
    }
}
