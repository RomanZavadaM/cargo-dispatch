# Routing probe

Ця гілка — окрема технічна проба дорожньої маршрутизації. Вона не є planner/control product branch.

Мета:
- deterministic synthetic OSM fixture;
- Valhalla як primary candidate;
- thin normalized Rust geo contract;
- T01–T20 із приватної ROUTING_PROBE_SPEC.md;
- machine-readable CI report;
- без planner DB та UI.

Перший bootstrap-крок:
1. перевіряє Rust geo contract;
2. перевіряє XML fixture і конвертує його в PBF;
3. фіксує hash fixture та версію converter;
4. pull-ить Valhalla 3.9.3 і записує точний image digest;
5. генерує Cargo.lock як CI artifact.

Після bootstrap PASS Cargo.lock і exact image digest мають бути committed, після чого всі Cargo-команди переходять на --locked.
