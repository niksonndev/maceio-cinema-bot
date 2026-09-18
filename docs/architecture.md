# Arquitetura

> Documentação técnica da arquitetura do **Maceió Cine Bot**.

## Visão geral

Aplicação Node.js (ES Modules) que consulta a API pública do Ingresso.com e
expõe a programação de cinemas de Maceió via bot do Telegram. Sem banco de dados —
usa **cache JSON** (`data/cache.json` localmente, ou **S3** quando `S3_BUCKET` está
definido) e **preferências persistidas** (`data/prefs.json` / S3 `prefs.json`).
Ratings continuam em Map em memória (TTL 24h).

### Produção (AWS SAM)

```
Telegram ──POST /webhook──► API Gateway HTTP API ──► Lambda (src/lambda.handler)
                                                         │
                                                         ├─► cache.js + prefs (S3)
                                                         └─► Ingresso.com API

EventBridge (cron 03:00 UTC) ──► Lambda (src/lambda.fetchHandler)
                                      ├─► fetch + grava cache no S3
                                      └─► setWebHook(WEBHOOK_URL)
```

A BotFunction **não** define `WEBHOOK_URL` (isso criaria ciclo no CloudFormation
com o HttpApi). A FetchFunction registra o webhook no Telegram.

Cada invoke da BotFunction recarrega cache e prefs do S3 e **aguarda**
`handleUpdate()` antes de devolver HTTP 200.

### Local / desenvolvimento (polling)

```
npm run bot:listen → src/bot.js (polling + Express health check)
                         └─► cache.js / cinemas.js → data/cache.json + data/prefs.json
```

Não rode polling local com o mesmo token enquanto o webhook de produção estiver ativo.

## Módulos (`src/`)

| Módulo          | Responsabilidade                                                                                |
| --------------- | ----------------------------------------------------------------------------------------------- |
| `api.js`        | Cliente HTTP (Axios) para a API pública do Ingresso.com. Busca sessões e lançamentos por `theaterId`. |
| `normalize.js`  | Separa dados **estáticos** de filmes dos **dinâmicos** de sessões. Contém `denormalize()`.      |
| `cache.js`      | Persistência JSON (arquivo ou S3): filmes, sessões por teatro/data, lançamentos.                |
| `data.js`       | Orquestra `cache ↔ api ↔ normalize` com lógica de cache hit antes de chamar a API.               |
| `cinemas.js`    | Definição dos 3 cinemas e preferências por usuário (arquivo local ou S3).                       |
| `format.js`     | Formatação de mensagens Markdown para Telegram (cards, preços, datas).                          |
| `ratings.js`    | Busca notas (IMDb/RT via OMDb, fallback TMDb) com cache em memória (TTL 24h).                   |
| `keyboards.js`  | Builders de teclados inline do Telegram.                                                        |
| `handlers.js`   | `handleUpdate` + comandos (`/start`, `/hoje`, `/proximos`, `/cinemas`, `/atualizar`) e callbacks. |
| `bot.js`        | Entry local: Telegram polling + Express health check + graceful shutdown.                       |
| `lambda.js`     | Entry produção: webhook (`handler`) + warm diário (`fetchHandler`).                             |
| `index.js`      | CLI para verificação manual (fetch + console). Sem token.                                       |

## Fluxo de dados

### 1. Filmes de hoje (`/hoje`, `filmes_hoje`)

```
handlers.js
  └─ getMoviesForDate(cache, date, theaterId)        ← src/data.js
       ├─ cache.getSessions(date, theaterId) → HIT? devolve do cache
       └─ MISS → api.fetchNormalized(date, theaterId)
                    └─ normalize.normalizeSessionsResponse(raw)
                         ├─ mergeMovies() → cache.movies (estático)
                         ├─ setSessions()  → cache.sessions (dinâmico)
                         └─ denormalize(movies, sessions) → array de filmes + sessões
                        └─ format.formatSingleMovieCard() → mensagem Telegram
```

### 2. Próximos lançamentos (`/proximos`, `proximos_lancamentos`)

```
handlers.js
  └─ getUpcomingMovies(cache, theaterId)             ← src/data.js
       ├─ cache.getUpcoming(theaterId) → HIT? devolve do cache
       └─ MISS → api.fetchUpcoming(theaterId)
                    └─ normalize.normalizeUpcomingFromSessions(futureDates, todayIds)
                         └─ setUpcoming() → cache.upcoming
                        └─ format.formatSingleUpcomingCard() → mensagem Telegram
```

## Teatros suportados

| `theaterId` | Cinema     | Shopping                    |
| ----------- | ---------- | --------------------------- |
| `1162`      | Cinesystem | Parque Shopping Maceió      |
| `1230`      | Centerplex | Shopping Pátio Maceió       |
| `924`       | Kinoplex   | Maceió Shopping             |

ID da cidade na API: `53` (Maceió).

## Entry points (`package.json`)

| Script              | Comando              | Propósito                                           |
| ------------------- | -------------------- | --------------------------------------------------- |
| `npm start`         | `node src/index.js`  | CLI — valida a pipeline (fetch + console). Sem token. |
| `npm run bot:listen`| `node src/bot.js`    | Bot local (polling) + Express health check. Exige `TELEGRAM_BOT_TOKEN`. |
| `npm test`          | Docker Compose       | Suites Vitest (Node 22 + LocalStack S3). Exige Docker. |
| `npm run sam:build` | `sam build`          | Empacota a app SAM.                                 |
| `npm run sam:deploy`| `sam deploy`         | Publica o stack (exige credenciais AWS).            |
| `npm run sam:warm`  | `scripts/sam-warm.sh`| Invoca FetchFunction (webhook + cache warm).        |
| `npm run sam:local` | `scripts/sam-local.sh` | `sam local invoke` por evento (Docker).           |
