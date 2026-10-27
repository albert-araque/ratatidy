# Ratatidy 🐀📁

A TUI tool to manage your media library and downloads, ensuring hardlinks are intact and helping you clean up orphans.

## 🚀 Quick Start (Debian/Linux)

### 1. Installation
Ensure you have Rust installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`).

```bash
git clone <your-repo-url>
cd ratatidy
cargo build --release
```

### 2. Configuration
You can configure Ratatidy using command-line arguments, environment variables, or a `.env` file.

#### Option A: Command Line Arguments
```bash
./target/release/ratatidy --download-dir /path/to/downloads -m /path/to/movies,/path/to/tv --qbit-pass mypassword
```

#### Option B: Environment Variables
```bash
export RATATIDY_DOWNLOAD_DIR="/path/to/downloads"
export RATATIDY_MEDIA_DIRS="/path/to/movies,/path/to/tv"
export QBIT_PASS="mypassword"
./target/release/ratatidy
```

#### Option C: .env file
Create a `.env` file in the project root:
```ini
RATATIDY_DOWNLOAD_DIR=/path/to/downloads
RATATIDY_MEDIA_DIRS=/path/to/movies,/path/to/tv
QBIT_URL=http://localhost:8080
QBIT_USER=admin
QBIT_PASS=mypassword
```

## 🛰️ Remote mode (run the TUI on your PC)

Ratatidy can run on your PC while an **Agent** (the same binary) does the scanning, deleting and qBittorrent work on the server over SSH (see `docs/adr/0001-agent-remoto-por-ssh.md`).

1. On the server, configure ratatidy as usual (`~/.config/ratatidy/config.toml`) and install `rsync`.
2. Build a static binary on your PC and copy it to the server:
   ```bash
   nix develop .#musl -c cargo build --release
   ssh <alias> mkdir -p .local/bin
   scp target/x86_64-unknown-linux-musl/release/ratatidy <alias>:.local/bin/ratatidy
   ```
3. On your PC (needs `ssh` and `rsync`; key-based auth, no password prompts):
   ```bash
   ratatidy --remote <alias>            # or RATATIDY_REMOTE / `remote = "<alias>"` in config.toml
   ```
   `<alias>` is a host from `~/.ssh/config`, or `user@host`. The Agent path defaults to `~/.local/bin/ratatidy`; override it with `--remote-agent`. A key that is not the default one is `--remote-identity <file>`, the same `-i` you would pass to `ssh`. A relative path such as `.ssh/server/server` is read from your home directory.

In remote mode, **c** on a Media tab queues a **Transfer**: an rsync copy of the selected Group's folder to a directory on your PC (Tab completes paths, the last one is remembered). Transfers run one at a time in the background; quitting asks first and cancels them, and rsync resumes next time.

## Web catalog

On the Host (the machine that holds the files), `ratatidy serve` publishes a read-only catalog at <http://127.0.0.1:8787>. It scans in the background, groups the library the same way the TUI does, and does not delete or transfer anything. It only binds to a loopback address.

```bash
ratatidy serve
ratatidy serve --bind 127.0.0.1:8787   # or RATATIDY_BIND
```

Open another SSH session if you also want the TUI. To see a Remote, run `serve` there and forward the port (`ssh -L 8787:127.0.0.1:8787`).

## 🧑‍💻 Development

`nix develop` gives a shell with Rust, rust-analyzer, rsync and ssh.

## ⌨️ Controls
- **Tab**: Switch between Media and Downloads views.
- **Arrows/j/k**: Navigate groups.
- **i**: Toggle details panel (see exact file paths and link status).
- **/**: Search.
- **s**: Cycle sort modes (Name, Size, Date Added).
- **S**: Toggle sort order (Ascending/Descending).
- **f**: Cycle filters (All, Orphans, Hardlinked).
- **r**: Rescan files (refresh without restart).
- **d**: Open delete/purge menu.
- **c**: Transfer the selected Group to this PC (remote mode, Media tabs).
- **Enter**: Confirm action in menus.
- **Esc**: Cancel / Close search.
- **q**: Quit.

## ⚠️ Safety
Ratatidy performs **permanent deletions**. 
- Deleting from **Downloads** will also remove the torrent from qBittorrent.
- Use the **Details Panel (`i`)** to verify which files are being targeted before confirming.
- Default mode is **Dry Run: False**.
