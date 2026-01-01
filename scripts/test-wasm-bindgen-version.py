#!/usr/bin/env python3
"""Exercise SDK tool selection against independent Cargo lock fixtures."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HELPER = Path(__file__).with_name("wasm-bindgen-version.py")


class WasmBindgenVersionTests(unittest.TestCase):
    def select(self, lock):
        with tempfile.TemporaryDirectory(prefix="stdb-wasm-version-") as directory:
            path = Path(directory) / "Cargo.lock"
            path.write_text(lock, encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(HELPER), str(path)],
                capture_output=True,
                text=True,
                check=False,
            )

    def test_uses_resolved_version(self):
        selected = self.select('''version = 4
[[package]]
name = "wasm-bindgen"
version = "0.2.130"
[[package]]
name = "wasm-bindgen-macro"
version = "0.2.104"
''')
        self.assertEqual(selected.returncode, 0, selected.stderr)
        self.assertEqual(selected.stdout, "0.2.130\n")

    def test_unrelated_dependency_changes_preserve_sdk_input(self):
        lock = '''version = 4
[[package]]
name = "wasm-bindgen"
version = "0.2.130"
[[package]]
name = "fixture-unrelated"
version = "{}"
'''
        for version in ("1.0.0", "2.0.0"):
            with self.subTest(version=version):
                selected = self.select(lock.format(version))
                self.assertEqual(selected.returncode, 0, selected.stderr)
                self.assertEqual(selected.stdout, "0.2.130\n")

    def test_missing_tool_fails(self):
        selected = self.select('''version = 4
[[package]]
name = "wasm-bindgen-macro"
version = "0.2.130"
''')
        self.assertNotEqual(selected.returncode, 0)
        self.assertEqual(selected.stdout, "")
        self.assertIn("exactly one wasm-bindgen version", selected.stderr)

    def test_multiple_tool_versions_fail(self):
        selected = self.select('''version = 4
[[package]]
name = "wasm-bindgen"
version = "0.2.104"
[[package]]
name = "wasm-bindgen"
version = "0.2.130"
''')
        self.assertNotEqual(selected.returncode, 0)
        self.assertEqual(selected.stdout, "")
        self.assertIn("exactly one wasm-bindgen version", selected.stderr)


if __name__ == "__main__":
    unittest.main()
