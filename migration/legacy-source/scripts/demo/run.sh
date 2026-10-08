#!/usr/bin/env bash
# Starts a throwaway SCOPENET panel on 127.0.0.1:18080 and fills it with demo data.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
PORT="${DEMO_PORT:-18080}"
BASE="http://127.0.0.1:$PORT"
DATA="$HERE/.data"
BIN="$ROOT/target/debug/scopenet-panel"

# Stop a previous demo panel first.
"$HERE/stop.sh" >/dev/null 2>&1 || true

# Web UI: build when missing or older than the sources.
WEB="$ROOT/panel/web"
if [ ! -f "$WEB/dist/index.html" ] || [ -n "$(find "$WEB/src" "$WEB/index.html" "$WEB/package.json" -newer "$WEB/dist/index.html" -print -quit 2>/dev/null)" ]; then
  echo ">> building panel/web"
  (cd "$WEB" && { [ -d node_modules ] || npm ci; } && npm run build)
fi

# Panel binary.
if [ ! -x "$BIN" ]; then
  echo ">> building scopenet-panel"
  (cd "$ROOT" && cargo build -p scopenet-panel)
fi

# Fresh data dir.
rm -rf "$DATA"
mkdir -p "$DATA"

echo ">> starting panel on $BASE"
SCOPENET_BIND="127.0.0.1:$PORT" \
SCOPENET_DATA_DIR="$DATA" \
SCOPENET_WEB_DIR="$ROOT/panel/web/dist" \
SCOPENET_ICONS_DIR="$ROOT/panel/icons/dist" \
ADMIN_USERNAME=admin \
ADMIN_PASSWORD=demo-admin-pass \
JWT_SECRET=scopenet-demo-jwt-secret-0123456789-abcdefghijklmnop \
PUBLIC_URL="$BASE" \
RUST_LOG="${RUST_LOG:-info}" \
  nohup "$BIN" >"$HERE/.panel.log" 2>&1 &
echo $! >"$HERE/.pid"

for _ in $(seq 1 100); do
  if curl -fsS "$BASE/healthz" >/dev/null 2>&1; then break; fi
  if ! kill -0 "$(cat "$HERE/.pid")" 2>/dev/null; then
    echo "panel exited early, see $HERE/.panel.log" >&2; tail -20 "$HERE/.panel.log" >&2; exit 1
  fi
  sleep 0.3
done
curl -fsS "$BASE/healthz" >/dev/null || { echo "panel did not become healthy, see $HERE/.panel.log" >&2; exit 1; }

echo ">> seeding"
if ! DEMO_BASE="$BASE" node "$HERE/seed.mjs"; then
  echo "seeding failed (panel left running, log: $HERE/.panel.log)" >&2
  exit 1
fi

# Keep the demo game servers "online" (a heartbeat sync every 30 s).
nohup node "$HERE/seed.mjs" --heartbeat >"$HERE/.heartbeat.log" 2>&1 &
echo $! >"$HERE/.hb.pid"

cat <<MSG

Demo panel is running (pid $(cat "$HERE/.pid")) at $BASE
  admin   : admin / demo-admin-pass
  player  : Alex_Miner / demo-pass-1234   (all demo players use demo-pass-1234)
  servers : id 1 'Survival SMP', id 2 'Creative Build'
Stop with scripts/demo/stop.sh
MSG
