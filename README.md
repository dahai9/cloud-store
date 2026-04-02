# cloud_store

Rust + Dioxus fullstack system for selling and managing NAT VPS instances.

## Current Scope
- Backend API server (Axum + SQLite)
- Frontend Dioxus storefront (run by dx)
- Admin Dioxus console (run by dx, isolated console)
- Shared domain models for orders, billing, tickets, subscriptions
- Provider adapter abstraction for VPS lifecycle operations
- Worker skeleton for provisioning and renewal jobs
- SQLite (default) + Redis via Docker Compose
- Nix + direnv development environment

## Quick Start
1. Enable direnv in your shell.
2. Run `direnv allow` in project root.
3. Copy env: `cp .env.example .env` and adjust secrets.
4. Create local db directory: `mkdir -p data`.
5. Start optional services if needed: `docker compose up -d`.
6. If you need Mailhog UI in development: `docker compose --profile dev-tools up -d`.
7. Start backend API: `just serve-api` (default `APP_PORT=8081`).
8. Start storefront app: `just serve-frontend` (runs `dx` on 8080).
9. Start admin console: `just serve-admin-frontend` (runs `dx` on 8083).
10. Run workspace checks: `just check`.

## Frontend / Backend Split
- Frontend: `crates/frontend` (Dioxus web, managed by `dx`)
- Backend: `crates/web-app` (Axum API server)
- API routes are under `/api/*` and CORS is enabled for local frontend development.

## Next Implementation Milestones
- Auth/session and RBAC
- PayPal checkout + webhook idempotency
- Order and subscription state machine
- NAT port-pool allocation and conflict-safe provisioning
- Ticket center with priority/category/attachments
