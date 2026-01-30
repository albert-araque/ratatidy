# Development Plan (Adjusted): Rust TUI for Media Management with Hardlinks

## 0) Objective and Problem
A Rust-based TUI to detect and manage hardlinks between:
- `/downloads` (qBittorrent)
- `/media/movies` and `/media/tvshows` (Radarr/Sonarr)

**Confirmed Key Fact:** qBittorrent seeds from `/downloads`. Deleting there **breaks seeding** even if a hardlink exists in `/media`.

---

## 1) Finalized Decisions + Implications
1. **Seeding from `/downloads` (Confirmed)**  
   - Actions affecting `/downloads` must be **protected**: locked by default or requires "strong confirmation."
2. **Media Files**  
   - Usually just video files, but **deletion actions will be per container folder** (not just per file) to clean up all related junk.
3. **No extras/samples or weird structure**  
   - Simplifies mapping and reduces heuristics.
4. **Permissions/User (Clarification)**  
   - "Trees" = *directory trees* (`/downloads` and `/media...`).  
   - The app needs:
     - **Read access** to scan both.
     - **Write access** to move to trash or delete (in the relevant tree).

---

## 2) New Approach: 2 POVs (Media vs. Downloads)
Layer-based approach.

### POV 1 — Media
List of **Movies/Series** (grouped by folder in `/media`), showing:
- Video files (episodes or movie files).
- Related qBittorrent torrents/files (linked via inode).

Group Actions:
- Delete/Move to Trash: **media-only**, **downloads-only**, or **both**.
- In series: Option to act per episode (file) or per group (folder/season/series) based on what is detected as a "safe container."

### POV 2 — Downloads
List of **torrents (or their content root folder/file)** in `/downloads`, showing:
- Which movies/series/episodes are related in `/media`.

Actions:
- Delete/Move to Trash: **downloads-only**, **media-only**, or **both** (with guards if seeding).

---

## 3) Data Model (Adjusted for "folder deletion" + 2 POVs)
### 3.1 Hardlink Identity (Robust)
Hardlinks identified by `(dev, inode)`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileKey { dev: u64, inode: u64 }
```

### 3.2 Basic Node: Physical File (file-level)
```rust
struct FileNode {
    key: FileKey,
    size: u64,
    nlink: u64,
    paths: Vec<std::path::PathBuf>, // can be in downloads, media, or both

    // Derived
    has_downloads: bool,
    has_media: bool,

    // qBittorrent enrichment (if applicable)
    torrent_hash: Option<String>,
    torrent_name: Option<String>,
    is_seeding: bool,
    ratio: Option<f32>,
    qb_match_confident: bool,
}
```

### 3.3 Groupings (group-level)
Groups needed for Media and Downloads POVs:

```rust
enum GroupKind { Movie, Series, DownloadTorrent }

struct Group {
    kind: GroupKind,
    title: String,

    // "Containers" to delete/move (key requirement)
    media_container: Option<std::path::PathBuf>,     // folder in /media to operate on
    downloads_container: Option<std::path::PathBuf>, // folder/file in /downloads to operate on

    nodes: Vec<FileNode>,     // involved files
    related_torrents: Vec<String>, // related hashes (if any)
}
```

**How to pick a container (safe rules):**
- `media_container`: Folder containing the group's videos (e.g., movie folder or series folder).
- `downloads_container`: Preferably the root of the torrent content (if it's a folder, that folder; if it's a single file, the file).

**Container Safety Rule:** Only allow "delete folder" if the container doesn't include files outside the group (or if the user accepts a strong confirmation).

---

## 4) Mapping qBittorrent ↔ Disk (Adjusted)
**Strategy:**
1. Use `torrents/info` for status, ratio, and paths.
2. Use `torrents/files` to list files inside the torrent.
3. Build an index of download paths → torrent hash.
4. Cross-reference with `FileNode` by exact path in `/downloads` (and then link back to `/media` via hardlinks).
5. Set `qb_match_confident = true` only when paths match unambiguously.

---

## 5) TUI (Adjusted)
### Recommended Layout (Simple but useful)
- **Header:** Stats (orphans, seeding, space saved).
- **Main Panel:** List based on active POV.
- **Secondary Panel (Detail):** Paths, related torrents, available actions.
- **Footer:** Shortcuts.

### Controls
- **Tab:** Switch POV (Media / Downloads).
- **`/`**: Search.
- **`f`**: Filters (orphans, seeding, media-only, downloads-only).
- **`Enter`**: Open detail / expand group.
- **`t`**: Move to Trash (default).
- **`d`**: Permanent deletion (requires `dry_run=false` + strong confirmation).
- **`i`**: Info (dev/inode/nlink + relations).
- **`q`**: Quit.

### Mandatory Guards (for seeding cases)
If `is_seeding = true` and the action affects `downloads_container`:
- Block by default or ask for strong (2-step) confirmation.
- Show explicit warning: "THIS WILL STOP SEEDING."

---

## 6) Config (Recommended TOML)
Add:
- `delete_mode = "container"` (default) or `"file"`.
- `trash_dir` (recommended).
- `dry_run` (default `true`).
- `video_extensions` (only detect videos, but "container" action affects the whole folder).

---

## 7) Development Phases
1. **Phase 1 — Setup + Config + Minimal TUI** [x]
2. **Phase 2 — (dev, inode) Scanner + FileNode** [x]
3. **Phase 3 — qBittorrent MVP (Mock API)** [x]
4. **Phase 4 — Grouping and 2 POVs** [x]
5. **Phase 5 — "Container" Actions with Trash + Confirmations** [ ]
6. **Phase 6 — Filters + Search + Dashboard** [ ]
7. **Phase 7 — Real API + Refinements + Logs** [ ]

---

## 8) Risks and Mitigation
- **Breaking Seeding:** Guards + strong confirmation + default to "Trash."
- **Deleting Unrelated Files:** Content checks + fallback to file-only or strong confirmation.
- **Ambiguous qBit Mapping:** Mark confidence level; show "Heuristic Match" in UI.

---

## 9) Summary
Manageable if done incrementally:
1. File-level (hardlinks).
2. qBittorrent mapping.
3. Grouping + 2 POVs.
4. Container-based actions with safety.