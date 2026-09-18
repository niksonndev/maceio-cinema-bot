# Development — Como rodar localmente

> Guia para desenvolvedores que desejam rodar, testar e debugar o projeto localmente.

## Pré-requisitos

| Ferramenta  | Versão / notas |
| ----------- | -------------- |
| Node.js     | >= 20 (22 no Docker de testes / Lambda) |
| npm         | >= 10          |
| Docker      | **Obrigatório para `npm test`** e para `sam local invoke` |
| AWS SAM CLI | Para `sam validate` / `sam build` / deploy / `sam:local` |
| AWS CLI     | Para deploy e `sam:warm` |

## 1. Clone e instale

```bash
git clone https://github.com/seu-usuario/maceio-cinema-bot.git
cd maceio-cinema-bot

npm install
# ou, para instalação determinística:
npm ci
```

## 2. Configure variáveis de ambiente

```bash
cp .env.example .env
```

Edite `.env`:

```env
TELEGRAM_BOT_TOKEN=seu_token_aqui        # obrigatório para o bot (via @BotFather)
OMDb_API_KEY=sua_chave_omdb              # opcional (notas IMDb/RT)
TMDB_API_KEY=sua_chave_tmdb              # opcional (fallback TMDb)
# PORT = 10000                           # opcional, padrão local
# S3_BUCKET=                             # só se for testar cache S3 localmente
# CACHE_KEY=cache.json
# PREFS_KEY=prefs.json
```

> 💡 Apenas `TELEGRAM_BOT_TOKEN` é obrigatório para `npm run bot:listen`.
> A CLI (`npm start`) funciona **sem nenhum token**.

## 3. Scripts

| Script             | Comando               | Descrição                                                     |
| ------------------ | --------------------- | ------------------------------------------------------------- |
| `start`            | `node src/index.js`   | CLI — valida a pipeline (fetch + console). Sem tokens.        |
| `bot:listen`       | `node src/bot.js`     | Bot Telegram **local** (polling) + Express health check.      |
| `test`             | Docker Compose        | Vitest (Node 22 + LocalStack S3). Exige Docker.               |
| `lint`             | `eslint src/ test/`   | Lint (ESLint + Prettier).                                     |
| `lint:fix`         | `eslint src/ test/ --fix` | Lint + correção automática.                               |
| `format`           | `prettier --write src/ test/` | Formatação automática.                              |
| `format:check`     | `prettier --check src/ test/` | Verifica formatação.                                |
| `sam:build`        | `sam build`           | Empacota a aplicação SAM.                                     |
| `sam:deploy`       | `sam deploy`          | Publica o stack (exige AWS configurado).                      |
| `sam:warm`         | `scripts/sam-warm.sh` | Invoca FetchFunction (webhook + cache).                       |
| `sam:local`        | `scripts/sam-local.sh`| `sam local invoke` por evento (Docker).                       |

## 4. Como validar a pipeline sem Telegram

```bash
npm start
```

Isso executa `src/index.js`, que:
1. Faz fetch da programação de hoje (Cinesystem, teatro `1162`).
2. Normaliza os dados.
3. Imprime no console a lista de filmes e sessões.

Este é o método **recomendado** para validar que a codebase funciona — não requer
nenhum token.

## 5. Testes (Docker)

```bash
npm test
```

Sobe LocalStack (S3) + um container `node:22-bookworm` quando o Docker está
disponível (`npm run test:docker` força esse caminho). Sem Docker, `npm test`
cai para Vitest no host (prefs em arquivo temporário). Cobre `/start`, `/hoje`,
`/proximos`, `/cinemas`, `/atualizar`, callbacks, persistência de prefs e o
await do handler Lambda.

Não usa a API do Ingresso.com nem o Telegram de verdade (bot e data layer são mockados,
exceto o round-trip S3 de prefs).

Smoke extra no runtime Lambda (não valida entrega no Telegram):

```bash
npm run sam:local
```

## 6. Como rodar o bot (polling local)

```bash
npm run bot:listen
```

- O bot inicia em **polling mode** (sem webhook).
- Health check disponível em `http://localhost:10000/`.
- No Telegram, envie `/start` e escolha um cinema.
- **Não** use o mesmo token com webhook de produção ativo.

### Debug: verificar conexão

```bash
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMyCommands"
# Polling local: url deve estar vazia
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
curl -s http://localhost:10000/
```

## 7. SAM local (opcional)

Requer SAM CLI + Docker:

```bash
sam validate
sam build
npm run sam:local
sam local start-api
```

## 8. Lint e formatação

```bash
npm run lint
npm run lint:fix
npm run format
npm run format:check
```

## 9. Estrutura de diretórios

```
maceio-cinema-bot/
├── src/
│   ├── api.js
│   ├── normalize.js
│   ├── cache.js            # arquivo local ou S3
│   ├── data.js
│   ├── cinemas.js          # lista de cinemas + prefs persistidas
│   ├── format.js
│   ├── ratings.js
│   ├── keyboards.js
│   ├── handlers.js         # handleUpdate (polling e webhook)
│   ├── bot.js              # polling local
│   ├── lambda.js           # webhook + fetchHandler (produção)
│   └── index.js
├── test/                   # Vitest (rodado via Docker Compose)
├── events/
│   ├── webhook-event.json
│   ├── env.json            # env dummy para sam local
│   ├── commands/
│   └── callbacks/
├── scripts/
│   ├── sam-warm.sh
│   └── sam-local.sh
├── docker-compose.test.yml
├── docs/
├── data/                   # runtime local (não commitado)
├── template.yaml           # AWS SAM
├── samconfig.toml
├── .env.example
├── eslint.config.js
├── package.json
└── README.md
```

## 10. Fluxo de desenvolvimento recomendado

```bash
# 1. Valida pipeline sem Telegram
npm start

# 2. Testes (Docker)
npm test

# 3. Lint + format
npm run lint:fix
npm run format

# 4. Roda o bot localmente (se o webhook de prod estiver off)
npm run bot:listen

# 5. Valida o template SAM
sam validate && sam build
```

## 11. Observações

- **Nenhum banco de dados** — estado em `data/cache.json` + `data/prefs.json` (local) ou S3 (produção). Ratings em Map em memória.
- O diretório `data/` é criado automaticamente na primeira execução local.
- O cache de sessões expira na virada do dia (fuso `America/Maceio`).
- Em produção o warm diário é feito pelo EventBridge → `fetchHandler`.
- Preferências de cinema sobrevivem a cold start / outra instância Lambda porque vão para S3.
