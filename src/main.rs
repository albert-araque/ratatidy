mod app;
mod config;
mod grouping;
mod qbittorrent;
mod scanner;
mod tui;
mod ui;

use crate::app::App;
use crate::config::Config;
use crate::qbittorrent::{MockQbitClient, QbitClient};
use crate::scanner::Scanner;
use crate::tui::Tui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::path::Path;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config (eventually from file, for now default)
    let mut config = Config::default();

    // Auto-detect mock environment for easier testing
    if Path::new("mock_env").exists() {
        config.download_dir = std::fs::canonicalize("mock_env/downloads")?;
        config.media_dirs = vec![
            std::fs::canonicalize("mock_env/media/movies")?,
            std::fs::canonicalize("mock_env/media/tvshows")?,
        ];
    }

    // Phase 3: Fetch qBittorrent data
    let qbit = MockQbitClient;
    let torrents = qbit.get_torrents().await.unwrap_or_default();

    let scanner = Scanner::new(config.download_dir.clone(), config.media_dirs.clone());
    let mut nodes = scanner.scan().unwrap_or_default();

    // Enrich nodes with qbit data
    for node in &mut nodes {
        for path in &node.paths {
            let path_str = path.to_string_lossy();
            for torrent in &torrents {
                // Heuristic: if torrent name or files match (simplified for mock)
                if path_str.contains(&torrent.name) {
                    node.torrent_hash = Some(torrent.hash.clone());
                    node.is_seeding =
                        torrent.state.contains("UP") || torrent.state.contains("uploading");
                }
            }
        }
    }

    let mut app = App::new(config, nodes, torrents);

    let mut tui = Tui::new()?;
    tui.init()?;

    while app.running {
        tui.terminal.draw(|f| ui::render(&mut app, f))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                        KeyCode::Char('i') => app.toggle_details(),
                        _ => {}
                    }
                }
            }
        }
        app.tick();
    }

    tui.restore()?;
    Ok(())
}
