use crate::config::Config;
use crate::grouping::{Group, group_by_downloads, group_by_media};
use crate::qbittorrent::TorrentInfo;
use crate::scanner::FileNode;

pub struct App {
    pub config: Config,
    pub running: bool,
    pub active_tab: Tab,
    pub nodes: Vec<FileNode>,
    pub media_groups: Vec<Group>,
    pub download_groups: Vec<Group>,
    pub torrents: Vec<TorrentInfo>,
    pub selected_index: usize,
    pub show_details: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Media,
    Downloads,
}

impl App {
    pub fn new(config: Config, nodes: Vec<FileNode>, torrents: Vec<TorrentInfo>) -> Self {
        let media_groups = group_by_media(&nodes, &config.media_dirs);
        let download_groups = group_by_downloads(&nodes, &config.download_dir);

        Self {
            config,
            running: true,
            active_tab: Tab::Media,
            nodes,
            media_groups,
            download_groups,
            torrents,
            selected_index: 0,
            show_details: false,
        }
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    pub fn current_groups(&self) -> &Vec<Group> {
        match self.active_tab {
            Tab::Media => &self.media_groups,
            Tab::Downloads => &self.download_groups,
        }
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
}
