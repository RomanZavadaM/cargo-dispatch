# Публічний тестовий вузол connectivity-прототипу

Цей каталог готує **лише технічний тестовий вузол**, не production-інфраструктуру.

## Що запускається

- `gateway` — HTTPS-оболонка/реєстрація/signaling/ICE config. Сам контейнер слухає HTTP на `127.0.0.1:8080`; HTTPS завершує зовнішній reverse proxy.
- `coturn` — STUN/TURN. Офіційний образ `coturn/coturn:4.19.2-r0`, host networking.
- Дані рейсів, PDF, фото й відео на сервері не архівуються.

## Порти тестового вузла

Назовні потрібні:
- TCP 443 — HTTPS/WSS до reverse proxy gateway;
- UDP/TCP 3478 — STUN/TURN;
- UDP 49160–49200 — тестовий TURN relay range.

`8080` не відкривати назовні: compose публікує його лише на `127.0.0.1`.

## TURN authentication

Анонімний TURN не використовується. Gateway і coturn мають один server-side secret.
Gateway формує короткоживучий username `<expires_at>:<trip_session_id>` і password через HMAC-SHA1 + Base64; coturn перевіряє їх через `--use-auth-secret --static-auth-secret`.

Постійний TURN password у браузері чи control не зберігається.

## Перед реальним тестом

1. Створити окремий тестовий DNS name, наприклад `cargo-test.example.org`.
2. Налаштувати HTTPS reverse proxy цього імені на `127.0.0.1:8080`.
3. Скопіювати `.env.example` у `.env` і замінити host/realm/secret.
4. Відкрити лише потрібні порти у firewall.
5. Запустити compose.
6. У control задати:
   - `CARGO_GATEWAY_WS=wss://<host>/signal`
   - `CARGO_DRIVER_BASE_URL=https://<host>/`

## Що вважати реальною перевіркою

CI доводить компіляцію й логіку, але **не NAT/CGNAT**.

На публічному вузлі треба окремо зафіксувати:
- direct WebRTC між control і телефоном у різних мережах;
- примусовий TURN relay;
- browser→control файл;
- control→browser PDF;
- status/position;
- обрив і reconnect;
- два одночасні рейси;
- перемикання Wi-Fi ↔ mobile data;
- перехід у зовнішній навігатор і повернення.

Результати записати за T1–T14 із приватного `CONNECTIVITY_PROTOTYPE_SPEC.md`.

## Важливо

Це тестовий deployment. До production потрібні окремо:
- політика оновлень і резервування конфігів;
- rate limits/DoS protection;
- метрики й алерти;
- ротація TURN secret;
- перевірка firewall/IPv6;
- рішення щодо TLS termination та домену.
