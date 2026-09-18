# Development — Running locally

> Guide for developers who want to run, test, and debug the project locally.

## Prerequisites

| Tool        | Version / notes |
| ----------- | --------------- |
| TypeScript  | ^5 (compiles to `dist/` for SAM) |
| Node.js     | >= 20 (22 in test Docker / Lambda) |
| npm         | >= 10          |
| Docker      | **Required for `npm test`** and for `sam local invoke` |
| AWS SAM CLI | For `sam validate` / `sam build` / deploy / `sam:local` |
| AWS CLI     | For deploy and `sam:warm` |

## 1. Clone and install

```bash
git clone https://github.com/seu-usuario/maceio-cinema-bot.git
cd maceio-cinema-bot

npm install
# or, for a deterministic install:
npm ci
```

## 2. Configure environment variables

```bash
cp .env.example .env
```

Edit `.env`:

```env
TELEGRAM_BOT_TOKEN=your_token_here        # required for the bot (via @BotFather)
OMDb_API_KEY=your_omdb_key                # optional (IMDb/RT ratings)
TMDB_API_KEY=your_tmdb_key                # optional (TMDb fallback)
# PORT = 10000                            # optional, local default
# S3_BUCKET=                              # only if testing S3 cache locally
# CACHE_KEY=cache.json
# PREFS_KEY=prefs.json
```

> 💡 Only `TELEGRAM_BOT_TOKEN` is required for `npm run bot:listen`.
> The CLI (`npm start`) works **with no tokens**.

## 3. Scripts

| Script             | Command               | Description                                                   |
| ------------------ | --------------------- | ------------------------------------------------------------- |
| `start`            | `tsx src/index.ts`    | CLI — validates the pipeline (fetch + console). No tokens.    |
| `bot:listen`       | `tsx src/bot.ts`      | **Local** Telegram bot (polling) + Express health check.      |
| `build`            | `tsc -p tsconfig.json`| Compiles `src/` → `dist/` (required for SAM).                 |
| `typecheck`        | `tsc --noEmit`        | TypeScript check for `src/` and `test/`.                      |
| `test`             | Docker Compose        | Vitest (Node 22 + LocalStack S3). Requires Docker.            |
| `lint`             | `eslint src/ test/`   | Lint (ESLint + Prettier).                                     |
| `lint:fix`         | `eslint src/ test/ --fix` | Lint + auto-fix.                                       |
| `format`           | `prettier --write src/ test/` | Auto-format.                                        |
| `format:check`     | `prettier --check src/ test/` | Verify formatting.                                  |
| `sam:build`        | `npm run build && sam build` | Compiles TS and packages the SAM application.          |
| `sam:deploy`       | `sam deploy`          | Publishes the stack (requires AWS configured).                |
| `sam:warm`         | `scripts/sam-warm.sh` | Invokes FetchFunction (webhook + cache).                      |
| `sam:local`        | `scripts/sam-local.sh`| `sam local invoke` per event (Docker).                        |

## 4. Validate the pipeline without Telegram

```bash
npm start
```

This runs `src/index.ts`, which:
1. Fetches today's schedule (Cinesystem, theater `1162`).
2. Normalizes the data.
3. Prints the list of movies and sessions to the console.

This is the **recommended** way to verify the codebase works — it requires
no tokens.

## 5. Tests (Docker)

```bash
npm test
```

Starts LocalStack (S3) + a `node:22-bookworm` container when Docker is
available (`npm run test:docker` forces that path). Without Docker, `npm test`
falls back to Vitest on the host (prefs in a temp file). Covers `/start`, `/hoje`,
`/proximos`, `/cinemas`, `/atualizar`, callbacks, prefs persistence, and the
Lambda handler await.

Does not call the real Ingresso.com API or Telegram (bot and data layer are mocked,
except the S3 prefs round-trip).

Extra Lambda runtime smoke (does not validate Telegram delivery):

```bash
npm run sam:local
```

## 6. Run the bot (local polling)

```bash
npm run bot:listen
```

- The bot starts in **polling mode** (no webhook).
- Health check available at `http://localhost:10000/`.
- In Telegram, send `/start` and pick a cinema.
- **Do not** use the same token while a production webhook is active.

### Debug: verify connection

```bash
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMyCommands"
# Local polling: url should be empty
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
curl -s http://localhost:10000/
```

## 7. SAM local (optional)

Requires SAM CLI + Docker:

```bash
sam validate
npm run sam:build
npm run sam:local
sam local start-api
```

## 8. Lint and formatting

```bash
npm run lint
npm run typecheck
npm run lint:fix
npm run format
npm run format:check
```

## 9. Directory structure

```
maceio-cinema-bot/
├── src/
│   ├── api.ts
│   ├── normalize.ts
│   ├── cache.ts            # local file or S3
│   ├── data.ts
│   ├── cinemas.ts          # cinema list + persisted prefs
│   ├── format.ts
│   ├── ratings.ts
│   ├── keyboards.ts
│   ├── handlers.ts         # handleUpdate (polling and webhook)
│   ├── bot.ts              # local polling
│   ├── lambda.ts           # webhook + fetchHandler (production)
│   ├── types.ts
│   └── index.ts
├── test/                   # Vitest (run via Docker Compose)
├── dist/                   # tsc emit (not committed)
├── events/
│   ├── webhook-event.json
│   ├── env.json            # dummy env for sam local
│   ├── commands/
│   └── callbacks/
├── scripts/
│   ├── sam-warm.sh
│   └── sam-local.sh
├── .github/workflows/ci-cd.yml
├── docker-compose.test.yml
├── docs/
├── data/                   # local runtime (not committed)
├── template.yaml           # AWS SAM
├── samconfig.toml
├── tsconfig.json
├── .env.example
├── eslint.config.js
├── package.json
└── README.md
```

## 10. Recommended development flow

```bash
# 1. Validate pipeline without Telegram
npm start

# 2. Tests (Docker)
npm test

# 3. Lint + format
npm run lint:fix
npm run format

# 4. Run the bot locally (if prod webhook is off)
npm run bot:listen

# 5. Validate the SAM template
sam validate && npm run sam:build
```

## 11. Notes

- **No database** — state in `data/cache.json` + `data/prefs.json` (local) or S3 (production). Ratings in an in-memory Map.
- The `data/` directory is created automatically on the first local run.
- Session cache expires at midnight (`America/Maceio` timezone).
- In production, daily warm is done by EventBridge → `fetchHandler`.
- Cinema preferences survive cold starts / other Lambda instances because they go to S3.
