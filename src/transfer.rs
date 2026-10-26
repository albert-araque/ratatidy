use std::collections::VecDeque;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};

/// Copy of a Group's Container from the Remote to this machine. The source is never touched.
#[derive(Debug, Clone)]
pub struct Transfer {
    pub title: String,
    pub source: PathBuf,
    pub dest_dir: PathBuf,
}

struct Active {
    transfer: Transfer,
    child: Child,
    progress: Receiver<u8>,
    stderr: Receiver<String>,
    percent: u8,
}

/// Runs Transfers one at a time with rsync over the Remote's SSH alias
pub struct TransferQueue {
    remote: String,
    pending: VecDeque<Transfer>,
    active: Option<Active>,
}

impl TransferQueue {
    pub fn new(remote: String) -> Self {
        Self {
            remote,
            pending: VecDeque::new(),
            active: None,
        }
    }

    pub fn push(&mut self, transfer: Transfer) {
        self.pending.push_back(transfer);
    }

    pub fn is_busy(&self) -> bool {
        self.active.is_some() || !self.pending.is_empty()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn active(&self) -> Option<(&Transfer, u8)> {
        self.active.as_ref().map(|a| (&a.transfer, a.percent))
    }

    /// Kills the running rsync (--partial keeps what it got) and drops the queue
    pub fn cancel_all(&mut self) {
        self.pending.clear();
        if let Some(mut active) = self.active.take() {
            let _ = active.child.kill();
            let _ = active.child.wait();
        }
    }

    /// Advances the queue; returns the Transfers that ended since the last tick
    pub fn tick(&mut self) -> Vec<(Transfer, Result<(), String>)> {
        let mut finished = Vec::new();

        if let Some(active) = &mut self.active {
            while let Ok(percent) = active.progress.try_recv() {
                active.percent = percent;
            }
            let result = match active.child.try_wait() {
                Ok(None) => return finished,
                Ok(Some(status)) if status.success() => Ok(()),
                Ok(Some(status)) => {
                    let stderr = active.stderr.recv().unwrap_or_default();
                    Err(format!("rsync failed ({status}): {}", last_line(&stderr)))
                }
                Err(e) => Err(e.to_string()),
            };
            let active = self.active.take().expect("checked above");
            finished.push((active.transfer, result));
        }

        while self.active.is_none() {
            let Some(transfer) = self.pending.pop_front() else { break };
            match self.start(transfer.clone()) {
                Ok(active) => self.active = Some(active),
                Err(e) => finished.push((transfer, Err(e))),
            }
        }

        finished
    }

    fn start(&self, transfer: Transfer) -> Result<Active, String> {
        std::fs::create_dir_all(&transfer.dest_dir)
            .map_err(|e| format!("{}: {e}", transfer.dest_dir.display()))?;
        let mut child = rsync_command(&self.remote, &transfer)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Could not run rsync: {e}"))?;

        let (progress_tx, progress) = channel();
        let mut stdout = child.stdout.take().expect("stdout is piped");
        std::thread::spawn(move || {
            // progress2 rewrites a single line with \r, so read raw chunks rather than lines
            let mut buf = [0u8; 4096];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if let Some(percent) = parse_progress(&String::from_utf8_lossy(&buf[..n])) {
                    if progress_tx.send(percent).is_err() {
                        break;
                    }
                }
            }
        });

        let (stderr_tx, stderr) = channel();
        let mut err_pipe = child.stderr.take().expect("stderr is piped");
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = err_pipe.read_to_string(&mut text);
            let _ = stderr_tx.send(text);
        });

        Ok(Active {
            transfer,
            child,
            progress,
            stderr,
            percent: 0,
        })
    }
}

fn rsync_command(remote: &str, transfer: &Transfer) -> Command {
    let mut cmd = Command::new("rsync");
    cmd.args([
        "--archive",
        "--partial",
        // Remote path is passed verbatim, no shell quoting needed for spaces
        "--protect-args",
        "--info=progress2",
        // Without it the overall percentage restarts as rsync discovers files
        "--no-inc-recursive",
        "-e",
        "ssh -o BatchMode=yes",
    ]);
    // No trailing slash on the source: rsync creates dest_dir/<Container name>
    cmd.arg(format!("{remote}:{}", transfer.source.display()));
    cmd.arg(&transfer.dest_dir);
    cmd
}

/// Last percentage in a chunk of `rsync --info=progress2` output
fn parse_progress(chunk: &str) -> Option<u8> {
    chunk
        .split(['\r', '\n'])
        .rev()
        .find_map(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_suffix('%')?.parse::<u8>().ok())
        })
}

