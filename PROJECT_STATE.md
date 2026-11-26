# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Поточний Stable checkpoint: **v0.1.0**.
- Product positioning не обмежується малотонажним парком або масою 7,5 т.
- Control desktop підтримується для Windows, macOS і Linux.
- Planner desktop також має виходити для Windows, macOS і Linux.
- Control/Driver Browser, Addressing Gateway, Android Driver, iOS/iPadOS Driver і 8-language localization інтегровані.
- Control лишається authoritative source фактичного стану активного рейсу.
- Planner має Valhalla adapter, schema 1.2 derived road geometry та **real runtime HTTP transport**:
  - runtime endpoint + map dataset id;
  - connect/request timeouts;
  - Valhalla `/status` availability + live engine version;
  - `/route` POST;
  - normalized timeout/unavailable/dataset-missing/error states;
  - невідомі параметри автомобіля не підставляються.
- Наступний функціональний крок: **live reroute geometry** після operator/driver reorder без зміни ownership Control.
- Користувацькі compiled test/stable builds мають публікуватися в GitHub Releases, не залишатися тільки в Actions artifacts.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
