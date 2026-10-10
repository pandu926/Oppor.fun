#!/usr/bin/env bash
set -euo pipefail
umask 077
cd /root/arc
if [ "$#" -ne 1 ]; then printf 'Usage: ops/restore-check.sh /path/to/oppor-backup.tar.gz.age\n' >&2; exit 2; fi
restore_work=$(mktemp -d /var/lib/oppor/backups/.restore.XXXXXX)
restore_database=oppor_restore_check_$(date -u +%Y%m%d%H%M%S)
cleanup() {
    docker compose -f ops/compose.production.yml exec -T postgres dropdb -U oppor_owner --if-exists "$restore_database" >/dev/null 2>&1 || true
    rm -rf "$restore_work"
}
trap cleanup EXIT
age -d -i /root/.config/oppor/production/backup-age.key "$1" | tar -xz -C "$restore_work"
docker compose -f ops/compose.production.yml exec -T postgres createdb -U oppor_owner -O oppor_migrator "$restore_database"
docker compose -f ops/compose.production.yml exec -T postgres pg_restore -U oppor_migrator --dbname="$restore_database" --exit-on-error --no-owner < "$restore_work/database.dump"
restore_count=$(docker compose -f ops/compose.production.yml exec -T postgres psql -U oppor_owner -d "$restore_database" -Atc 'SELECT count(*) FROM _sqlx_migrations WHERE success')
if [ "$restore_count" -lt 5 ]; then printf 'Recovery check failed: missing migrations.\n' >&2; exit 1; fi
printf 'Encrypted recovery archive decrypted and database restored successfully (%s migrations).\n' "$restore_count"
