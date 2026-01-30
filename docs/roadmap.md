# Future Roadmap (Updated)

## Phase 7: Real API + Compatibility (Finishing) [/]
- [x] Real qBittorrent API integration.
- [x] Linux / Debian compatibility.
- [ ] **Rescan Logic (`r`)**: Allow manual refresh of the master list without app restart.
- [ ] **Empty Folder Cleanup**: Automatically (or optionally) remove parent directories when they become empty after a purge.

## Phase 8: Safety & Robustness [ ]
- [ ] **Move to Trash (`t`)**: Implement optional file movement to a `.trash` folder instead of immediate deletion.
- [ ] **Container Safety Rule**: Prevent deleting folders that contain files not belonging to the selected group.
- [ ] **Inode-based qBit Matching**: Use absolute paths or internal IDs for 100% accurate torrent-to-file matching.
- [ ] **Rescan Logic (`r`)**: Allow manual refresh of the master list without app restart.
- [ ] **Empty Folder Cleanup**: Automatically remove parent directories when they become empty after a purge.

## Phase 9: Advanced Features [ ]
- [ ] **Remote Scan**: Support scanning a remote server via SSH or a small agent.
- [ ] **Web UI**: A simple read-only web view of the dashboard.

## Current Known Issues
- **Scaling**: Scanning huge libraries (10k+ files) might pause the TUI slightly (needs async scanning).
- **Matching**: Torrent name changes can break the current heuristic matching.
