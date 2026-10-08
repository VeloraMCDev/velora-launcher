"""Synthetic process acceptance; no production store, account or provider access."""
import argparse
import base64
import hashlib
import http.client
import http.server
import json
import os
from pathlib import Path
import socket
import sqlite3
import struct
import subprocess
import tempfile
import threading
import time
import urllib.parse
import zlib


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 64, 64, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\x00" + b"\x20\x80\xa0\xff" * 64) * 64)) + chunk(b"IEND", b""))


def der_item(data, offset=0):
    tag, length = data[offset], data[offset + 1]
    cursor = offset + 2
    if length & 128:
        size = length & 127
        length = int.from_bytes(data[cursor:cursor + size], "big")
        cursor += size
    end = cursor + length
    assert end <= len(data), "truncated synthetic public key"
    return tag, data[cursor:end], end


def verify_property(public_pem, property_value):
    # Public-key-only PKCS#1 v1.5 / SHA-1 verification; no private key bytes read.
    der = base64.b64decode("".join(line for line in public_pem.splitlines() if not line.startswith("-----")))
    tag, outer, _ = der_item(der)
    assert tag == 0x30
    _, _, cursor = der_item(outer)
    tag, bits, _ = der_item(outer, cursor)
    assert tag == 3 and bits[0] == 0
    _, rsa, _ = der_item(bits[1:])
    _, modulus, cursor = der_item(rsa)
    _, exponent, _ = der_item(rsa, cursor)
    n, e = int.from_bytes(modulus, "big"), int.from_bytes(exponent, "big")
    signature = base64.b64decode(property_value["signature"])
    assert len(signature) == (n.bit_length() + 7) // 8
    encoded = pow(int.from_bytes(signature, "big"), e, n).to_bytes(len(signature), "big")
    expected = bytes.fromhex("3021300906052b0e03021a05000414") + hashlib.sha1(property_value["value"].encode()).digest()
    padding = len(encoded) - len(expected) - 3
    assert padding >= 8 and encoded == b"\x00\x01" + b"\xff" * padding + b"\x00" + expected, "profile signature changed"


def free_port(excluded):
    while True:
        with socket.socket() as socket_value:
            socket_value.bind(("127.0.0.1", 0))
            port = socket_value.getsockname()[1]
        if port not in excluded:
            excluded.add(port)
            return port


