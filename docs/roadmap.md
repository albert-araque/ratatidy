# Future Roadmap (Updated 2026-01-31)

## Phase 7: Real API + Compatibility [x]
- [x] Real qBittorrent API integration.
- [x] Linux / Debian compatibility (RustTLS instead of OpenSSL).
- [x] **Rescan Logic (`r`)**: Refresh file list without app restart.
- [ ] **Empty Folder Cleanup**: Remove parent directories when they become empty.

## Phase 8: Configuration & UX Improvements [x]
- [x] **Interactive First Run**: Prompt user for paths if not configured.
- [x] **Optional qBittorrent**: Works without credentials (offline mode).
- [x] **CLI + Env Coexistence**: `--dry-run` flag and env vars work together.
- [x] **dotenvy Support**: Load `.env` file from current directory.

## Phase 9: Performance & Scaling [x]
- [x] **Async Scanning**: Use background thread/async for disk scan.
- [x] **Progress Indicator**: Show scanning progress for large libraries (2-3TB+).
- [x] **Lazy Loading**: Load groups on-demand instead of all at startup.
- [x] **Caching**: Cache scan results to avoid full rescan on every startup.

## Phase 10: Safety & Robustness [ ]
- [ ] **Move to Trash (`t`)**: Optional file movement to a `.trash` folder.
- [ ] **Container Safety Rule**: Prevent deleting folders with files outside the group.
- [ ] **Inode-based qBit Matching**: Use absolute paths for accurate torrent matching.

## Phase 11: Advanced Features [ ]
- [ ] **Remote Scan**: Support scanning via SSH or agent.
- [ ] **Web UI**: Simple read-only dashboard.

## Current Known Issues
1. **Scaling**: Scanning huge libraries (thousands of files) causes a noticeable pause.
2. **Matching**: Torrent name changes can break current heuristic matching.
3. **Empty Folders**: Deleting files leaves empty parent folders behind.
4. **Interactive Setup UX**: Multi-path autocompletion (comma-separated) only works for the first path.
    - *Plan*: Custom `Completer` for `rustyline` to handle comma-delimited tokens.
