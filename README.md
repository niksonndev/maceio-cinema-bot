# 🎬 Maceió Cine Bot

> Telegram bot that queries real-time cinema schedules in Maceió, with showtimes, prices, and a direct ticket-purchase link.

[Try it on Telegram](https://t.me/MaceioCine_bot)

---

## 🟢 Deploy status

Production target: **AWS SAM** (Lambda + API Gateway webhook + EventBridge + S3).
Local polling (`npm run bot:listen`) is for development only.

| Item | Detail |
| --- | --- |
| ☁️ **IaC** | [`template.yaml`](template.yaml) (AWS SAM) |
| 🔌 **Ingress** | Telegram webhook → API Gateway `POST /webhook` → `dist/lambda.handler` |
| 💾 **Cache** | S3 (`cache.json` + `prefs.json`) in production; `data/*.json` locally |
| ⏰ **Warm** | Daily EventBridge cron → `dist/lambda.fetchHandler` |
| 📋 **Guide** | [`docs/deployment.md`](docs/deployment.md) |

---

## ✨ Features

🎥 **Today's movies** — Full schedule with showtimes, formats (2D, 3D, Cinépic, VIP), and prices

🏢 **Multiple cinemas** — Supports 3 cinema chains in Maceió:

| Cinema | Mall |
| --- | --- |
| Cinesystem | Parque Shopping Maceió |
| Centerplex | Shopping Pátio Maceió |
| Kinoplex | Maceió Shopping |

🆕 **Upcoming releases** — Future movies with release dates and pre-sales

🎫 **Direct purchase** — Inline button that opens the cinema page on Ingresso.com

⚡ **Smart cache** — Daily per-cinema cache with automatic midnight expiry (`America/Maceio` timezone), avoiding unnecessary requests

🔄 **Data normalization** — Prices, showtimes, and room types normalized from the API

---

## 🤖 Preview

Try it now on Telegram: **[@MaceioCine_bot](https://t.me/MaceioCine_bot)**

```
User:   /start
Bot:    Olá! Eu sou o seu guia de cinema em Maceió. 🍿
        Escolha abaixo qual cinema você deseja consultar:

        [ Cinesystem (Parque Shopping Maceió) ]
        [ Centerplex (Shopping Pátio Maceió)  ]
        [ Kinoplex (Maceió Shopping)           ]

User:   taps "Cinesystem"
Bot:    ✅ Cinema selecionado: Cinesystem (Parque Shopping Maceió)

        [ 🎬 Filmes de Hoje ] [ 🆕 Próximos Lançamentos ]
        [ 🔄 Trocar de Cinema                            ]

User:   taps "Filmes de Hoje"
Bot:    🎬 PROGRAMAÇÃO
        📍 Cinesystem (Parque Shopping Maceió)
        📅 24 de fevereiro de 2026

        🎭 Avatar: Fogo E Cinzas
           🎞 2D: 14:30, 17:45, 20:45 — R$ 55,86
           ⭐ VIP: 21:00 — R$ 72,00
        ...

        [ 🎫 Comprar Ingressos           ]
        [ ⬅️ Voltar ao menu ] [ 🔄 Trocar cinema ]
```

---

## 📋 Commands

| Command | Description |
| --- | --- |
| `/start` | Start the bot and choose a cinema |
| `/hoje` | Movies showing at the selected cinema |
| `/proximos` | Upcoming releases and pre-sales |
| `/cinemas` | Switch selected cinema |

---

## 🛠️ Tech stack

| Technology | Use |
| --- | --- |
| **Node.js 22** | Runtime (local, tests, Lambda) |
| **TypeScript** | Source in `src/`; emit to `dist/` for SAM |
| **Telegram Bot API** | Bot via [node-telegram-bot-api](https://github.com/yagop/node-telegram-bot-api) (polling mode) |
| **Axios** | HTTP requests to the Ingresso.com API |
| **Express** | HTTP server for health check (dynamic port) |
| **dotenv** | Environment variable management |

---

## 🚀 Getting started

### Run locally

**1. Clone the repository**

```bash
git clone https://github.com/seu-usuario/maceio-cinema-bot.git
cd maceio-cinema-bot
```

**2. Install dependencies**

```bash
npm install
```

**3. Configure environment variables**

```bash
cp .env.example .env
```

Edit `.env` and set the bot token (from [@BotFather](https://t.me/BotFather)):

```env
TELEGRAM_BOT_TOKEN=your_token_here
```

Optionally set `PORT` (local default: `10000`).

**4. Start the bot** (local polling — do not use the same production token)

```bash
npm run bot:listen
```

The bot will listen for Telegram commands. The health check is at `http://localhost:10000/` (or the port set in `PORT`).

**5. Tests** (requires Docker)

```bash
npm test
```

---

## 📂 Architecture

The project is a TypeScript (Node.js) application with modules in `src/`. See the
[full architecture docs](docs/architecture.md) for data-flow and responsibility details.

| Module          | Short responsibility                                         |
| --------------- | ------------------------------------------------------------ |
| `api.ts`        | HTTP client (Axios) for the public Ingresso.com API          |
| `normalize.ts`  | Separates static movie data from dynamic session data        |
| `cache.ts`      | JSON cache (local file or S3) with daily per-cinema expiry   |
| `data.ts`       | Orchestrates cache ↔ API ↔ normalize (cache hit before API)  |
| `cinemas.ts`    | Definition of the 3 cinemas + per-user preferences           |
| `format.ts`     | Telegram message formatting (Markdown)                       |
| `ratings.ts`    | IMDb/RT ratings (OMDb) + TMDb fallback, 24h cache            |
| `keyboards.ts`  | Telegram inline keyboard builders                            |
| `handlers.ts`   | Command and callback handlers                                |
| `bot.ts`        | Local entry (polling + Express + graceful shutdown)          |
| `lambda.ts`     | Production entry (webhook Lambda + fetch/cache warm)         |
| `index.ts`      | CLI for quick terminal verification                          |
| `types.ts`      | Domain types (cache, sessions, Telegram, BotLike)            |

See also the technical docs in [`docs/`](./docs):

- [`docs/architecture.md`](docs/architecture.md) — data flows and modules
- [`docs/data-model.md`](docs/data-model.md) — full `cache.json` structure
- [`docs/caching.md`](docs/caching.md) — expiration rules and TTL
- [`docs/api-reference.md`](docs/api-reference.md) — endpoints and contracts
- [`docs/deployment.md`](docs/deployment.md) — AWS SAM deploy (legacy Render)
- [`docs/development.md`](docs/development.md) — how to run locally

---

## 🔧 Environment variables

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `TELEGRAM_BOT_TOKEN` | Yes | — | Bot token from @BotFather |
| `PORT` | No | `10000` | Express port (health check) in local polling mode |
| `OMDb_API_KEY` | No | — | [OMDb](https://www.omdbapi.com/apikey.aspx) key for IMDb/RT |
| `TMDB_API_KEY` | No | — | [TMDb](https://www.themoviedb.org/settings/api) fallback |
| `S3_BUCKET` | No* | — | Cache and prefs bucket (*injected by SAM in production) |
| `CACHE_KEY` | No | `cache.json` | S3 cache object key |
| `PREFS_KEY` | No | `prefs.json` | S3 prefs object key |
| `WEBHOOK_URL` | No* | — | Webhook URL (*SAM on FetchFunction: `…/prod/webhook`); `setWebHook` on fetch |

---

## ☁️ Deploy (AWS SAM — Lambda + API Gateway)

Deploy via **SAM**: Lambda + API Gateway (webhook) + EventBridge (cache warm) + S3.
Cinema preferences persist in S3 (`prefs.json`).

See the [full guide](docs/deployment.md).

```bash
npm run sam:build
sam deploy            # first time (guided) or subsequent
```

---

## 📄 License

MIT
