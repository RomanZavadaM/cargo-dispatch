# Addressing Gateway: запуск із релізного пакета

Gateway слухає звичайний HTTP на самому сервері й має стояти за HTTPS reverse proxy. Рейсів, документів, медіа й позицій він не зберігає.

## Docker

Виконувати в теці пакета:

```bash
export CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN='replace-with-a-random-secret-of-32-characters-or-more'
docker compose up -d --build
```

Приклад відкриває порт лише на самому сервері (`127.0.0.1:8080`). HTTPS дає Caddy, nginx або Traefik на тому ж сервері.

## Без Docker

```bash
export CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN='replace-with-a-random-secret-of-32-characters-or-more'
./cargo-dispatch-addressing-gateway
```

Бінарний файл зібраний на Ubuntu 22.04 (x86-64).

## Вимоги до reverse proxy

- назовні лише HTTPS;
- метод, шлях, рядок запиту й тіла передаються без змін;
- `/driver/*`, `/api/trip/*` і `/v1/*` не кешуються;
- довгий запит `/v1/control/*/next` може тривати щонайменше 30 секунд;
- секрет реєстрації Control лишається тільки в налаштуваннях сервера й Control, у посилання водія він не потрапляє.
