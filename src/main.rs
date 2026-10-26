mod agent;
mod app;
mod config;
mod grouping;
mod host;
mod qbittorrent;
mod scanner;
mod transfer;
mod tui;
mod ui;

use crate::app::{App, AppState};
use crate::config::Config;
use crate::host::{Host, Remote};
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
use std::sync::Arc;
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

// Custom helper for multi-path completion (comma separated)
struct MultiPathHelper {
    completer: FilenameCompleter,
}

impl MultiPathHelper {
    fn new() -> Self {
        Self {
            completer: FilenameCompleter::new(),
        }
    }
}

impl Completer for MultiPathHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, path_part) = if let Some(last_comma) = line[..pos].rfind(',') {
            (last_comma + 1, &line[last_comma + 1..pos])
        } else {
            (0, &line[..pos])
        };

        let trimmed_start = path_part.len() - path_part.trim_start().len();
        let relative_start = start + trimmed_start;
        let p_part = path_part.trim_start();

        // Delegate completion to the underlying filename completer with the partial path
        let (idx, candidates) = self.completer.complete(p_part, p_part.len(), ctx)?;

        // Adjust the index to be absolute to the line
        Ok((relative_start + idx, candidates))
    }
}

impl Hinter for MultiPathHelper {
    type Hint = String;
}

impl Highlighter for MultiPathHelper {}
impl Validator for MultiPathHelper {}
impl Helper for MultiPathHelper {}

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
    // Use MultiPathHelper here
    let mut rl: Editor<MultiPathHelper, rustyline::history::DefaultHistory> =
        Editor::with_config(config)?;
    rl.set_helper(Some(MultiPathHelper::new()));

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

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("agent") {
        return agent::run(args.get(2).map(String::as_str)).await;
    }

    // Try to load saved config first, fall back to CLI/env
    let cli = Config::load();
    let mut config = Config::load_from_file().unwrap_or_else(|| cli.clone());
    // A Remote given on the command line wins over the saved config
    if cli.remote.is_some() {
        config.remote = cli.remote.clone();
        config.remote_agent = cli.remote_agent.clone();
    }

    let (host, torrents) = match config.remote.clone() {
        Some(alias) => {
            let remote = Remote {
                alias,
                agent: config.remote_agent.clone(),
            };
            println!("Connecting to Agent on '{}'...", remote.alias);
            let info = remote.info()?;
            config.download_dir = Some(info.download_dir);
            config.media_dirs = info.media_dirs;
            (Host::Remote(remote), info.torrents)
        }
        None => setup_local(&mut config).await?,
    };
    let host = Arc::new(host);
    let mut app = App::new(config, Vec::new(), torrents);

    // Initial async scan
    let (tx, rx) = std::sync::mpsc::channel();
    host.scan_async(tx);
    app.state = AppState::Scanning {
        processed: 0,
        receiver: rx,
    };

    let (delete_tx, delete_rx) = std::sync::mpsc::channel::<Vec<String>>();

    let mut tui = Tui::new()?;
    tui.init()?;

    while app.running {
        tui.terminal.draw(|f| ui::render(&mut app, f))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(input) = app.transfer_prompt.as_mut() {
                        match key.code {
                            KeyCode::Esc => app.transfer_prompt = None,
                            KeyCode::Enter => app.confirm_transfer(),
                            KeyCode::Tab => *input = transfer::complete_dir(input),
                            KeyCode::Backspace => {
                                input.pop();
                            }
                            KeyCode::Char(c) => input.push(c),
                            _ => {}
                        }
                        continue;
                    }
                    if app.confirm_quit {
                        match key.code {
                            KeyCode::Enter => app.confirm_quit(),
                            KeyCode::Esc => app.confirm_quit = false,
                            _ => {}
                        }
                        continue;
                    }
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
                        KeyCode::Char(c)
                            if !app.search_active
                                && !app.show_confirmation
                                && c.is_ascii_digit()
                                && c != '0' =>
                        {
                            if let Some(digit) = c.to_digit(10) {
                                app.set_tab_index((digit - 1) as usize);
                            }
                        }
                        KeyCode::Char('q') => app.request_quit(),
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
                        KeyCode::Char('c') => {
                            if !app.show_confirmation {
                                app.request_transfer();
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
                                host.scan_async(tx);
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

        // Hand deletions to the Host off the UI thread (remote ones go over SSH)
        if !app.pending_path_deletions.is_empty() || !app.pending_qbit_deletions.is_empty() {
            let paths = std::mem::take(&mut app.pending_path_deletions);
            let hashes = std::mem::take(&mut app.pending_qbit_deletions);
            let host = Arc::clone(&host);
            let delete_tx = delete_tx.clone();
            tokio::spawn(async move {
                let _ = delete_tx.send(host.delete(paths, hashes).await);
            });
        }
        while let Ok(errors) = delete_rx.try_recv() {
            if !errors.is_empty() {
                app.status = Some(format!("Delete errors: {}", errors.join("; ")));
            }
        }

        app.tick();
    }

    tui.restore()?;
    Ok(())
}

/// Local Host: prompts for missing paths and connects to qBittorrent directly
async fn setup_local(config: &mut Config) -> Result<(Host, Vec<qbittorrent::TorrentInfo>)> {
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
    let qbit = qbittorrent::connect(&config.qbittorrent, Path::new("mock_env").exists()).await;
    let torrents = qbit.get_torrents().await.unwrap_or_default();

    let scanner = Scanner::new(download_dir, config.media_dirs.clone());
    Ok((Host::Local { scanner, qbit }, torrents))
}
