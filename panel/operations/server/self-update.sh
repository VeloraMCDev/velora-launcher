#!/bin/sh
# Runs inside the Docker CLI helper, independently of the dashboard being replaced.
set -eu
project=$1
env_file=$2
rollback_file=$3
result_file=$(dirname "$env_file")/.ops-update-result
compose() { docker compose -p "$project" "$@"; }
healthy() {
  for attempt in $(seq 60); do
    container=$(compose ps -q ops) || container=
    if [ -n "$container" ]; then
      health=$(docker inspect --format '{{.State.Health.Status}}' "$container") || health=
      [ "$health" = healthy ] && return 0
    fi
    sleep 3
  done
  return 1
}
restore() {
  trap - EXIT
  cp -p "$rollback_file" "$env_file.tmp" && mv "$env_file.tmp" "$env_file"
  if compose up -d --no-deps ops && healthy; then
    printf '%s\n' rolled-back > "$result_file"
    rm -f "$rollback_file"
  else
    printf '%s\n' rollback-failed > "$result_file"
    echo 'Dashboard rollback failed; retained .env.ops-rollback for recovery.' >&2
  fi
  exit 1
}
trap restore EXIT
printf '%s\n' running > "$result_file"
sleep 3
compose pull ops
compose up -d --no-deps ops
healthy
printf '%s\n' succeeded > "$result_file"
rm -f "$rollback_file"
trap - EXIT
