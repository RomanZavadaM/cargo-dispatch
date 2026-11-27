# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Native Apple driver client. One SwiftUI target supports iPhone and iPad and uses the **same Control API** as Driver Browser and Android Driver App.

Current scope: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver URL; trip snapshot; complete/reorder; native MapKit map with current road geometry, markers and latest position; one-shot foreground CoreLocation; file/photo/video transfer and system camera; durable retry queue for positions and media; explicit Start/Stop background CoreLocation tracking with `UIBackgroundModes=location` and a 25 m filter; secure session recovery with the token in a non-synchronizing Keychain item (`AfterFirstUnlockThisDeviceOnly`) and only the non-secret base URL in UserDefaults; automatic migration away from the legacy raw Driver URL after a successful reconnect; uk/en/de/fr/es/ko/ja/zh-Hans localization; pure Swift protocol tests and connected Simulator smoke against a real synthetic Control trip.

Local HTTP is allowed only for Simulator/LAN Debug; Release remains HTTPS-first.

**In the driver's daily work**

- an “Open in navigator” button next to each stop that is not done;
- “Mark as done” asks for confirmation;
- new files can be attached to a stop or to the whole trip;
- the screen refreshes by itself every 15 seconds while open and shows whether there is a connection;
- without a connection the travelled track is kept (up to the latest 500 positions) and sent in order;
- a file the dispatch program refused stays visible: send it again or remove it;
- error messages are in the phone's language, without internal terms.

Still ahead: real-device LAN/background-tracking acceptance, HTTPS/public/NAT/CGNAT field acceptance, push/notifications, and production signing/TestFlight.
