# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Поточний Stable checkpoint: **v0.1.0**.
- Product positioning не обмежується малотонажним парком або масою 7,5 т.
- Control desktop підтримується для Windows, macOS і Linux.
- Planner desktop також має виходити для Windows, macOS і Linux.
- Control/Driver Browser, Addressing Gateway, Android Driver, iOS/iPadOS Driver і 8-language localization інтегровані.
- Control лишається authoritative source фактичного стану активного рейсу.
- Planner/routing має:
  - Valhalla adapter;
  - schema 1.3 derived road geometry + actual VehicleRoutingProfile; legacy 1.0–1.2 readable;
  - real runtime HTTP transport;
  - runtime endpoint + map dataset id + bounded connect/request timeouts у Desktop Control;
  - Valhalla `/status` availability + live engine version;
  - `/route` POST;
  - normalized timeout/unavailable/dataset-missing/error states;
  - live geometry recalculation після operator/driver reorder;
  - stale response protection: запізніла geometry не переписує новішу ревізію Control;
  - невідомі параметри автомобіля не підставляються;
  - відмова routing не скасовує reorder — похідна geometry просто може бути відсутня.
- Наступний великий функціональний етап: **Planner desktop UI composition** для Windows/macOS/Linux.
- Користувацькі compiled test/stable builds мають публікуватися в GitHub Releases, не залишатися тільки в Actions artifacts.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
