# Connectivity prototype

Окремий технічний каркас для перевірки зв'язку **control ↔ gateway ↔ browser**.

Це не продуктовий код і не реалізація планувальника.

Перший етап каркаса:
- спільний JSON-протокол signaling;
- gateway на Axum;
- реєстрація control/trip сесій;
- WebSocket signaling;
- синтетичний control;
- браузерна оболонка;
- GitHub CI.

Наступний етап після зеленого CI:
- реальний WebRTC PeerConnection у control;
- browser RTCPeerConnection;
- два data channels: службовий і файловий;
- STUN/TURN;
- direct/TURN сценарії;
- chunked file transfer + SHA-256;
- reconnect/offline queue.

Усі дані прототипу синтетичні.

CI status: workflow має запускатися на кожен push у цю прототипну гілку.

## Поточний стан прототипу

Працює в гілці `prototype/connectivity-r1`:
- gateway реєструє control і кілька trip-сесій;
- браузер отримує ICE config після автентифікації;
- WebRTC offer/answer через signaling gateway;
- окремі `control` і `files` data-channel;
- STUN та підготовка TURN з короткоживучими credentials;
- двосторонні файли з SHA-256 та ACK;
- browser reconnect і черга легких подій;
- control reconnect зі стабільними локальними ID;
- два одночасні незалежні тестові рейси в одному control.

JavaScript у `web/` — лише браузерний тестовий harness для перевірки WebRTC/Web APIs. Це не рішення щодо фінального UI продукту.

Для ручної проби control друкує готові URL виду:
`https://gateway.example/?trip=<trip_session_id>&token=<access_token>`.

Автоматично перевірено в GitHub Actions:
- Rust format/check/clippy/tests;
- окремий MSRV 1.91 job;
- синтаксис браузерного harness;
- збірка на Windows і macOS;
- Compose validation;
- relay-only WebRTC data-channel через локально піднятий coturn з time-limited secret credentials.

Gateway Docker image і Compose перевірені; контейнер реально запускається в CI, `/health` відповідає `ok`, а стартова сторінка водія віддається з контейнера.

Ще не перевірено без реального публічного вузла й телефона:
- direct WebRTC через реальні NAT/CGNAT;
- TURN relay через реальну зовнішню мережу;
- перемикання Wi-Fi ↔ mobile data на реальному телефоні;
- поведінка мобільного браузера при переході в навігатор.


Технічний baseline прототипу: Rust 1.91+. Причина — `webrtc-rs 0.21.1` / `rtc-mdns` використовує API `Ipv4Addr::from_octets`, стабільний від Rust 1.91.
