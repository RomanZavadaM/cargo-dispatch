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
