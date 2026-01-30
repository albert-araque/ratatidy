# Implementation Plan - Phase 1: Setup + Minimal TUI

This plan covers the initial setup of the ratatidy project, including dependencies, configuration, and a basic TUI structure.

## Proposed Changes

### [ratatidy]

#### [MODIFY] Cargo.toml
Add dependencies:
- **ratatui**: TUI framework.
- **crossterm**: Terminal handling.
- **serde & serde_derive**: Serialization/deserialization.
- **toml**: Configuration file parsing.
- **config**: Configuration management.
- **anyhow**: Error handling.

#### [NEW] config.rs
Define the `Config` struct based on the requirements in `plan.md`.

#### [NEW] tui.rs
Setup the basic TUI loop and terminal initialization/restoration.

#### [NEW] app.rs
Define the `App` state and basic UI drawing logic.

#### [MODIFY] main.rs
Entry point to initialize config, TUI, and run the app loop.

## Verification Plan

### Automated Tests
- `cargo test` (once logic is added).

### Manual Verification
- Run `cargo run` and verify that a TUI opens with a header, footer, and basic navigation indicators.
- Check if it correctly loads a default configuration or errors gracefully if missing.