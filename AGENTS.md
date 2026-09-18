# AGENTS.md

## Cursor Cloud specific instructions

### Overview

**Maceió Cine Bot** is a Telegram bot that queries the Ingresso.com public API for real-time cinema schedules in Maceió, Brazil. It is a TypeScript Node.js application (ES Modules, `"type": "module"`), compiled with `tsc` to `dist/` for Lambda.

**Production** runs on AWS SAM (Lambda + API Gateway webhook + EventBridge + S3 cache + S3 prefs). **Local/dev** can use polling via Express.

### Entry points

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

### Environment variables

Copy `.env.example` to `.env`. Only `TELEGRAM_BOT_TOKEN` is required for the bot; the CLI works without any tokens.

- `TELEGRAM_BOT_TOKEN` — required for `npm run bot:listen` / Lambda
- `OMDb_API_KEY` — optional, enables IMDb/RT ratings
- `TMDB_API_KEY` — optional, fallback ratings from TMDb
- `S3_BUCKET` / `CACHE_KEY` / `PREFS_KEY` — set by SAM in production; when `S3_BUCKET` is set, cache and cinema prefs use S3 instead of `data/*.json`
- `WEBHOOK_URL` — set by SAM on **FetchFunction** to `${HttpApi.ApiEndpoint}/prod/webhook` (not on BotFunction — that would circular-depend on HttpApi); `setWebHook` runs when Fetch initializes
- `AWS_ENDPOINT_URL` — tests only (LocalStack)
- `PORT` — local polling health check only (default `10000`)

### Running without Telegram token

Use `npm start` to exercise the core data pipeline (API fetch, normalization, cache) without needing a Telegram token. The best automated check is `npm test` (Docker).

### Lint / Test / Build

The project includes ESLint, Prettier, and Vitest (run in Docker):

- `npm test` — Docker Compose (Node 22 + LocalStack) when Docker is available; otherwise Vitest on the host
- `npm run typecheck` — TypeScript (`src/` + `test/`)
- `npm run lint` — lint `src/` and `test/` with ESLint
- `npm run lint:fix` — lint + auto-fix
- `npm run format` — format `src/` and `test/` with Prettier
- `npm run format:check` — verify formatting
- `npm run build` — compile TypeScript to `dist/`
- `sam validate` / `npm run sam:build` — SAM template validation and package build

### Verifying the bot without Telegram login

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

- No database — cache is `data/cache.json` locally, or S3 (`CACHE_KEY`, default `cache.json`) when `S3_BUCKET` is set. Cinema preferences are `data/prefs.json` locally or S3 (`PREFS_KEY`, default `prefs.json`).
- The `data/` directory is created automatically on first local run.
- The Ingresso.com API is public and requires no API key; it uses browser-like User-Agent headers (defined in `src/api.ts`).
- Express health check runs on `PORT` (default `10000`) and is part of `bot.ts` (local/dev only).
- Do not run `npm run bot:listen` against the same bot token while the production webhook is active (polling vs webhook conflict).
- The `.env` file must be created from `.env.example` and `TELEGRAM_BOT_TOKEN` filled in for bot mode. The `TELEGRAM_BOT_TOKEN` secret is injected as an env var; write it to `.env` with: `sed -i "s|^TELEGRAM_BOT_TOKEN=.*|TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}|" .env`
- BotFunction reloads cache + prefs on every invoke and awaits `handleUpdate` (does not use `processUpdate`).
- GitHub Actions (`.github/workflows/ci-cd.yml`): PRs run lint + typecheck + test; push to `main` also `sam deploy` (OIDC) and warms FetchFunction.
