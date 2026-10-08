#!/usr/bin/env bash
# Read-only host checks. Does not create containers, change services or read secrets.
set -uo pipefail

failures=0
pass() { printf 'PASS %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1"; failures=$((failures + 1)); }

if [[ $(uname -s) == Linux ]]; then pass 'Linux host'; else fail 'Linux is required'; fi
architecture=$(uname -m)
if [[ $architecture == x86_64 ]]; then
  pass 'linux/amd64 matches the initial probe image'
else
  fail 'Initial probe is linux/amd64 only; add and verify the actual host architecture before deployment'
fi

if command -v systemctl >/dev/null 2>&1 && [[ -d /run/systemd/system ]]; then
  pass 'systemd runtime is available'
else
  fail 'Native systemd service runtime was not detected'
fi

if ! command -v timeout >/dev/null 2>&1; then
  fail 'GNU timeout is required for bounded Docker checks'
elif ! command -v docker >/dev/null 2>&1; then
  fail 'Docker CLI is unavailable to this user'
else
  runtime=$(timeout 15 docker info --format '{{.OSType}} {{.Architecture}}' 2>/dev/null)
  if [[ $runtime == 'linux x86_64' || $runtime == 'linux amd64' ]]; then
    pass 'Docker daemon is reachable and matches the probe platform'
  else
    fail 'Docker daemon must be reachable by the intended service user and use linux/amd64'
  fi
  compose=$(timeout 15 docker compose version --short 2>/dev/null)
  if [[ $compose =~ ^v?[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    pass "Docker Compose plugin is available ($compose)"
  else
    fail 'Docker Compose plugin is unavailable'
  fi
fi

printf 'This check does not verify enrollment, outbound control-plane HTTPS, disk budgets, environment isolation or deployed health.\n'
exit "$((failures > 0))"