class Fixture(http.server.BaseHTTPRequestHandler):
    paths = []
    def do_GET(self):
        self.paths.append(self.path)
        body = b'{"fixture":"public synthetic upstream"}'
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def do_POST(self):
        self.paths.append(self.path)
        self.send_response(409)
        self.send_header("Content-Length", "0")
        self.end_headers()
    def log_message(self, *_):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--auth", type=Path, required=True)
    parser.add_argument("--gateway", type=Path, required=True)
    parser.add_argument("--tools", type=Path, required=True)
    parser.add_argument("--scratch-root", type=Path, required=True,
                        help="protected directory for new synthetic fixtures; outputs are retained")
    args = parser.parse_args()
    binaries = {key: getattr(args, key).resolve(strict=True) for key in ("auth", "gateway", "tools")}
    args.scratch_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    root = Path(tempfile.mkdtemp(prefix="game-auth-gateway-", dir=args.scratch_root.resolve()))
    root.chmod(0o700)
    data = root / "fresh"
    data.mkdir(mode=0o700)
    password = root / "bootstrap"
    password.write_bytes(b"synthetic-gateway-password")
    password.chmod(0o600)
    allocated = set()
    auth_port, gateway_port = free_port(allocated), free_port(allocated)
    prefix = "/velora-test"
    public = f"http://127.0.0.1:{gateway_port}{prefix}"
    fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
    fixture_thread = threading.Thread(target=fixture.serve_forever, daemon=True)
    fixture_thread.start()
    environment = {key: value for key, value in os.environ.items()
                   if not key.upper().startswith(("VELORA_AUTH_", "VELORA_GATEWAY_"))}
    processes, logs, checks = [], [], []

    def start(name, variables):
        log = (root / f"{name}-{len(processes)}.log").open("xb")
        logs.append(log)
        process = subprocess.Popen([str(binaries[name])], env={**environment, **variables}, stdout=log, stderr=log)
        processes.append(process)
        return process

    def stop(process):
        if process.poll() is None:
            # Unix is graceful SIGTERM; Windows is a process-termination/recovery fixture.
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        if os.name != "nt":
            assert process.returncode == 0, "service did not exit cleanly on SIGTERM"

    def request(method, path, payload=None, headers=None):
        body = json.dumps(payload).encode() if isinstance(payload, dict) else payload
        headers = dict(headers or {})
        if isinstance(payload, dict):
            headers["Content-Type"] = "application/json"
        connection = http.client.HTTPConnection("127.0.0.1", gateway_port, timeout=5)
        try:
            connection.request(method, prefix + path, body, headers)
            response = connection.getresponse()
            return response.status, dict(response.getheaders()), response.read()
        finally:
            connection.close()

    def wait_ready(process):
        deadline = time.monotonic() + 90
        while time.monotonic() < deadline:
            assert process.poll() is None, "synthetic service exited during startup; inspect retained fixture logs"
            try:
                if request("GET", "/health/ready")[0] == 200:
                    return
            except (OSError, http.client.HTTPException):
                pass
            time.sleep(0.05)
        raise AssertionError("synthetic service readiness timed out")

    def auth_variables(directory):
        return {
            "VELORA_AUTH_BIND": f"127.0.0.1:{auth_port}", "VELORA_AUTH_PUBLIC_ORIGIN": public,
            "VELORA_AUTH_DATA_DIR": str(directory), "VELORA_AUTH_TRUSTED_PROXIES": "127.0.0.1",
            "VELORA_AUTH_BOOTSTRAP_USERNAME": "SyntheticAdmin", "VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE": str(password),
        }

    def tools(*arguments):
        result = subprocess.run([str(binaries["tools"]), *map(str, arguments)], env=environment,
                                capture_output=True, timeout=30)
        assert result.returncode == 0, "offline tools failed; no fixture credentials or records are printed"
        return json.loads(result.stdout)

    def identity(directory):
        uri = (directory / "auth.sqlite").resolve().as_uri() + "?mode=ro"
        with sqlite3.connect(uri, uri=True) as connection:
            return connection.execute("SELECT id,uuid,password_hash,auth_version FROM users WHERE username='SyntheticAdmin'").fetchone()

    def signed_profile(uuid, public_key):
        status, _, body = request("GET", f"/api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}?unsigned=false")
        assert status == 200
        profile = json.loads(body)
        assert profile["id"] == uuid
        property_value = profile["properties"][0]
        verify_property(public_key, property_value)
        texture = json.loads(base64.b64decode(property_value["value"]))["textures"]["SKIN"]
        assert texture["url"].startswith(public + "/textures/")
        assert texture["metadata"]["model"] == "slim"
        path = urllib.parse.urlsplit(texture["url"]).path.removeprefix(prefix)
        status, _, image = request("GET", path)
        assert status == 200
        return path, image

    try:
        auth = start("auth", auth_variables(data))
        gateway = start("gateway", {
            "VELORA_GATEWAY_BIND": f"127.0.0.1:{gateway_port}", "VELORA_GATEWAY_PUBLIC_ORIGIN": public,
            "VELORA_GATEWAY_AUTH_UPSTREAM": f"http://127.0.0.1:{auth_port}",
            "VELORA_GATEWAY_PANEL_UPSTREAM": f"http://127.0.0.1:{fixture.server_port}",
        })
        wait_ready(gateway)
        status, headers, body = request("GET", "/api/yggdrasil", headers={"Host": "spoof.invalid", "X-Forwarded-Host": "spoof.invalid"})
        assert status == 200 and headers["x-authlib-injector-api-location"].rstrip("/") == public + "/api/yggdrasil"
        public_key = json.loads(body)["signaturePublickey"]
        status, _, body = request("POST", "/api/yggdrasil/authserver/authenticate", {
            "username": "SyntheticAdmin", "password": "synthetic-gateway-password", "clientToken": "synthetic-client"})
        assert status == 200
        authentication = json.loads(body)
        token, uuid = authentication["accessToken"], authentication["selectedProfile"]["id"]
        boundary = b"synthetic-velora-boundary"
        upload = (b"--" + boundary + b'\r\nContent-Disposition: form-data; name="model"\r\n\r\nslim\r\n--' + boundary
                  + b'\r\nContent-Disposition: form-data; name="file"; filename="synthetic.png"\r\nContent-Type: image/png\r\n\r\n'
                  + png() + b"\r\n--" + boundary + b"--\r\n")
        assert request("PUT", f"/api/yggdrasil/api/user/profile/{uuid}/skin", upload,
                       {"Authorization": "Bearer " + token, "Content-Type": "multipart/form-data; boundary=" + boundary.decode()})[0] == 204
        original_texture = signed_profile(uuid, public_key)
        assert request("POST", "/api/yggdrasil/sessionserver/session/minecraft/join", {
            "accessToken": token, "selectedProfile": uuid, "serverId": "synthetic-server"},
            {"X-Forwarded-For": "198.51.100.88"})[0] == 204
        query = "/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=SyntheticAdmin&serverId=synthetic-server&ip="
        assert request("GET", query + "198.51.100.88")[0] == 204
        assert request("GET", query + "127.0.0.1")[0] == 200
        checks.append("fresh bootstrap, game login/join, spoof rejection, multipart skin, RSA signature and prefixed texture origin")
        stop(auth)
        before = list(Fixture.paths)
        assert request("GET", "/health/ready")[0] == 503
        assert request("GET", "/health/live")[0] == 200
        assert request("POST", "/api/yggdrasil/authserver/validate", {"accessToken": token})[0] in (502, 504)
        assert Fixture.paths == before, "Authentication outage fell back to Panel"
        assert request("GET", "/api/me")[0] == 200
        checks.append("authority outage denies without Panel fallback; gateway liveness and independent Panel forwarding survive")
        original_identity = identity(data)
        signing_hashes = {name: digest(data / name) for name in ("yggdrasil-signing.pem", "jwt.secret")}
        checkpoint = root / "closed.sqlite"
        tools("checkpoint", data / "auth.sqlite", checkpoint)
        restored = root / "restored"
        tools("assets", data, checkpoint, restored)
        tools("checkpoint", checkpoint, restored / "auth.sqlite")
        assert digest(checkpoint) == digest(restored / "auth.sqlite")
        assert identity(restored) == original_identity
        assert {name: digest(restored / name) for name in signing_hashes} == signing_hashes
        assert digest(data / "auth.sqlite") == digest(checkpoint), "backup validation changed the source"
        password.unlink()  # existing administrator must skip a now-missing bootstrap file
        auth = start("auth", auth_variables(restored))
        wait_ready(auth)
        assert request("POST", "/api/yggdrasil/authserver/validate", {"accessToken": token, "clientToken": "synthetic-client"})[0] == 204
        assert signed_profile(uuid, public_key) == original_texture
        assert identity(restored) == original_identity
        assert {name: digest(restored / name) for name in signing_hashes} == signing_hashes
        assert request("POST", "/api/yggdrasil/authserver/invalidate", {"accessToken": token, "clientToken": "synthetic-client"})[0] == 204
        assert request("POST", "/api/yggdrasil/authserver/validate", {"accessToken": token, "clientToken": "synthetic-client"})[0] == 403
        checks.append("closed checkpoint/assets restore preserves identity, password, keys, token and texture bytes; live revocation remains enforced")
        stop(auth)
        stop(gateway)
        print(json.dumps({"checks": checks, "binaries_sha256": {name: digest(path) for name, path in binaries.items()},
                          "shutdown": "Windows process termination/recovery" if os.name == "nt" else "Unix SIGTERM",
                          "scope": "synthetic game-authentication/gateway component only; full migration acceptance remains open"}, indent=2))
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
        fixture.shutdown()
        fixture.server_close()
        fixture_thread.join(timeout=5)
        for log in logs:
            log.close()
        # Preserve synthetic fixture evidence. No computed recursive delete or source mutation.


if __name__ == "__main__":
    main()
