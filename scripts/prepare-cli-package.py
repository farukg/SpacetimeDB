#!/usr/bin/env python3
"""Prepare the repository-owned resources needed by a standalone CLI crate."""
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile


def prepare(root):
    archive = root / "crates/cli/build/package-assets.tar"
    paths = subprocess.check_output(
        [
            "git", "ls-files", "-z", "--", "templates", "skills",
            "Cargo.toml", "crates/bindings-typescript/package.json",
        ],
        cwd=root,
    ).split(b"\0")
    with tempfile.TemporaryDirectory(prefix=".package-assets-", dir=archive.parent) as directory:
        staged = Path(directory) / archive.name
        with tarfile.open(staged, mode="w") as bundle:
            for raw_path in sorted(filter(None, paths)):
                relative = Path(raw_path.decode("utf-8"))
                source = (root / relative).resolve(strict=True)
                if not source.is_relative_to(root) or not source.is_file():
                    raise ValueError(f"CLI package resource is not a repository file: {relative}")
                info = bundle.gettarinfo(str(source), arcname=relative.as_posix())
                info.mtime = 0
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                with source.open("rb") as content:
                    bundle.addfile(info, content)
        staged.replace(archive)
    print(f"CLI package resources: {archive}")


if __name__ == "__main__":
    if len(sys.argv) > 2:
        raise SystemExit("Usage: prepare-cli-package.py [workspace-root]")
    prepare(Path(sys.argv[1]).resolve() if len(sys.argv) == 2 else Path(__file__).resolve().parent.parent)
