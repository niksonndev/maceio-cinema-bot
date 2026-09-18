# 🎬 Maceió Cine Bot

> Bot de Telegram que consulta a programação dos cinemas de Maceió em tempo real, com horários, preços e link direto para compra de ingressos.

[Testar no Telegram](https://t.me/MaceioCine_bot)

---

## 🟢 Status do Deploy

Produção alvo: **AWS SAM** (Lambda + API Gateway webhook + EventBridge + S3).
Polling local (`npm run bot:listen`) é só para desenvolvimento.

| Item | Detalhe |
| --- | --- |
| ☁️ **IaC** | [`template.yaml`](template.yaml) (AWS SAM) |
| 🔌 **Ingresso** | Telegram webhook → API Gateway `POST /webhook` → `dist/lambda.handler` |
| 💾 **Cache** | S3 (`cache.json` + `prefs.json`) em produção; `data/*.json` em local |
| ⏰ **Warm** | EventBridge cron diário → `dist/lambda.fetchHandler` |
| 📋 **Guia** | [`docs/deployment.md`](docs/deployment.md) |

---

## ✨ Funcionalidades

🎥 **Filmes de Hoje** — Programação completa com horários, formatos (2D, 3D, Cinépic, VIP) e preços

🏢 **Múltiplos Cinemas** — Suporte a 3 redes de cinema em Maceió:

| Cinema | Shopping |
| --- | --- |
| Cinesystem | Parque Shopping Maceió |
| Centerplex | Shopping Pátio Maceió |
| Kinoplex | Maceió Shopping |

🆕 **Próximos Lançamentos** — Filmes futuros com datas de estreia e pré-vendas

🎫 **Compra Direta** — Botão inline que redireciona para a página do cinema no Ingresso.com

⚡ **Cache Inteligente** — Cache diário por cinema com expiração automática à meia-noite (fuso `America/Maceio`), evitando requisições desnecessárias

🔄 **Normalização de Dados** — Preços, horários e tipos de sala normalizados a partir da API

---

## 🤖 Preview

Teste agora mesmo no Telegram: **[@@MaceioCine_bot](https://t.me/MaceioCine_bot)**

```
Usuário: /start
Bot:     Olá! Eu sou o seu guia de cinema em Maceió. 🍿
         Escolha abaixo qual cinema você deseja consultar:

         [ Cinesystem (Parque Shopping Maceió) ]
         [ Centerplex (Shopping Pátio Maceió)  ]
         [ Kinoplex (Maceió Shopping)           ]

Usuário: clica em "Cinesystem"
Bot:     ✅ Cinema selecionado: Cinesystem (Parque Shopping Maceió)

         [ 🎬 Filmes de Hoje ] [ 🆕 Próximos Lançamentos ]
         [ 🔄 Trocar de Cinema                            ]

Usuário: clica em "Filmes de Hoje"
Bot:     🎬 PROGRAMAÇÃO
         📍 Cinesystem (Parque Shopping Maceió)
         📅 24 de fevereiro de 2026

         🎭 Avatar: Fogo E Cinzas
            🎞 2D: 14:30, 17:45, 20:45 — R$ 55,86
            ⭐ VIP: 21:00 — R$ 72,00
         ...

         [ 🎫 Comprar Ingressos           ]
         [ ⬅️ Voltar ao menu ] [ 🔄 Trocar cinema ]
```

---

## 📋 Comandos

| Comando | Descrição |
| --- | --- |
| `/start` | Iniciar o bot e escolher cinema |
| `/hoje` | Filmes em cartaz no cinema selecionado |
| `/proximos` | Lançamentos futuros e pré-vendas |
| `/cinemas` | Trocar de cinema selecionado |

---

## 🛠️ Tecnologias

| Tecnologia | Uso |
| --- | --- |
| **Node.js 22** | Runtime (local, testes, Lambda) |
| **TypeScript** | Código-fonte em `src/`; emit para `dist/` no SAM |
| **Telegram Bot API** | Bot via [node-telegram-bot-api](https://github.com/yagop/node-telegram-bot-api) (modo polling) |
| **Axios** | Requisições HTTP para a API do Ingresso.com |
| **Express** | Servidor HTTP para health check (porta dinâmica) |
| **dotenv** | Gerenciamento de variáveis de ambiente |

---

## 🚀 Como Rodar

### Rodar localmente

**1. Clone o repositório**

```bash
git clone https://github.com/seu-usuario/maceio-cinema-bot.git
cd maceio-cinema-bot
```

**2. Instale as dependências**

```bash
npm install
```

**3. Configure as variáveis de ambiente**

```bash
cp .env.example .env
```

Edite o `.env` e defina o token do bot (obtido via [@BotFather](https://t.me/BotFather)):

```env
TELEGRAM_BOT_TOKEN=seu_token_aqui
```

Opcionalmente, defina `PORT` (padrão local: `10000`).

**4. Inicie o bot** (polling local — não use o mesmo token da produção)

```bash
npm run bot:listen
```

O bot ficará escutando comandos no Telegram. O health check estará em `http://localhost:10000/` (ou na porta definida em `PORT`).

**5. Testes** (exige Docker)

```bash
npm test
```

---

## 📂 Arquitetura

O projeto é uma aplicação TypeScript (Node.js) com módulos em `src/`. Veja a
[documentação completa de arquitetura](docs/architecture.md) para detalhes sobre
fluxos de dados e responsabilidades.

| Módulo          | Responsabilidade curta                                           |
| --------------- | ---------------------------------------------------------------- |
| `api.ts`        | Cliente HTTP (Axios) para a API pública do Ingresso.com          |
| `normalize.ts`  | Separa dados estáticos de filmes dos dinâmicos de sessões        |
| `cache.ts`      | Cache JSON (arquivo local ou S3) com expiração diária por cinema |
| `data.ts`       | Orquestra cache ↔ API ↔ normalize (cache hit antes da API)        |
| `cinemas.ts`    | Definição dos 3 cinemas + preferências por usuário                |
| `format.ts`     | Formatação de mensagens Telegram (Markdown)                       |
| `ratings.ts`    | Notas IMDb/RT (OMDb) + fallback TMDb, cache 24h                   |
| `keyboards.ts`  | Builders de teclados inline do Telegram                           |
| `handlers.ts`   | Handlers de comandos e callbacks                                 |
| `bot.ts`        | Entry local (polling + Express + graceful shutdown)               |
| `lambda.ts`     | Entry produção (webhook Lambda + fetch/cache warm)                |
| `index.ts`      | CLI para verificação rápida via terminal                          |
| `types.ts`      | Tipos de domínio (cache, sessões, Telegram, BotLike)              |

Veja também os documentos técnicos na pasta [`docs/`](./docs):

- [`docs/architecture.md`](docs/architecture.md) — fluxos de dados e módulos
- [`docs/data-model.md`](docs/data-model.md) — estrutura completa de `cache.json`
- [`docs/caching.md`](docs/caching.md) — regras de expiração e TTL
- [`docs/api-reference.md`](docs/api-reference.md) — endpoints e contratos
- [`docs/deployment.md`](docs/deployment.md) — deploy AWS SAM (Render legado)
- [`docs/development.md`](docs/development.md) — como rodar localmente

---

## 🔧 Variáveis de Ambiente

| Variável | Obrigatória | Padrão | Descrição |
| --- | --- | --- | --- |
| `TELEGRAM_BOT_TOKEN` | Sim | — | Token do bot obtido via @BotFather |
| `PORT` | Não | `10000` | Porta do Express (health check) no modo polling local |
| `OMDb_API_KEY` | Não | — | Chave da [OMDb](https://www.omdbapi.com/apikey.aspx) para IMDb/RT |
| `TMDB_API_KEY` | Não | — | Fallback [TMDb](https://www.themoviedb.org/settings/api) |
| `S3_BUCKET` | Não* | — | Bucket do cache e prefs (*injetado pelo SAM em produção) |
| `CACHE_KEY` | Não | `cache.json` | Chave do objeto de cache no S3 |
| `PREFS_KEY` | Não | `prefs.json` | Chave do objeto de preferências no S3 |
| `WEBHOOK_URL` | Não* | — | URL do webhook (*SAM na FetchFunction: `…/prod/webhook`); `setWebHook` no fetch |

---

## ☁️ Deploy (AWS SAM — Lambda + API Gateway)

Deploy via **SAM**: Lambda + API Gateway (webhook) + EventBridge (cache warm) + S3.
Preferências de cinema persistem em S3 (`prefs.json`).

Veja o [guia completo](docs/deployment.md).

```bash
npm run sam:build
sam deploy            # primeira vez (modo guiado) ou subsequente
```

---

## 📄 Licença

MIT
