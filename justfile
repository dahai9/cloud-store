set shell := ["bash", "-cu"]

up:
  docker compose up -d

down:
  docker compose down

logs:
  docker compose logs -f

check:
  cargo check --workspace

fmt:
  cargo fmt --all

clippy:
  cargo clippy --workspace --all-targets --all-features -- -D warnings
