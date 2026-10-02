#!/usr/bin/env bash
# Reproduce compact-open stale state using public controls only.
# Expected RED on current candidate: desktop slot remains compact-open and inert.
set -euo pipefail
url=${1:-http://127.0.0.1:34647/}
session=${2:-frontend-candidate-redtest}
out=${3:-/tmp/frontend-run/panel-regressions}
mkdir -p "$out"
export AGENT_BROWSER_SESSION="$session"
agent-browser open "$url" > "$out/compact-open.log"
agent-browser wait 400 > "$out/landing-wait.log"
agent-browser find role button click --name 'REVIUNG41 copy' > "$out/open-project.log"
agent-browser wait 1000 > "$out/editor-wait.log"
agent-browser set viewport 720 650 > "$out/set-compact.log"
agent-browser find role button click --name Objects > "$out/open-objects.log"
agent-browser wait 150 > "$out/compact-open-wait.log"
agent-browser eval 'JSON.stringify({viewport:[innerWidth,innerHeight],slot:document.querySelector("#m1-objects-panel")?.className,mode:document.querySelector("#m1-objects-panel")?.dataset.mode,inertAttr:document.querySelector("#m1-objects-panel")?.getAttribute("inert"),inert:document.querySelector("#m1-objects-panel")?.inert})' > "$out/compact-open-state.json"
agent-browser screenshot "$out/compact-open.png" > "$out/compact-open-screenshot.log"
agent-browser set viewport 1280 650 > "$out/set-desktop.log"
agent-browser wait 350 > "$out/desktop-wait.log"
agent-browser eval 'JSON.stringify({viewport:[innerWidth,innerHeight],slot:document.querySelector("#m1-objects-panel")?.className,mode:document.querySelector("#m1-objects-panel")?.dataset.mode,inertAttr:document.querySelector("#m1-objects-panel")?.getAttribute("inert"),inert:document.querySelector("#m1-objects-panel")?.inert,optionsInAccessibilityTree:document.querySelector("#m1-objects-panel-options")?.getAttribute("aria-label")})' > "$out/desktop-state.json"
agent-browser screenshot "$out/compact-open-after-desktop.png" > "$out/desktop-screenshot.log"
cat "$out/compact-open-state.json"
cat "$out/desktop-state.json"
agent-browser eval '(()=>{const e=document.querySelector("#m1-objects-panel");if(!e||e.inert||e.classList.contains("compact-open"))throw new Error(`compact mode leaked into desktop: class=${e?.className}; inert=${e?.inert}; inertAttr=${e?.getAttribute("inert")}`);return "PASS"})()'
