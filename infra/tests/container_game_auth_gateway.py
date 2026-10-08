"""Run only a new, disposable synthetic Compose project; never an existing deployment."""
import argparse
import hashlib
import http.client
import importlib.util
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.parse
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compose", type=Path, required=True)
    parser.add_argument("--helpers", type=Path, required=True)
    parser.add_argument("--scratch-root", type=Path, required=True)
    args = parser.parse_args()
    compose = args.compose.resolve(strict=True)
    helpers_path = args.helpers.resolve(strict=True)
    spec = importlib.util.spec_from_file_location("public_game_auth_fixtures", helpers_path)
    helpers = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helpers)
    args.scratch_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    root = Path(tempfile.mkdtemp(prefix="container-component-", dir=args.scratch_root.resolve()))
    root.chmod(0o700)
    secret = root / "bootstrap"
    secret.write_bytes(b"synthetic-container-password")
    # Source directory is private; bind-mounted file must be readable by container UID 65532.
    secret.chmod(0o444)
    with socket.socket() as reserved:
        reserved.bind(("127.0.0.1", 0))
        port = reserved.getsockname()[1]
    prefix = "/velora-container-test"
    public = f"http://127.0.0.1:{port}{prefix}"
    project = "velora-component-check-" + uuid.uuid4().hex[:12]
    environment = {**os.environ, "VELORA_PUBLIC_ORIGIN": public, "VELORA_HEALTH_PATH": prefix + "/health/ready",
                   "VELORA_BOOTSTRAP_PASSWORD_FILE": str(secret), "VELORA_BOOTSTRAP_USERNAME": "ContainerAdmin",
                   "VELORA_GATEWAY_PORT": str(port), "VELORA_AUTH_DATA_DIR": "/data", "VELORA_AUTH_VOLUME": "auth_data",
                   "VELORA_INTERNAL_SUBNET": "172.30.50.0/24", "VELORA_GATEWAY_INTERNAL_IP": "172.30.50.2",
                   "VELORA_AUTH_INTERNAL_IP": "172.30.50.3"}
    checks = []

    def command(*arguments, json_output=False):
        result = subprocess.run(["docker", "compose", "-f", str(compose), "--project-name", project, *arguments],
                                env=environment, capture_output=True, timeout=1200)
        with (root / "operations.log").open("ab") as log:
            log.write(result.stdout + result.stderr)
        if result.returncode:
            # These are public source builds and synthetic data; show diagnostic output on failure.
            raise RuntimeError(result.stderr.decode(errors="replace") + result.stdout.decode(errors="replace"))
        return json.loads(result.stdout) if json_output else result.stdout

    def request(method, path, payload=None, headers=None):
        body = json.dumps(payload).encode() if isinstance(payload, dict) else payload
        headers = dict(headers or {})
        if isinstance(payload, dict):
            headers["Content-Type"] = "application/json"
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        try:
            connection.request(method, prefix + path, body, headers)
            response = connection.getresponse()
            return response.status, dict(response.getheaders()), response.read()
        finally:
            connection.close()

    def ready():
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            try:
                if request("GET", "/health/ready")[0] == 200:
                    return
            except (OSError, http.client.HTTPException):
                pass
            time.sleep(0.1)
        raise AssertionError("container component readiness timed out")

    def tools(*arguments):
        return command("--profile", "offline", "run", "--rm", "--no-deps", "tools", *arguments, json_output=True)

    def profile():
        status, _, body = request("GET", f"/api/yggdrasil/sessionserver/session/minecraft/profile/{player_uuid}?unsigned=false")
        assert status == 200
        property_value = json.loads(body)["properties"][0]
        helpers.verify_property(public_key, property_value)
        import base64
        skin = json.loads(base64.b64decode(property_value["value"]))["textures"]["SKIN"]
        assert skin["url"].startswith(public + "/textures/")
        assert skin["metadata"]["model"] == "slim"
        path = urllib.parse.urlsplit(skin["url"]).path.removeprefix(prefix)
        status, _, image = request("GET", path)
        assert status == 200
        return path, image

    try:
        resolved = command("--profile", "offline", "config", "--format", "json", json_output=True)
        assert set(resolved.get("volumes", {})) == {"auth_data", "restored_data"}, "only new synthetic volumes are permitted"
        for name, volume in resolved["volumes"].items():
            assert not volume.get("external") and volume["name"] == project + "_" + name
        for service in resolved["services"].values():
            assert not service.get("container_name"), "fixed container names are not permitted"
        mounts = resolved["services"]["authentication"]["volumes"]
        assert len(mounts) == 1 and mounts[0]["type"] == "volume" and mounts[0]["source"] == "auth_data" and mounts[0]["target"] == "/data"
        assert not resolved["services"]["gateway"].get("volumes")
        assert not resolved["services"]["authentication"].get("ports")
        tool_mounts = resolved["services"]["tools"]["volumes"]
        assert len(tool_mounts) == 2 and {mount["source"] for mount in tool_mounts} == {"auth_data", "restored_data"}
        assert all(mount["type"] == "volume" for mount in tool_mounts)
        assert any(mount["target"] == "/source" and mount.get("read_only") for mount in tool_mounts)
        for network in resolved.get("networks", {}).values():
            assert not network.get("external") and network["name"].startswith(project + "_")
        command("build")
        command("up", "--detach", "--wait", "--wait-timeout", "120")
        ready()
        status, headers, body = request("GET", "/api/yggdrasil", headers={"Host": "spoof.invalid", "X-Forwarded-Host": "spoof.invalid"})
        assert status == 200 and headers["x-authlib-injector-api-location"].rstrip("/") == public + "/api/yggdrasil"
        public_key = json.loads(body)["signaturePublickey"]
        status, _, body = request("POST", "/api/yggdrasil/authserver/authenticate", {
            "username": "ContainerAdmin", "password": "synthetic-container-password", "clientToken": "synthetic-client"})
        assert status == 200
        authentication = json.loads(body)
        token, player_uuid = authentication["accessToken"], authentication["selectedProfile"]["id"]
        boundary = b"synthetic-container-boundary"
        upload = (b"--" + boundary + b'\r\nContent-Disposition: form-data; name="model"\r\n\r\nslim\r\n--' + boundary
                  + b'\r\nContent-Disposition: form-data; name="file"; filename="synthetic.png"\r\nContent-Type: image/png\r\n\r\n'
                  + helpers.png() + b"\r\n--" + boundary + b"--\r\n")
        assert request("PUT", f"/api/yggdrasil/api/user/profile/{player_uuid}/skin", upload,
                       {"Authorization": "Bearer " + token, "Content-Type": "multipart/form-data; boundary=" + boundary.decode()})[0] == 204
        original = profile()
        assert request("GET", "/api/me")[0] == 503  # no hidden Panel/private dependency
        checks.append("fresh non-root/read-only containers, persistent owned volume, multipart skin, public prefix and RSA signature")
        command("stop", "authentication")
        assert request("GET", "/health/ready")[0] == 503
        assert request("GET", "/health/live")[0] == 200
        assert request("POST", "/api/yggdrasil/authserver/validate", {"accessToken": token})[0] in (502, 504)
        checks.append("authority outage denies without fallback and leaves gateway live")
        backup = tools("checkpoint", "/source/auth.sqlite", "/data/closed.sqlite")
        assets = tools("assets", "/source", "/data/closed.sqlite", "/data/restored")
        restored = tools("checkpoint", "/data/closed.sqlite", "/data/restored/auth.sqlite")
        assert backup["sha256"] == restored["sha256"]
        # Recreate only the stopped synthetic authority, using its restored owned store.
        environment["VELORA_AUTH_DATA_DIR"] = "/data/restored"
        environment["VELORA_AUTH_VOLUME"] = "restored_data"
        command("up", "--detach", "--no-deps", "authentication")
        ready()
        assert request("POST", "/api/yggdrasil/authserver/validate", {"accessToken": token, "clientToken": "synthetic-client"})[0] == 204
        assert profile() == original
        assert request("POST", "/api/yggdrasil/authserver/authenticate", {
            "username": "ContainerAdmin", "password": "synthetic-container-password", "clientToken": "synthetic-client"})[0] == 200
        assert request("POST", "/api/yggdrasil/authserver/invalidate", {"accessToken": token,"clientToken": "synthetic-client"})[0] == 204
        assert request("POST", "/api/yggdrasil/authserver/validate", {"accessToken": token,"clientToken": "synthetic-client"})[0] == 403
        checks.append("offline volume checkpoint/assets restore retains signing identity, password, game token, texture bytes and live revocation")
        command("stop")
        print(json.dumps({"checks":checks,"backup_sha256":backup["sha256"],"assets_report":assets,
                          "compose_sha256":hashlib.sha256(compose.read_bytes()).hexdigest(),
                          "scope":"synthetic game components only; full deployment/migration acceptance remains open"},indent=2))
    finally:
        # Only this generated project has been started. Never prune unrelated resources.
        assert project.startswith("velora-component-check-") and len(project)==35
        command("down", "--volumes", "--remove-orphans")
        # Preserve local synthetic logs; no recursive filesystem deletion.


if __name__ == "__main__":
    main()
