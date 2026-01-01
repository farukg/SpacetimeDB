#!/usr/bin/env python3
"""Check that published CLI resources are self-contained and reproducible."""
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest


HELPER = Path(__file__).with_name("prepare-cli-package.py")


class CliPackageResourcesTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="stdb-cli-package-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "crates/cli/build").mkdir(parents=True)
        for name, content in {
            "Cargo.toml": '[workspace.package]\nversion = "2.13.0"\n',
            "crates/bindings-typescript/package.json": '{"version":"2.13.0"}',
            "templates/basic-rs/.template.json": '{"server_lang":"rust"}',
            "templates/basic-rs/spacetimedb/src/lib.rs": "pub fn fixture() {}\n",
            "skills/fixture/SKILL.md": "fixture skill\n",
            "LICENSE": "shared license\n",
        }.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        (self.root / "templates/basic-rs/LICENSE").symlink_to("../../LICENSE")
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        self.archive = self.root / "crates/cli/build/package-assets.tar"

    def prepare(self):
        return subprocess.run(
            [sys.executable, str(HELPER), str(self.root)],
            capture_output=True, text=True, check=False,
        )

    def test_resources_include_metadata_hidden_catalog_and_shared_license(self):
        (self.root / "templates/basic-rs/.env").write_text("untracked\n", encoding="utf-8")
        result = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        with tarfile.open(self.archive) as bundle:
            self.assertIn("templates/basic-rs/.template.json", bundle.getnames())
            self.assertIn("skills/fixture/SKILL.md", bundle.getnames())
            self.assertIn("Cargo.toml", bundle.getnames())
            self.assertIn("crates/bindings-typescript/package.json", bundle.getnames())
            self.assertNotIn("templates/basic-rs/.env", bundle.getnames())
            license = bundle.getmember("templates/basic-rs/LICENSE")
            self.assertTrue(license.isfile())
            self.assertEqual(bundle.extractfile(license).read(), b"shared license\n")

    def test_repeat_generation_is_identical_and_current_sources_replace_old_bytes(self):
        self.assertEqual(self.prepare().returncode, 0)
        original = self.archive.read_bytes()
        self.assertEqual(self.prepare().returncode, 0)
        self.assertEqual(self.archive.read_bytes(), original)
        (self.root / "templates/basic-rs/spacetimedb/src/lib.rs").write_text(
            "pub fn changed() {}\n", encoding="utf-8",
        )
        self.assertEqual(self.prepare().returncode, 0)
        self.assertNotEqual(self.archive.read_bytes(), original)

    def test_external_symlink_fails_and_preserves_existing_archive(self):
        self.assertEqual(self.prepare().returncode, 0)
        original = self.archive.read_bytes()
        with tempfile.NamedTemporaryFile() as external:
            link = self.root / "templates/basic-rs/external"
            link.symlink_to(external.name)
            subprocess.run(["git", "add", str(link)], cwd=self.root, check=True)
            result = self.prepare()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not a repository file", result.stderr)
        self.assertEqual(self.archive.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
