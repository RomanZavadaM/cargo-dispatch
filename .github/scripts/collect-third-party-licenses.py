#!/usr/bin/env python3
"""Collect license and notice files of third-party Rust crates into one text file.

Usage: collect-third-party-licenses.py OUTPUT [METADATA_JSON]
Without METADATA_JSON the script runs `cargo metadata --locked`.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

PREFIXES = ("license", "licence", "copying", "copyright", "notice", "unlicense")
RULE = "=" * 78


def third_party(metadata: dict) -> list[dict]:
    members = set(metadata["workspace_members"])
    packages = [item for item in metadata["packages"] if item["id"] not in members]
    return sorted(packages, key=lambda item: (item["name"], item["version"]))


def license_files(package: dict) -> list[Path]:
    root = Path(package["manifest_path"]).parent
    found = set()
    for path in root.iterdir():
        if not path.name.lower().startswith(PREFIXES):
            continue
        if path.is_file():
            found.add(path)
        elif path.is_dir():
            found.update(item for item in path.rglob("*") if item.is_file())
    declared = package.get("license_file")
    if declared and (root / declared).is_file():
        found.add(root / declared)
    return sorted(found)


def render(packages: list[dict]) -> tuple[str, list[str]]:
    parts = [
        "Third-party Rust components of Cargo Dispatch: license and notice files",
        "as shipped in each component's source package. The list covers every crate",
        "in Cargo.lock, including crates used only on some platforms or at build time.",
        "",
    ]
    without = []
    for package in packages:
        label = f"{package['name']} {package['version']}"
        expression = package.get("license") or "see license file"
        parts += [RULE, f"{label} ({expression})"]
        if package.get("repository"):
            parts.append(package["repository"])
        files = license_files(package)
        if not files:
            without.append(f"{label} ({expression})")
        for path in files:
            text = path.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n")
            parts += ["", f"--- {path.name} ---", text.rstrip()]
        parts.append("")
    if without:
        parts += [RULE, "Components whose source package contains no license file:"]
        parts += [f"- {item}" for item in without]
        parts.append("")
    return "\n".join(parts), without


def main(argv: list[str]) -> int:
    if len(argv) not in (2, 3):
        print(__doc__, file=sys.stderr)
        return 2
    if len(argv) == 3:
        metadata = json.loads(Path(argv[2]).read_text(encoding="utf-8"))
    else:
        raw = subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"])
        metadata = json.loads(raw)
    packages = third_party(metadata)
    text, without = render(packages)
    if not packages or len(without) * 4 > len(packages):
        print(f"Too many components without license files: {len(without)}/{len(packages)}", file=sys.stderr)
        return 1
    Path(argv[1]).write_text(text, encoding="utf-8", newline="\n")
    print(f"{len(packages)} components, {len(without)} without a license file")
    for item in without:
        print(f"  no license file: {item}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
