# GEMINI.md

## Project Overview
**Cloud Store** is a fullstack system built with Rust and Dioxus for selling and managing NAT VPS instances. It features a micro-crate architecture within a Cargo workspace, separating concerns between the backend API, user storefront, and admin console. Virtualization is handled by **Incus (LXC)**, providing high-performance containers for NAT VPS deployments.

### Key Technologies
- **Backend:** [Axum](https://github.com/tokio-rs/axum) (API layer), [SQLite](https://sqlite.org/) (Persistence), [SQLx](https://github.com/launchbadge/sqlx) (Database toolkit).
- **Frontend:** [Dioxus](https://dioxuslabs.com/) (Fullstack GUI library, compiled to WebAssembly for the browser).
- **Virtualization:** [Incus](https://linuxcontainers.org/incus/) (LXC-based container management).
- **Payments:** [PayPal](https://developer.paypal.com/) integration for automated checkout and billing.
- **Tooling:** [Just](https://github.com/casey/just) (Command runner), [Nix](https://nixos.org/) & [direnv](https://direnv.net/) (Development environment), [.pre-commit](https://pre-commit.com/) (Git hooks).

### Architecture
- `crates/web-app`: The central Axum backend. It hosts two separate API listeners: a **Guest API** (port 8081) and an **Admin API** (port 8082). The Admin API supports full guest instance lifecycle management (list, stop, delete, add).
- `crates/frontend`: The user-facing storefront. Compiled to WASM, it interacts with the Guest API.
- `crates/admin-frontend`: The administrative dashboard. Supports managing guest users and their associated instances directly.
- `crates/shared-domain`: Common business logic, models (`NatPlan`, `Node`, `Instance`, `NatPortLease`), and enums shared across all crates.
- `crates/provider-adapter`: Abstraction layer for interacting with VPS providers. Implements `ComputeProvider` with `IncusProvider` using certificate-based authentication.
- `crates/worker`: Background worker for long-running tasks:
  - **Provisioning**: Automated node selection, container creation with secure root password initialization, and dynamic NAT allocation.
  - **Synchronization**: Periodic synchronization of instance statuses and metrics from remote Incus nodes.
  - **Renewals**: Automated instance renewal processing.
  - **Reconciliation**: Expiring overdue invoices and maintenance.

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
  - `just migrate`: Runs database migrations.
  - `pre-commit install`: Sets up automated git hooks.
- **Development:**
  - `just serve-api`: Runs the Axum backend.
  - `just serve-frontend`: Runs the Dioxus storefront (port 8080).
  - `just serve-admin-frontend`: Runs the Dioxus admin console (port 8083).
- **Verification:**
  - `just check`: Runs global compile checks for the workspace, including both frontends.
  - `just fmt`: Formats all code.
  - `just clippy`: Runs lints across the workspace.
  - `just test`: Runs all workspace tests.

---

## Development Conventions

### Code Style
- **Rust First:** All components are implemented in Rust.
- **Surgical Updates:** Prefer minimal, localized changes. Adhere to existing patterns (e.g., using `tracing` for logs, `anyhow` for top-level errors).
- **Frontend Patterns:** Modular page structure in `crates/frontend/src/pages/` and `crates/admin-frontend/src/pages/`. Global state is managed via signals and context providers.

### Database
- **Migrations:** All schema changes must be co-located in the `migrations/` directory using SQLx migration files.
- **Compatibility:** Keep SQL queries compatible with SQLite.

### Security
- **Isolation:** Guest and Admin APIs are isolated by port and authentication logic.
- **Incus Client Auth:** `IncusProvider` requires a client PEM identity (default `data/incus-client.pem`) for authenticating with remote Incus APIs.
- **Secrets:** Never log or commit sensitive data. Use `.env` for local configuration.

### Deployment & CI
- **Automated Checks:** All commits are checked via `.pre-commit` hooks for formatting (`fmt`), lints (`clippy`), and basic compilation (`check`).
- Ensure all `just check`, `just fmt`, and `just clippy` pass before finalizing changes.
- Frontend assets (CSS, images) are located in `assets/` within their respective crates.

