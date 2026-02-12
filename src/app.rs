use crate::config::Config;
use crate::grouping::{Group, group_by_downloads, group_by_media};
use crate::qbittorrent::TorrentInfo;
use crate::scanner::FileNode;
use std::collections::HashMap;

static EMPTY_GROUPS: Vec<Group> = Vec::new();

pub struct App {
    pub config: Config,
    pub running: bool,
    pub active_tab: Tab,
    pub media_groups: Option<Vec<Group>>, // For Tab::Media (Aggregated)
    pub folder_groups: HashMap<usize, Vec<Group>>, // For Tab::MediaFolder(i)
    pub download_groups: Option<Vec<Group>>, // For Tab::Downloads
    pub nodes: Vec<FileNode>,
    pub selected_index: usize,
    pub show_details: bool,
    pub show_confirmation: bool,
    pub delete_scope: DeleteScope,
    pub available_scopes: Vec<DeleteScope>,
    pub search_query: String,
    pub search_active: bool,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
    pub filter: FilterMode,
    pub pending_qbit_deletions: Vec<String>,
    pub state: AppState,
    pub torrents: Vec<TorrentInfo>,
}

#[derive(Debug)]
pub enum AppState {
    Ready,
    Scanning {
        processed: usize,
        receiver: std::sync::mpsc::Receiver<crate::scanner::ScanEvent>,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Orphans,
    Hardlinked,
}

impl FilterMode {
    pub fn next(self) -> Self {
        match self {
            FilterMode::All => FilterMode::Orphans,
            FilterMode::Orphans => FilterMode::Hardlinked,
            FilterMode::Hardlinked => FilterMode::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    DateAdded,
}

impl SortBy {
    pub fn next(self) -> Self {
        match self {
            SortBy::Name => SortBy::Size,
            SortBy::Size => SortBy::DateAdded,
            SortBy::DateAdded => SortBy::Name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn toggle(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteScope {
    Downloads,
    Media,
    All,
}

impl DeleteScope {
    pub fn next(&self, available: &[DeleteScope]) -> Self {
        if available.is_empty() {
            return *self;
        }
        let pos = available.iter().position(|x| x == self).unwrap_or(0);
        available[(pos + 1) % available.len()]
    }
    pub fn prev(&self, available: &[DeleteScope]) -> Self {
        if available.is_empty() {
            return *self;
        }
        let pos = available.iter().position(|x| x == self).unwrap_or(0);
        if pos == 0 {
            available[available.len() - 1]
        } else {
            available[pos - 1]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Media,
    MediaFolder(usize),
    Downloads,
}

impl App {
    pub fn new(config: Config, nodes: Vec<FileNode>, _torrents: Vec<TorrentInfo>) -> Self {
        let mut app = Self {
            config,
            running: true,
            active_tab: Tab::Media,
            media_groups: None,
            folder_groups: HashMap::new(),
            download_groups: None,
            nodes,
            selected_index: 0,
            show_details: false,
            show_confirmation: false,
            delete_scope: DeleteScope::Downloads,
            available_scopes: Vec::new(),
            search_query: String::new(),
            search_active: false,
            sort_by: SortBy::Name,
            sort_order: SortOrder::Ascending,
            filter: FilterMode::All,
            pending_qbit_deletions: Vec::new(),
            state: AppState::Ready,
            torrents: _torrents,
        };
        app.refresh_groups();
        app
    }

    pub fn refresh_groups(&mut self) {
        self.media_groups = None;
        self.folder_groups.clear();
        self.download_groups = None;
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    pub fn ensure_groups(&mut self) {
        match self.active_tab {
            Tab::Media => {
                if self.media_groups.is_none() {
                    self.media_groups = Some(group_by_media(&self.nodes, &self.config.media_dirs));
                }
            }
            Tab::MediaFolder(idx) => {
                if !self.folder_groups.contains_key(&idx) {
                    if let Some(dir) = self.config.media_dirs.get(idx) {
                        let groups = group_by_media(&self.nodes, &[dir.clone()]);
                        self.folder_groups.insert(idx, groups);
                    }
                }
            }
            Tab::Downloads => {
                if self.download_groups.is_none() {
                    if let Some(ref download_dir) = self.config.download_dir {
                        self.download_groups = Some(group_by_downloads(&self.nodes, download_dir));
                    }
                }
            }
        }
    }

    pub fn current_groups(&self) -> Vec<&Group> {
        let groups = match self.active_tab {
            Tab::Media => self.media_groups.as_ref().unwrap_or(&EMPTY_GROUPS),
            Tab::MediaFolder(idx) => self.folder_groups.get(&idx).unwrap_or(&EMPTY_GROUPS),
            Tab::Downloads => self.download_groups.as_ref().unwrap_or(&EMPTY_GROUPS),
        };

        let mut filtered: Vec<&Group> = if self.search_query.is_empty() {
            groups.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            groups
                .iter()
                .filter(|g| g.title.to_lowercase().contains(&query))
                .collect()
        };

        // Apply Filtering
        filtered = match self.filter {
            FilterMode::All => filtered,
            FilterMode::Orphans => filtered
                .into_iter()
                .filter(|g| g.nodes.iter().any(|n| !(n.has_downloads && n.has_media)))
                .collect(),
            FilterMode::Hardlinked => filtered
                .into_iter()
                .filter(|g| g.nodes.iter().all(|n| n.has_downloads && n.has_media))
                .collect(),
        };

        match self.sort_by {
            SortBy::Name => filtered.sort_by(|a, b| {
                let cmp = a.title.cmp(&b.title);
                if self.sort_order == SortOrder::Descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            }),
            SortBy::Size => filtered.sort_by(|a, b| {
                let size_a: u64 = a.nodes.iter().map(|n| n.size).sum();
                let size_b: u64 = b.nodes.iter().map(|n| n.size).sum();
                let cmp = size_a.cmp(&size_b).then_with(|| a.title.cmp(&b.title));
                if self.sort_order == SortOrder::Descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            }),
            SortBy::DateAdded => filtered.sort_by(|a, b| {
                let date_a = a.nodes.iter().filter_map(|n| n.modified).max();
                let date_b = b.nodes.iter().filter_map(|n| n.modified).max();
                let cmp = date_a.cmp(&date_b).then_with(|| a.title.cmp(&b.title));
                if self.sort_order == SortOrder::Descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            }),
        }

        filtered
    }

    pub fn select_next(&mut self) {
        let len = self.current_groups().len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    pub fn select_prev(&mut self) {
        let len = self.current_groups().len();
        if len > 0 {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = len - 1;
            }
        }
    }

    pub fn tick(&mut self) {
        if let AppState::Scanning {
            ref mut processed,
            ref receiver,
        } = self.state
        {
            while let Ok(event) = receiver.try_recv() {
                match event {
                    crate::scanner::ScanEvent::FileScanned => {
                        *processed += 1;
                    }
                    crate::scanner::ScanEvent::Finished(mut nodes) => {
                        // Enrich nodes with torrent data
                        for node in &mut nodes {
                            for path in &node.paths {
                                let path_str = path.to_string_lossy();
                                for torrent in &self.torrents {
                                    if path_str.contains(&torrent.name) {
                                        node.torrent_hash = Some(torrent.hash.clone());
                                        node.is_seeding = torrent.state.contains("UP")
                                            || torrent.state.contains("uploading");
                                    }
                                }
                            }
                        }
                        self.nodes = nodes;
                        self.refresh_groups();
                        self.state = AppState::Ready;
                        break;
                    }
                    crate::scanner::ScanEvent::Error(e) => {
                        self.state = AppState::Error(e);
                        break;
                    }
                }
            }
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    // Helper to generate the list of tabs based on current config
    pub fn get_tabs_list(&self) -> Vec<Tab> {
        let mut tabs = vec![Tab::Media];

        // Only show individual folders if there's more than 1
        if self.config.media_dirs.len() > 1 {
            for i in 0..self.config.media_dirs.len() {
                tabs.push(Tab::MediaFolder(i));
            }
        }

        tabs.push(Tab::Downloads);
        tabs
    }

    pub fn next_tab(&mut self) {
        let tabs = self.get_tabs_list();
        if let Some(pos) = tabs.iter().position(|t| *t == self.active_tab) {
            self.active_tab = tabs[(pos + 1) % tabs.len()];
            self.selected_index = 0;
        }
    }

    // Jump to a specific tab index (0-based) from the UI/keyboard perspective
    // 0: Media, 1..N: Folders (if any), Last: Downloads
    pub fn set_tab_index(&mut self, index: usize) {
        let tabs = self.get_tabs_list();
        if index < tabs.len() {
            self.active_tab = tabs[index];
            self.selected_index = 0;
        }
    }

    pub fn request_delete(&mut self) {
        let groups = self.current_groups();
        if groups.is_empty() {
            return;
        }

        if let Some(group) = groups.get(self.selected_index) {
            let mut available = Vec::new();
            let mut has_downloads = false;
            let mut has_media = false;

            for node in &group.nodes {
                if node.has_downloads {
                    has_downloads = true;
                }
                if node.has_media {
                    has_media = true;
                }
            }

            if has_downloads {
                available.push(DeleteScope::Downloads);
            }
            if has_media {
                available.push(DeleteScope::Media);
            }
            if has_downloads && has_media {
                available.push(DeleteScope::All);
            }

            if available.is_empty() {
                return;
            }

            self.available_scopes = available;
            self.delete_scope = self.available_scopes[0];
            self.show_confirmation = true;
        }
    }

    pub fn confirm_delete(&mut self) {
        self.execute_delete();
        self.show_confirmation = false;
    }

    pub fn cancel_delete(&mut self) {
        self.show_confirmation = false;
    }

    fn execute_delete(&mut self) {
        let group_title = if let Some(g) = self.current_groups().get(self.selected_index) {
            g.title.clone()
        } else {
            return;
        };

        let mut hashes_to_delete = Vec::new();
        let mut paths_to_remove = Vec::new();

        // 1. Identify what needs to be deleted in the master nodes
        for node in &mut self.nodes {
            // Check if this node belongs to the selected group
            let is_in_group = match self.active_tab {
                Tab::Media => self.config.media_dirs.iter().any(|m| {
                    node.paths.iter().any(|p| {
                        if let Ok(rel) = p.strip_prefix(m) {
                            if let Some(first) = rel.components().next() {
                                return first.as_os_str().to_string_lossy() == group_title;
                            }
                        }
                        false
                    })
                }),
                Tab::MediaFolder(idx) => {
                    if let Some(m) = self.config.media_dirs.get(idx) {
                        node.paths.iter().any(|p| {
                            if let Ok(rel) = p.strip_prefix(m) {
                                if let Some(first) = rel.components().next() {
                                    return first.as_os_str().to_string_lossy() == group_title;
                                }
                            }
                            false
                        })
                    } else {
                        false
                    }
                }
                Tab::Downloads => {
                    if let Some(ref download_dir) = self.config.download_dir {
                        node.paths.iter().any(|p| {
                            if let Ok(rel) = p.strip_prefix(download_dir) {
                                if let Some(first) = rel.components().next() {
                                    return first.as_os_str().to_string_lossy() == group_title;
                                }
                            }
                            false
                        })
                    } else {
                        false
                    }
                }
            };

            if !is_in_group {
                continue;
            }

            match self.delete_scope {
                DeleteScope::Downloads => {
                    if let Some(hash) = &node.torrent_hash {
                        hashes_to_delete.push(hash.clone());
                    } else if let Some(ref download_dir) = self.config.download_dir {
                        for path in &node.paths {
                            if path.starts_with(download_dir) {
                                paths_to_remove.push(path.clone());
                            }
                        }
                    }
                }
                DeleteScope::Media => {
                    for path in &node.paths {
                        if self.config.media_dirs.iter().any(|m| path.starts_with(m)) {
                            paths_to_remove.push(path.clone());
                        }
                    }
                }
                DeleteScope::All => {
                    if let Some(hash) = &node.torrent_hash {
                        hashes_to_delete.push(hash.clone());
                    } else if let Some(ref download_dir) = self.config.download_dir {
                        for path in &node.paths {
                            if path.starts_with(download_dir) {
                                paths_to_remove.push(path.clone());
                            }
                        }
                    }
                    for path in &node.paths {
                        if self.config.media_dirs.iter().any(|m| path.starts_with(m)) {
                            paths_to_remove.push(path.clone());
                        }
                    }
                }
            }
        }

        // 2. Perform actual disk deletion for non-torrent paths
        for path in &paths_to_remove {
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }

        // 3. Queue qBit deletions
        for hash in hashes_to_delete {
            if !self.pending_qbit_deletions.contains(&hash) {
                self.pending_qbit_deletions.push(hash);
            }
        }

        // 4. Update the master nodes state
        for node in &mut self.nodes {
            node.paths.retain(|p| !paths_to_remove.contains(p));

            // Re-calculate flags
            node.has_downloads = if let Some(ref download_dir) = self.config.download_dir {
                node.paths.iter().any(|p| p.starts_with(download_dir))
            } else {
                false
            };
            node.has_media = node
                .paths
                .iter()
                .any(|p| self.config.media_dirs.iter().any(|m| p.starts_with(m)));
        }

        // 5. Cleanup empty nodes
        self.nodes.retain(|n| !n.paths.is_empty());

        // 6. Refresh views
        self.refresh_groups();

        if self.selected_index >= self.current_groups().len() && !self.current_groups().is_empty() {
            self.selected_index = self.current_groups().len() - 1;
        }
    }
}
