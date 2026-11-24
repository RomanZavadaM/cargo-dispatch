# Third-Party Notices

cargo-dispatch може містити або розповсюджуватися разом зі сторонніми компонентами. Вони лишаються власністю своїх правовласників і підпорядковуються власним ліцензіям. Власницька ліцензія cargo-dispatch **не змінює** і не обмежує ліцензії сторонніх компонентів.

## Rust crates
Файловий обмін використовує crates.io: serde і serde_json для опису рейсу (MIT або Apache-2.0), sha2 для відбитка плану (MIT або Apache-2.0), tempfile для запису без перезапису наявних даних (MIT або Apache-2.0) та zip для переносного архіву (MIT). Перелік версій і непрямих залежностей фіксує Cargo.lock. Ліцензії й повідомлення про вразливості перевіряються під час збірки.

### Control loopback HTTP boundary
- `axum 0.8.9` — MIT, crates.io / upstream `tokio-rs/axum`; HTTP router/server boundary усередині Control.
- `tokio 1.54.1` — MIT, crates.io / upstream `tokio-rs/tokio`; loopback TCP runtime та graceful shutdown.
- Цей product slice не додає WebRTC/TURN/gateway runtime. Непрямі залежності цих crates фіксує `Cargo.lock` і перевіряє `cargo deny`.

## Дані
- Дорожні дані OpenStreetMap, якщо будуть використані, поширюються за ліцензією ODbL; потрібна атрибуція «© OpenStreetMap contributors».
- Реальні дані користувачів (авто, водії, заявки, координати) у репозиторії не зберігаються.


## CI / build-only tools
Ці компоненти використовуються лише під час збірки й перевірок та не входять до runtime-продукту.

- `actions/checkout` — MIT; workflow зафіксовано на commit `11d5960a326750d5838078e36cf38b85af677262` (лінія v4).
- `dtolnay/rust-toolchain` — MIT; workflow зафіксовано на commit `df2a88e9fe4098bad09cb42c3dec7187f15861fc`.
- `cargo-deny` — CI-only dependency/license/advisory checker, версія `0.21.0`; встановлюється з crates.io з `--locked`.

Зміна pin-ів є окремою чутливою зміною build supply chain і повинна проходити звичайний CI перед подальшим використанням.
