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
| Handlers | `dist/lambda.handler`, `dist/lambda.fetchHandler` |
| Scripts npm | `sam:build` (`tsc` + `sam build`), `sam:deploy`, `sam:warm`, `sam:local` |

### Pré-requisitos

- AWS CLI configurado (`aws configure` → `aws sts get-caller-identity`)
- [SAM CLI](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/install-sam_cli.html) instalado
- IAM com permissão para criar Lambda, API Gateway, S3, EventBridge, IAM roles (`CAPABILITY_IAM`)
- `TELEGRAM_BOT_TOKEN` (e opcionalmente OMDb/TMDb)

### 1. Build

```bash
sam validate
npm run sam:build   # tsc → dist/ então sam build
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
npm run sam:build && sam deploy
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

### CI/CD (GitHub Actions)

O workflow [`.github/workflows/ci-cd.yml`](../.github/workflows/ci-cd.yml) roda em:

- **Pull request para `main`:** lint, typecheck e testes (Docker + LocalStack). Sem deploy.
- **Push para `main`** (inclui merge de PR): os mesmos checks, depois `sam deploy` e `sam:warm`.

Autenticação AWS é via **OIDC** (`aws-actions/configure-aws-credentials`), sem access keys de longa duração.

#### Secrets e variáveis no repositório

| Nome | Onde | Uso |
| --- | --- | --- |
| `AWS_ROLE_ARN` | Variable ou secret | ARN da role IAM assumida pelo workflow (`sa-east-1`) |
| `TELEGRAM_BOT_TOKEN` | Secret | Parâmetro SAM `TelegramBotToken` |
| `OMDb_API_KEY` | Secret (opcional) | Parâmetro SAM `OMDbApiKey` |
| `TMDB_API_KEY` | Secret (opcional) | Parâmetro SAM `TMDbApiKey` |

A role deve confiar no provedor OIDC `token.actions.githubusercontent.com`, restrita a este repositório e a `ref:refs/heads/main`, com permissões equivalentes ao deploy guiado (CloudFormation, SAM S3, Lambda, API Gateway, EventBridge, IAM).

Esboço de trust policy da role:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": { "Federated": "arn:aws:iam::<ACCOUNT_ID>:oidc-provider/token.actions.githubusercontent.com" },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:<OWNER>/<REPO>:ref:refs/heads/main"
        }
      }
    }
  ]
}
```

Substitua `<ACCOUNT_ID>`, `<OWNER>` e `<REPO>`. Também é preciso criar o identity provider OIDC da AWS para GitHub se ainda não existir.

Deploys manuais (`npm run sam:deploy`) continuam válidos; o `confirm_changeset` do `samconfig.toml` aplica só ao CLI local. O workflow passa `--no-confirm-changeset --no-fail-on-empty-changeset`.

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
