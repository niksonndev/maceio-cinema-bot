# Development — Running locally

> Telegram lives in `telegram/`. WhatsApp lives in `whatsapp/`.

## Telegram

### Prerequisites

| Tool | Version / notes |
| --- | --- |
| Rust | Stable toolchain; teloxide requires Rust 1.82+ |
| Cargo Lambda | Builds Linux Lambda `bootstrap` artifacts and installs Zig |
| Rustfmt / Clippy | Installed via rustup components |
| Docker | Required for `sam local invoke` |
| AWS SAM CLI | For `sam validate` / `sam build` / deploy |
| AWS CLI | For deploy and `scripts/sam-warm.sh` |

### Clone and install

```bash
git clone https://github.com/seu-usuario/maceio-cinema-bot.git
cd maceio-cinema-bot/telegram
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

`TELEGRAM_BOT_TOKEN` is required for polling. `OMDb_API_KEY` and `TMDB_API_KEY` are optional.

### Scripts

Run Cargo commands from the repository root or pass `--manifest-path` as below.

### Rust tests

```bash
cargo fmt --manifest-path telegram/rust/Cargo.toml --check
cargo clippy --manifest-path telegram/rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path telegram/rust/Cargo.toml --locked
```

### Run the bot (local polling)

```bash
cargo run --manifest-path telegram/rust/Cargo.toml --bin telegram-poll
```

Health check: `http://localhost:10000/`. Polling refuses to start if the token has an active webhook; use a staging token.

### SAM local (optional)

```bash
cd telegram
sam validate --template-file template.yaml
sam build --template-file template.yaml
sam local start-api --template-file .aws-sam/build/template.yaml
```

### Directory structure (Telegram)

```
telegram/
├── rust/         # Rust/teloxide application and SAM Makefile
├── events/
├── scripts/
├── template.yaml
├── samconfig.toml
└── .env.example
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
├── telegram/               # Rust/teloxide + SAM
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
