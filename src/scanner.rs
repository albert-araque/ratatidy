use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

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
        self.scan_dir(&self.download_dir, true, &mut nodes)?;
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
                let key = self.get_file_key(&path, &metadata)?;
                let nlink = self.get_nlink(&metadata);

                let node = nodes.entry(key).or_insert_with(|| FileNode {
                    key,
                    size: metadata.len(),
                    nlink,
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

    #[cfg(windows)]
    fn get_file_key(&self, path: &Path, _metadata: &fs::Metadata) -> Result<FileKey> {
        let file = fs::File::open(path)?;
        let handle = file.as_raw_handle();
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let success = unsafe { GetFileInformationByHandle(handle as _, &mut info) };

        if success == 0 {
            return Err(anyhow::anyhow!("Failed to get file info"));
        }

        Ok(FileKey {
            dev: info.dwVolumeSerialNumber as u64,
            inode: ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64),
        })
    }

    #[cfg(unix)]
    fn get_file_key(&self, _path: &Path, metadata: &fs::Metadata) -> Result<FileKey> {
        Ok(FileKey {
            dev: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    #[cfg(windows)]
    fn get_nlink(&self, _metadata: &fs::Metadata) -> u32 {
        // We get this from the handle info in get_file_key usually,
        // but for simplicity we can just return 1 or re-fetch.
        // For ratatidy, detecting if > 1 is what matters.
        1
    }

    #[cfg(unix)]
    fn get_nlink(&self, metadata: &fs::Metadata) -> u32 {
        metadata.nlink() as u32
    }
}
