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
9. Поточний `main` уже випереджає `preview.7`: Planner manual draft зберігається у versioned append-only journal в app-data і відновлюється після перезапуску; у поточному Planner-зрізі також є real Valhalla road preview з реальною дорожньою geometry поверх OSM.
10. Control залишається єдиним джерелом фактичного стану активного рейсу. Gateway не зберігає trip DB, документи, медіа чи історію позицій.
11. Повна локалізація 8 мовами інтегрована й захищена CI, включно з Planner UI.
12. Наступний Planner етап після цього зрізу — package export із Planner UI через наявний versioned trip package; preview geometry лишається derived і не змінює чернетку при помилці routing.
