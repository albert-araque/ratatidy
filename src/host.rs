use crate::agent::{AgentInfo, DeleteRequest, DeleteResponse, PROTOCOL_VERSION};
use crate::qbittorrent::QbitClient;
use crate::scanner::{ScanEvent, Scanner};
use anyhow::{Result, anyhow};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

/// Where Downloads and Media live: this machine, or a Remote reached through its Agent
pub enum Host {
    Local {
        scanner: Scanner,
        qbit: Box<dyn QbitClient>,
    },
    Remote(Remote),
}

#[derive(Clone)]
pub struct Remote {
    pub alias: String,
    pub agent: String,
    /// Private key file, the `-i` of `ssh -i <file> user@host`
    pub identity: Option<String>,
}

impl Remote {
    fn command(&self, agent_cmd: &str) -> Command {
        let mut cmd = Command::new("ssh");
        // BatchMode: a password prompt would corrupt the TUI, fail instead
        cmd.args(["-o", "BatchMode=yes"]);
        if let Some(identity) = &self.identity {
            cmd.args(["-i", identity]);
        }
        cmd.args([&self.alias, &self.agent, "agent", agent_cmd]);
        cmd
    }

    /// Asks the Agent for its config and torrents, checking the protocol version
    pub fn info(&self) -> Result<AgentInfo> {
        let output = self.command("info").output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "Agent on '{}' failed ({}): {}",
                self.alias,
                self.agent,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let info: AgentInfo = serde_json::from_slice(&output.stdout)
            .map_err(|e| anyhow!("Unexpected answer from the Agent on '{}': {e}", self.alias))?;
        if info.protocol_version != PROTOCOL_VERSION {
            return Err(anyhow!(
                "Agent on '{}' speaks protocol v{}, this ratatidy speaks v{}. Update the Agent.",
                self.alias,
                info.protocol_version,
                PROTOCOL_VERSION
            ));
        }
        Ok(info)
    }

    fn scan_async(&self, sender: Sender<ScanEvent>) {
        let mut cmd = self.command("scan");
        std::thread::spawn(move || {
            let child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
            let mut child = match child {
                Ok(child) => child,
                Err(e) => {
                    let _ = sender.send(ScanEvent::Error(format!("Could not run ssh: {e}")));
                    return;
                }
            };

            let stdout = child.stdout.take().expect("stdout is piped");
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                match serde_json::from_str::<ScanEvent>(&line) {
                    Ok(event) => {
                        let done = !matches!(event, ScanEvent::FileScanned);
                        let _ = sender.send(event);
                        if done {
                            let _ = child.wait();
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(ScanEvent::Error(format!("Bad scan event: {e}")));
                        let _ = child.kill();
                        return;
                    }
                }
            }

            // stdout closed without a final event: the Agent or ssh died
            let mut stderr = String::new();
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_string(&mut stderr);
            }
            let _ = child.wait();
            let _ = sender.send(ScanEvent::Error(format!(
                "Remote scan aborted: {}",
                stderr.trim()
            )));
        });
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse> {
        let mut cmd = tokio::process::Command::from(self.command("delete"));
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let body = serde_json::to_vec(&request)?;
        {
            use tokio::io::AsyncWriteExt;
            let mut stdin = child.stdin.take().expect("stdin is piped");
            stdin.write_all(&body).await?;
        }
        let output = child.wait_with_output().await?;
        if !output.status.success() {
            return Err(anyhow!(
                "Remote delete failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}

impl Host {
    pub fn scan_async(&self, sender: Sender<ScanEvent>) {
        match self {
            Host::Local { scanner, .. } => scanner.scan_async(sender),
            Host::Remote(remote) => remote.scan_async(sender),
        }
    }

    /// Removes the paths and torrents (with their files); returns one message per failure
    pub async fn delete(&self, paths: Vec<PathBuf>, hashes: Vec<String>) -> Vec<String> {
        let request = DeleteRequest { paths, hashes };
        match self {
            Host::Local { qbit, .. } => delete_local(&request, qbit.as_ref()).await.errors,
            Host::Remote(remote) => match remote.delete(request).await {
                Ok(response) => response.errors,
                Err(e) => vec![e.to_string()],
            },
        }
    }
}

pub async fn delete_local(request: &DeleteRequest, qbit: &dyn QbitClient) -> DeleteResponse {
    let mut errors = Vec::new();
    for path in &request.paths {
        if path.exists() {
            if let Err(e) = std::fs::remove_file(path) {
                errors.push(format!("{}: {e}", path.display()));
            }
        }
    }
    for hash in &request.hashes {
        if let Err(e) = qbit.delete_torrent(hash, true).await {
            errors.push(format!("torrent {hash}: {e}"));
        }
    }
    DeleteResponse { errors }
}

/// `ssh` command rsync runs via `-e`. Includes `-i` when the Remote uses a specific key.
/// `known_hosts`, when set, is how cwRsync's ssh finds the host key file.
pub fn ssh_rsh(program: &str, identity: Option<&str>, known_hosts: Option<&str>) -> String {
    let mut parts = vec![quote_ssh_arg(program), "-o".into(), "BatchMode=yes".into()];
    if known_hosts.is_some() {
        parts.push("-o".into());
        parts.push("StrictHostKeyChecking=accept-new".into());
    }
    if let Some(known_hosts) = known_hosts {
        parts.push("-o".into());
        parts.push(format!("UserKnownHostsFile={}", quote_ssh_arg(known_hosts)));
    }
    if let Some(identity) = identity {
        parts.push("-i".into());
        parts.push(quote_ssh_arg(identity));
    }
    parts.join(" ")
}

/// What rsync should use for `-e` on this machine.
pub struct SshForTransfer {
    pub program: String,
    pub identity: Option<String>,
    pub known_hosts: Option<String>,
    /// Cygwin path of a running cwRsync agent, when one exists
    pub auth_sock: Option<String>,
}

/// cwRsync's own ssh. Windows OpenSSH translates the rsync protocol and the copy dies.
pub fn ssh_for_transfer(identity: Option<&str>) -> SshForTransfer {
    let bundled = bundled_rsync_ssh();
    if bundled == "ssh" {
        return SshForTransfer {
            program: "ssh".into(),
            identity: identity.map(|p| p.to_string()),
            known_hosts: None,
            auth_sock: None,
        };
    }
    let program = bundled.clone();
    let auth_sock = crate::agent_bridge::windows_agent_bridge_sock()
        .or_else(|| cygwin_auth_sock(std::path::Path::new(&program)));
    SshForTransfer {
        identity: identity.map(|p| cygdrive_path(std::path::Path::new(p))),
        known_hosts: known_hosts_file().map(|p| cygdrive_path(&p)),
        auth_sock,
        program,
    }
}

fn known_hosts_file() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".ssh").join("known_hosts"))
}

/// ssh.exe shipped next to cwRsync, not `C:\Windows\System32\OpenSSH\ssh.exe`
fn bundled_rsync_ssh() -> String {
    let Ok(output) = Command::new("where.exe").arg("rsync").output() else {
        return "ssh".into();
    };
    let first = String::from_utf8_lossy(&output.stdout);
    let Some(rsync) = first.lines().map(str::trim).find(|l| !l.is_empty()) else {
        return "ssh".into();
    };
    let rsync = PathBuf::from(rsync);
    let Some(dir) = rsync.parent() else {
        return "ssh".into();
    };
    let beside = dir.join("ssh.exe");
    if beside.is_file() && !is_windows_openssh(&beside) {
        return beside.display().to_string();
    }
    if let Some(root) = dir.parent() {
        let choco = root
            .join("lib")
            .join("rsync")
            .join("tools")
            .join("bin")
            .join("ssh.exe");
        if choco.is_file() {
            return choco.display().to_string();
        }
    }
    "ssh".into()
}

fn is_windows_openssh(path: &std::path::Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("OpenSSH")
    })
}

