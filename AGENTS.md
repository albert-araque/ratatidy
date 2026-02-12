# AGENTS.md - Developer Context & Memory

> **System Instruction**: Always read this file first to understand the project context, architectural invariants, and current operational state.

## 1. Project Identity
**Name**: Ratatidy
**Goal**: TUI-based media manager to prune downloaded files while preserving hardlinks and seeding status.
**Stack**: Rust (2021), Ratatui (TUI), Crossterm, SQL/JSON (Caching), qBittorrent API (reqwest).

## 2. Architecture Invariants
1.  **Single Source of Truth**: `App.nodes` is the master list of physical files. Views (Groups) are derived/computed from this list.
2.  **Physical Identity**: Files are identified by `(DeviceID, Inode)`. A `FileNode` represents a physical file on disk; it may have multiple `paths`.
3.  **Safety First**: Deletions are context-aware. The app distinguishes between removing a *hardlink* (path) and destroying the *file* (inode).
4.  **Async/Sync Split**: 
    - **Scanner**: Runs in a background thread, communicates via `mpsc` channel.
    - **UI**: Runs in the main thread with a synchronous event polling loop (`tick`).

## 3. Map of the Territory
- **`src/scanner.rs`**: Low-level FS operations, cross-platform inode reading (Win32/Unix), caching logic.
- **`src/app.rs`**: State container, action handling (delete/update), business logic.
- **`src/ui.rs`**: Pure rendering logic. Stateless drawing based on `App` state.
- **`docs/`**:
- **docs/**:
    - `STATUS.md`: **PRIMARY SOURCE OF TRUTH**. Combines implementation state and future roadmap.
    - `flow_analysis.md`: Detailed explanation of the app's internal logic cycles.
    - `design_concepts.md`: High-level architectural thoughts and initial planning artifacts.

## 4. Common Commands
- **Run (Interactive)**: `cargo run` (Prompts for paths if missing/configured).
- **Run (Quick)**: `cargo run` (Uses `.env` or `config.toml` if present).
- **Test**: `cargo test`
- **Build**: `cargo build --release`

## 5. Development Rules
- **Documentation**: Update `docs/implementation_state.md` immediately after completing a feature.
- **Concurrency**: Do not block the main TUI thread. Use channels for heavy lifting.
- **UI UX**: Prioritize responsiveness. Keep the "Game Loop" fast.
