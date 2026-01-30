# Architecture Overview - Checkpoint 2

`ratatidy` is a TUI tool for managing hardlinks between downloads (qBittorrent) and media folders.

## Source Code Structure

- **`main.rs`**: Entry point. Handles the async runtime (Tokio), configuration loading, and the main TUI event loop. Prioritizes search input over global hotkeys when active.
- **`app.rs`**: Core application state (`App` struct).
    - **State**: Manages groups, selection, search query (via `search_active`), and UI toggles.
    - **Logic**: Implements `current_groups()` which handles dynamic **Filtering** (Orphans/Seeding/Linked) and **Sorting** (Name/Size/Seeding).
    - **Actions**: Handles **Permanent Deletion** logic with context-aware scope detection (determining if a group exists in Downloads, Media, or both).
- **`config.rs`**: Configuration structure (`Config`). Defines `download_dir`, `media_dirs`, and environment settings.
- **`scanner.rs`**: Hardware-level file analysis. Uses the Win32 API to find hardlinks by comparing `VolumeSerialNumber` and `IndexNumber`.
- **`grouping.rs`**: Logic to transform raw `FileNode`s into displayable `Group`s based on the selected POV (Media vs. Downloads).
- **`qbittorrent.rs`**: Integration layer for qBittorrent. Defines the `QbitClient` trait and `MockQbitClient` implementation.
- **`tui.rs`**: Low-level terminal management.
- **`ui.rs`**: Rendering logic using `ratatui`.
    - **Layout**: Header (Tabs), **Dashboard** (Total stats/savings), Main (List + Sidebar), and Footer.
    - **Components**: Includes a centered **Confirmation Overlay** for irreversible actions and a dynamic search bar.

## Key Data Flow
1. **Initial Scan**: `Scanner` finds all files and links them by logical identity (Device + Inode).
2. **Enrichment**: qBit torrent data is matched against disk paths to detect seeding status.
3. **Grouping**: `App` takes nodes and creates tab-specific POVs.
4. **Interactive Loop**:
    - **Search/Filter**: `current_groups()` dynamically calculates the visible subset.
    - **Sort**: Groups are ordered based on the active `SortBy` mode.
5. **Rendering**: `ui` renders the final state, including a "Success Dashboard" that calculates disk savings in real-time.
