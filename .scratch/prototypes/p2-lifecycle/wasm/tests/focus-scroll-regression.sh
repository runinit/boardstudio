#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec node "$script_dir/focus-scroll-regression.mjs" "${1:?Usage: focus-scroll-regression.sh <prototype-url>}"
