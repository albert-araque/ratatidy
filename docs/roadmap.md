# Future Roadmap

## Phase 5: Container Actions (Purge) [x]
- **Objective**: Allow the user to permanently remove items with context awareness.
- **Tasks**:
    - [x] Implement context-aware scope detection (Downloads vs Media vs Both).
    - [x] Permanent deletion logic using `std::fs::remove_file`.
    - [x] Irreversible confirmation dialog.

## Phase 6: Filters + Search + UI Polish
- **Objective**: Improve usability and visibility.
- **Tasks**:
    - [x] Dynamic search bar (`/`).
    - [ ] Fix **Search Key Conflict** (Bug: actions firing during search).
    - [ ] Dynamic File Size columns (MB/GB).
    - [ ] Sorting: Alpha, Size, Date, Savings.
    - [ ] Quick filters: `Orphans Only`, `No Seeding`, `Duplicates`.
    - [ ] Top dashboard showing total recovered space.

## Phase 7: Real API + Compatibility
- **Objective**: Transition to production and Linux.
- **Tasks**:
    - [ ] Implement `RealQbitClient` using `reqwest`.
    - [ ] Linux Compatibility (Debian/Ubuntu focus).
    - [ ] Auto or manual rescan logic (`r`).
    - [ ] Connection error handling and persistent logs.

## Future Ideas & Brainstorming
- (Add new ideas here)

## Current Known Issues & Debt
- **Heuristic Matching**: Name-based matching between qBit and disk is a heuristic (to be refined).
- **UI State vs Disk**: Actions only remove items from the UI list; a full re-scan is needed for consistency if the disk changes externally.
- **Hardcoded Paths**: Configuration is hardcoded to `mock_env` if it exists.

## BUGS
- (Add new bugs here)
