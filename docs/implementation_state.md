# Implementation State - Checkpoint 3 (2026-01-31)

## Completed Phases

- **Phase 1: Setup + Minimal TUI** [x]
- **Phase 2: Hardlink Scanner** [x]
- **Phase 3: qBittorrent MVP (Mock)** [x]
- **Phase 4: Grouping and 2 POVs** [x]
- **Phase 5: Container Actions (Permanent Delete)** [x]
- **Phase 6: Filters + Search + Dashboard** [x]
- **Phase 7: Real API + Portability** [/]
    - [x] `RealQbitClient` implementation.
    - [x] CLI & Environment Variables configuration.
    - [x] Cross-platform support (Debian/Linux via RustTLS).
    - [x] Single Source of Truth Refactor.
    - [ ] Rescan Logic (`r` key).
    - [ ] Empty Folder Cleanup.

## Current Status
The application is functional for basic usage. It accurately detects hardlinks on both Windows and Linux,
integrates with real qBittorrent instances, and provides a context-aware deletion interface.

## Pending Work (Based on User Feedback)

### Configuration UX
1. **Interactive First Run**: Prompt for `download_dir` and `media_dirs` if not set (instead of crashing).
2. **Optional qBittorrent**: Allow running without qBit credentials (offline mode).
3. **dotenvy Support**: Automatically load `.env` from current directory.

### Performance
1. **Slow Startup**: Scanning 2-3TB libraries with thousands of files causes noticeable delay.
2. **Planned Fix**: Async scanning with progress indicator.

## Technical Debt / Known Issues
1. **Startup Crash**: Missing `download_dir` causes immediate failure.
2. **Empty Folder Cleanup**: "Container" mode leaves empty parent folders.
3. **Fragile Matching**: qBit-to-disk matching uses simplified name-contains logic.
4. **No Rescan**: Must restart app to refresh file list.
