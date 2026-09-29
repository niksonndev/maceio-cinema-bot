#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

sam build --template-file template.yaml

echo "=== Rust webhook Lambda smoke test (no Telegram API call) ==="
AWS_ACCESS_KEY_ID=test \
AWS_SECRET_ACCESS_KEY=test \
AWS_SESSION_TOKEN=test \
AWS_DEFAULT_REGION="${AWS_REGION:-sa-east-1}" \
AWS_EC2_METADATA_DISABLED=true \
sam local invoke BotFunction \
  --template-file .aws-sam/build/template.yaml \
  --event events/webhook-noop.json \
  --env-vars events/env.json
