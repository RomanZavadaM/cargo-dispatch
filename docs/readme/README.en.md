# cargo-dispatch

[🇺🇦 Українська](../../README.md) · **🇬🇧 English** · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Dispatching for a cargo fleet: planning a trip, handing it over for execution, staying in touch with the driver, and recording what actually happened. The main development language is **Rust**.

**Stable version: [v0.2.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.2.0).** Development continues; test builds are listed under [Releases](https://github.com/RomanZavadaM/cargo-dispatch/releases).

## Components

| Program | Who uses it | Platforms |
|---|---|---|
| **Control** | dispatcher: runs the active trips | Windows, macOS, Linux |
| **Planner** | planner: builds the route and the trip package | Windows, macOS, Linux |
| **Driver page** | driver in any browser, nothing to install | phone, tablet, computer |
| **Driver for Android** | driver | Android 8.0 or later |
| **Driver for iPhone and iPad** | driver | iOS / iPadOS 17 or later |
| **Addressing Gateway** | connection over the internet | Linux server |

Control is the only source of truth for an active trip. It serves the page, route and files to the driver and receives marks, positions and attachments. There is no central trip database.

## Getting started

1. Download an archive from the release page, unpack it and start **Control**. The Control package contains made-up test data (`local-browser-test-data`).
2. **“Add trip”** → “Open an existing trip” (a trip folder or a Planner package) or “Create a new trip manually”.
3. **“Connect drivers”** → “This computer”, “Same Wi-Fi network” or “Anywhere, over the internet”.
4. Give the driver the link from the **“What to do next”** block: “Open” or “Copy”.
5. The driver opens the link in a browser or pastes it into the Driver app.

## Features

**Control**
- several trips at once, a separate window per trip;
- a trip from a Planner package or created manually; state is stored locally and restored after a restart;
- order of open stops, map, latest driver position, plan versus fact;
- files to the driver; files, photos and videos from the driver;
- finds the computer's local network address itself; the connection mode switches without stopping it by hand;
- error messages in the interface language; position time, file size and stop name in plain words.

**Planner**
- route draft: add, remove and reorder points by coordinates; the draft survives a restart;
- route preview and road line from your Valhalla server with explicitly entered vehicle parameters (unknown values are not invented);
- export of a trip package (ZIP, schema 1.4) with cargo and planning basis; an existing file is never overwritten.

**Driver: browser, Android, iPhone and iPad**
- route and map, marking a stop as done, reordering open stops;
- receiving documents; sending files, photos and videos;
- position sharing; in the apps also in the background, switched on and off explicitly, with a retry queue;
- marking a stop as done asks for confirmation; the apps also offer a navigator button, attaching a file to a stop, a self-refreshing screen, keeping the travelled track without a connection (up to 500 positions) and visible refused files.

## Connection and security

- **This computer** and **local network** modes are unencrypted (HTTP): use them only on a network you trust. Network access is never switched on by itself and no router ports are opened.
- A link with a secret code gives access to the trip: share it only with that trip's driver.
- Phone browsers usually allow geolocation only over HTTPS, that is, through the Gateway.
- For a phone to reach Control on the network, allow the program in the computer's firewall.
- **Over the internet:** your own Gateway behind an HTTPS reverse proxy. Control connects to it itself, so no inbound port is needed on the computer. The Gateway stores no trips, documents, media or positions. The default relay limit is 64 MiB per request (configurable up to 512 MiB).

## Maps and routes

The map in Control, Planner, the browser and Android is [OpenStreetMap](https://www.openstreetmap.org/copyright) (© OpenStreetMap contributors); on iPhone and iPad it is Apple MapKit. The map provider sees the device's IP address and the area shown. The road line needs your own Valhalla server; without it the trip works, only the line is missing.

## Languages

All programs are available in українська, English, Français, Deutsch, Español, 한국어, 日本語 and 简体中文. CI checks that translations are complete; technical system details in error messages stay in English.

## Not yet verified, and limitations

- The Android and iPhone/iPad apps have not been tested on physical devices — only in an emulator, a simulator and automated tests.
- Mobile internet and the Gateway on a real network are verified by automated tests only.
- Packages are unsigned; Android is a debug APK, iOS is an Apple Simulator build only.
- After Control restarts, trips and the connection have to be switched on again by hand.
- The dispatcher cannot edit stops or mark them as done yet.

## Building

Builds, tests and releases run in GitHub Actions; see `.github/workflows`.

## Documentation

Project status (in Ukrainian): [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md). Extended working documentation is kept privately.

## License

Proprietary software © 2026 Roman Zavada (Роман Завада). A public repository does not grant open-source rights. See [LICENSE.md](../../LICENSE.md), [COPYRIGHT.md](../../COPYRIGHT.md) and [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
