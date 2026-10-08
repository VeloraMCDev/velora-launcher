"""Fail before publication if a release artifact is missing, stale, or mis-stamped."""
import argparse
import json
from pathlib import Path
import re
import tomllib
import zipfile

MINECRAFT = ("1.20.1", "1.21.1", "26.3")
# One Paper plugin covers every Minecraft version, so it is built once (by the job for PAPER_BUILD_MC) and named without a version.
PAPER_TARGET = "1.20+"
PAPER_BUILD_MC = "1.20.1"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def expected_jars(version, minecraft=None):
    targets = (minecraft,) if minecraft else MINECRAFT
    expected = {
        f"scopenet-{loader}-{mc}-{version}.jar": (loader, mc)
        for mc in targets for loader in ("fabric", "forge")
    }
    if minecraft in (None, PAPER_BUILD_MC):
        expected[f"scopenet-paper-{version}.jar"] = ("paper", PAPER_TARGET)
    if "26.3" in targets:
        expected[f"scopenet-client-fabric-26.3-{version}.jar"] = ("client", "26.3")
    return expected


def manifest_attributes(text):
    attributes = {}
    key = None
    for line in text.splitlines():
        if line.startswith(" ") and key:
            attributes[key] += line[1:]
        elif ": " in line:
            key, value = line.split(": ", 1)
            attributes[key] = value
    return attributes


def verify_jar(path, version, loader, mc):
    with zipfile.ZipFile(path) as jar:
        if loader in ("fabric", "client"):
            metadata = json.loads(jar.read("fabric.mod.json"))
            require(metadata["version"] == version, "Fabric version differs from release tag")
            require(metadata["depends"]["minecraft"] == mc, "Fabric Minecraft target differs from filename")
            require(metadata["environment"] == ("client" if loader == "client" else "server"), "Wrong Fabric side")
        elif loader == "forge":
            metadata = tomllib.loads(jar.read("META-INF/mods.toml").decode("utf-8"))
            mod = next(mod for mod in metadata["mods"] if mod["modId"] == "scopenet")
            require(mod["version"] == version, "Forge version differs from release tag")
            dependency = next(d for d in metadata["dependencies"]["scopenet"] if d["modId"] == "minecraft")
            require(dependency["versionRange"] == f"[{mc}]", "Forge Minecraft target differs from filename")
        else:
            descriptor = jar.read("plugin.yml").decode("utf-8")
            stamped = re.search(r"(?m)^version:\s*['\"]?([^'\"\s]+)", descriptor)
            require(stamped and stamped[1] == version, "Paper plugin version differs from release tag")
            manifest = manifest_attributes(jar.read("META-INF/MANIFEST.MF").decode("utf-8"))
            require(manifest["Implementation-Version"] == version, "Paper manifest version differs from release tag")
            require(manifest["SCOPENET-Minecraft-Target"] == mc, "Paper target differs from filename")


def mobile_names(version):
    return {f"scopenet-player-{version}.apk", f"scopenet-player-{version}-unsigned.ipa"}


def desktop_names(version):
    return {
        f"scopenet-launcher-{version}-linux-x64.AppImage",
        f"scopenet-launcher-{version}-linux-x64.deb",
        f"scopenet-launcher-{version}-macos-universal.dmg",
    }


def verify_assets(directory, version, minecraft=None, installer=False, mobile=False, desktop=False):
    expected = expected_jars(version, minecraft)
    files = {p.name: p for p in directory.iterdir() if p.is_file()}
    installers = [name for name in files if name.endswith("-setup.exe")]
    if installer:
        require(len(installers) == 1, "Expected exactly one NSIS installer")
        require(installers[0].endswith(f"_{version}_x64-setup.exe"), "Installer filename differs from release tag")
        require(files[installers[0]].stat().st_size > 0, "Installer is empty")
    else:
        require(not installers, "Unexpected installer in JAR build output")
    apps = {name for name in files if name.endswith((".apk", ".ipa"))}
    if mobile:
        # The phone apps are optional (their jobs may be skipped or fail without blocking the release), but whatever is attached must be right.
        require(apps <= mobile_names(version), f"Unexpected mobile app names: {sorted(apps - mobile_names(version))}")
        for name in apps:
            require(files[name].stat().st_size > 0 and zipfile.is_zipfile(files[name]), f"{name} is empty or not a valid archive")
    else:
        require(not apps, "Unexpected mobile app in JAR build output")
    desktop_apps = {name for name in files if name.endswith((".AppImage", ".deb", ".dmg"))}
    if desktop:
        # The macOS and Linux apps are optional too: their jobs may fail without blocking the Windows release.
        require(desktop_apps <= desktop_names(version), f"Unexpected desktop app names: {sorted(desktop_apps - desktop_names(version))}")
        for name in desktop_apps:
            require(files[name].stat().st_size > 0, f"{name} is empty")
    else:
        require(not desktop_apps, "Unexpected desktop app in JAR build output")
    actual = set(files) - set(installers) - apps - desktop_apps
    require(actual == set(expected), f"Release set differs: missing={sorted(set(expected) - actual)}, unexpected={sorted(actual - set(expected))}")
    for name, (loader, mc) in expected.items():
        try:
            verify_jar(files[name], version, loader, mc)
        except Exception as error:
            raise ValueError(f"{name}: {error}") from error
    return len(expected) + len(installers) + len(apps) + len(desktop_apps)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--assets", required=True, type=Path)
    parser.add_argument("--minecraft", choices=MINECRAFT)
    parser.add_argument("--installer", action="store_true")
    parser.add_argument("--mobile", action="store_true", help="allow the optional Android/iOS apps")
    parser.add_argument("--desktop", action="store_true", help="allow the optional macOS and Linux apps")
    args = parser.parse_args()
    count = verify_assets(args.assets, args.version, args.minecraft, args.installer, args.mobile, args.desktop)
    print(f"Verified {count} release artifacts stamped {args.version}")
