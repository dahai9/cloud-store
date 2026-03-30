set shell := ["bash", "-cu"]

up:
  docker compose up -d

down:
  docker compose down

logs:
  docker compose logs -f

check:
  cargo check --workspace

serve-api:
  cargo run -p web-app

serve-frontend:
  cd crates/frontend && dx serve --platform web --port 8080

migrate:
  sqlx migrate run --database-url "$DATABASE_URL"

fmt:
  cargo fmt --all

clippy:
  cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
  cargo test --workspace
