#!/usr/bin/env bash
set -euo pipefail

# Run from sdk/ directory only
if [ ! -f "package.json" ]; then
  echo "Please run this script from the sdk/ directory." >&2
  exit 1
fi

# 1. Clean previous artefacts
rm -rf node_modules wasm/pkg

# 2. Build WASM package so pkg/ exists before install
wasm-pack build ./wasm --target nodejs --out-dir pkg

# 3. Fresh install (now pkg exists)
pnpm install

# 4. Build TypeScript bundle
pnpm run build

# 5. Run Vitest suite
pnpm test 