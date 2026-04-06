set shell := ["bash", "-cu"]

up:
  docker compose up -d

down:
  docker compose down

logs:
  docker compose logs -f

check:
  cargo check --workspace
  just check-frontend
  just check-admin-frontend

check-backend:
  cargo check -p web-app -p worker -p provider-adapter

check-frontend:
  cd crates/frontend && timeout 30s dx build --platform web

check-admin-frontend:
  cd crates/admin-frontend && timeout 30s dx build --platform web
  
serve-api:
  cargo run -p web-app

serve-worker:
  cargo run -p worker

serve-backend:
  # Starts both web-app and worker concurrently
  (cargo run -p web-app & cargo run -p worker & wait)

serve-frontend:
  cd crates/frontend && dx serve --platform web --port 8080

serve-admin-frontend:
  cd crates/admin-frontend && dx serve --platform web --port 8083

migrate:
  sqlx migrate run --database-url "$DATABASE_URL"

fmt:
  cargo fmt --all

fmt-backend:
  cargo fmt --package shared-domain --package provider-adapter --package web-app --package worker --package admin-frontend

clippy:
  cargo clippy --workspace --all-targets --all-features -- -D warnings

clippy-backend:
  cargo clippy --package shared-domain --package provider-adapter --package web-app --package worker --package admin-frontend --all-targets --all-features -- -D warnings

test:
  cargo test --workspace

seed:
  sqlite3 data/cloud_store.db < scripts/seed_dev_data.sql
