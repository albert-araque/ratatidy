# Architecture Overview - Checkpoint 1

`ratatidy` is a TUI tool for managing hardlinks between downloads (qBittorrent) and media folders.

## Source Code Structure

- **`main.rs`**: Entry point. Handles the async runtime (Tokio), configuration loading, data fetching (qBit + Scanner), and the main TUI event loop.
- **`app.rs`**: Core application state (`App` struct). Manages the active tab, the current list of groups, selection index, and UI toggles (like `show_details`).
- **`config.rs`**: Configuration structure (`Config`). Defines `download_dir`, `media_dirs`, and deletion modes.
- **`scanner.rs`**: Hardware-level file analysis. Uses the Win32 API to find hardlinks by comparing `VolumeSerialNumber` and `FileIndex`. Groups physical files into `FileNode`s.
- **`grouping.rs`**: Logic to transform raw `FileNode`s into displayable `Group`s based on the selected POV (Media vs. Downloads).
    - *Media POV*: Groups by the top-level folder in the media library (Movie/Show name).
    - *Downloads POV*: Groups by the folder/item in the downloads directory.
- **`qbittorrent.rs`**: Integration layer for qBittorrent. Currently uses a `MockQbitClient` to simulate API responses for testing without a real server.
- **`tui.rs`**: Low-level terminal management. Handles raw mode, alternate screen, and cleanup.
- **`ui.rs`**: Rendering logic using `ratatui`. Defines the layout (Header, Main, Details, Footer) and styling.

## Key Data Flow
1. **Initial Scan**: `Scanner` finds all files and links them by "inode".
2. **Enrichment**: qBit mock data is matched against disk paths.
3. **Grouping**: `App` takes enriched nodes and creates POVs.
4. **Rendering**: `ui` renders the `current_groups()` of the selected tab.
