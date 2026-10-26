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
}

impl Remote {
    fn command(&self, agent_cmd: &str) -> Command {
        let mut cmd = Command::new("ssh");
        // BatchMode: a password prompt would corrupt the TUI, fail instead
        cmd.args(["-o", "BatchMode=yes", &self.alias, &self.agent, "agent", agent_cmd]);
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

/// Writes one JSON line; used by the Agent to stream to the TUI
pub fn write_json_line<T: serde::Serialize>(out: &mut impl Write, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    out.write_all(b"\n")?;
    Ok(())
}
