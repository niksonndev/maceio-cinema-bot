# AGENTS.md

## Cursor Cloud specific instructions

### Overview

**Maceió Cine Bot** is a monorepo with two apps that query the Ingresso.com public API for cinema schedules in Maceió, Brazil:

| App | Path | Host | Messaging |
|---|---|---|---|
| Telegram | `telegram/` | AWS SAM (Lambda + API Gateway + EventBridge + S3) | Telegram Bot API |
| WhatsApp | `whatsapp/` | Render Free Web Service | [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) (unofficial; dedicated number) |

The Telegram app is Rust (`teloxide`) built as two AWS Lambda `bootstrap` binaries. The WhatsApp app is a separate Rust binary (`maceio-cine-whatsapp`).

### Entry points

#### Telegram (`cd telegram`)

| Script | Command | Purpose |
|---|---|---|
| `cargo test --manifest-path rust/Cargo.toml --locked` | Rust unit tests for data, formatting and routing. |
| `cargo run --manifest-path rust/Cargo.toml --bin telegram-poll` | Local polling + Axum health check. Requires `TELEGRAM_BOT_TOKEN`; refuses when a webhook is active. |
| `sam validate --template-file template.yaml` | Validate the Rust SAM stack. |
| `sam build --template-file template.yaml` | Build both Rust Lambda binaries with Cargo Lambda. |
| `sam deploy` | Deploy the SAM stack (needs AWS credentials). |
| `bash scripts/sam-warm.sh` | Invoke FetchFunction (setWebhook + cache warm). |
| `bash scripts/sam-local.sh` | Build and invoke a no-op webhook smoke event (Docker). |
| Lambda | `telegram-webhook` / `telegram-fetch` | Production webhook + daily cache warm (`provided.al2023`). |

#### WhatsApp (`cd whatsapp`)

| Command | Purpose |
|---|---|
| `cargo run` | Local WhatsApp bot + health on `0.0.0.0:$PORT`. Prints a QR on first link. |
| `cargo test` | Unit tests (normalize/format/cinemas). |
| `cargo build --release` | Release binary (used by Docker / Render). |

### Environment variables

**Telegram** — copy `telegram/.env.example` to `telegram/.env`. `TELEGRAM_BOT_TOKEN` is required for local polling and Lambda.

- `TELEGRAM_BOT_TOKEN` — required for local polling / Lambda
- `OMDb_API_KEY` — optional, enables IMDb/RT ratings
- `TMDB_API_KEY` — optional, fallback ratings from TMDb
- `S3_BUCKET` / `CACHE_KEY` / `PREFS_KEY` — set by SAM in production; when `S3_BUCKET` is set, cache and cinema prefs use S3 instead of `data/*.json`
- `WEBHOOK_URL` — set by SAM on **FetchFunction** to `${HttpApi.ApiEndpoint}/prod/webhook` (not on BotFunction — that would circular-depend on HttpApi); `setWebHook` runs when Fetch initializes
- `AWS_ENDPOINT_URL` — optional custom S3 endpoint (e.g. LocalStack)
- `PORT` — local polling health check only (default `10000`)

**WhatsApp** — copy `whatsapp/.env.example` to `whatsapp/.env`.

- `PORT` — health HTTP port (Render injects this; default `10000`)
- `SESSION_PATH` — SQLite session file (default `data/whatsapp.db`; `/tmp/whatsapp.db` on Render)
- `RENDER_EXTERNAL_URL` — set by Render; keep-alive ping every 10 minutes
- `OMDb_API_KEY` / `TMDB_API_KEY` — optional ratings
- Cinema prefs are **in-memory only** (lost on restart)

### Running without Telegram token

From the repository root, `cargo test --manifest-path telegram/rust/Cargo.toml --locked` tests the Rust data and formatting pipeline without a Telegram token. From `whatsapp/`, `cargo test` does not need WhatsApp.

### Lint / Test / Build

Telegram (`cd telegram`):

- `cargo fmt --manifest-path rust/Cargo.toml --check`
- `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path rust/Cargo.toml --locked`
- `sam validate --template-file template.yaml` / `sam build --template-file template.yaml`
- `bash scripts/sam-local.sh` — local webhook smoke test (Docker)

WhatsApp (`cd whatsapp`):

- `cargo test`
- `cargo build --release`

### Verifying the Telegram bot without Telegram login

With `TELEGRAM_BOT_TOKEN` set:

```bash
# Bot identity
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"
# Registered commands (should show 4)
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMyCommands"
# Webhook: production = API Gateway URL; local polling = empty url
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
# Local health check (polling mode only)
curl -s http://localhost:10000/
```

### Notes

- Telegram: no database — cache is `data/cache.json` locally, or S3 (`CACHE_KEY`, default `cache.json`) when `S3_BUCKET` is set. Cinema preferences are `data/prefs.json` locally or S3 (`PREFS_KEY`, default `prefs.json`).
- WhatsApp: in-process schedule cache + in-memory prefs. Session is SQLite at `SESSION_PATH` (ephemeral on Render Free — QR again after sleep/redeploy).
- The Ingresso.com API is public and requires no API key; it uses browser-like User-Agent headers.
- Local polling checks Telegram webhook status and refuses to start if the token has an active webhook. Use a staging token.
- Telegram `.env` is created from `telegram/.env.example`. Write the token with: `sed -i "s|^TELEGRAM_BOT_TOKEN=.*|TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}|" telegram/.env`
- BotFunction reloads cache + prefs on every invoke and awaits `handle_update` before returning HTTP 200.
- GitHub Actions (`.github/workflows/ci-cd.yml`): PRs run Telegram Cargo fmt/Clippy/tests/SAM build **and** WhatsApp `cargo test`/`cargo build --release`; push to `main` also `sam deploy` (OIDC) from `telegram/` and warms FetchFunction.
- WhatsApp is unofficial (whatsapp-rust). Use a **dedicated number** you can afford to lose. Render Free sleeps after 15 minutes without traffic unless keep-alive is working (`RENDER_EXTERNAL_URL`).
