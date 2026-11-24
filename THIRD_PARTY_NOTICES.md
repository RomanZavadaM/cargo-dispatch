# Third-Party Notices

cargo-dispatch може містити або розповсюджуватися разом зі сторонніми компонентами. Вони лишаються власністю своїх правовласників і підпорядковуються власним ліцензіям. Власницька ліцензія cargo-dispatch **не змінює** і не обмежує ліцензії сторонніх компонентів.

## Rust crates
Файловий обмін використовує crates.io: serde і serde_json для опису рейсу (MIT або Apache-2.0), sha2 для відбитка плану (MIT або Apache-2.0), tempfile для запису без перезапису наявних даних (MIT або Apache-2.0) та zip для переносного архіву (MIT). Перелік версій і непрямих залежностей фіксує Cargo.lock. Ліцензії й повідомлення про вразливості перевіряються під час збірки.

### Control loopback HTTP boundary
- `axum 0.8.9` — MIT, crates.io / upstream `tokio-rs/axum`; HTTP router/server boundary усередині Control.
- `tokio 1.54.1` — MIT, crates.io / upstream `tokio-rs/tokio`; TCP/runtime, async file streaming та graceful shutdown.
- `futures-util 0.3.34` — MIT або Apache-2.0; використовується напряму для потокової передачі upload/download без завантаження всього відео/файла в RAM.
- `tempfile 3.27.0` — MIT або Apache-2.0; уже використовувався файловим шаром, тепер також створює тимчасовий файл browser upload до атомарного приймання в trip folder.
- Цей product slice не додає WebRTC/TURN/gateway runtime. Непрямі залежності цих crates фіксує `Cargo.lock` і перевіряє `cargo deny`.

### Control desktop UI
- `tauri 2.12.7` — MIT або Apache-2.0; accepted desktop shell для Windows/macOS/Linux.
- `tauri-build 2.7.6` — MIT або Apache-2.0; build-time Tauri configuration.
- `tauri-plugin-dialog 2.8.4` — MIT або Apache-2.0; native folder/file picker for Control without moving file bytes through the Leptos JSON bridge.
- `leptos 0.8.23` — MIT; accepted CSR UI layer.
- `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `console_error_panic_hook` — permissive Rust/WASM bridge dependencies for the Leptos frontend; exact versions and transitive graph are fixed by `Cargo.lock` and checked by `cargo deny`.
- `Trunk 0.22.2` — CI/build-only frontend bundler; installed as a prebuilt/checksummed tool through `taiki-e/install-action`.
- `taiki-e/install-action` — MIT або Apache-2.0, CI-only; pinned to commit `fe3acb53a98b62c772db35de75ce0e4595e7ca8c` (release 2.90.8), fallback disabled.
- Tauri/Leptos versions are the exact starting versions already accepted by UI probe P01–P15; product code still receives its own CI and dependency/license gate.
- Control desktop shell reuses the already accepted UI-probe icon resource; no additional icon/runtime dependency is introduced.

## Дані
- Дорожні дані OpenStreetMap, якщо будуть використані, поширюються за ліцензією ODbL; потрібна атрибуція «© OpenStreetMap contributors».
- Реальні дані користувачів (авто, водії, заявки, координати) у репозиторії не зберігаються.


## CI / build-only tools
Ці компоненти використовуються лише під час збірки й перевірок та не входять до runtime-продукту.

- `actions/checkout` — MIT; workflow зафіксовано на commit `11d5960a326750d5838078e36cf38b85af677262` (лінія v4).
- `dtolnay/rust-toolchain` — MIT; workflow зафіксовано на commit `df2a88e9fe4098bad09cb42c3dec7187f15861fc`.
- `cargo-deny` — CI-only dependency/license/advisory checker, версія `0.21.0`; встановлюється з crates.io з `--locked`.
- `taiki-e/install-action` — MIT; Control UI workflow зафіксовано на commit `fe3acb53a98b62c772db35de75ce0e4595e7ca8c` і використовує його лише для встановлення exact `trunk@0.22.2` без зміни product MSRV.

Зміна pin-ів є окремою чутливою зміною build supply chain і повинна проходити звичайний CI перед подальшим використанням.
