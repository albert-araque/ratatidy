# Implementation State - Checkpoint 2

## Completed Phases

- **Phase 1: Setup + Minimal TUI** [x]
- **Phase 2: Hardlink Scanner** [x]
- **Phase 3: qBittorrent MVP (Mock)** [x]
- **Phase 4: Grouping and 2 POVs** [x]
- **Phase 5: Container Actions (Permanent Delete)** [x]
- **Phase 6: Filters + Search + Dashboard** [x]
    - Search conflict fixed.
    - Sorting by Name/Size.
    - Filters for Orphans/Hardlinked.
    - Space savings dashboard.
- **Phase 7: Real API + Portability** [/]
    - [x] `RealQbitClient` implementation.
    - [x] CLI & Environment Variables configuration.
    - [x] Cross-platform support (Debian/Linux).
    - [x] Single Source of Truth Refactor (app state sync).

## Current Status
The application is production-ready for basic usage. It accurately detects hardlinks on both Windows and Linux, integrates with real qBittorrent instances, and provides a safe, context-aware deletion interface.

## Technical Debt / Known Issues
1. **Empty Folder Cleanup**: "Container" mode deletes files but leaves the parent folders if they become empty.
2. **Rescan Logic**: No "r" key to refresh data from disk without restarting.
3. **Fragile Matching**: qBit-to-disk matching uses simplified name-contains logic.
