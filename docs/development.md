# Development — Running locally

> Telegram lives in `telegram/`. WhatsApp lives in `whatsapp/`.

## Telegram

### Prerequisites

| Tool | Version / notes |
| --- | --- |
| TypeScript | ^5 (compiles to `dist/` for SAM) |
| Node.js | >= 20 (22 in test Docker / Lambda) |
| npm | >= 10 |
| Docker | **Required for `npm test`** and for `sam local invoke` |
| AWS SAM CLI | For `sam validate` / `sam build` / deploy / `sam:local` |
| AWS CLI | For deploy and `sam:warm` |

### Clone and install

```bash
git clone https://github.com/seu-usuario/maceio-cinema-bot.git
cd maceio-cinema-bot/telegram

npm install
# or, for a deterministic install:
npm ci
```

### Configure environment variables

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

> Only `TELEGRAM_BOT_TOKEN` is required for `npm run bot:listen`.
> The CLI (`npm start`) works **with no tokens**.

### Scripts

Run from `telegram/`. See [`AGENTS.md`](../AGENTS.md) for the full table (`start`, `bot:listen`, `build`, `test`, `lint`, `sam:*`).

### Validate the pipeline without Telegram

```bash
cd telegram
npm start
```

This runs `src/index.ts` (today, Cinesystem `1162`). No tokens.

### Tests (Docker)

```bash
cd telegram
npm test
```

### Run the bot (local polling)

```bash
cd telegram
npm run bot:listen
```

Health check: `http://localhost:10000/`. **Do not** use the same token while a production webhook is active.

### SAM local (optional)

```bash
cd telegram
sam validate
npm run sam:build
npm run sam:local
```

### Directory structure (Telegram)

```
telegram/
├── src/          # TypeScript bot
├── test/
├── events/
├── scripts/
├── template.yaml
├── samconfig.toml
├── docker-compose.test.yml
└── package.json
```

---

## WhatsApp

### Prerequisites

Rust **stable** 1.94+ (`rustup`). `whatsapp-rust` is pulled **without** the `simd` feature.

```bash
cd whatsapp
cp .env.example .env
cargo test
cargo run
```

Health: `http://localhost:10000/` (`PORT`). First run prints a QR — link a **dedicated** WhatsApp number (Settings → Linked devices).

Session file: `SESSION_PATH` (default `data/whatsapp.db`). Prefs are in memory.

### Docker (same image as Render)

```bash
cd whatsapp
docker build -t maceio-wa .
docker run --rm -p 10000:10000 -e PORT=10000 maceio-wa
```

### Commands to try

`start`, then `1` / `2` / `3`, then `hoje` / `proximos`.

---

## Repo layout

```
maceio-cinema-bot/
├── telegram/               # Node + SAM
├── whatsapp/               # Rust + Dockerfile
├── render.yaml
├── docs/
├── .github/workflows/ci-cd.yml
├── AGENTS.md
└── README.md
```

## Notes

- Telegram: no database — `data/cache.json` + `data/prefs.json` (local) or S3 (production).
- WhatsApp: in-process cache + in-memory prefs; unofficial client; Render Free keep-alive via `RENDER_EXTERNAL_URL`.
- Ingresso.com is public (browser-like User-Agent).
- Session cache expires at midnight (`America/Maceio`).
