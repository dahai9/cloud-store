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

## Adding Nodes

There are two different node-related workflows in this repository:

### 1. Register a node in the admin console

Use this when you want the backend to know about a node and track its capacity.

- Sign in to the admin console.
- Open the `Nodes` page.
- Click `Add Node`.
- Fill in the node metadata:
  - name
  - region
  - total CPU cores
  - total memory in MB
  - total storage in GB
  - optional API endpoint
  - optional API token
- Submit the form.

The admin console sends the request to `POST /api/admin/nodes`, and the node will appear in the dashboard after the list is refreshed.

### 2. Provision Incus hosts in bulk

Use this when you want to install and initialize Incus on one or more machines.

- Create `scripts/nodes.txt` with one target IP address per line.
- Make sure the target machines allow SSH access as `root`, or change `SSH_USER` in `scripts/cluster_deploy.sh`.
- Review `INCUS_PORT` in `scripts/cluster_deploy.sh` if you need a non-default HTTPS port.
- Run `bash scripts/cluster_deploy.sh` from the repository root.

The cluster script copies `scripts/deploy_incus.sh` to each host and executes it remotely. The remote script installs Incus, writes the preseed configuration, creates the `incusbr0` NAT bridge, and runs `incus admin init --preseed`.

If you run `scripts/deploy_incus.sh` directly, only `INCUS_PORT` matters. The Cloud Store backend caches its Incus client certificate in `data/incus-client.pem` and reuses it across restarts. The trust token is only needed the first time the certificate is added to a node.

These two flows complement each other: the script prepares the host, and the admin console registers the node record that the backend uses for scheduling.

## Next Implementation Milestones
- Auth/session and RBAC
- Order and subscription state machine
- NAT port-pool allocation and conflict-safe provisioning
- Ticket center with priority/category/attachments
