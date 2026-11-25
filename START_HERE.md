# START_HERE — cargo-dispatch

Публічний вхід у проєкт.

1. Опис продукту, Stable release і мовні версії: [README.md](README.md).
2. Короткий публічний якір стану: [HANDOFF_CURRENT.md](HANDOFF_CURRENT.md) і [PROJECT_STATE.md](PROJECT_STATE.md).
3. Ліцензія та сторонні компоненти: [LICENSE.md](LICENSE.md), [COPYRIGHT.md](COPYRIGHT.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
4. Розширені правила, журнали, плани й архітектура: private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.
5. Перед продовженням роботи звірити live GitHub: `main`, open PR, Actions, branches і Releases. GitHub має пріоритет.
6. Stable checkpoint — `v0.1.0`.
7. У Stable входять:
   - Control desktop для Windows, macOS і Linux;
   - Driver Android;
   - Driver iPhone/iPad Simulator build;
   - Driver Browser;
   - Addressing Gateway для Linux;
   - maps/files/media/location/reconnect/background tracking;
   - durable mobile retry queues;
   - публічна Gateway-трансляція Control ↔ Driver без відкритого inbound-порту на машині Control.
8. Control залишається єдиним джерелом стану рейсу. Gateway не зберігає trip DB, документи, медіа чи історію позицій.
9. Повна локалізація 8 мовами інтегрована й захищена CI. Наступний великий напрямок — Planner маршруту для Windows, macOS і Linux.
