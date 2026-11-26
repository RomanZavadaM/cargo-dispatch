# cargo-dispatch

[🇺🇦 Українська](../../README.md) · **🇬🇧 English** · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** active development. The current stable version is **v0.1.0**. Full 8-language localization is integrated; the next major focus is the route Planner.

## About

**cargo-dispatch** is a dispatch system for cargo fleets. It is intended to select loads, build and re-plan trips, hand trips over for execution, and give the dispatcher control over actual progress.

The main development language is **Rust**.

## What already exists

- a separate desktop **Control** application built with Tauri/Leptos;
- durable local storage of the active trip and restart recovery;
- a versioned trip package between Planner and Control;
- one adaptive **Driver Browser** for phone, tablet, laptop, and desktop browsers;
- reordering of open stops from both Control and Driver Browser;
- PDF and regular files **Control → Driver Browser**;
- files, photos, and videos **Driver Browser → Control** with local saving;
- two simultaneous browser clients for one trip and reconnect without a separate browser database;
- an interactive OpenStreetMap map in both Control and Driver Browser;
- loopback and explicit LAN modes for testing and local operation.

## Native Driver Apps

Both native clients are already integrated in `main`:

- **Android** — Jetpack Compose, route/complete/reorder, files/photo/video, foreground GPS, restart recovery, native MapLibre + OpenStreetMap, current road geometry, route markers/latest position, and system photo/video capture; Emulator build/install/launch/screenshot PASS.
- **iPhone/iPad** — SwiftUI, route/complete/reorder, files/photo/video, foreground CoreLocation, restart recovery, native MapKit, current road geometry, route markers/latest position, and system photo/video capture; Simulator build/install/launch/screenshot PASS.

Both use the same Control API and do not create a separate trip database.

After r4/r5, both native clients also have a durable retry queue for location and media. Android r5 adds explicit Start/Stop background trip tracking through a foreground location service, a persistent Stop notification, and a 15 s / 25 m policy. Apple r5 adds explicit Start/Stop CoreLocation background tracking, `UIBackgroundModes=location`, a 25 m distance filter, and does not request `Always` permission without a separate need.

## Driver Browser

Driver Browser is **one product**, not separate phone and desktop versions. It has three normal layouts:

- narrow phone;
- tablet;
- wide desktop/laptop.

Trip data, route actions, files, and backend calls are shared; only layout changes with screen size.

## Map

Control and Driver Browser now contain a real interactive basemap with trip points and the latest accepted driver position.

The `main` branch now contains a versioned road-geometry pipeline (schema 1.3): Planner carries derived geometry together with the actual vehicle routing profile, while schema 1.0–1.2 packages remain readable. Control stores geometry and, after a reorder, recalculates the road line through the configured runtime Valhalla service. A late routing response cannot overwrite a newer trip state.

The Valhalla address, map dataset ID and timeouts are configured directly in Desktop Control; availability can be checked through `/status`, or routing can be explicitly disabled. If Valhalla is not configured or is temporarily unavailable, the new point order is still stored by Control and derived geometry may simply be absent.

## Stable v0.1.0

[Open Stable v0.1.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0).

Stable is rebuilt from one exact source and publishes:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Android Driver debug APK for test installation;
- iOS/iPadOS Driver Simulator ZIP;
- Linux Addressing Gateway x64 package with Docker/Compose;
- SHA256SUMS.

**Linux Control** is the operator desktop application. **Linux Addressing Gateway** is the separate server relay module; they are different packages.

Android and Apple packages are not yet store-signed. App Store/TestFlight/Google Play distribution and production signing will be handled separately later.

## What is still ahead

- Planner/editor desktop UI and release builds for Windows, macOS, and Linux;

- manual local-browser acceptance;
- testing the same Driver Browser on a real phone in the same LAN;
- HTTPS/public addressing path;
- full browser GPS acceptance over HTTPS;
- field testing across different networks/NAT/CGNAT;


## Interface languages

Control, Driver Browser, Android Driver, and iOS/iPadOS Driver are implemented and verified with the same **8-language set: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Control and Driver Browser use system-language selection plus a persisted manual choice, and CI verifies complete coverage for all 8 locales.

## Documentation

Public recovery/state entry points: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), and [PROJECT_STATE.md](../../PROJECT_STATE.md).

Extended working project documentation is kept privately.

## License

Proprietary software © 2026 Roman Zavada. Public visibility of the repository does not grant open-source rights. See [LICENSE.md](../../LICENSE.md) and [COPYRIGHT.md](../../COPYRIGHT.md).
