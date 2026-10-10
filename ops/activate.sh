#!/usr/bin/env bash
set -euo pipefail
cd /root/arc
exec 9>/root/.config/oppor/production/.activation.lock
flock -n 9 || { printf 'Another activation is running.\n' >&2; exit 1; }
if [ "$#" -ne 1 ]; then printf 'Usage: ops/activate.sh /path/to/verified-deployment-manifest.json\n' >&2; exit 2; fi
# This verifies chain, transaction input, receipt, confirmations, and runtime bytecode.
node ops/configure-production.mjs --manifest="$1"
docker compose -f ops/compose.production.yml --profile operations run --rm migrate
docker compose -f ops/compose.production.yml exec -T postgres psql -U oppor_migrator -d oppor -v ON_ERROR_STOP=1 < ops/runtime-grants.sql
docker compose -f ops/compose.production.yml --profile application up -d api worker
ready=0
for attempt in $(seq 1 60); do
    if curl --fail --silent http://127.0.0.1:18821/v1/health/ready >/dev/null; then ready=1; break; fi
    sleep 2
done
if [ "$ready" -ne 1 ]; then printf 'API readiness failed. Public activation was stopped.\n' >&2; exit 1; fi
# Migrations, grants, and indexing must pass before the live frontend is published.
(cd frontend && npm run build)
node ops/publish-web.mjs
python3 - <<'PY'
from pathlib import Path
p=Path('ops/nginx/site.conf')
p.write_text(Path('ops/nginx/site.live.conf').read_text())
PY
docker compose -f ops/compose.web.yml exec -T web nginx -t
docker compose -f ops/compose.web.yml exec -T web nginx -s reload
curl --fail --silent https://oppor.fun/v1/health/ready >/dev/null
printf 'Oppor live API and frontend activated after verified readiness.\n'
