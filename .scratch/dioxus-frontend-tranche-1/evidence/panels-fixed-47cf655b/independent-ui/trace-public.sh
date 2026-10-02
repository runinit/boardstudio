#!/usr/bin/env bash
set -euo pipefail
url=${1:-http://127.0.0.1:5173/}
session=${2:-frontend-ui-reference}
out=${3:-/tmp/frontend-run/ui-verifier}
mkdir -p "$out"
export AGENT_BROWSER_SESSION="$session"
agent-browser open "$url" > "$out/open.txt"
agent-browser snapshot -i > "$out/entry.txt"
agent-browser read > "$out/entry-text.txt"
agent-browser screenshot "$out/entry.png" > "$out/entry-screenshot.txt"
