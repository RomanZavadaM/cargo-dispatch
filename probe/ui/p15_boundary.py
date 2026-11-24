#!/usr/bin/env python3
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
files = {
    "core_cargo": root / "crates/ui-probe-core/Cargo.toml",
    "planner_cargo": root / "apps/leptos-web/Cargo.toml",
    "driver_cargo": root / "apps/driver-web/Cargo.toml",
    "desktop_cargo": root / "apps/tauri-shell/src-tauri/Cargo.toml",
    "core_source": root / "crates/ui-probe-core/src/lib.rs",
    "planner_source": root / "apps/leptos-web/src/main.rs",
    "driver_source": root / "apps/driver-web/src/main.rs",
    "desktop_source": root / "apps/tauri-shell/src-tauri/src/main.rs",
}
text = {name: path.read_text(encoding="utf-8") for name, path in files.items()}

checks = {
    "core_has_no_leptos_dependency": "leptos" not in text["core_cargo"].lower(),
    "core_has_no_tauri_dependency": "tauri" not in text["core_cargo"].lower(),
    "planner_depends_on_shared_core": "ui-probe-core" in text["planner_cargo"],
    "driver_depends_on_shared_core": "ui-probe-core" in text["driver_cargo"],
    "desktop_depends_on_shared_core": "ui-probe-core" in text["desktop_cargo"],
    "application_service_defined_in_core": "pub struct ApplicationService" in text["core_source"],
    "core_source_has_no_leptos_api": "leptos::" not in text["core_source"],
    "core_source_has_no_tauri_api": "tauri::" not in text["core_source"],
    "planner_uses_application_service": "ApplicationService" in text["planner_source"],
    "desktop_uses_application_service": "ApplicationService" in text["desktop_source"],
    "driver_uses_shared_trip_model": "synthetic_driver_trip" in text["driver_source"],
}
report = {
    "probe": "P15",
    "pass": all(checks.values()),
    "checks": checks,
    "meaning": "UI adapters depend inward on the shared application/core layer; the core does not depend on Leptos or Tauri.",
}
out = root / "probe/ui/out/p15-boundary.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(json.dumps(report, indent=2, sort_keys=True))
if not report["pass"]:
    raise SystemExit(1)
