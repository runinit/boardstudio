#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
ATTEMPT="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="evidence/logs/${ATTEMPT}"
mkdir -p "$LOG_DIR"

run_logged() {
  local name="$1"
  shift
  local log="${LOG_DIR}/${name}.log"
  local status=0
  {
    printf 'command:'
    printf ' %q' "$@"
    printf '\n'
  } > "$log"
  "$@" >> "$log" 2>&1 || status=$?
  printf 'exit=%s\n' "$status" >> "$log"
  cat "$log"
  if [[ "$status" -ne 0 ]]; then
    return "$status"
  fi
}

run_logged native-fmt cargo fmt --manifest-path Cargo.toml -- --check
run_logged native-tests cargo test --manifest-path Cargo.toml --locked
run_logged native-clippy cargo clippy --manifest-path Cargo.toml --locked --all-targets -- -D warnings
run_logged wasm-page-clippy cargo clippy --manifest-path Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --locked -- -D warnings
run_logged wasm-service-worker-clippy cargo clippy --manifest-path Cargo.toml --target wasm32-unknown-unknown --no-default-features --features service-worker --locked -- -D warnings

if [[ -d dist ]]; then
  mv dist "dist-attempt-${ATTEMPT}"
fi
mkdir -p dist/root/pkg dist/update
run_logged wasm-page-v1 env P3_CACHE_VERSION=v1 wasm-pack build . --target web --release --out-dir dist/root/pkg --out-name p3_durability --no-typescript --no-pack -- --no-default-features --features page --locked
run_logged wasm-worker-v1 env P3_CACHE_VERSION=v1 wasm-pack build . --target web --release --out-dir dist/root --out-name p3_sw --no-typescript --no-pack -- --no-default-features --features service-worker --locked
mv dist/root/p3_sw_bg.wasm dist/root/p3_sw_v1_bg.wasm
cp browser/index.html browser/leave.html browser/cold.html browser/app.js browser/version.txt dist/root/
mkdir -p dist/root/fixtures
cp fixtures/reviung41-original.boardstudio dist/root/fixtures/reviung41-original.boardstudio
run_logged embed-worker-v1 node scripts/embed-worker-wasm.mjs dist/root v1
run_logged wasm-worker-v2 env P3_CACHE_VERSION=v2 wasm-pack build . --target web --release --out-dir dist/update --out-name p3_sw --no-typescript --no-pack -- --no-default-features --features service-worker --locked
mv dist/update/p3_sw_bg.wasm dist/update/p3_sw_v2_bg.wasm
run_logged embed-worker-v2 node scripts/embed-worker-wasm.mjs dist/update v2
cp browser/version.txt dist/update/
sed -i 's/p3-cache-v1/p3-cache-v2/' dist/update/version.txt

run_logged chromium-browser-check node browser/run.mjs
