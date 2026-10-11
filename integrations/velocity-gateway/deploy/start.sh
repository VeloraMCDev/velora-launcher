#!/usr/bin/env bash
# Run from an operator-owned working directory, never from the source checkout.
set -euo pipefail
: "${VELORA_GATEWAY_YGGDRASIL:?Set the existing Velora Yggdrasil HTTPS API root}"
: "${VELOCITY_SHA256:?Pin the tested Velocity JAR SHA256}"
: "${AUTHLIB_INJECTOR_SHA256:?Pin the tested authlib-injector JAR SHA256}"
: "${GATEWAY_SHA256:?Pin the reviewed gateway plugin JAR SHA256}"
[[ "$VELORA_GATEWAY_YGGDRASIL" == https://* ]] || { echo 'HTTPS Yggdrasil URL required' >&2; exit 1; }
for digest in "$VELOCITY_SHA256" "$AUTHLIB_INJECTOR_SHA256" "$GATEWAY_SHA256"; do
  [[ "$digest" =~ ^[a-f0-9]{64}$ ]] || { echo 'Invalid artifact digest' >&2; exit 1; }
done
printf '%s  %s\n' "$VELOCITY_SHA256" velocity.jar "$AUTHLIB_INJECTOR_SHA256" authlib-injector.jar "$GATEWAY_SHA256" plugins/velora-velocity-gateway.jar | sha256sum --check --strict
exec java -Xms256M -Xmx1G "-javaagent:$(pwd)/authlib-injector.jar=$VELORA_GATEWAY_YGGDRASIL" -Dauthlibinjector.mojangNamespace=disabled -jar velocity.jar
