# Backend verification

The backend is tested against isolated PostgreSQL, Redis, S3-compatible storage, Anvil, and the actual factory/escrow implementation.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib
bash scripts/test.sh
python3 scripts/generate-openapi.py --check
python3 scripts/validate-openapi.py
```

Install the dependencies documented in [README.md](README.md), including `scripts/requirements.txt` for OpenAPI validation. The integration suite captures API responses for schema validation in the ignored build directory.

Coverage includes SIWE challenge replay, authorization, CSRF, idempotency, optimistic versions, concurrent registration, deadlines, evidence ownership, review, allocation consistency, onchain event indexing, administration, and claims. Contract-wallet authentication and verification-failure handling are included in the implementation's checks.

Dependency scanning uses `scripts/security-audit.py` with `cargo-audit`. It distinguishes the active dependency graph from optional lockfile entries; a successful scan is not an application audit. The source and API consistency checks are part of [CI](../.github/workflows/backend.yml).

See [project verification](../docs/VERIFICATION.md) for recorded live results and [security boundaries](SECURITY.md) for remaining assumptions. Local benchmark output is not a production throughput or availability guarantee.
