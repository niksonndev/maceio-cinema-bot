# Architecture

> Technical architecture for **Maceió Cine Bot** (Telegram + WhatsApp).

## Overview

Two apps share the same Ingresso.com domain (Maceió, city `53`) but not runtime code.

```
Telegram users ──POST /webhook──► API Gateway ──► Rust Lambda (teloxide, one update/invoke)
                                                      ├─► cache + prefs (S3)
                                                      └─► Ingresso.com

EventBridge (03:00 UTC) ──► Rust warm Lambda ──► S3 cache + setWebhook + commands

WhatsApp users ──► whatsapp-rust session (Render)
                      ├─► in-memory schedule cache + prefs
                      ├─► Ingresso.com
                      └─► GET / health + RENDER_EXTERNAL_URL keep-alive
```

## Telegram (`telegram/`)

Rust (`teloxide` + `lambda_http`). Production: AWS SAM custom runtime `provided.al2023`; each API Gateway request processes one Telegram update and awaits its Bot API calls before returning HTTP 200. Local: explicit `telegram-poll` binary + Axum health endpoint.

JSON cache (`data/cache.json` or S3) and prefs (`data/prefs.json` / S3). Ratings: in-memory cache (24h TTL).

`BotFunction` reloads cache and prefs from S3 on **every** invoke and awaits `handle_update()` before HTTP 200. `FetchFunction` is invoked by EventBridge and also registers the webhook and bot commands.

Local polling checks `getWebhookInfo` and refuses to start if a webhook is active. Use a staging bot token for local polling.

### Modules (`telegram/rust/src/`)

| Module | Responsibility |
| --- | --- |
| `api.rs` / `normalize.rs` | Ingresso.com client, normalization and denormalization |
| `store.rs` / `prefs.rs` | Local JSON or S3 cache and saved cinema preferences |
| `data.rs` / `types.rs` | Cache-aside orchestration and persisted data model |
| `format.rs` / `ratings.rs` | Telegram Markdown cards and OMDb/TMDb (24h) |
| `keyboards.rs` / `handlers.rs` | Inline keyboards, commands and carousel callbacks |
| `main.rs` / `fetch.rs` | Webhook Lambda and daily warm Lambda |
| `polling.rs` | Local polling + health endpoint; rejects active webhooks |

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
| `telegram/` | `cargo test --manifest-path rust/Cargo.toml` | Rust tests |
| `telegram/` | `cargo run --manifest-path rust/Cargo.toml --bin telegram-poll` | Local Telegram polling |
| `telegram/` | `sam build` / `sam deploy` / `bash scripts/sam-warm.sh` | SAM build and deploy |
| `whatsapp/` | `cargo run` | WhatsApp bot + health |
| `whatsapp/` | `cargo test` | Unit tests |
