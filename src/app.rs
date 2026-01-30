use crate::config::Config;
use crate::grouping::{Group, group_by_downloads, group_by_media};
use crate::qbittorrent::TorrentInfo;
use crate::scanner::FileNode;

pub struct App {
    pub config: Config,
    pub running: bool,
    pub active_tab: Tab,
    pub media_groups: Vec<Group>,
    pub download_groups: Vec<Group>,
    pub selected_index: usize,
    pub show_details: bool,
    pub show_confirmation: bool,
    pub delete_scope: DeleteScope,
    pub available_scopes: Vec<DeleteScope>,
    pub search_query: String,
    pub search_active: bool,
    pub sort_by: SortBy,
    pub filter: FilterMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Orphans,
    Hardlinked,
    Seeding,
    NotSeeding,
}

impl FilterMode {
    pub fn next(self) -> Self {
        match self {
            FilterMode::All => FilterMode::Orphans,
            FilterMode::Orphans => FilterMode::Hardlinked,
            FilterMode::Hardlinked => FilterMode::Seeding,
            FilterMode::Seeding => FilterMode::NotSeeding,
            FilterMode::NotSeeding => FilterMode::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Seeding,
}

impl SortBy {
    pub fn next(self) -> Self {
        match self {
            SortBy::Name => SortBy::Size,
            SortBy::Size => SortBy::Seeding,
            SortBy::Seeding => SortBy::Name,
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
    Downloads,
}

impl App {
    pub fn new(config: Config, nodes: Vec<FileNode>, _torrents: Vec<TorrentInfo>) -> Self {
        let media_groups = group_by_media(&nodes, &config.media_dirs);
        let download_groups = group_by_downloads(&nodes, &config.download_dir);

        Self {
            config,
            running: true,
            active_tab: Tab::Media,
            media_groups,
            download_groups,
            selected_index: 0,
            show_details: false,
            show_confirmation: false,
            delete_scope: DeleteScope::Downloads,
            available_scopes: Vec::new(),
            search_query: String::new(),
            search_active: bool::from(false),
            sort_by: SortBy::Name,
            filter: FilterMode::All,
        }
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    pub fn current_groups(&self) -> Vec<&Group> {
        let groups = match self.active_tab {
            Tab::Media => &self.media_groups,
            Tab::Downloads => &self.download_groups,
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
            FilterMode::Seeding => filtered
                .into_iter()
                .filter(|g| g.nodes.iter().any(|n| n.is_seeding))
                .collect(),
            FilterMode::NotSeeding => filtered
                .into_iter()
                .filter(|g| g.nodes.iter().all(|n| !n.is_seeding))
                .collect(),
        };

        match self.sort_by {
            SortBy::Name => filtered.sort_by(|a, b| a.title.cmp(&b.title)),
            SortBy::Size => filtered.sort_by(|a, b| {
                let size_a: u64 = a.nodes.iter().map(|n| n.size).sum();
                let size_b: u64 = b.nodes.iter().map(|n| n.size).sum();
                size_b.cmp(&size_a) // Descending size
            }),
            SortBy::Seeding => filtered.sort_by(|a, b| {
                let seed_a = a.nodes.iter().filter(|n| n.is_seeding).count();
                let seed_b = b.nodes.iter().filter(|n| n.is_seeding).count();
                seed_b.cmp(&seed_a)
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
        // Here we will eventually handle background updates
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Media => Tab::Downloads,
            Tab::Downloads => Tab::Media,
        };
        self.selected_index = 0;
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
        if let Some(group_ref) = self.current_groups().get(self.selected_index) {
            // We need to find the actual index in the original list
            let title = &group_ref.title;
            let group_opt = match self.active_tab {
                Tab::Media => self.media_groups.iter().position(|g| &g.title == title),
                Tab::Downloads => self.download_groups.iter().position(|g| &g.title == title),
            };

            if let Some(idx) = group_opt {
                let group = match self.active_tab {
                    Tab::Media => &self.media_groups[idx],
                    Tab::Downloads => &self.download_groups[idx],
                };
                match self.delete_scope {
                    DeleteScope::Downloads => {
                        for node in &group.nodes {
                            for path in &node.paths {
                                if path.starts_with(&self.config.download_dir) {
                                    let _ = std::fs::remove_file(path);
                                }
                            }
                        }
                    }
                    DeleteScope::Media => {
                        for node in &group.nodes {
                            for path in &node.paths {
                                let is_in_media = self
                                    .config
                                    .media_dirs
                                    .iter()
                                    .any(|media_dir| path.starts_with(media_dir));
                                if is_in_media {
                                    let _ = std::fs::remove_file(path);
                                }
                            }
                        }
                    }
                    DeleteScope::All => {
                        for node in &group.nodes {
                            for path in &node.paths {
                                let _ = std::fs::remove_file(path);
                            }
                        }
                    }
                }

                match self.active_tab {
                    Tab::Media => {
                        self.media_groups.remove(idx);
                    }
                    Tab::Downloads => {
                        self.download_groups.remove(idx);
                    }
                }
            }
        }
        if self.selected_index >= self.current_groups().len() && !self.current_groups().is_empty() {
            self.selected_index = self.current_groups().len() - 1;
        }
    }
}
