# Deploy

> Complete guide for deploying and operating the bot. The primary method is **AWS SAM**
> (Lambda + API Gateway + EventBridge + S3). Render deploy is available
> as historical reference (legacy).

## Deploy via AWS SAM (primary)

### Overview

| Item | Detail |
|---|---|
| IaC | AWS SAM (`template.yaml`) |
| Runtime | AWS Lambda (Node.js 22.x) |
| HTTP | Amazon API Gateway — HTTP API (`POST /webhook`) |
| Schedule | Amazon EventBridge (`cron(0 3 * * ? *)` — midnight Maceió) |
| Cache | Amazon S3 (`cache.json` + `prefs.json`) |
| Handlers | `dist/lambda.handler`, `dist/lambda.fetchHandler` |
| npm scripts | `sam:build` (`tsc` + `sam build`), `sam:deploy`, `sam:warm`, `sam:local` |

### Prerequisites

- AWS CLI configured (`aws configure` → `aws sts get-caller-identity`)
- [SAM CLI](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/install-sam_cli.html) installed
- IAM with permission to create Lambda, API Gateway, S3, EventBridge, IAM roles (`CAPABILITY_IAM`)
- `TELEGRAM_BOT_TOKEN` (and optionally OMDb/TMDb)

### 1. Build

```bash
sam validate
npm run sam:build   # tsc → dist/ then sam build
```

### 2. Deploy (first time — guided mode)

```bash
sam deploy --guided   # or: npm run sam:deploy (without --guided)
```

| Prompt | Suggested |
|---|---|
| `Stack name` | `maceio-cine-bot` |
| `AWS Region` | e.g. `us-east-1` or `sa-east-1` |
| `TelegramBotToken` | bot token (via @BotFather) |
| `OMDbApiKey` | optional |
| `TMDbApiKey` | optional |

The `WebhookUrl` output and the **FetchFunction** `WEBHOOK_URL` env use
`${HttpApi.ApiEndpoint}/prod/webhook` (the `prod` stage is part of the path).
We do not put `WEBHOOK_URL` on BotFunction: referencing `HttpApi` on the same
function that integrates with it creates a circular dependency in CloudFormation.

FetchFunction calls `bot.setWebHook()` (daily cron or `npm run sam:warm`).
The URL only changes if the stack (or `HttpApi`) is recreated.

### 3. Register / verify the webhook on Telegram

After deploy, invoke FetchFunction once (cache warm + `setWebHook`):

```bash
set -a && source .env && set +a
npm run sam:warm

curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
```

The `url` should be the `WebhookUrl` output (ends in `/prod/webhook`), with no
`last_error_message`. Manual alternative:

```bash
WEBHOOK=$(aws cloudformation describe-stacks \
  --stack-name maceio-cine-bot \
  --region sa-east-1 \
  --query "Stacks[0].Outputs[?OutputKey=='WebhookUrl'].OutputValue" \
  --output text)

curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/setWebhook" \
  -F "url=${WEBHOOK}"
```

### 4. Cutover (leave Render / polling)

Order matters:

1. Successful `sam deploy`
2. `npm run sam:warm` (warm + `setWebHook`) and confirm `getWebhookInfo`
3. Smoke test on Telegram (`/start`, pick cinema, `/hoje`, `/proximos`)
4. Confirm S3 objects (`CacheBucketName`: `cache.json` and `prefs.json` after use)
5. **Only then** suspend/delete the Render Web Service and stop any `bot:listen` with the same token

### 5. Subsequent deploys

```bash
npm run sam:build && sam deploy
```

### 6. Local SAM development / testing

Requires Docker (SAM Lambda images):

```bash
npm test              # Vitest in Docker Compose + LocalStack
npm run sam:local     # sam build + invoke per event in events/
sam local start-api
```

### Configuration (`samconfig.toml`)

```toml
version = 0.1
[default.deploy.parameters]
stack_name = "maceio-cine-bot"
capabilities = "CAPABILITY_IAM"
resolve_s3 = true
confirm_changeset = true
region = "sa-east-1"
```

No need to pass `WebhookUrl` in `parameter_overrides` — the URL comes from
`HttpApi` in `template.yaml`.

### CI/CD (GitHub Actions)

The [`.github/workflows/ci-cd.yml`](../.github/workflows/ci-cd.yml) workflow runs on:

- **Pull request to `main`:** lint, typecheck, and tests (Docker + LocalStack). No deploy.
- **Push to `main`** (including PR merge): the same checks, then `sam deploy` and `sam:warm`.

AWS auth is via **OIDC** (`aws-actions/configure-aws-credentials`), with no long-lived access keys.

#### Repository secrets and variables

| Name | Where | Use |
| --- | --- | --- |
| `AWS_ROLE_ARN` | Variable or secret | ARN of the IAM role assumed by the workflow (`sa-east-1`) |
| `TELEGRAM_BOT_TOKEN` | Secret | SAM parameter `TelegramBotToken` |
| `OMDb_API_KEY` | Secret (optional) | SAM parameter `OMDbApiKey` |
| `TMDB_API_KEY` | Secret (optional) | SAM parameter `TMDbApiKey` |

The role must trust the OIDC provider `token.actions.githubusercontent.com`, restricted to this repository and `ref:refs/heads/main`, with permissions equivalent to guided deploy (CloudFormation, SAM S3, Lambda, API Gateway, EventBridge, IAM).

Role trust policy sketch:

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

Replace `<ACCOUNT_ID>`, `<OWNER>`, and `<REPO>`. You also need to create the AWS OIDC identity provider for GitHub if it does not already exist.

Manual deploys (`npm run sam:deploy`) remain valid; `confirm_changeset` in `samconfig.toml` applies only to the local CLI. The workflow passes `--no-confirm-changeset --no-fail-on-empty-changeset`.

### `template.yaml` resources

- `CacheBucket` — S3 (name generated by CloudFormation)
- `HttpApi` — HTTP API (`StageName: prod`) with `POST /webhook` route
- `BotFunction` — webhook + `S3CrudPolicy` (no `WEBHOOK_URL` — avoids CFN cycle); `PREFS_KEY`
- `FetchFunction` — daily cron + `WEBHOOK_URL` (`…/prod/webhook`) + `S3CrudPolicy`
- Outputs: `WebhookUrl` (`…/prod/webhook`), `FetchFunctionArn`, `CacheBucketName`

### Verification via curl

```bash
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMyCommands"
# Production: url = stack WebhookUrl
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
```

---

## Deploy on Render (legacy)

> Archived. Prefer AWS SAM above.

| Item | Detail |
| --- | --- |
| Platform | Render Web Service |
| Start | `npm run bot:listen` |
| Cache | `data/cache.json` (ephemeral in the container) |
| Keep-alive | Auto-ping via `RENDER_EXTERNAL_URL` (removed from current code) |

Useful only as historical reference while the `cinesystem-scrapper.onrender.com`
service still exists. After cutover, it can be shut down.
