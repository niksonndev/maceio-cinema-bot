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
| Telegram | **Rust + teloxide** on AWS SAM (`provided.al2023`) — webhook → API Gateway `POST /webhook` → Lambda. Cache in S3; EventBridge warms daily. |
| WhatsApp | **Render Free** — always-on-ish Docker service (`render.yaml`). Health `GET /`. Keep-alive via `RENDER_EXTERNAL_URL`. |
| Guides | [`docs/deployment.md`](docs/deployment.md) |

Local Telegram polling (`cargo run --manifest-path telegram/rust/Cargo.toml --bin telegram-poll`) is for development only. It refuses to start if that token already has a webhook configured.

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

### Telegram (Rust)

```bash
cd telegram
cp .env.example .env   # set TELEGRAM_BOT_TOKEN
cargo test --manifest-path rust/Cargo.toml
cargo run --manifest-path rust/Cargo.toml --bin telegram-poll
```

SAM (from `telegram/`):

```bash
sam validate --template-file template.yaml
sam build --template-file template.yaml
sam deploy
bash scripts/sam-warm.sh
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

See [`docs/architecture.md`](docs/architecture.md). Telegram Rust modules live under `telegram/rust/src/`; WhatsApp remains an independent Rust app under `whatsapp/`.

---

## License

MIT
