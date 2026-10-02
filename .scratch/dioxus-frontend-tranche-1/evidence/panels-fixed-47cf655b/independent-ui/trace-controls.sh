#!/usr/bin/env bash
# Public UI trace with selectors supplied for the target implementation.
# Usage: trace-controls.sh URL SESSION OUT_DIR [LIBRARY_TRIGGER] [OBJECTS_OPTIONS] [INSPECTOR_OPTIONS]
set -euo pipefail
url=${1:?URL required}
session=${2:?named browser session required}
out=${3:?output directory required}
library_trigger=${4:-}
objects_options=${5:-Objects options}
inspector_options=${6:-Inspector options}
mkdir -p "$out"
export AGENT_BROWSER_SESSION="$session"
agent-browser open "$url" > "$out/open.txt"
agent-browser snapshot -i > "$out/initial.txt"
agent-browser read > "$out/initial-text.txt"
if [[ -n "$library_trigger" ]]; then
  agent-browser find role button click --name "$library_trigger" > "$out/library-click.txt"
  agent-browser wait 1200 > "$out/library-wait.txt"
  agent-browser snapshot -i > "$out/library.txt"
  agent-browser read > "$out/library-text.txt"
  agent-browser screenshot "$out/library.png" > "$out/library-screenshot.txt"
fi
for item in "$objects_options" "$inspector_options"; do
  safe=${item//[^a-zA-Z0-9_-]/_}
  if agent-browser find role button click --name "$item" > "$out/$safe-click.txt" 2>&1; then
    agent-browser snapshot -i > "$out/$safe-open.txt"
    agent-browser press Escape > "$out/$safe-escape.txt"
    agent-browser snapshot -i > "$out/$safe-closed.txt"
  else
    printf '%s\n' "not available: $item" > "$out/$safe-unavailable.txt"
  fi
done
