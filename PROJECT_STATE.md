# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Поточний Stable checkpoint: **v0.1.0**.
- Поточний Test/prerelease: **v0.1.0-preview.8**, exact source `47352ca341a2d3747d503164de0555b891db6bf1`.
- Product positioning не обмежується малотонажним парком або масою 7,5 т.
- Control desktop підтримується для Windows, macOS і Linux.
- Planner desktop також підтримується для Windows, macOS і Linux; compiled Planner test builds входять у `preview.8`.
- Planner manual draft у `main` уже durable:
  - add/remove/reorder;
  - navigation coordinates;
  - revision через Planner core;
  - versioned append-only journal в app-data;
  - restart recovery;
  - захист від незавершеного trailing journal record.
- Control/Driver Browser, Addressing Gateway, Android Driver, iOS/iPadOS Driver і 8-language localization інтегровані.
- Автоматичний Driver Browser acceptance інтегрований для двох прямих шляхів без Gateway: loopback і явний private-LAN bind. Він перевіряє реальний Control session, HTTP/state, snapshot і mutation; ручна візуальна responsive-перевірка та фізичний телефон у LAN/VPN лишаються окремим acceptance.
- Control лишається authoritative source фактичного стану активного рейсу.
- Planner/routing foundation має:
  - Valhalla adapter і real runtime HTTP transport;
  - schema 1.4 planning basis; legacy schema залишаються читабельними;
  - Planner planning profile використовується лише для конкретного розрахунку і не є постійною прив'язкою рейсу до авто;
  - Control має окремий mutable actual routing context активної сесії;
  - runtime endpoint + map dataset id + bounded connect/request timeouts у Desktop Control;
  - Valhalla `/status` availability + `/route` POST;
  - normalized timeout/unavailable/dataset-missing/error states;
  - live geometry recalculation після operator/driver reorder;
  - stale response protection і для Control, і для Planner preview;
  - невідомі параметри автомобіля не підставляються;
  - відмова routing не скасовує Control reorder і не змінює Planner draft;
  - Planner desktop показує real road geometry поверх OSM через власний runtime Valhalla preview;
  - Planner desktop експортує versioned schema 1.4 ZIP через native Save As, з explicit planning basis, optional matching routing snapshot, durable TripVersion history та stable CargoId.
- Control уже вміє **створювати новий активний рейс вручну** без Planner ZIP, на тому самому durable trip state; planning basis при цьому не вигадується, а TripId не прив'язується до автомобіля або endpoint.
- Поточний Test: **v0.1.0-preview.8** уже опублікований з повним набором Control/Planner Windows/macOS/Linux + Android + iOS Simulator + Linux Gateway + SHA256SUMS.
- Користувацькі compiled test/stable builds мають публікуватися в GitHub Releases, не залишатися тільки в Actions artifacts.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
