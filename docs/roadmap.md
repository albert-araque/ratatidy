# Future Roadmap

## Phase 5: "Container" Actions with Trash
- **Objective**: Allow the user to perform actions on entire groups.
- **Tasks**:
    - [ ] Implement Windows Recycle Bin integration (`trash` crate).
    - [ ] Confirmation dialog before deletion.
    - [ ] "Delete only in Downloads" (safe for Media) vs. "Full deletion" logic.

## Phase 6: Filters + Search + Dashboard
- **Objective**: Improve usability with large datasets.
- **Tasks**:
    - [ ] Dynamic search bar (`/`).
    - [ ] Quick filters: `Orphans Only`, `No Seeding`, `Duplicates`.
    - [ ] Top dashboard showing total recovered space.

## Phase 7: Real API + Refinement
- **Objective**: Transition from Mock to Production.
- **Tasks**:
    - [ ] Implement `RealQbitClient` using `reqwest`.
    - [ ] Auto or manual rescan logic (`r`).
    - [ ] Connection error handling and persistent logs.
