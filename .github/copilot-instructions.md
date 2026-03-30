# Project Guidelines

## Code Style

- Use Rust across the workspace.
- Keep modules focused and small; prefer explicit domain types in shared crates.
- Run formatting and lint checks before finalizing changes:
  - `just fmt`
  - `just clippy`

## Architecture

- Workspace layout:
  - `crates/web-app`: Dioxus + Axum app layer (routes, API handlers, UI entrypoints)
  - `crates/shared-domain`: shared business models and enums used across services
  - `crates/provider-adapter`: provider abstraction for VPS lifecycle operations
  - `crates/worker`: background processing for provisioning and renewals
- Database target is SQLite for low-memory runtime (`DATABASE_URL=sqlite://data/cloud_store.db`).
- Redis is optional runtime support and is memory-capped in compose.

## Build and Test

- Preferred setup:
  - `direnv allow`
  - `cp .env.example .env`
  - `mkdir -p data`
- Core commands:
  - `just check` for workspace compile checks
  - `just fmt` for formatting
  - `just clippy` for lints
  - `just up` to start optional local services
  - `just down` to stop services

## Conventions

- Keep database-related changes synchronized with SQLx migrations in `migrations/`.
- Use SQLite-compatible SQL in migrations; avoid PostgreSQL-specific features.
- Keep changes minimal and localized; avoid broad refactors when implementing feature slices.
- For project context and setup details, reference `README.md` rather than duplicating content here.
