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

check-frontend:
  cd crates/frontend && timeout 30s dx build --platform web

check-admin-frontend:
  cd crates/admin-frontend && timeout 30s dx build --platform web
  
serve-api:
  cargo run -p web-app

serve-frontend:
  cd crates/frontend && dx serve --platform web --port 8080

serve-admin-frontend:
  cd crates/admin-frontend && dx serve --platform web --port 8083

migrate:
  sqlx migrate run --database-url "$DATABASE_URL"

fmt:
  cargo fmt --all

clippy:
  cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
  cargo test --workspace
