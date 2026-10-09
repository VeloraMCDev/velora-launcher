#!/usr/bin/env bash
set -euo pipefail
[[ ${GITHUB_ACTIONS:-} == true && ${RUNNER_OS:-} == Linux ]] || { echo 'Run only on a disposable GitHub Linux runner.' >&2; exit 1; }
old="$RUNNER_TEMP/velora-upgrade-from-1.3.0.deb"
curl -fsSL https://github.com/VeloraMCDev/velora-launcher/releases/download/launcher-v1.3.0/Velora-Launcher_1.3.0_amd64.deb -o "$old"
printf '%s  %s\n' a7272527d38abfc2dbe0fedb3de24021710957ca59fce4292d2ddd92d4b71779 "$old" | sha256sum -c -
[[ $(dpkg-deb -f "$old" Package) == scopenet-launcher ]]
fixture="$HOME/.local/share/net.scopenet.launcher/upgrade-fixture.txt"
mkdir -p "$(dirname "$fixture")"
printf '%s\n' synthetic-upgrade-continuity > "$fixture"
sudo apt-get install -y "$old"
[[ $(dpkg-query -W -f='${Version}' scopenet-launcher) == 1.3.0 ]]
packages=(../target/release/bundle/deb/*.deb)
[[ ${#packages[@]} == 1 ]]
new=$(realpath "${packages[0]}")
[[ $(dpkg-deb -f "$new" Package) == velora-launcher ]]
sudo apt-get install -y "$new"
version=$(node -p "require('./src-tauri/tauri.conf.json').version")
[[ $(dpkg-query -W -f='${Version}' velora-launcher) == "$version" ]]
[[ $(dpkg-query -W -f='${db:Status-Status}' scopenet-launcher 2>/dev/null || true) != installed ]]
test -x /usr/bin/velora-launcher
[[ $(cat "$fixture") == synthetic-upgrade-continuity ]]
echo 'Debian 1.3.0 upgrade passed: replaces the old package, installs Velora and preserves user data.'
