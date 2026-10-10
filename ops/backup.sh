#!/usr/bin/env bash
set -euo pipefail
umask 077
cd /root/arc
exec 9>/var/lib/oppor/backups/.lock
flock -n 9 || exit 0
backup_stamp=$(date -u +%Y%m%dT%H%M%SZ)
backup_work=$(mktemp -d /var/lib/oppor/backups/.staging.XXXXXX)
trap 'rm -rf "$backup_work"' EXIT
backup_recipient=$(age-keygen -y /root/.config/oppor/production/backup-age.key)
backup_file=/var/lib/oppor/backups/oppor-${backup_stamp}.tar.gz.age
docker compose -f ops/compose.production.yml exec -T postgres pg_dump -U oppor_migrator -d oppor --format=custom > "$backup_work/database.dump"
# Back up the seed key and service secrets as the same encrypted recovery set.
cp /root/.config/oppor/production/service-credentials.json "$backup_work/"
cp /root/.config/oppor/production/application.env "$backup_work/"
cp /root/.config/oppor/production/migration.env "$backup_work/"
tar -C "$backup_work" -czf - database.dump service-credentials.json application.env migration.env | age -r "$backup_recipient" -o "$backup_file"
# Prepare curl credentials in a private file rather than process arguments.
python3 - "$backup_work/curl.conf" "$backup_file" <<'PY'
import json,sys,pathlib
c=json.load(open('/root/.config/oppor/cloudflare.json'))
config='aws-sigv4 = "aws:amz:auto:s3"\nuser = "'+c['access_key_id']+':'+c['secret_access_key']+'"\nurl = "'+c['endpoint']+'/oppor-backups/backups/'+pathlib.Path(sys.argv[2]).name+'"\n'
pathlib.Path(sys.argv[1]).write_text(config)
PY
curl --config "$backup_work/curl.conf" --fail --silent --show-error --upload-file "$backup_file" --output /dev/null
# Local copies supplement the encrypted off-server archive.
find /var/lib/oppor/backups -maxdepth 1 -type f -name 'oppor-*.tar.gz.age' -mtime +7 -delete
printf 'Encrypted database recovery set uploaded: %s\n' "$(basename "$backup_file")"
