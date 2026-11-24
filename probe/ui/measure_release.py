#!/usr/bin/env python3
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--label", required=True)
parser.add_argument("--output", required=True)
parser.add_argument("--binary", required=True)
parser.add_argument("--copy-to", required=True)
parser.add_argument("command", nargs=argparse.REMAINDER)
args = parser.parse_args()

command = list(args.command)
if command and command[0] == "--":
    command = command[1:]
if not command:
    raise SystemExit("No build command supplied")

started = time.perf_counter()
subprocess.run(command, check=True)
duration = round(time.perf_counter() - started, 3)

binary = Path(args.binary)
if os.name == "nt" and binary.suffix.lower() != ".exe":
    binary = binary.with_suffix(".exe")
if not binary.is_file():
    raise SystemExit(f"Release binary not found: {binary}")

copy_dir = Path(args.copy_to)
copy_dir.mkdir(parents=True, exist_ok=True)
copy_path = copy_dir / binary.name
shutil.copy2(binary, copy_path)

result = {
    "probe": "P13",
    "label": args.label,
    "platform": platform.system(),
    "machine": platform.machine(),
    "duration_seconds": duration,
    "binary_bytes": binary.stat().st_size,
    "binary": str(binary),
    "artifact_copy": str(copy_path),
    "package_mode": "raw_release_binary_probe",
    "installer": False,
    "signed": False,
    "note": "Architecture probe only; this raw binary is not a product release or installer.",
}
output = Path(args.output)
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(json.dumps(result, indent=2, sort_keys=True))
