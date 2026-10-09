#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
command -v docker >/dev/null
command -v forge >/dev/null
command -v anvil >/dev/null
docker compose -f compose.test.yml up -d postgres redis storage
for attempt in $(seq 1 60); do
  if docker compose -f compose.test.yml exec -T postgres pg_isready -U oppor -d oppor >/dev/null 2>&1 && curl --fail --silent http://127.0.0.1:59009/moto-api/ >/dev/null; then break; fi
  sleep 1
done
bash ../contracts/scripts/bootstrap.sh
forge build --root ../contracts
forge build --root tests/fixtures
cargo test --locked --lib
cargo test --locked --test integration -- --ignored --nocapture
# Test containers are isolated under the oppor-backend-tests Compose project.
# Stop them explicitly with: docker compose -f compose.test.yml down
