# Zing Game - Rust Workspace Agent Guide

## Build/Test Commands
- `cargo test` - Run all tests (NOTE: zing-server tests require `shuttle run` to be running first)
- `cargo test [TESTNAME]` - Run specific test by name
- `cargo build` - Build all workspace members
- `shuttle run` - Run server (requires Docker)
- `cargo clippy` - Lint code
- `cargo fmt` - Format code
- WASM build: `RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build zing-ui-lib --release --target web`

## Architecture
- **Workspace members**: zing-game (game logic), zing-server (REST API + WebSockets + Postgres), zing-ui-lib (Bevy UI), zing-ui (binary wrapper)
- **Database**: PostgreSQL via sea-orm, with migration sub-crate in zing-server/migration
- **API**: axum-based REST + WebSocket for real-time game updates
- **Frontend**: Bevy-based UI (can compile to WASM) + minimal Quasar web UI in zing-server/assets/

## Code Style
- Standard Rust formatting with `cargo fmt`
- Use qualified imports (`use bevy::prelude::*`, `use zing_game::{game::CardState, Back, Rank, Suit}`)
- Components derive `Component, TypePath` for Bevy systems
- Error handling with standard Result types
- Snake_case for modules/functions, PascalCase for types
- Public API modules re-exported in lib.rs
