# Architecture

> Technical architecture for **Maceió Cine Bot** (Telegram + WhatsApp).

## Overview

Two apps share the same Ingresso.com domain (Maceió, city `53`) but not runtime code.

```
Telegram users ──POST /webhook──► API Gateway ──► Lambda (telegram/dist/lambda.handler)
                                                      ├─► cache + prefs (S3)
                                                      └─► Ingresso.com

EventBridge (03:00 UTC) ──► Lambda fetchHandler ──► S3 cache + setWebHook

WhatsApp users ──► whatsapp-rust session (Render)
                      ├─► in-memory schedule cache + prefs
                      ├─► Ingresso.com
                      └─► GET / health + RENDER_EXTERNAL_URL keep-alive
```

## Telegram (`telegram/`)

TypeScript (Node.js, ES Modules). Production: AWS SAM. Local: polling + Express.

JSON cache (`data/cache.json` or S3) and prefs (`data/prefs.json` / S3). Ratings: in-memory Map (24h TTL).

BotFunction reloads cache and prefs from S3 on **every** invoke and **awaits** `handleUpdate()` before HTTP 200.

Do not run local polling with the same token while the production webhook is active.

### Modules (`telegram/src/`)

| Module | Responsibility |
| --- | --- |
| `api.ts` | Ingresso.com HTTP client |
| `normalize.ts` | Static movies vs dynamic sessions; `denormalize()` |
| `cache.ts` | File or S3 persistence |
| `data.ts` | Cache-aside orchestration |
| `cinemas.ts` | 3 theaters + persisted prefs |
| `format.ts` | Telegram Markdown cards |
| `ratings.ts` | OMDb / TMDb (24h) |
| `keyboards.ts` | Inline keyboards |
| `handlers.ts` | Commands + carousel callbacks |
| `bot.ts` | Local polling |
| `lambda.ts` | Webhook + daily warm |
| `index.ts` | CLI |
| `types.ts` | Domain types |

## WhatsApp (`whatsapp/`)

Rust binary (`maceio-cine-whatsapp`) using [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) (Baileys-class unofficial client). One Render Free service: session + cinema logic + health HTTP.

| Module | Responsibility |
| --- | --- |
| `ingresso.rs` | Ingresso HTTP (same URLs/headers as `api.ts`) |
| `normalize.rs` / `cache.rs` / `data.rs` | In-process port of the TS pipeline |
| `cinemas.rs` / `prefs.rs` | Same 3 theaters; **in-memory** jid → theater |
| `format.rs` / `ratings.rs` | Plain-text cards |
| `handlers.rs` | `start` / `hoje` / `proximos` / `cinemas` / `atualizar` |
| `http.rs` | `GET /` on `0.0.0.0:$PORT` + keep-alive ping |
| `wa.rs` | QR, connect, `on_message` |
| `main.rs` | Process entry |

No Telegram carousel. Prefs and schedule cache die with the process. Session SQLite is ephemeral on Render Free (QR again after sleep/redeploy). Keep-alive every 10 minutes when `RENDER_EXTERNAL_URL` is set. One Free instance (~750 h/month) is enough for 24/7.

## Data flow (both channels)

### Today's movies

```
handlers
  └─ get_movies_for_date(cache, date, theaterId)
       ├─ cache hit → denormalize
       └─ miss → fetch_normalized → merge movies / set sessions → denormalize
                        └─ format card
```

### Upcoming

```
handlers
  └─ get_upcoming_movies(cache, theaterId)
       ├─ cache hit
       └─ miss → fetch_upcoming (pre-sale only) → set upcoming → format card
```

## Supported theaters

| `theaterId` | Cinema | Shopping |
| --- | --- | --- |
| `1162` | Cinesystem | Parque Shopping Maceió |
| `1230` | Centerplex | Shopping Pátio Maceió |
| `924` | Kinoplex | Maceió Shopping |

## Entry points

| Where | Command | Purpose |
| --- | --- | --- |
| `telegram/` | `npm start` | CLI pipeline (no token) |
| `telegram/` | `npm run bot:listen` | Local Telegram polling |
| `telegram/` | `npm test` / `sam:build` / `sam:deploy` / `sam:warm` | Tests and SAM |
| `whatsapp/` | `cargo run` | WhatsApp bot + health |
| `whatsapp/` | `cargo test` | Unit tests |
