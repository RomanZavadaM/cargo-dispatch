# cargo-dispatch

[🇺🇦 Українська](../../README.md) · **🇬🇧 English** · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** active development. The current stable version is **v0.1.0**. The current test prerelease is **v0.1.0-preview.7**, now including a separate Planner for Windows, macOS, and Linux. Full 8-language localization is integrated.

## About

**cargo-dispatch** is a dispatch system for cargo fleets. It is intended to select loads, build and re-plan trips, hand trips over for execution, and give the dispatcher control over actual progress.

The main development language is **Rust**.

## What already exists

- a separate desktop **Control** application built with Tauri/Leptos;
- a separate desktop **Planner** built with Tauri/Leptos: manual route draft, point add/remove/reorder, navigation coordinates, and Planner-core revision tracking; the draft is stored in a versioned append-only journal in app data and recovered after restart;
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

Control and Driver Browser already contain a real interactive basemap with trip points and the latest accepted driver position.

The current `main` uses schema 1.4 with an immutable planning basis; older schemas remain readable. A Planner routing profile is only the input for a particular planning calculation, not a permanent trip-to-vehicle binding. After import, Control maintains a separate mutable actual routing context for the active session.

Control keeps derived geometry and recalculates the road line through its configured runtime Valhalla service after a reorder. A late routing response cannot overwrite newer trip state. If Valhalla is not configured or is temporarily unavailable, the new point order is still stored and derived geometry may be absent.

Planner desktop now has its own runtime Valhalla preview: the operator supplies the Valhalla address, map dataset ID and timeouts, probes `/status`, and `/route` is calculated only from an explicitly supplied planning profile. Unknown numeric constraints are not invented. A successful result draws the real road polyline over OSM; no fake straight connector is drawn without a route result. The preview is derived from route/revision/profile: editing the draft or planning profile clears the old line, and a late response is rejected. Routing failure does not mutate the Planner draft.

Planner desktop now also exports the existing versioned trip package as schema 1.4 through a native Save As dialog. The package carries an explicit planning basis (vehicle type/class and cargos), ordered points and, when available, only the matching planning routing profile/Valhalla geometry snapshot. TripId remains stable for one draft, TripVersion continues across restarts, CargoId is independent of row position, and an existing ZIP is never overwritten.

## Test v0.1.0-preview.7

[Open Test v0.1.0-preview.7](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.7).

This is a **prerelease**, not Stable. It is the first integrated preview to publish a separate desktop Planner together with the other product components from one exact source.

The release contains:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Windows Planner x64 ZIP;
- macOS Planner universal ZIP — Intel + Apple Silicon;
- Linux Planner x64 tar.gz;
- Android Driver debug APK;
- iOS/iPadOS Driver Simulator ZIP;
- Linux Addressing Gateway x64 package;
- SHA256SUMS.

The first Planner slice supports a manual route draft and point editing. In preview.7, the Planner draft is not yet persisted across restarts; that is the next stage.

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

- creating a new trip directly in Control when no Planner ZIP is available;

- manual local-browser acceptance;
- testing the same Driver Browser on a real phone in the same LAN;
- HTTPS/public addressing path;
- full browser GPS acceptance over HTTPS;
- field testing across different networks/NAT/CGNAT;


## Interface languages

Control, Planner, Driver Browser, Android Driver, and iOS/iPadOS Driver are implemented and verified with the same **8-language set: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Control and Driver Browser use system-language selection plus a persisted manual choice, and CI verifies complete coverage for all 8 locales.

## Documentation

Public recovery/state entry points: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), and [PROJECT_STATE.md](../../PROJECT_STATE.md).

Extended working project documentation is kept privately.

## License

Proprietary software © 2026 Roman Zavada. Public visibility of the repository does not grant open-source rights. See [LICENSE.md](../../LICENSE.md) and [COPYRIGHT.md](../../COPYRIGHT.md).
