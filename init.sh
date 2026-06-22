#!/usr/bin/env bash
set -euo pipefail

echo "=== Agent Manager Desktop Verification ==="

if [ -f package.json ] && [ -d node_modules ]; then
  echo "=== npm test ==="
  npm test

  echo "=== npm build ==="
  npm run build
else
  echo "Skipping npm checks because node_modules is not installed."
fi

echo "=== cargo test ==="
cargo test

echo "=== Verification Complete ==="

