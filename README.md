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
- **Enter**: Confirm action in menus.
- **Esc**: Cancel / Close search.
- **q**: Quit.

## ⚠️ Safety
Ratatidy performs **permanent deletions**. 
- Deleting from **Downloads** will also remove the torrent from qBittorrent.
- Use the **Details Panel (`i`)** to verify which files are being targeted before confirming.
- Default mode is **Dry Run: False**.
