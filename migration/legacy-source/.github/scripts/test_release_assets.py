import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("release_assets", Path(__file__).with_name("verify-release-assets.py"))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)
VERSION = "1.2.3-beta.1"


def write_jar(path, loader, mc, version=VERSION):
    with zipfile.ZipFile(path, "w") as jar:
        if loader in ("fabric", "client"):
            jar.writestr("fabric.mod.json", json.dumps({"version": version, "depends": {"minecraft": mc}, "environment": "client" if loader == "client" else "server"}))
        elif loader == "forge":
            jar.writestr("META-INF/mods.toml", f'[[mods]]\nmodId="scopenet"\nversion="{version}"\n[[dependencies.scopenet]]\nmodId="minecraft"\nversionRange="[{mc}]"\n')
        else:
            jar.writestr("plugin.yml", f"name: SCOPENET\nversion: '{version}'\n")
            jar.writestr("META-INF/MANIFEST.MF", f"Manifest-Version: 1.0\r\nImplementation-Version: {version}\r\nSCOPENET-Minecraft-Target: {mc}\r\n")


class ReleaseAssetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        for name, (loader, mc) in release.expected_jars(VERSION).items():
            write_jar(self.directory / name, loader, mc)
        self.installer = self.directory / f"SCOPENET Launcher_{VERSION}_x64-setup.exe"
        self.installer.write_bytes(b"installer fixture, never executed")

    def test_complete_release_and_each_matrix_output(self):
        self.assertEqual(release.verify_assets(self.directory, VERSION, installer=True), 9)
        for mc in release.MINECRAFT:
            with tempfile.TemporaryDirectory() as subset:
                target = Path(subset)
                for name in release.expected_jars(VERSION, mc):
                    (target / name).write_bytes((self.directory / name).read_bytes())
                expected = {"1.20.1": 3, "1.21.1": 2, "26.3": 3}[mc]  # fabric + forge, plus the single Paper jar (1.20.1) or the client (26.3)
                self.assertEqual(release.verify_assets(target, VERSION, mc), expected)

    def test_missing_or_unexpected_artifacts_fail(self):
        paper = self.directory / f"scopenet-paper-{VERSION}.jar"
        paper.rename(self.directory / f"scopenet-paper-26.3-{VERSION}.jar")
        with self.assertRaisesRegex(ValueError, "missing=.*scopenet-paper-1"):
            release.verify_assets(self.directory, VERSION, installer=True)

    def test_desktop_apps_are_optional_but_checked(self):
        base = release.verify_assets(self.directory, VERSION, installer=True, desktop=True)
        dmg = self.directory / f"scopenet-launcher-{VERSION}-macos-universal.dmg"
        dmg.write_bytes(b"disk image")
        self.assertEqual(release.verify_assets(self.directory, VERSION, installer=True, desktop=True), base + 1)
        with self.assertRaisesRegex(ValueError, "Unexpected desktop app"):
            release.verify_assets(self.directory, VERSION, installer=True)
        dmg.rename(self.directory / "scopenet-launcher-0.1.0-macos-universal.dmg")
        with self.assertRaisesRegex(ValueError, "Unexpected desktop app names"):
            release.verify_assets(self.directory, VERSION, installer=True, desktop=True)

    def test_mobile_apps_are_optional_but_checked(self):
        self.assertEqual(release.verify_assets(self.directory, VERSION, installer=True, mobile=True), 9)
        apk = self.directory / f"scopenet-player-{VERSION}.apk"
        with zipfile.ZipFile(apk, "w") as z:
            z.writestr("AndroidManifest.xml", "x")
        self.assertEqual(release.verify_assets(self.directory, VERSION, installer=True, mobile=True), 10)
        with self.assertRaisesRegex(ValueError, "Unexpected mobile app"):
            release.verify_assets(self.directory, VERSION, installer=True)
        apk.rename(self.directory / "scopenet-player-0.1.0.apk")
        with self.assertRaisesRegex(ValueError, "Unexpected mobile app names"):
            release.verify_assets(self.directory, VERSION, installer=True, mobile=True)

    def test_embedded_version_and_target_must_match_filename(self):
        for loader in ("fabric", "forge", "paper", "client"):
            mc = release.PAPER_TARGET if loader == "paper" else "26.3"
            wrong = "1.21.1" if loader == "paper" else "1.20.1"
            name = next(n for n, pair in release.expected_jars(VERSION).items() if pair == (loader, mc))
            with self.subTest(loader=loader):
                write_jar(self.directory / name, loader, mc, "0.1.0")
                with self.assertRaisesRegex(ValueError, "version differs"):
                    release.verify_assets(self.directory, VERSION, installer=True)
                write_jar(self.directory / name, loader, wrong)
                with self.assertRaisesRegex(ValueError, "target differs"):
                    release.verify_assets(self.directory, VERSION, installer=True)
                write_jar(self.directory / name, loader, mc)

    def test_installer_must_have_release_version_and_bytes(self):
        self.installer.write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "Installer is empty"):
            release.verify_assets(self.directory, VERSION, installer=True)
        self.installer.write_bytes(b"fixture")
        self.installer.rename(self.directory / "SCOPENET Launcher_0.1.0_x64-setup.exe")
        with self.assertRaisesRegex(ValueError, "filename differs"):
            release.verify_assets(self.directory, VERSION, installer=True)


if __name__ == "__main__":
    unittest.main()
