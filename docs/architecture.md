# Architecture

> Technical architecture documentation for **Maceió Cine Bot**.

## Overview

TypeScript application (Node.js, ES Modules) that queries the public Ingresso.com API
and exposes Maceió cinema schedules via a Telegram bot. No database —
uses a **JSON cache** (`data/cache.json` locally, or **S3** when `S3_BUCKET` is
set) and **persisted preferences** (`data/prefs.json` / S3 `prefs.json`).
Ratings stay in an in-memory Map (24h TTL).

### Production (AWS SAM)

```
Telegram ──POST /webhook──► API Gateway HTTP API ──► Lambda (dist/lambda.handler)
                                                         │
                                                         ├─► cache.ts + prefs (S3)
                                                         └─► Ingresso.com API

EventBridge (cron 03:00 UTC) ──► Lambda (dist/lambda.fetchHandler)
                                      ├─► fetch + write cache to S3
                                      └─► setWebHook(WEBHOOK_URL)
```

BotFunction does **not** set `WEBHOOK_URL` (that would create a CloudFormation
cycle with HttpApi). FetchFunction registers the webhook with Telegram.

Each BotFunction invoke reloads cache and prefs from S3 and **awaits**
`handleUpdate()` before returning HTTP 200.

### Local / development (polling)

```
npm run bot:listen → src/bot.ts (polling + Express health check)
                         └─► cache.ts / cinemas.ts → data/cache.json + data/prefs.json
```

Do not run local polling with the same token while the production webhook is active.

## Modules (`src/`)

| Module          | Responsibility                                                                                  |
| --------------- | ----------------------------------------------------------------------------------------------- |
| `api.ts`        | HTTP client (Axios) for the public Ingresso.com API. Fetches sessions and releases by `theaterId`. |
| `normalize.ts`  | Separates **static** movie data from **dynamic** session data. Includes `denormalize()`.      |
| `cache.ts`      | JSON persistence (file or S3): movies, sessions by theater/date, upcoming releases.             |
| `data.ts`       | Orchestrates `cache ↔ api ↔ normalize` with cache-hit logic before calling the API.             |
| `cinemas.ts`    | Definition of the 3 cinemas and per-user preferences (local file or S3).                        |
| `format.ts`     | Markdown message formatting for Telegram (cards, prices, dates).                                |
| `ratings.ts`    | Fetches ratings (IMDb/RT via OMDb, TMDb fallback) with in-memory cache (24h TTL).               |
| `keyboards.ts`  | Telegram inline keyboard builders.                                                              |
| `handlers.ts`   | `handleUpdate` + commands (`/start`, `/hoje`, `/proximos`, `/cinemas`, `/atualizar`) and callbacks. |
| `bot.ts`        | Local entry: Telegram polling + Express health check + graceful shutdown.                       |
| `lambda.ts`     | Production entry: webhook (`handler`) + daily warm (`fetchHandler`).                            |
| `index.ts`      | CLI for manual verification (fetch + console). No token required.                               |
| `types.ts`      | Shared domain types.                                                                            |

## Data flow

### 1. Today's movies (`/hoje`, `filmes_hoje`)

```
handlers.ts
  └─ getMoviesForDate(cache, date, theaterId)        ← src/data.ts
       ├─ cache.getSessions(date, theaterId) → HIT? return from cache
       └─ MISS → api.fetchNormalized(date, theaterId)
                    └─ normalize.normalizeSessionsResponse(raw)
                         ├─ mergeMovies() → cache.movies (static)
                         ├─ setSessions()  → cache.sessions (dynamic)
                         └─ denormalize(movies, sessions) → movies + sessions array
                        └─ format.formatSingleMovieCard() → Telegram message
```

### 2. Upcoming releases (`/proximos`, `proximos_lancamentos`)

```
handlers.ts
  └─ getUpcomingMovies(cache, theaterId)             ← src/data.ts
       ├─ cache.getUpcoming(theaterId) → HIT? return from cache
       └─ MISS → api.fetchUpcoming(theaterId)
                    └─ normalize.normalizeUpcomingFromSessions(futureDates, todayIds)
                         └─ setUpcoming() → cache.upcoming
                        └─ format.formatSingleUpcomingCard() → Telegram message
```

## Supported theaters

| `theaterId` | Cinema     | Shopping                    |
| ----------- | ---------- | --------------------------- |
| `1162`      | Cinesystem | Parque Shopping Maceió      |
| `1230`      | Centerplex | Shopping Pátio Maceió       |
| `924`       | Kinoplex   | Maceió Shopping             |

City ID in the API: `53` (Maceió).

## Entry points (`package.json`)

| Script              | Command              | Purpose                                             |
| ------------------- | -------------------- | --------------------------------------------------- |
| `npm start`         | `tsx src/index.ts`   | CLI — validates the pipeline (fetch + console). No token. |
| `npm run bot:listen`| `tsx src/bot.ts`     | Local bot (polling) + Express health check. Requires `TELEGRAM_BOT_TOKEN`. |
| `npm test`          | Docker Compose       | Vitest suites (Node 22 + LocalStack S3). Requires Docker. |
| `npm run typecheck` | `tsc --noEmit`       | TypeScript check.                                   |
| `npm run build`     | `tsc -p tsconfig.json` | Compiles `src/` → `dist/`.                        |
| `npm run sam:build` | `npm run build && sam build` | Packages the SAM app.                        |
| `npm run sam:deploy`| `sam deploy`         | Deploys the stack (requires AWS credentials).       |
| `npm run sam:warm`  | `scripts/sam-warm.sh`| Invokes FetchFunction (webhook + cache warm).       |
| `npm run sam:local` | `scripts/sam-local.sh` | `sam local invoke` per event (Docker).            |
