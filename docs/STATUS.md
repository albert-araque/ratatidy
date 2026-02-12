# Ratatidy Proyect Status

> **Last Updated**: 2026-02-12
> **Current Version**: 0.2.0-beta

## 1. Overview
The application is fully functional for its core purpose: detecting and managing hardlinks between downloads (qBittorrent) and media libraries (Radarr/Sonarr). It supports cross-platform scanning, context-aware deletions, and sync integration with qBittorrent.

## 2. Completed Features
### Core
- [x] **Hardlink Detection**: Correctly identifies files sharing the same `(DeviceID, Inode)` on Windows and Linux.
- [x] **Scanning**: Async scanning with progress indicator for large libraries (multi-TB).
- [x] **Two-Pane View**: Media-centric and Download-centric views.
- [x] **Grouping**: Logical grouping of files into movies/series.

### Integration
- [x] **qBittorrent**: Real API integration (list torrents, delete torrents).
- [x] **Offline Mode**: Graceful fallback if qBittorrent is unreachable.

### UX/UI
- [x] **Interactive Setup**: Auto-prompts for paths if config is missing (supports multi-path comma completion).
- [x] **Dashboard**: Real-time stats on space saved and orphan files.
- [x] **Filters**: Toggle between All, Orphans, and Hardlinked files (`f` key).
- [x] **Search**: Fuzzy search by group name (`/` key).
- [x] **Sort**: Sort by Name, Size, or Date (`s` / `S` keys).
- [x] **Rescan**: Trigger a rescan without restarting (`r` key).

### Safety
- [x] **Context-Aware Delete**: Options to delete from Media, Downloads, or Both.
- [x] **Sync Deletion**: Deleting from "Downloads" scope also removes the torrent from qBittorrent.

## 3. Pending Features (Roadmap)

### Near Term (Polishing)
- [ ] **Empty Folder Cleanup**: Automatically remove parent directories when they become empty after a purge.
- [ ] **Trash Support**: Move files to a `.trash` or recycle bin instead of permanent deletion (`t` key).

### Medium Term (robustness)
- [ ] **Inode-based Matching**: Improve qBittorrent matching accuracy by using absolute paths instead of name heuristics.
- [ ] **Container Safety**: strictly prevent deleting folders that contain files outside the known group structure.

### Long Term
- [ ] **Web UI**: A read-only web dashboard for remote viewing.
- [ ] **Remote Scan**: Agent-based scanning for remote servers via SSH.

## 4. Known Issues
1. **Empty Folder Clutter**: "Container" deletion mode works great for files, but leaves empty directories behind.
2. **Matching Heuristics**: If a torrent name differs significantly from the file path, the app might fail to link them (currently relies on `path.contains(torrent_name)`).
3. **Sort Indicators**: The UI shows sort indicators, but comprehensive verification of all sort permutations is pending.