fn last_line(text: &str) -> &str {
    text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim()
}

fn last_dir_file() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "ratatidy", "ratatidy")
        .map(|dirs| dirs.data_dir().join("last_transfer_dir"))
}

/// Directory to prefill the Transfer prompt with: last used, else the Downloads folder
pub fn last_dir() -> String {
    last_dir_file()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            directories::UserDirs::new()
                .and_then(|u| u.download_dir().map(|d| d.display().to_string()))
        })
        .unwrap_or_else(|| "~/Downloads".to_string())
}

pub fn save_last_dir(dir: &str) {
    if let Some(file) = last_dir_file() {
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(file, dir);
    }
}

pub fn expand_tilde(input: &str) -> PathBuf {
    let home = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf());
    match (input.strip_prefix('~'), home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(input),
    }
}

/// Tab completion of a directory path: completes the unique match, or the common prefix
pub fn complete_dir(input: &str) -> String {
    let (dir_part, prefix) = match input.rfind('/') {
        Some(i) => (&input[..=i], &input[i + 1..]),
        None => ("", input),
    };
    let dir = if dir_part.is_empty() {
        PathBuf::from(".")
    } else {
        expand_tilde(dir_part)
    };

    let mut matches = list_subdirs(&dir)
        .into_iter()
        .filter(|name| name.starts_with(prefix) && (prefix.starts_with('.') || !name.starts_with('.')))
        .collect::<Vec<_>>();
    matches.sort();

    match matches.as_slice() {
        [] => input.to_string(),
        [only] => format!("{dir_part}{only}/"),
        [first, rest @ ..] => {
            let common = rest.iter().fold(first.as_str(), |acc, name| common_prefix(acc, name));
            format!("{dir_part}{common}")
        }
    }
}

fn list_subdirs(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn common_prefix<'a>(a: &'a str, b: &str) -> &'a str {
    let len = a
        .char_indices()
        .zip(b.chars())
        .take_while(|((_, ca), cb)| ca == cb)
        .last()
        .map(|((i, c), _)| i + c.len_utf8())
        .unwrap_or(0);
    &a[..len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_takes_the_latest_update_in_a_chunk() {
        let chunk = "     32,768   0%    0.00kB/s    0:00:00\r  1,048,576  42%   10.50MB/s    0:01:23\r";
        assert_eq!(parse_progress(chunk), Some(42));
    }

    #[test]
    fn progress_ignores_chunks_without_a_percentage() {
        assert_eq!(parse_progress("receiving file list ...\n"), None);
    }

    #[test]
    fn progress_reads_completion_line() {
        let chunk = "  4,294,967,296 100%   95.00MB/s    0:00:43 (xfr#12, to-chk=0/14)\n";
        assert_eq!(parse_progress(chunk), Some(100));
    }

    #[test]
    fn rsync_gets_remote_source_and_local_dest() {
        let transfer = Transfer {
            title: "Inception (2010)".into(),
            source: "/mnt/data/media/movies/Inception (2010)".into(),
            dest_dir: "/home/me/Videos".into(),
        };
        let cmd = rsync_command("media-server", &transfer);
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert_eq!(
            &args[args.len() - 2..],
            ["media-server:/mnt/data/media/movies/Inception (2010)", "/home/me/Videos"]
        );
    }

    #[test]
    fn completes_unique_and_common_prefix_dirs() {
        let root = std::env::temp_dir().join(format!("ratatidy-complete-{}", std::process::id()));
        for d in ["Movies", "Music", "Videos", ".hidden"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        let base = format!("{}/", root.display());

        assert_eq!(complete_dir(&format!("{base}V")), format!("{base}Videos/"));
        assert_eq!(complete_dir(&format!("{base}M")), format!("{base}M"));
        assert_eq!(complete_dir(&format!("{base}Mo")), format!("{base}Movies/"));
        assert_eq!(complete_dir(&format!("{base}x")), format!("{base}x"));
        assert_eq!(complete_dir(&format!("{base}.h")), format!("{base}.hidden/"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn common_prefix_is_char_safe() {
        assert_eq!(common_prefix("Película", "Películas"), "Película");
        assert_eq!(common_prefix("añoA", "añoB"), "año");
    }

    #[test]
    fn tilde_expands_to_home_only_at_start() {
        let home = directories::BaseDirs::new().unwrap().home_dir().to_path_buf();
        assert_eq!(expand_tilde("~/Videos"), home.join("Videos"));
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("/tmp/~x"), PathBuf::from("/tmp/~x"));
    }
}
