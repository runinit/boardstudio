#!/usr/bin/env bash
# PROTOTYPE (ticket 013), throwaway. Builds and serves the Theme gallery: http://127.0.0.1:8393/?variant=A
# Variants: A Original light, B Original dark, C shadcn light, D shadcn dark (bottom bar or arrow keys).
set -euo pipefail
cd "$(dirname "$0")"
CARGO_TARGET_DIR="$PWD/target" dx build --platform web --release
P="target/dx/theme-proto/release/web/public"
mkdir -p "$P/assets/fonts" && cp assets/fonts/*.woff2 "$P/assets/fonts/"
echo "serving on http://127.0.0.1:8393/?variant=A  (Ctrl-C to stop)"
cd "$P" && exec python3 -m http.server 8393 --bind 127.0.0.1
