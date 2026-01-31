# Future Roadmap (Updated 2026-01-31)

## Phase 7: Real API + Compatibility (Finishing) [/]
- [x] Real qBittorrent API integration.
- [x] Linux / Debian compatibility (RustTLS instead of OpenSSL).
- [ ] **Rescan Logic (`r`)**: Allow manual refresh without app restart.
- [ ] **Empty Folder Cleanup**: Remove parent directories when they become empty.

## Phase 8: Configuration & UX Improvements [ ]
- [ ] **Interactive First Run**: If no paths configured, prompt user interactively for `download_dir` and `media_dirs`.
- [ ] **Optional qBittorrent**: If qBit credentials not set, skip torrent deletion (just delete files). Show seeding info only when configured.
- [ ] **CLI + Env Coexistence**: Ensure `--dry-run` flag and `RATATIDY_DRY_RUN` env var work together (CLI wins).
- [ ] **dotenvy Support**: Load `.env` file from current directory automatically.

## Phase 9: Performance & Scaling [ ]
- [ ] **Async Scanning**: Use background thread/async for disk scan so TUI remains responsive.
- [ ] **Progress Indicator**: Show scanning progress for large libraries (2-3TB+).
- [ ] **Lazy Loading**: Load groups on-demand instead of all at startup.
- [ ] **Caching**: Cache scan results to avoid full rescan on every startup.

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
3. **Startup**: Program crashes if `download_dir` is not set (no graceful error).
