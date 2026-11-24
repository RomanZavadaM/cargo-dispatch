# UI probe

Окрема технічна проба. Це не planner/control product branch.

Pinned baseline:
- Rust 1.91 project minimum;
- Leptos 0.8.23 (stable 0.8 line);
- Trunk 0.22.2;
- Tauri 2.12.7;
- Dioxus stable remains the secondary candidate and is not pulled into the first harness unless the Leptos/Tauri probe exposes a blocker.

Foundation goal:
1. one synthetic application service independent of any UI runtime;
2. Leptos CSR browser build over the same service;
3. Tauri shell over the same service through a narrow command;
4. GitHub Actions from the first UI commit;
5. no planner database, connectivity implementation, routing implementation or product business state.

P01–P15 are not considered PASS just because the foundation compiles.