fn consider_sock(best: &mut Option<(std::time::SystemTime, String)>, path: &std::path::Path) {
    let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) else {
        return;
    };
    let cyg = cygdrive_path(path);
    if best.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
        *best = Some((modified, cyg));
    }
}

/// Newest cwRsync agent socket, as a Cygwin path
fn cygwin_auth_sock(ssh_exe: &std::path::Path) -> Option<String> {
    let mut best = None;
    if let Some(agent_dir) =
        directories::BaseDirs::new().map(|d| d.home_dir().join(".ssh").join("agent"))
    {
        if let Ok(entries) = std::fs::read_dir(&agent_dir) {
            for entry in entries.flatten() {
                consider_sock(&mut best, &entry.path());
            }
        }
    }
    if let Some(tmp) = ssh_exe
        .parent()
        .and_then(|bin| bin.parent())
        .map(|root| root.join("tmp"))
    {
        if let Ok(dirs) = std::fs::read_dir(tmp) {
            for dir in dirs.flatten() {
                let name = dir.file_name().to_string_lossy().to_string();
                if !name.starts_with("ssh-") {
                    continue;
                }
                let Ok(socks) = std::fs::read_dir(dir.path()) else {
                    continue;
                };
                for sock in socks.flatten() {
                    if sock.file_name().to_string_lossy().starts_with("agent.") {
                        consider_sock(&mut best, &sock.path());
                    }
                }
            }
        }
    }
    best.map(|(_, path)| path)
}

/// cwRsync is Cygwin, so `C:\Movies` is a remote named `C`. Give it `/cygdrive/c/Movies`.
pub fn cygdrive_path(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy();
    let extended = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    let mut chars = extended.chars();
    if let (Some(drive), Some(':'), Some(sep)) = (chars.next(), chars.next(), chars.next()) {
        if drive.is_ascii_alphabetic() && (sep == '\\' || sep == '/') {
            let rest: String = chars.collect::<String>().replace('\\', "/");
            return format!("/cygdrive/{}/{rest}", drive.to_ascii_lowercase());
        }
    }
    extended.replace('\\', "/")
}

/// A relative key such as `.ssh/server/server` is the one in the home directory
/// when it is not in the current directory.
pub fn resolve_identity(path: &str) -> String {
    let given = std::path::Path::new(path);
    if given.is_absolute() || given.exists() {
        return path.to_string();
    }
    if let Some(home) = directories::BaseDirs::new() {
        let from_home = home.home_dir().join(path);
        if from_home.exists() {
            return from_home.display().to_string();
        }
    }
    path.to_string()
}

fn quote_ssh_arg(arg: &str) -> String {
    let arg = arg.replace('\\', "/");
    if arg.chars().any(|c| c.is_whitespace()) {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        arg
    }
}

/// Writes one JSON line; used by the Agent to stream to the TUI
pub fn write_json_line<T: serde::Serialize>(out: &mut impl Write, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    out.write_all(b"\n")?;
    Ok(())
}
