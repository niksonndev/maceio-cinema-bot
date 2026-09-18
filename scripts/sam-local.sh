#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

npm run sam:build

shopt -s nullglob
files=(events/webhook-event.json events/commands/*.json events/callbacks/*.json)
for f in "${files[@]}"; do
  echo "=== sam local invoke BotFunction -e $f ==="
  sam local invoke BotFunction -e "$f" --env-vars events/env.json
done
