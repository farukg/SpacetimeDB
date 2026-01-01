#!/usr/bin/env python3
"""Read the wasm-bindgen CLI version required by a resolved Cargo lock."""

from pathlib import Path
import sys
import tomllib

if len(sys.argv) != 2:
    raise SystemExit("Usage: wasm-bindgen-version.py <Cargo.lock>")
with Path(sys.argv[1]).open("rb") as source:
    packages = tomllib.load(source)["package"]
versions = {package["version"] for package in packages if package["name"] == "wasm-bindgen"}
if len(versions) != 1:
    raise SystemExit("Cargo.lock must resolve exactly one wasm-bindgen version for the SDK toolchain")
print(next(iter(versions)))
