# cargo-dispatch

[🇺🇦 Українська](../../README.md) · **🇬🇧 English** · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** active development. The current test prerelease is **v0.1.0-preview.4**. There is no stable finished product yet.

## About

**cargo-dispatch** is a dispatch system for a small-truck fleet (gross vehicle weight up to 7.5 t). It is intended to select loads, build and re-plan trips, hand trips over for execution, and give the dispatcher control over actual progress.

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

## Driver Browser

Driver Browser is **one product**, not separate phone and desktop versions. It has three normal layouts:

- narrow phone;
- tablet;
- wide desktop/laptop.

Trip data, route actions, files, and backend calls are shared; only layout changes with screen size.

## Map

Control and Driver Browser now contain a real interactive basemap with trip points and the latest accepted driver position.

The `main` branch now contains a versioned road-geometry pipeline (schema 1.2): Planner can carry derived geometry, Control stores it, and both maps draw the road line only while it matches the current stop order. Any reorder invalidates the stale line immediately.

What is still missing is the **live HTTP transport to Valhalla and automatic geometry recalculation**. The geometry contract and rendering exist; the external routing engine is not yet connected to the product runtime.

## Test build

Current prerelease: [v0.1.0-preview.4](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.4).

The Windows ZIP contains Control, a synthetic active trip, and a sample PDF for local loopback acceptance.

This is a **test/prerelease build**, not stable. Preview.4 already contains the Valhalla adapter and schema 1.2 road-geometry pipeline. `main` is ahead again: Android and iOS/iPadOS native Driver r2 were integrated after preview.4 was published.

## What is still ahead

- manual local-browser acceptance;
- testing the same Driver Browser on a real phone in the same LAN;
- HTTPS/public addressing path;
- full browser GPS acceptance over HTTPS;
- field testing across different networks/NAT/CGNAT;
- live Valhalla HTTP transport and geometry recalculation after reorder;
- native maps: MapLibre on Android and MapKit on iOS/iPadOS;\n- native camera capture, background GPS, connected virtual-device smoke, offline queue, and secure session storage;
- a stable release.

## Interface languages

The integrated desktop/browser UI currently implements and verifies **Ukrainian, English, and German**. The new native Android and iOS/iPadOS clients start with localization packs for **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** from the first screen, and their documentation follows the same language set.

## Documentation

Public recovery/state entry points: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), and [PROJECT_STATE.md](../../PROJECT_STATE.md).

Extended working project documentation is kept privately.

## License

Proprietary software © 2026 Roman Zavada. Public visibility of the repository does not grant open-source rights. See [LICENSE.md](../../LICENSE.md) and [COPYRIGHT.md](../../COPYRIGHT.md).
