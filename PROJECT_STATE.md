# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Поточний Stable checkpoint: **v0.1.0**.
- Поточний Test/prerelease: **v0.1.0-preview.7**, exact source `db3d6f7450bd886f80f8a48221f6254394fad08e`.
- Product positioning не обмежується малотонажним парком або масою 7,5 т.
- Control desktop підтримується для Windows, macOS і Linux.
- Planner desktop також підтримується для Windows, macOS і Linux; перші compiled Planner test builds уже є в `preview.7`.
- Поточний `main`: `a45034d498e961d244070ae1aaf185df595dd9ef`.
- Planner manual draft у `main` уже durable:
  - add/remove/reorder;
  - navigation coordinates;
  - revision через Planner core;
  - versioned append-only journal в app-data;
  - restart recovery;
  - захист від незавершеного trailing journal record.
- Control/Driver Browser, Addressing Gateway, Android Driver, iOS/iPadOS Driver і 8-language localization інтегровані.
- Control лишається authoritative source фактичного стану активного рейсу.
- Planner/routing foundation має:
  - Valhalla adapter;
  - schema 1.3 derived road geometry + actual VehicleRoutingProfile; legacy 1.0–1.2 readable;
  - real runtime HTTP transport;
  - runtime endpoint + map dataset id + bounded connect/request timeouts у Desktop Control;
  - Valhalla `/status` availability + live engine version;
  - `/route` POST;
  - normalized timeout/unavailable/dataset-missing/error states;
  - live geometry recalculation після operator/driver reorder;
  - stale response protection;
  - невідомі параметри автомобіля не підставляються;
  - відмова routing не скасовує reorder.
- Наступний великий функціональний етап: **map/routing preview у Planner desktop UI**, після нього package export із Planner UI.
- Користувацькі compiled test/stable builds мають публікуватися в GitHub Releases, не залишатися тільки в Actions artifacts.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
