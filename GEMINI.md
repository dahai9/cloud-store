# GEMINI.md

## Project Overview
**Cloud Store** is a fullstack system built with Rust and Dioxus for selling and managing NAT VPS instances. It features a micro-crate architecture within a Cargo workspace, separating concerns between the backend API, user storefront, and admin console.

### Key Technologies
- **Backend:** [Axum](https://github.com/tokio-rs/axum) (API layer), [SQLite](https://sqlite.org/) (Persistence), [SQLx](https://github.com/launchbadge/sqlx) (Database toolkit).
- **Frontend:** [Dioxus](https://dioxuslabs.com/) (Fullstack GUI library, compiled to WebAssembly for the browser).
- **Payments:** [PayPal](https://developer.paypal.com/) integration for automated checkout and billing.
- **Tooling:** [Just](https://github.com/casey/just) (Command runner), [Nix](https://nixos.org/) & [direnv](https://direnv.net/) (Development environment).

### Architecture
- `crates/web-app`: The central Axum backend. It hosts two separate API listeners: a **Guest API** (typically on port 8081) and an **Admin API** (typically on port 8082).
- `crates/frontend`: The user-facing storefront. Compiled to WASM, it interacts with the Guest API.
- `crates/admin-frontend`: The administrative dashboard. Also compiled to WASM, it interacts with the Admin API.
- `crates/shared-domain`: Common business logic, models, and enums shared across all crates.
- `crates/provider-adapter`: Abstraction layer for interacting with VPS providers (provisioning, lifecycle management).
- `crates/worker`: Background worker for long-running tasks like provisioning, renewals, and reconciliation.

---

## Building and Running

### Prerequisites
- [Rust](https://www.rust-lang.org/) toolchain.
- [Dioxus CLI](https://dioxuslabs.com/learn/0.6/getting_started) (`dx`) for frontend serving.
- [SQLite](https://sqlite.org/) for the database.
- (Optional) [Docker Compose](https://docs.docker.com/compose/) for external services (Redis, Mailhog).

### Key Commands (via `just`)
- **Setup:**
  - `direnv allow` (if using direnv)
  - `cp .env.example .env` (then configure secrets)
  - `mkdir -p data`
- **Development:**
  - `just serve-api`: Runs the Axum backend.
  - `just serve-frontend`: Runs the Dioxus storefront (port 8080).
  - `just serve-admin-frontend`: Runs the Dioxus admin console (port 8083).
- **Verification:**
  - `just check`: Runs global compile checks for the workspace, including both frontends.
  - `just check-frontend`: Targeted build check for the storefront.
  - `just check-admin-frontend`: Targeted build check for the admin console.
  - `just fmt`: Formats all code.
  - `just clippy`: Runs lints across the workspace.
  - `just test`: Runs all workspace tests.

---

## Development Conventions

### Code Style
- **Rust First:** All components are implemented in Rust.
- **Surgical Updates:** Prefer minimal, localized changes. Adhere to existing patterns (e.g., using `tracing` for logs, `anyhow` for top-level errors).
- **Frontend Patterns:** Use the modular page structure in `crates/frontend/src/pages/` and `crates/admin-frontend/src/pages/`. Global state should be managed via signals and context providers.

### Database
- **Migrations:** All schema changes must be co-located in the `migrations/` directory using SQLx migration files.
- **Compatibility:** Keep SQL queries compatible with SQLite.

### Security
- **Isolation:** The Guest and Admin APIs are strictly isolated by port and authentication logic.
- **Secrets:** Never log or commit sensitive data. Use `.env` for local configuration.

### Deployment & CI
- Ensure all `just check`, `just fmt`, and `just clippy` pass before finalizing changes.
- Frontend assets (CSS, images) are located in `assets/` within their respective crates.
