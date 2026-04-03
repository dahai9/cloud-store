# Project Guidelines

## Code Style

- Use Rust across the workspace.
- Keep modules focused and small; prefer explicit domain types in shared crates.
- Run formatting and lint checks before finalizing changes:
  - `just fmt`
  - `just clippy`

## Architecture

- Workspace layout:
  - `crates/frontend`: Dioxus WebAssembly frontend app. This owns routing, page composition, client session state, and browser-only UX.
  - `crates/admin-frontend`: Dioxus WebAssembly admin console. Isolated from the main storefront but shares visual components and domain models.
  - `crates/web-app`: Axum backend API layer. This owns HTTP routes, auth, payment/webhook handling, SQLite persistence, and server-side orchestration.
  - `crates/shared-domain`: shared business models, enums, and value types that need to stay consistent across services.
  - `crates/provider-adapter`: provider abstraction for VPS lifecycle operations and any provider-specific provisioning logic.
  - `crates/worker`: background processing for provisioning, renewals, and other deferred jobs.
- Database target is SQLite for low-memory runtime (`DATABASE_URL=sqlite://data/cloud_store.db`).
- Redis is optional runtime support and is memory-capped in compose.
- Runtime topology:
  - Frontend usually runs on `http://127.0.0.1:8080`.
  - Backend API usually runs on `http://127.0.0.1:8081`.
  - The frontend talks to the backend through `api_base` in client session state.
  - PayPal sandbox callbacks return to the backend first, then the backend redirects back to the frontend after capture/finalization.
- Frontend entrypoints:
  - `crates/frontend/src/main.rs` launches `pages::App` on wasm32.
  - `crates/frontend/src/pages/mod.rs` wires routing and global session context.
  - `crates/frontend/src/models.rs` is the source of truth for `Route`, `SessionState`, and the product plan catalog.
  - `crates/frontend/src/pages/public.rs` owns the public storefront and order flow.
  - `crates/frontend/src/pages/auth.rs` owns login-related UX.
  - `crates/frontend/src/pages/dashboard.rs` owns authenticated dashboard pages such as profile, services, tickets, and balance.
- Backend entrypoints:
  - `crates/web-app/src/main.rs` boots Axum, loads env vars, opens SQLite, runs migrations, and binds the listener.
  - `crates/web-app/src/routes.rs` defines the portal route surface used by the backend.
  - `crates/web-app/src/auth.rs` contains session and permission checks.
  - `crates/web-app/src/admin.rs` owns admin-side plan validation, including traffic limit rules.
  - `crates/web-app/src/billing.rs` and `crates/web-app/src/tickets.rs` handle core business APIs.
  - `crates/web-app/src/payment/paypal.rs` contains the PayPal checkout, return, capture, and webhook flow.
- Request flow summary:
  1. Anonymous user browses `StorefrontPage` and selects a plan.
  2. Login stores session state on the client, then the frontend fetches the authenticated bundle.
  3. Checkout creates an internal order and invoice first, then creates a PayPal order.
  4. PayPal approval can arrive through the return URL or webhook, and both paths are written to be idempotent.
  5. A successful payment updates local order/invoice state, then redirects the browser back to the frontend balance page.
- When changing behavior, prefer the narrowest file that owns the concern:
  - Routing and page state: `crates/frontend/src/models.rs` and `crates/frontend/src/pages/*`.
  - Client API calls: `crates/frontend/src/api.rs`.
  - Shared UI/session types: `crates/frontend/src/models.rs`.
  - Auth, DB, or payment flow: `crates/web-app/src/*.rs`.
  - Cross-crate domain types: `crates/shared-domain`.

- Traffic semantics are part of the product contract: a `traffic_gb` value of `-1` means unlimited traffic. Keep admin validation, billing mappings, frontend display, and provisioning logic aligned with that rule.

## Build and Test

- Preferred setup:
  - `direnv allow`
  - `cp .env.example .env`
  - `mkdir -p data`
- When operating this repository, use `just` targets for routine tasks instead of calling `cargo`, `dx`, or `docker compose` directly.
- If a workflow does not already have a `just` target, add one before relying on ad hoc commands when practical.
- Core commands:
  - `just check` for workspace compile checks (includes Rust workspace, frontend, and admin-frontend)
  - `just check-backend` for backend-only compile checks
  - `just check-frontend` specifically for the main storefront build
  - `just check-admin-frontend` specifically for the admin console build
  - `just fmt-backend` and `just clippy-backend` for backend/admin crates when storefront parsing is blocked
  - `just serve-api` to run the Axum backend
  - `just serve-frontend` to run the Dioxus storefront
  - `just serve-admin-frontend` to run the Dioxus admin console
  - `just fmt` for formatting
  - `just clippy` for lints
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` for strict workspace lint validation when chasing clippy blockers
  - `just up` to start optional local services
  - `just down` to stop services

## Conventions

- Keep database-related changes synchronized with SQLx migrations in `migrations/`.
- Use SQLite-compatible SQL in migrations; avoid PostgreSQL-specific features.
- Keep changes minimal and localized; avoid broad refactors when implementing feature slices.
- For project context and setup details, reference `README.md` rather than duplicating content here.
