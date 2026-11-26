# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Поточний Stable checkpoint: **v0.1.0**.
- Поточний Test/prerelease: **v0.1.0-preview.7**, exact source `db3d6f7450bd886f80f8a48221f6254394fad08e`.
- Product positioning не обмежується малотонажним парком або масою 7,5 т.
- Control desktop підтримується для Windows, macOS і Linux.
- Planner desktop також підтримується для Windows, macOS і Linux; перші compiled Planner test builds уже є в `preview.7`.
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
  - Planner desktop показує real road geometry поверх OSM через власний runtime Valhalla preview.
- Наступний великий функціональний етап: **package export із Planner UI** через наявний versioned trip package.
- Користувацькі compiled test/stable builds мають публікуватися в GitHub Releases, не залишатися тільки в Actions artifacts.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
