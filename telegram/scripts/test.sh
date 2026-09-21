#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -f docker-compose.test.yml ]]; then
  echo "Skipping tests (docker-compose.test.yml not packaged)."
  exit 0
fi

if docker info >/dev/null 2>&1; then
  exec docker compose -f docker-compose.test.yml run --rm --build test
fi

echo "Docker is not available; running Vitest on the host."
export TELEGRAM_BOT_TOKEN="${TELEGRAM_BOT_TOKEN:-test-token}"
export PREFS_FILE="${PREFS_FILE:-/tmp/maceio-prefs-test.json}"
exec npx vitest run "$@"
