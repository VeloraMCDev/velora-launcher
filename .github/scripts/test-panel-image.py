#!/usr/bin/env python3
"""Exercise instance storage and restart the packaged image on the same volume."""
import io
import json
import subprocess
import sys
import tarfile
import time
import urllib.error
import urllib.request
import uuid


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()


def main(image):
    name = "scopenet-image-test-" + uuid.uuid4().hex[:12]
    volume = name + "-data"
    password = uuid.uuid4().hex
    docker("volume", "create", volume)
    try:
        docker("run", "--detach", "--name", name,
               "--publish", "127.0.0.1::8080", "--volume", volume + ":/data",
               "--env", "ADMIN_PASSWORD=" + password, image)
        port = json.loads(docker("inspect", name))[0]["NetworkSettings"]["Ports"]["8080/tcp"][0]["HostPort"]
        base = "http://127.0.0.1:" + port

        def request(path, method="GET", body=None, token=None, instance=None):
            headers = {"Content-Type": "application/json"}
            if token:
                headers["Authorization"] = "Bearer " + token
            if instance:
                headers["X-SCOPENET-Instance"] = instance
            req = urllib.request.Request(base + path, method=method, headers=headers,
                                         data=None if body is None else json.dumps(body).encode())
            try:
                with urllib.request.urlopen(req, timeout=5) as response:
                    return json.load(response)
            except urllib.error.HTTPError as error:
                raise RuntimeError(f"{method} {path}: HTTP {error.code}: {error.read().decode()}") from error

        def ready():
            nonlocal base
            port = json.loads(docker("inspect", name))[0]["NetworkSettings"]["Ports"]["8080/tcp"][0]["HostPort"]
            base = "http://127.0.0.1:" + port
            deadline = time.monotonic() + 90
            while time.monotonic() < deadline:
                if docker("inspect", "--format", "{{.State.Running}}", name) != "true":
                    raise RuntimeError("panel exited during startup")
                try:
                    with urllib.request.urlopen(base + "/healthz", timeout=2) as response:
                        if response.status == 200:
                            return
                except (OSError, urllib.error.URLError):
                    pass
                time.sleep(0.5)
            raise RuntimeError("panel did not become healthy")

        def login():
            return request("/api/v1/auth/login", "POST", {"username": "admin", "password": password})["token"]

        ready()
        # The scratch image cannot run a shell. Inspect its temp directory via
        # Docker's archive API and verify access for the configured non-root UID.
        archive = subprocess.check_output(["docker", "cp", name + ":/tmp", "-"])
        with tarfile.open(fileobj=io.BytesIO(archive)) as files:
            temp = files.getmember("tmp")
            assert temp.isdir() and temp.mode & 0o003 == 0o003, "runtime /tmp must be writable/searchable by the panel user"
        token = login()
        instances = []
        for name_suffix, level_base in [("SMP", 321), ("Frontiers", 456)]:
            instance = request("/api/admin/instances", "POST",
                               {"name": "Smoke " + name_suffix, "mc_version": "1.21.1", "loader": "vanilla"}, token)["id"]
            request("/api/admin/progression", "PUT", {"level_base": level_base}, token, instance)
            instances.append((instance, level_base))
        # Opening scoped stores exercises SQLite TEMP views and legacy import.
        # A restart also exercises startup workers against persistent instances.
        for _ in range(2):
            docker("restart", name)
            ready()
            token = login()
            for instance, expected in instances:
                actual = request("/api/admin/progression", token=token, instance=instance)
                assert actual["settings"]["level_base"] == expected, actual
        print("Image passed: instance storage, isolation, and two persistent-volume restarts")
    except Exception:
        subprocess.run(["docker", "logs", name], check=False)
        raise
    finally:
        subprocess.run(["docker", "rm", "--force", name], check=False, stdout=subprocess.DEVNULL)
        subprocess.run(["docker", "volume", "rm", volume], check=False, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main(sys.argv[1])
