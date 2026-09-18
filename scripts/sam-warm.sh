#!/usr/bin/env bash
set -euo pipefail

REGION="${AWS_REGION:-sa-east-1}"
STACK="${STACK_NAME:-maceio-cine-bot}"
OUT="${TMPDIR:-/tmp}/maceio-fetch-out.json"

FETCH_ARN="$(aws cloudformation describe-stacks \
  --stack-name "$STACK" \
  --region "$REGION" \
  --query "Stacks[0].Outputs[?OutputKey=='FetchFunctionArn'].OutputValue" \
  --output text)"

if [[ -z "$FETCH_ARN" || "$FETCH_ARN" == "None" ]]; then
  echo "Could not resolve FetchFunctionArn from stack $STACK ($REGION)" >&2
  exit 1
fi

aws lambda invoke --region "$REGION" --function-name "$FETCH_ARN" "$OUT"
cat "$OUT"
echo
