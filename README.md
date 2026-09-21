# Maceió Cine Bot

Telegram **and** WhatsApp bots that query real-time cinema schedules in Maceió, with showtimes, prices, and a link to Ingresso.com.

This repo is a **monorepo**:

| App | Path | Production |
| --- | --- | --- |
| Telegram | [`telegram/`](telegram/) | AWS SAM (Lambda + API Gateway webhook + EventBridge + S3) |
| WhatsApp | [`whatsapp/`](whatsapp/) | Render Free Web Service ([whatsapp-rust](https://github.com/oxidezap/whatsapp-rust)) |

[Try it on Telegram](https://t.me/MaceioCine_bot)

---

## Deploy status

| Channel | Detail |
| --- | --- |
| Telegram | **AWS SAM** — webhook → API Gateway `POST /webhook` → `telegram/dist/lambda.handler`. Cache in S3. Daily EventBridge warm. |
| WhatsApp | **Render Free** — always-on-ish Docker service (`render.yaml`). Health `GET /`. Keep-alive via `RENDER_EXTERNAL_URL`. |
| Guides | [`docs/deployment.md`](docs/deployment.md) |

Local Telegram polling (`cd telegram && npm run bot:listen`) is for development only.

---

## Features

- **Today's movies** — showtimes, formats (2D, 3D, Cinépic, VIP), and prices
- **Multiple cinemas** — Cinesystem (Parque Shopping), Centerplex (Pátio), Kinoplex (Maceió Shopping)
- **Upcoming releases** — pre-sales and future dates
- **Smart cache** — daily per-cinema expiry in `America/Maceio`

WhatsApp uses text commands (`hoje`, `proximos`, `cinemas`) instead of Telegram inline keyboards. Prefs are in-memory (reset on process restart).

**WhatsApp is unofficial.** Use a dedicated phone number you can afford to lose.

---

## Commands

| Telegram | WhatsApp (text) | Description |
| --- | --- | --- |
| `/start` | `start` | Start and choose a cinema (`1`/`2`/`3`) |
| `/hoje` | `hoje` | Movies showing today |
| `/proximos` | `proximos` | Upcoming releases and pre-sales |
| `/cinemas` | `cinemas` | Switch selected cinema |
| `/atualizar` | `atualizar` | Force-refresh today's schedule |

---

## Getting started

### Telegram (TypeScript)

```bash
cd telegram
npm install
cp .env.example .env   # set TELEGRAM_BOT_TOKEN
npm start              # pipeline check, no token
npm run bot:listen     # local polling
npm test
```

SAM (from `telegram/`):

```bash
npm run sam:build
sam deploy
npm run sam:warm
```

### WhatsApp (Rust)

```bash
cd whatsapp
cp .env.example .env
cargo test
cargo run              # prints a QR on first link; health on :10000
```

Scan the QR: WhatsApp → Settings → Linked devices. Render deploy uses [`render.yaml`](render.yaml) (Docker, Free plan, health `/`).

---

## Architecture

See [`docs/architecture.md`](docs/architecture.md). Telegram modules live under `telegram/src/`; WhatsApp ports the same Ingresso pipeline in `whatsapp/src/`.

---

## License

MIT
