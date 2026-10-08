#!/usr/bin/env bash
# Stops the demo panel (and its heartbeat) started by run.sh.
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for f in .hb.pid .pid; do
  if [ -f "$HERE/$f" ]; then
    pid="$(cat "$HERE/$f")"
    if kill -0 "$pid" 2>/dev/null; then kill "$pid" && echo "stopped $f (pid $pid)"; fi
    rm -f "$HERE/$f"
  fi
done
