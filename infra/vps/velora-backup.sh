#!/usr/bin/env bash
# Nightly Velora Panel backup for a single-host (VPS) deployment.
# Installed as /usr/local/sbin/velora-backup; see infra/vps/README.md.
#
# 1. Copy panel-data to a staging tree while the Panel runs (bulk transfer).
# 2. Stop the Panel briefly, re-sync the delta and restart it, so the archive is
#    a cold, consistent copy of every SQLite store, signing key and upload.
# 3. Verify every database, archive and checksum it, keep KEEP_LOCAL_DAYS locally
#    and ship it to an offsite host (REMOTE) with a write-only rrsync key.
# Status for the operations dashboard is written to $STATE/last.json.
set -euo pipefail
umask 077

# Site values live in /etc/velora/backup.env (not in Git), for example:
#   DATA=/home/ubuntu/velora-platform/panel-data
#   CONTAINER=velora-platform-panel
#   REMOTE=user@offsite-host:
#   REMOTE_KEY=/root/.ssh/velora-backup-offsite
#   REMOTE_KNOWN_HOSTS=/root/.ssh/known_hosts_offsite
if [ -f /etc/velora/backup.env ]; then . /etc/velora/backup.env; fi
DATA=${DATA:?set DATA in /etc/velora/backup.env}
CONTAINER=${CONTAINER:-velora-platform-panel}
ROOT=${ROOT:-/var/backups/velora}
STAGING=$ROOT/staging
STATE=${STATE:-/var/lib/velora-backup}
KEEP_LOCAL_DAYS=${KEEP_LOCAL_DAYS:-7}
REMOTE=${REMOTE:-}
SSH="ssh -i ${REMOTE_KEY:-/root/.ssh/velora-backup-offsite} -o IdentitiesOnly=yes -o BatchMode=yes -o UserKnownHostsFile=${REMOTE_KNOWN_HOSTS:-/root/.ssh/known_hosts_offsite} -o StrictHostKeyChecking=yes -o ConnectTimeout=20"

mkdir -p "$STAGING" "$STATE"
# A request file from the operations dashboard is consumed by this run.
rm -f "$STATE/request"
started=$(date -u +%Y-%m-%dT%H:%M:%SZ)
stamp=$(date -u +%Y%m%dT%H%M%SZ)
archive="$ROOT/velora-panel-data-$stamp.tar.gz"
stopped=0

status() { # status <ok|failed> <message> [offsite]
  python3 -I - "$1" "$2" "${3:-false}" "$started" "$archive" "$STATE" <<'PY'
import json, os, sys, datetime
state, message, offsite, started, archive, directory = sys.argv[1:]
out = {"status": state, "message": message, "started_at": started,
       "finished_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
       "archive": os.path.basename(archive), "offsite": offsite == "true"}
if os.path.exists(archive):
    out["bytes"] = os.path.getsize(archive)
    out["sha256"] = open(archive + ".sha256").read().split()[0] if os.path.exists(archive + ".sha256") else None
tmp = os.path.join(directory, "last.json.tmp")
json.dump(out, open(tmp, "w"), indent=1)
os.chmod(tmp, 0o644)
os.replace(tmp, os.path.join(directory, "last.json"))
PY
}

restart_panel() {
  if [ "$stopped" = 1 ]; then
    docker start "$CONTAINER" >/dev/null && stopped=0
  fi
}
fail() { restart_panel || true; echo "velora-backup: $1" >&2; status failed "$1"; exit 1; }
trap 'fail "unexpected error on line $LINENO"' ERR
trap 'restart_panel' EXIT

[ -f "$DATA/panel.db" ] || fail "panel data not found at $DATA"

rsync -a --delete "$DATA/" "$STAGING/"
pause_start=$(date -u +%s)
docker stop -t 30 "$CONTAINER" >/dev/null
stopped=1
rsync -a --delete "$DATA/" "$STAGING/"
restart_panel
pause=$(( $(date -u +%s) - pause_start ))

python3 -I - "$STAGING" <<'PY' || fail "database integrity check failed"
import pathlib, sqlite3, sys
bad = []
for db in pathlib.Path(sys.argv[1]).rglob("panel.db"):
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    if c.execute("PRAGMA quick_check").fetchone()[0] != "ok":
        bad.append(str(db))
sys.exit(1 if bad else 0)
PY

tar -C "$STAGING" -czf "$archive.partial" .
mv "$archive.partial" "$archive"
( cd "$ROOT" && sha256sum "$(basename "$archive")" > "$archive.sha256" )
chmod 644 "$archive.sha256"

find "$ROOT" -maxdepth 1 -type f -name 'velora-panel-data-*' -mtime +"$KEEP_LOCAL_DAYS" -delete

if [ -z "$REMOTE" ]; then
  status ok "backup complete (no offsite target configured); panel paused ${pause}s" false
elif rsync -t -e "$SSH" "$archive" "$archive.sha256" "$REMOTE"; then
  status ok "backup complete; panel paused ${pause}s" true
else
  status failed "archive kept locally but the offsite copy failed" false
  exit 1
fi
echo "velora-backup: $(basename "$archive") stored locally${REMOTE:+ and offsite}"
