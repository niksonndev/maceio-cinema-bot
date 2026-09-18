# Deploy

> Guia completo para deploy e operação do bot. O método primário é **AWS SAM**
> (Lambda + API Gateway + EventBridge + S3). O deploy no Render está disponível
> como referência histórica (legado).

## Deploy via AWS SAM (primário)

### Visão geral

| Item | Detalhe |
|---|---|
| IaC | AWS SAM (`template.yaml`) |
| Runtime | AWS Lambda (Node.js 22.x) |
| HTTP | Amazon API Gateway — HTTP API (`POST /webhook`) |
| Agendamento | Amazon EventBridge (`cron(0 3 * * ? *)` — meia-noite Maceió) |
| Cache | Amazon S3 (`cache.json` + `prefs.json`) |
| Handlers | `src/lambda.handler`, `src/lambda.fetchHandler` |
| Scripts npm | `sam:build`, `sam:deploy`, `sam:warm`, `sam:local` |

### Pré-requisitos

- AWS CLI configurado (`aws configure` → `aws sts get-caller-identity`)
- [SAM CLI](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/install-sam_cli.html) instalado
- IAM com permissão para criar Lambda, API Gateway, S3, EventBridge, IAM roles (`CAPABILITY_IAM`)
- `TELEGRAM_BOT_TOKEN` (e opcionalmente OMDb/TMDb)

### 1. Build

```bash
sam validate
sam build          # ou: npm run sam:build
```

### 2. Deploy (primeira vez — modo guiado)

```bash
sam deploy --guided   # ou: npm run sam:deploy (sem o --guided)
```

| Prompt | Sugerido |
|---|---|
| `Stack name` | `maceio-cine-bot` |
| `AWS Region` | ex.: `us-east-1` ou `sa-east-1` |
| `TelegramBotToken` | token do bot (via @BotFather) |
| `OMDbApiKey` | opcional |
| `TMDbApiKey` | opcional |

O output `WebhookUrl` e a env `WEBHOOK_URL` da **FetchFunction** usam
`${HttpApi.ApiEndpoint}/prod/webhook` (o stage `prod` faz parte do path).
Não colocamos `WEBHOOK_URL` na BotFunction: referenciar o `HttpApi` na mesma
função que integra com ele gera dependência circular no CloudFormation.

A FetchFunction chama `bot.setWebHook()` (cron diário ou `npm run sam:warm`).
A URL só muda se o stack (ou o `HttpApi`) for recriado.

### 3. Registrar / verificar o webhook no Telegram

Após o deploy, invoque a FetchFunction uma vez (cache warm + `setWebHook`):

```bash
set -a && source .env && set +a
npm run sam:warm

curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
```

O `url` deve ser o output `WebhookUrl` (termina em `/prod/webhook`), sem
`last_error_message`. Alternativa manual:

```bash
WEBHOOK=$(aws cloudformation describe-stacks \
  --stack-name maceio-cine-bot \
  --region sa-east-1 \
  --query "Stacks[0].Outputs[?OutputKey=='WebhookUrl'].OutputValue" \
  --output text)

curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/setWebhook" \
  -F "url=${WEBHOOK}"
```

### 4. Cutover (sair do Render / polling)

Ordem importa:

1. `sam deploy` com sucesso
2. `npm run sam:warm` (warm + `setWebHook`) e confirmar `getWebhookInfo`
3. Smoke test no Telegram (`/start`, escolher cinema, `/hoje`, `/proximos`)
4. Confirmar objetos no S3 (`CacheBucketName`: `cache.json` e `prefs.json` após uso)
5. **Só então** suspender/apagar o Web Service no Render e parar qualquer `bot:listen` com o mesmo token

### 5. Deploys subsequentes

```bash
sam build && sam deploy
```

### 6. Desenvolvimento / teste local do SAM

Requer Docker (imagens Lambda do SAM):

```bash
npm test              # Vitest em Docker Compose + LocalStack
npm run sam:local     # sam build + invoke por evento em events/
sam local start-api
```

### Configuração (`samconfig.toml`)

```toml
version = 0.1
[default.deploy.parameters]
stack_name = "maceio-cine-bot"
capabilities = "CAPABILITY_IAM"
resolve_s3 = true
confirm_changeset = true
region = "sa-east-1"
```

Não é preciso passar `WebhookUrl` em `parameter_overrides` — a URL vem do
`HttpApi` no `template.yaml`.

### Recursos do `template.yaml`

- `CacheBucket` — S3 (nome gerado pela CloudFormation)
- `HttpApi` — HTTP API (`StageName: prod`) com rota `POST /webhook`
- `BotFunction` — webhook + `S3CrudPolicy` (sem `WEBHOOK_URL` — evita ciclo CFN); `PREFS_KEY`
- `FetchFunction` — cron diário + `WEBHOOK_URL` (`…/prod/webhook`) + `S3CrudPolicy`
- Outputs: `WebhookUrl` (`…/prod/webhook`), `FetchFunctionArn`, `CacheBucketName`

### Verificação via curl

```bash
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMyCommands"
# Produção: url = WebhookUrl do stack
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
```

---

## Deploy no Render (legado)

> Arquivado. Preferir AWS SAM acima.

| Item | Detalhe |
| --- | --- |
| Plataforma | Render Web Service |
| Start | `npm run bot:listen` |
| Cache | `data/cache.json` (efêmero no container) |
| Keep-alive | Auto-ping via `RENDER_EXTERNAL_URL` (removido do código atual) |

Útil apenas como referência histórica enquanto o serviço `cinesystem-scrapper.onrender.com`
ainda existir. Após o cutover, pode ser desligado.
