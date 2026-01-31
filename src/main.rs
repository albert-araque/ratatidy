mod app;
mod config;
mod grouping;
mod qbittorrent;
mod scanner;
mod tui;
mod ui;

use crate::app::{App, AppState};
use crate::config::Config;
use crate::qbittorrent::{MockQbitClient, QbitClient, RealQbitClient};
use crate::scanner::Scanner;
use crate::tui::Tui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config as RlConfig, Context, Editor, Helper};
use std::path::{Path, PathBuf};
use std::time::Duration;

// Custom helper for path completion
struct FilePathHelper {
    completer: FilenameCompleter,
}

impl FilePathHelper {
    fn new() -> Self {
        Self {
            completer: FilenameCompleter::new(),
        }
    }
}

impl Completer for FilePathHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for FilePathHelper {
    type Hint = String;
}

impl Highlighter for FilePathHelper {}
impl Validator for FilePathHelper {}
impl Helper for FilePathHelper {}

fn prompt_path(message: &str) -> Result<PathBuf> {
    let config = RlConfig::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl: Editor<FilePathHelper, rustyline::history::DefaultHistory> =
        Editor::with_config(config)?;
    rl.set_helper(Some(FilePathHelper::new()));

    let prompt = format!("{}: ", message);
    let input = rl.readline(&prompt)?;
    let path = PathBuf::from(input.trim());
    if path.exists() {
        Ok(std::fs::canonicalize(path)?)
    } else {
        Err(anyhow::anyhow!("Path does not exist: {}", input.trim()))
    }
}

fn prompt_paths(message: &str) -> Result<Vec<PathBuf>> {
    let config = RlConfig::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl: Editor<FilePathHelper, rustyline::history::DefaultHistory> =
        Editor::with_config(config)?;
    rl.set_helper(Some(FilePathHelper::new()));

    let prompt = format!("{}: ", message);
    let input = rl.readline(&prompt)?;
    let paths: Vec<PathBuf> = input
        .trim()
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| std::fs::canonicalize(s.trim()).ok())
        .collect();
    Ok(paths)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Try to load saved config first, fall back to CLI/env
    let mut config = Config::load_from_file().unwrap_or_else(Config::load);
    let mut config_was_prompted = false;

    // Development helper: If no dirs provided and mock_env exists, use it
    if config.download_dir.is_none()
        && config.media_dirs.is_empty()
        && Path::new("mock_env").exists()
    {
        config.download_dir = Some(std::fs::canonicalize("mock_env/downloads")?);
        config.media_dirs = vec![
            std::fs::canonicalize("mock_env/media/movies")?,
            std::fs::canonicalize("mock_env/media/tvshows")?,
        ];
    }

    // Interactive setup if paths are missing
    if config.download_dir.is_none() {
        println!("Welcome to Ratatidy! Let's set up your paths.\n");
        config.download_dir = Some(prompt_path("Enter your download directory")?);
        config_was_prompted = true;
    }
    if config.media_dirs.is_empty() {
        config.media_dirs =
            prompt_paths("Enter your media directories (comma separated, or press Enter to skip)")?;
        config_was_prompted = true;
    }

    // Save config if we prompted for it
    if config_was_prompted {
        config.save()?;
    }

    let download_dir = config.download_dir.clone().unwrap();

    // Phase 7: Fetch qBittorrent data (optional)
    let qbit: Box<dyn QbitClient> = if Path::new("mock_env").exists() {
        Box::new(MockQbitClient)
    } else if config.qbittorrent.is_configured() {
        match RealQbitClient::new(
            &config.qbittorrent.url,
            config.qbittorrent.username.clone(),
            config.qbittorrent.password.clone(),
        )
        .await
        {
            Ok(client) => Box::new(client),
            Err(_) => Box::new(MockQbitClient), // Fallback if connection fails
        }
    } else {
        Box::new(MockQbitClient) // No credentials configured
    };
    let torrents = qbit.get_torrents().await.unwrap_or_default();

    let scanner = Scanner::new(download_dir.clone(), config.media_dirs.clone());
    let mut app = App::new(config, Vec::new(), torrents);

    // Initial async scan
    let (tx, rx) = std::sync::mpsc::channel();
    scanner.scan_async(tx);
    app.state = AppState::Scanning {
        processed: 0,
        receiver: rx,
    };

    let mut tui = Tui::new()?;
    tui.init()?;

    while app.running {
        tui.terminal.draw(|f| ui::render(&mut app, f))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => {
                            if app.show_confirmation {
                                app.cancel_delete();
                            } else if app.search_active {
                                app.search_active = false;
                                app.search_query.clear();
                            }
                        }
                        KeyCode::Char(c) if app.search_active => {
                            app.search_query.push(c);
                            app.selected_index = 0;
                        }
                        KeyCode::Backspace if app.search_active => {
                            app.search_query.pop();
                            app.selected_index = 0;
                        }
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.show_confirmation {
                                app.delete_scope = app.delete_scope.next(&app.available_scopes);
                            } else {
                                app.select_next();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.show_confirmation {
                                app.delete_scope = app.delete_scope.prev(&app.available_scopes);
                            } else {
                                app.select_prev();
                            }
                        }
                        KeyCode::Char('i') => {
                            if !app.show_confirmation {
                                app.toggle_details();
                            }
                        }
                        KeyCode::Char('t') | KeyCode::Char('d') => {
                            if !app.show_confirmation {
                                app.request_delete();
                            }
                        }
                        KeyCode::Char('f') => {
                            if !app.show_confirmation && !app.search_active {
                                app.filter = app.filter.next();
                            }
                        }
                        KeyCode::Char('s') => {
                            if !app.show_confirmation && !app.search_active {
                                app.sort_by = app.sort_by.next();
                            }
                        }
                        KeyCode::Char('S') => {
                            if !app.show_confirmation && !app.search_active {
                                app.sort_order = app.sort_order.toggle();
                            }
                        }
                        KeyCode::Char('/') => {
                            if !app.show_confirmation {
                                app.search_active = true;
                            }
                        }
                        KeyCode::Char('r') => {
                            if !app.show_confirmation && !app.search_active {
                                // Rescan logic
                                let (tx, rx) = std::sync::mpsc::channel();
                                scanner.scan_async(tx);
                                app.state = AppState::Scanning {
                                    processed: 0,
                                    receiver: rx,
                                };
                            }
                        }
                        KeyCode::Enter => {
                            if app.show_confirmation {
                                app.confirm_delete();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Process qBit deletions
        if !app.pending_qbit_deletions.is_empty() {
            let hashes: Vec<String> = app.pending_qbit_deletions.drain(..).collect();
            for hash in hashes {
                let _ = qbit.delete_torrent(&hash, true).await;
            }
        }

        app.tick();
    }

    tui.restore()?;
    Ok(())
}
