# Implementation State - Checkpoint 1

## Completed Phases

- **Phase 1: Setup + Minimal TUI** [x]
    - Initial project setup, basic TUI layout, and tab navigation.
- **Phase 2: Hardlink Scanner** [x]
    - Win32 API integration for cross-volume hardlink detection.
    - Mock environment script (`setup_mock.ps1`) for local testing.
- **Phase 3: qBittorrent MVP (Mock)** [x]
    - Async client trait and mock implementation.
    - Seeding status labels in UI.
- **Phase 4: Grouping and 2 POVs** [x]
    - Logic for grouping by Media vs. Downloads.
    - Dynamic UI labels (`SEED`, `LINK`).
    - Toggleable Detail Panel (`i` key).

## Current Status
The application is fully functional in a "mock" environment. You can see how many files in a Movie folder are also in Downloads, if they are seeding, and if they are correctly hardlinked (saving space).

## Technical Debt / Known Issues
- Name-based matching between qBit and disk is a heuristic (to be refined).
- Manual "Trash/Delete" actions are not yet implemented.
- Configuration is hardcoded to `mock_env` if it exists.
