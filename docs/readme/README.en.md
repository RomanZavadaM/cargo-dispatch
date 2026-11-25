# cargo-dispatch

[🇺🇦 Українська](../../README.md) · **🇬🇧 English** · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** active development. The current test prerelease is **v0.1.0-preview.3**. There is no stable finished product yet.

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

This is **not yet full road-route geometry**. Real Valhalla route geometry is still to be integrated.

## Test build

Current prerelease: [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

The Windows ZIP contains Control, a synthetic active trip, and a sample PDF for local loopback acceptance.

This is a **test/prerelease build**, not stable.

## What is still ahead

- manual local-browser acceptance;
- testing the same Driver Browser on a real phone in the same LAN;
- HTTPS/public addressing path;
- full browser GPS acceptance over HTTPS;
- field testing across different networks/NAT/CGNAT;
- real Valhalla road geometry;
- a future native Driver App;
- a stable release.

## Interface languages

The current UI implements and verifies **Ukrainian, English, and German**. The other README translations are documentation/target languages and do not yet mean that the program UI supports them.

## Documentation

Public recovery/state entry points: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), and [PROJECT_STATE.md](../../PROJECT_STATE.md).

Extended working project documentation is kept privately.

## License

Proprietary software © 2026 Roman Zavada. Public visibility of the repository does not grant open-source rights. See [LICENSE.md](../../LICENSE.md) and [COPYRIGHT.md](../../COPYRIGHT.md).
