# START_HERE — cargo-dispatch

Публічний вхід у проєкт.

1. Опис продукту, Stable/Test release і мовні версії: [README.md](README.md).
2. Короткий публічний якір стану: [HANDOFF_CURRENT.md](HANDOFF_CURRENT.md) і [PROJECT_STATE.md](PROJECT_STATE.md).
3. Ліцензія та сторонні компоненти: [LICENSE.md](LICENSE.md), [COPYRIGHT.md](COPYRIGHT.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
4. Розширені правила, журнали, плани й архітектура: private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.
5. Перед продовженням роботи звірити live GitHub: `main`, open PR, Actions, branches і Releases. GitHub має пріоритет.
6. Stable checkpoint — `v0.1.0`.
7. Поточний Test/prerelease — `v0.1.0-preview.7`, exact source `db3d6f7450bd886f80f8a48221f6254394fad08e`.
8. У `preview.7` опубліковані:
   - Control desktop для Windows x64, macOS universal і Linux x64;
   - Planner desktop для Windows x64, macOS universal і Linux x64;
   - Driver Android;
   - Driver iPhone/iPad Simulator build;
   - Addressing Gateway для Linux;
   - `SHA256SUMS.txt`.
9. Поточний `main` уже випереджає `preview.7`: Planner manual draft durable, має real Valhalla road preview і експортує versioned schema 1.4 ZIP через native Save As; Control може імпортувати цей ZIP або створити новий активний рейс вручну без Planner ZIP. TripId не прив'язаний до автомобіля чи endpoint.
10. Control залишається єдиним джерелом фактичного стану активного рейсу. Gateway не зберігає trip DB, документи, медіа чи історію позицій.
11. Повна локалізація 8 мовами інтегрована й захищена CI, включно з Planner UI.
12. Ручне створення рейсу в Control уже інтегроване й використовує той самий durable trip state без вигаданого planning basis. Поточна release-підготовка — повний `v0.1.0-preview.8` з Control/Planner Windows/macOS/Linux, Android, iOS Simulator, Gateway і SHA256SUMS.
