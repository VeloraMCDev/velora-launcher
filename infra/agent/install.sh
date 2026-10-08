#!/usr/bin/env bash
# First-install bootstrap; never replaces an identity, configuration or service.
set -euo pipefail
set +x
umask 077
fail() { printf '%s\n' "$1" >&2; exit 1; }
[[ $EUID == 0 ]] || fail 'Run this installer with sudo in your terminal.'
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || fail 'This artifact requires Linux x86_64.'
[[ -d /run/systemd/system ]] || fail 'Systemd is required.'
for task_command in python3 sha256sum runuser systemctl; do
  command -v "$task_command" >/dev/null || fail "Required command unavailable: $task_command"
done

if [[ ${1:-} == enroll && $# == 1 ]]; then
  [[ -f /etc/velora-agent/config.toml && -f /etc/velora-agent/identity.key ]] || fail 'Install the verified agent first.'
  [[ -t 0 ]] || fail 'Enrollment requires your interactive terminal.'
  trap 'unset task_token' EXIT
  printf 'Paste the one-use code from the deployment panel, then press Enter (input is hidden): ' >&2
  IFS= read -rs task_token
  printf '\n' >&2
  [[ ${#task_token} == 43 && $task_token != *[!A-Za-z0-9_-]* ]] || fail 'Invalid enrollment code format.'
  printf '%s\n' "$task_token" | runuser -u velora-agent -- /usr/local/lib/velora-agent/velora-agent enroll --config /etc/velora-agent/config.toml
  unset task_token
  runuser -u velora-agent -- /usr/local/lib/velora-agent/velora-agent heartbeat --config /etc/velora-agent/config.toml
  systemctl enable --now velora-agent.service
  systemctl is-active --quiet velora-agent.service
  printf 'DONE. The Development heartbeat service is running.\n'
  exit 0
fi

[[ ${1:-} == install && $# == 4 ]] || fail 'Usage: install.sh install BUNDLE_DIRECTORY MANIFEST_SHA256 HTTPS_CONTROL_ORIGIN | enroll'
task_bundle=$(realpath -- "$2")
task_checksum=$3
task_origin=$4
[[ $task_checksum =~ ^[a-f0-9]{64}$ ]] || fail 'Expected manifest checksum required.'
[[ -f $task_bundle/artifact-manifest.json && ! -L $task_bundle/artifact-manifest.json ]] || fail 'Manifest unavailable.'
[[ $(sha256sum "$task_bundle/artifact-manifest.json" | cut -d ' ' -f 1) == "$task_checksum" ]] || fail 'Manifest checksum mismatch.'

# Validate all bytes and the origin before creating any account or destination.
python3 - "$task_bundle" "$task_origin" <<'PY'
import hashlib, json, pathlib, re, sys, urllib.parse
bundle = pathlib.Path(sys.argv[1])
origin = urllib.parse.urlsplit(sys.argv[2])
if (origin.scheme != 'https' or not origin.hostname or origin.username or origin.password
    or origin.path not in ('', '/') or origin.query or origin.fragment
    or not re.fullmatch(r'https://[a-zA-Z0-9.-]+', sys.argv[2])):
    raise SystemExit('A plain HTTPS control origin is required.')
manifest = json.loads((bundle / 'artifact-manifest.json').read_text())
if (manifest.get('schema') != 1 or manifest.get('service') != 'velora-agent'
    or manifest.get('repository') != 'VeloraMCDev/infra'
    or manifest.get('platform') != 'linux-amd64' or manifest.get('protocol') != 1
    or manifest.get('capabilities') != ['REPORT_HEARTBEAT']
    or not re.fullmatch('[a-f0-9]{40}', manifest.get('source_sha', ''))):
    raise SystemExit('Unsupported bootstrap manifest.')
files = manifest.get('files', [])
if len(files) != 3 or {f.get('path') for f in files} != {'velora-agent', 'velora-agent.service', 'install.sh'}:
    raise SystemExit('Unexpected artifact file inventory.')
for item in files:
    path = bundle / item['path']
    if path.is_symlink() or not path.is_file():
        raise SystemExit('Artifact must contain regular files.')
    data = path.read_bytes()
    if len(data) != item['bytes'] or hashlib.sha256(data).hexdigest() != item['sha256']:
        raise SystemExit('Artifact checksum mismatch.')
PY

for task_path in /etc/velora-agent /usr/local/lib/velora-agent /etc/systemd/system/velora-agent.service; do
  [[ ! -e $task_path && ! -L $task_path ]] || fail 'An agent installation already exists; no files were replaced.'
done
! getent passwd velora-agent >/dev/null || fail 'The service account already exists; review it before installation.'
! getent group velora-agent >/dev/null || fail 'The service group already exists; review it before installation.'
useradd --system --user-group --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin velora-agent
install -d -o root -g root -m 0755 /usr/local/lib/velora-agent
install -o root -g root -m 0755 "$task_bundle/velora-agent" /usr/local/lib/velora-agent/velora-agent
install -o root -g root -m 0755 "$task_bundle/install.sh" /usr/local/lib/velora-agent/install.sh
install -o root -g root -m 0644 "$task_bundle/velora-agent.service" /etc/systemd/system/velora-agent.service
install -d -o velora-agent -g velora-agent -m 0700 /etc/velora-agent
printf 'schema = 1\ncontrol_origin = "%s"\nenvironment = "development"\nidentity_path = "/etc/velora-agent/identity.key"\nheartbeat_seconds = 45\n' "$task_origin" > /etc/velora-agent/config.toml
chown root:velora-agent /etc/velora-agent/config.toml
chmod 0640 /etc/velora-agent/config.toml
runuser -u velora-agent -- /usr/local/lib/velora-agent/velora-agent identity-init --config /etc/velora-agent/config.toml
chown root:velora-agent /etc/velora-agent
chmod 0750 /etc/velora-agent
systemctl daemon-reload
runuser -u velora-agent -- /usr/local/lib/velora-agent/velora-agent doctor --config /etc/velora-agent/config.toml
printf 'Installed without starting the service. Next: sudo /usr/local/lib/velora-agent/install.sh enroll\n'
