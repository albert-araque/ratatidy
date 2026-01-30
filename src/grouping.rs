use crate::scanner::FileNode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum GroupKind {
    Movie,
    Series,
    DownloadTorrent,
    Other,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Group {
    pub kind: GroupKind,
    pub title: String,
    pub media_container: Option<PathBuf>,
    pub downloads_container: Option<PathBuf>,
    pub nodes: Vec<FileNode>,
}

pub fn group_by_media(nodes: &[FileNode], media_dirs: &[PathBuf]) -> Vec<Group> {
    let mut groups: HashMap<PathBuf, Vec<FileNode>> = HashMap::new();

    for node in nodes {
        if node.has_media {
            for path in &node.paths {
                // Find which media dir this path belongs to
                for media_dir in media_dirs {
                    if path.starts_with(media_dir) {
                        // Heuristic: Group by the first folder inside the media_dir
                        // e.g. /media/movies/Inception (2010)/Inception.mkv -> Inception (2010)
                        if let Ok(relative) = path.strip_prefix(media_dir) {
                            let components: Vec<_> = relative.components().collect();
                            if !components.is_empty() {
                                let group_folder = media_dir.join(components[0].as_os_str());
                                groups.entry(group_folder).or_default().push(node.clone());
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    groups
        .into_iter()
        .map(|(path, nodes)| {
            let title = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Group {
                kind: GroupKind::Movie, // Simplify for now, could detect Series if it has subfolders
                title,
                media_container: Some(path),
                downloads_container: None,
                nodes,
            }
        })
        .collect()
}

pub fn group_by_downloads(nodes: &[FileNode], download_dir: &Path) -> Vec<Group> {
    let mut groups: HashMap<PathBuf, Vec<FileNode>> = HashMap::new();

    for node in nodes {
        if node.has_downloads {
            for path in &node.paths {
                if path.starts_with(download_dir) {
                    if let Ok(relative) = path.strip_prefix(download_dir) {
                        let components: Vec<_> = relative.components().collect();
                        if !components.is_empty() {
                            let group_item = download_dir.join(components[0].as_os_str());
                            groups.entry(group_item).or_default().push(node.clone());
                        }
                    }
                }
            }
        }
    }

    groups
        .into_iter()
        .map(|(path, nodes)| {
            let title = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Group {
                kind: GroupKind::DownloadTorrent,
                title,
                media_container: None,
                downloads_container: Some(path),
                nodes,
            }
        })
        .collect()
}
