# AGENTS.md

## Cursor Cloud specific instructions

### Overview

**Maceió Cine Bot** is a monorepo with two apps that query the Ingresso.com public API for cinema schedules in Maceió, Brazil:

| App | Path | Host | Messaging |
|---|---|---|---|
| Telegram | `telegram/` | AWS SAM (Lambda + API Gateway + EventBridge + S3) | Telegram Bot API |
| WhatsApp | `whatsapp/` | Render Free Web Service | [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) (unofficial; dedicated number) |

The TypeScript Telegram app is compiled with `tsc` to `telegram/dist/` for Lambda. The WhatsApp app is a Rust binary (`maceio-cine-whatsapp`).

### Entry points

#### Telegram (`cd telegram`)

| Script | Command | Purpose |
|---|---|---|
| `npm start` | `tsx src/index.ts` | CLI — validates the data pipeline (fetch + console output). No tokens needed. |
| `npm run bot:listen` | `tsx src/bot.ts` | Local Telegram bot (polling) + Express health check. **Requires** `TELEGRAM_BOT_TOKEN`. |
| `npm test` | Docker Compose | Vitest command suites (Node 22 + LocalStack S3). **Requires Docker**. |
| `npm run typecheck` | `tsc --noEmit` | TypeScript check (`src/` + `test/`). |
| `npm run build` | `tsc -p tsconfig.json` | Emit compiled JS to `dist/` (used by SAM). |
| `npm run sam:build` | `npm run build && sam build` | Compile TypeScript then build the SAM application. |
| `npm run sam:deploy` | `sam deploy` | Deploy the SAM stack (needs AWS credentials). |
| `npm run sam:warm` | `scripts/sam-warm.sh` | Invoke FetchFunction (setWebhook + cache warm). |
| `npm run sam:local` | `scripts/sam-local.sh` | `sam local invoke` per event file (Docker). |
| Lambda | `dist/lambda.handler` / `dist/lambda.fetchHandler` | Production webhook + daily cache warm. |

#### WhatsApp (`cd whatsapp`)

| Command | Purpose |
|---|---|
| `cargo run` | Local WhatsApp bot + health on `0.0.0.0:$PORT`. Prints a QR on first link. |
| `cargo test` | Unit tests (normalize/format/cinemas). |
| `cargo build --release` | Release binary (used by Docker / Render). |

### Environment variables

**Telegram** — copy `telegram/.env.example` to `telegram/.env`. Only `TELEGRAM_BOT_TOKEN` is required for the bot; the CLI works without any tokens.

- `TELEGRAM_BOT_TOKEN` — required for `npm run bot:listen` / Lambda
- `OMDb_API_KEY` — optional, enables IMDb/RT ratings
- `TMDB_API_KEY` — optional, fallback ratings from TMDb
- `S3_BUCKET` / `CACHE_KEY` / `PREFS_KEY` — set by SAM in production; when `S3_BUCKET` is set, cache and cinema prefs use S3 instead of `data/*.json`
- `WEBHOOK_URL` — set by SAM on **FetchFunction** to `${HttpApi.ApiEndpoint}/prod/webhook` (not on BotFunction — that would circular-depend on HttpApi); `setWebHook` runs when Fetch initializes
- `AWS_ENDPOINT_URL` — tests only (LocalStack)
- `PORT` — local polling health check only (default `10000`)

**WhatsApp** — copy `whatsapp/.env.example` to `whatsapp/.env`.

- `PORT` — health HTTP port (Render injects this; default `10000`)
- `SESSION_PATH` — SQLite session file (default `data/whatsapp.db`; `/tmp/whatsapp.db` on Render)
- `RENDER_EXTERNAL_URL` — set by Render; keep-alive ping every 10 minutes
- `OMDb_API_KEY` / `TMDB_API_KEY` — optional ratings
- Cinema prefs are **in-memory only** (lost on restart)

### Running without Telegram token

From `telegram/`, use `npm start` to exercise the core data pipeline without a Telegram token. The best automated check is `npm test` (Docker). From `whatsapp/`, `cargo test` does not need WhatsApp.

### Lint / Test / Build

Telegram (`cd telegram`):

- `npm test` — Docker Compose (Node 22 + LocalStack) when Docker is available; otherwise Vitest on the host
- `npm run typecheck` — TypeScript (`src/` + `test/`)
- `npm run lint` — lint `src/` and `test/` with ESLint
- `npm run lint:fix` — lint + auto-fix
- `npm run format` — format `src/` and `test/` with Prettier
- `npm run format:check` — verify formatting
- `npm run build` — compile TypeScript to `dist/`
- `sam validate` / `npm run sam:build` — SAM template validation and package build (cwd `telegram/`)

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
- Do not run `npm run bot:listen` against the same bot token while the production webhook is active (polling vs webhook conflict).
- Telegram `.env` is created from `telegram/.env.example`. Write the token with: `sed -i "s|^TELEGRAM_BOT_TOKEN=.*|TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}|" telegram/.env`
- BotFunction reloads cache + prefs on every invoke and awaits `handleUpdate` (does not use `processUpdate`).
- GitHub Actions (`.github/workflows/ci-cd.yml`): PRs run Telegram lint/typecheck/test **and** WhatsApp `cargo test`/`cargo build --release`; push to `main` also `sam deploy` (OIDC) from `telegram/` and warms FetchFunction.
- WhatsApp is unofficial (whatsapp-rust). Use a **dedicated number** you can afford to lose. Render Free sleeps after 15 minutes without traffic unless keep-alive is working (`RENDER_EXTERNAL_URL`).
