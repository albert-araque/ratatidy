# Implementation State - Checkpoint 3 (2026-01-31)

## Completed Phases

- **Phase 1: Setup + Minimal TUI** [x]
- **Phase 2: Hardlink Scanner** [x]
- **Phase 3: qBittorrent MVP (Mock)** [x]
- **Phase 4: Grouping and 2 POVs** [x]
- **Phase 5: Container Actions (Permanent Delete)** [x]
- **Phase 6: Filters + Search + Dashboard** [x]
- **Phase 7: Real API + Portability** [x]
    - [x] `RealQbitClient` implementation.
    - [x] CLI & Environment Variables configuration.
    - [x] Cross-platform support (Debian/Linux via RustTLS).
    - [x] Single Source of Truth Refactor.
    - [x] Rescan Logic (`r` key).
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
1. **Performance**: Scanning multi-TB libraries (thousands of files) causes a noticeable pause at startup.
2. **Empty Folder Cleanup**: "Container" mode (deleting logical groups) leaves empty parent folders.
3. **Matching Accuracy**: qBit-to-disk matching uses a name heuristic; ideally, it should use file indices or paths.
4. **Sort indicator**: Currently added (Shift+S), but needs verification for all sort modes.
5. **Multi-path Autocompletion**: During interactive setup, when prompted for `media_dirs` (comma-separated), autocompletion only works for the first path.
    - **Proposed Solution**: Implement a custom `rustyline` Helper/Completer that splits the input by commas and performs path completion only on the substring following the last comma.
