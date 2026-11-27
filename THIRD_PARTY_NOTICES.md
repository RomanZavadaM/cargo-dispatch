# Third-Party Notices

cargo-dispatch може містити або розповсюджуватися разом зі сторонніми компонентами. Вони лишаються власністю своїх правовласників і підпорядковуються власним ліцензіям. Власницька ліцензія cargo-dispatch **не змінює** і не обмежує ліцензії сторонніх компонентів.

## Rust crates
Файловий обмін використовує crates.io: serde і serde_json для опису рейсу (MIT або Apache-2.0), sha2 для відбитка плану (MIT або Apache-2.0), tempfile для запису без перезапису наявних даних (MIT або Apache-2.0) та zip для переносного архіву (MIT). Перелік версій і непрямих залежностей фіксує Cargo.lock. Ліцензії й повідомлення про вразливості перевіряються під час збірки.

### Planner Valhalla HTTP transport
- `reqwest 0.13.5` — MIT або Apache-2.0; уже є в dependency graph Control Gateway, тепер напряму використовується Planner geo layer для HTTPS/HTTP звернень до налаштованого оператором Valhalla.
- Runtime transport звертається лише до Valhalla `/status` для доступності/версії tileset і `/route` для маршруту. URL Valhalla та ідентифікатор набору карт задаються runtime-конфігурацією; код не підставляє вигаданих параметрів автомобіля.
- Маршрутний запит передає налаштованому Valhalla координати точок і лише ті обмеження автомобіля, які реально задані. Реальні production endpoint/дані/секрети в репозиторії не зберігаються.

### Control loopback HTTP boundary
- `axum 0.8.9` — MIT, crates.io / upstream `tokio-rs/axum`; HTTP router/server boundary усередині Control.
- `tokio 1.54.1` — MIT, crates.io / upstream `tokio-rs/tokio`; TCP/runtime, async file streaming та graceful shutdown.
- `futures-util 0.3.34` — MIT або Apache-2.0; використовується напряму для потокової передачі upload/download без завантаження всього відео/файла в RAM.
- `tempfile 3.27.0` — MIT або Apache-2.0; уже використовувався файловим шаром, тепер також створює тимчасовий файл browser upload до атомарного приймання в trip folder.
- Непрямі залежності цих crates фіксує `Cargo.lock` і перевіряє `cargo deny`.

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

## Native Driver clients
- Android Driver використовує AndroidX/Jetpack (Apache-2.0) для Compose UI, lifecycle, Activity Result contracts і FileProvider; exact dependency graph фіксується Gradle/Maven metadata.
- Android native map використовує `org.maplibre.gl:android-sdk-opengl:13.7.1`, upstream `maplibre/maplibre-native`, BSD-2-Clause. Обрано stable OpenGL artifact для ширшої сумісності реальних пристроїв та Emulator.
- MapLibre 13.7.1 HTTP implementation формує User-Agent з application package/version/code через `HttpIdentifier` перед MapLibre version, тому tile requests ідентифікують саме `com.romanzavada.cargodispatch.driver`, а не лише generic map library; це відповідає OSM native-app identification rule без окремого custom network client.
- MapLibre є лише renderer/client map layer. Source of truth рейсу лишається Control API; route geometry, stop order і latest driver location не зберігаються в окремій map-базі.
- `org.jetbrains.kotlinx:kotlinx-serialization-json:1.13.0` — Apache-2.0; розбір і запис JSON інтерфейсу Control в Android Driver.
- Захоплення фото/відео викликає системний Android camera contract; окремий camera SDK не додається.
- Непрямі залежності Android цим файлом ще не перелічені.
- iOS/iPadOS Driver використовує лише системні бібліотеки Apple; сторонніх пакетів Swift немає.

## Компоненти під MPL-2.0
Настільні програми через залежності Tauri використовують без змін такі crates під [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/). Їхній вихідний код безкоштовно доступний на crates.io:

- [`cssparser` 0.37.0](https://crates.io/crates/cssparser/0.37.0) і [0.38.0](https://crates.io/crates/cssparser/0.38.0);
- [`cssparser-macros` 0.7.1](https://crates.io/crates/cssparser-macros/0.7.1);
- [`dtoa-short` 0.3.5](https://crates.io/crates/dtoa-short/0.3.5);
- [`selectors` 0.38.0](https://crates.io/crates/selectors/0.38.0) і [0.41.0](https://crates.io/crates/selectors/0.41.0);
- [`option-ext` 0.2.0](https://crates.io/crates/option-ext/0.2.0).

Ліцензія cargo-dispatch не обмежує права отримувача на вихідний код цих компонентів за MPL-2.0.

## Дані
- Базова карта використовує стандартні raster tiles OpenStreetMap через `https://tile.openstreetmap.org/{z}/{x}/{y}.png`; дані OpenStreetMap поширюються за ODbL і у Control, Driver Browser і native Android map показується атрибуція «© OpenStreetMap contributors». Це runtime network request до стороннього tile service: провайдер бачить IP клієнта та запитану область карти. Driver Browser використовує звичайний browser User-Agent + cross-origin origin Referer; головні вікна Control і Planner ідентифікуються як `CargoDispatchControl/0.3.0` та `CargoDispatchPlanner/0.3.0` (+https://github.com/RomanZavadaM/cargo-dispatch); окремі вікна рейсових сесій Control власного ідентифікатора поки не задають. Browser cache не обходиться, bulk/offline prefetch не виконується; недоступність tiles не повинна блокувати route/files/media.
- Реальні дані користувачів (авто, водії, заявки, координати) у репозиторії не зберігаються.


## CI / build-only tools
Ці компоненти використовуються лише під час збірки й перевірок та не входять до runtime-продукту.

- `actions/checkout` — MIT; workflow зафіксовано на commit `11d5960a326750d5838078e36cf38b85af677262` (лінія v4).
- `dtolnay/rust-toolchain` — MIT; workflow зафіксовано на commit `df2a88e9fe4098bad09cb42c3dec7187f15861fc`.
- `cargo-deny` — CI-only dependency/license/advisory checker, версія `0.21.0`; встановлюється з crates.io з `--locked`.
- `cloudflared 2026.11.2` — Apache-2.0, CI-only; офіційний Linux amd64 binary завантажується з release `cloudflare/cloudflared` і перевіряється за SHA256 `56f13335143ec0e22772963290683a84a04114a35ec64cbc794a4d39ff9bb8cf`. Тимчасовий Quick Tunnel використовується лише для public HTTPS/NAT-style acceptance із синтетичним рейсом, не входить до продукту або release assets, не зберігає production secrets і припиняє існування після job.
- `taiki-e/install-action` — MIT; Control UI workflow зафіксовано на commit `fe3acb53a98b62c772db35de75ce0e4595e7ca8c` і використовує його лише для встановлення exact `trunk@0.22.2` без зміни product MSRV.
- `actions/upload-artifact` — MIT; Control UI workflow зафіксовано на commit `cf430e030ddbb5b0abf93d22962f4752f3646cd9` (лінія v7) і використовує його лише для короткоживучого Windows CI-артефакту ручного loopback-тесту; release не створюється.

Зміна pin-ів є окремою чутливою зміною build supply chain і повинна проходити звичайний CI перед подальшим використанням.
