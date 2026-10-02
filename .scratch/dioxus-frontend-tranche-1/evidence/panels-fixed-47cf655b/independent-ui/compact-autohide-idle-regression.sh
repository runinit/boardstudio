#!/usr/bin/env bash
# Real public-control reproduction for compact-open -> desktop -> Auto-hide idle.
# Expected RED before the compact_open timer guard is repaired.
set -euo pipefail
url=${1:-http://127.0.0.1:34649/}
session=${2:-frontend-panels-autohide-red}
out=${3:-/tmp/frontend-run/panel-regressions}
mkdir -p "$out"
export AGENT_BROWSER_SESSION="$session"
agent-browser open "$url" > "$out/autohide-open.log"
agent-browser wait 300 > "$out/autohide-landing-wait.log"
agent-browser find role button click --name 'REVIUNG41 copy' > "$out/autohide-open-project.log"
agent-browser wait 1000 > "$out/autohide-editor-wait.log"
agent-browser set viewport 720 650 > "$out/autohide-set-compact.log"
agent-browser find role button click --name Objects > "$out/autohide-open-objects.log"
agent-browser wait 100 > "$out/autohide-compact-wait.log"
agent-browser snapshot -i > "$out/autohide-compact-open.txt"
agent-browser screenshot "$out/autohide-compact-open.png" > "$out/autohide-compact-screen.log"
agent-browser set viewport 1280 650 > "$out/autohide-set-desktop.log"
agent-browser wait 100 > "$out/autohide-desktop-wait.log"
agent-browser find role button click --name 'Objects options' > "$out/autohide-options.log"
agent-browser snapshot -i > "$out/autohide-options-open.txt"
agent-browser find role button click --name 'Auto-hide objects' > "$out/autohide-select.log"
agent-browser find role tab click --name Layout > "$out/autohide-focus-out.log"
agent-browser mouse move 620 260 > "$out/autohide-pointer-out.log"
agent-browser wait 400 > "$out/autohide-idle-wait.log"
agent-browser snapshot -i > "$out/autohide-after-idle.txt"
agent-browser eval 'JSON.stringify({viewport:[innerWidth,innerHeight],mode:document.querySelector("#m1-objects-panel")?.dataset.mode,contentHidden:document.querySelector("#m1-objects-panel-content")?.getAttribute("aria-hidden"),revealed:document.querySelector("#m1-objects-panel")?.dataset.revealed,railExpanded:document.querySelector("#m1-objects-panel-rail")?.getAttribute("aria-expanded"),focusWithinPanel:document.querySelector("#m1-objects-panel")?.contains(document.activeElement),pointer:[...document.querySelectorAll("#m1-objects-panel")].map(e=>{let r=e.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]})})' > "$out/autohide-after-idle.json"
agent-browser screenshot "$out/autohide-after-idle.png" > "$out/autohide-after-idle-screen.log"
cat "$out/autohide-after-idle.json"
agent-browser eval '(()=>{const p=document.querySelector("#m1-objects-panel");const c=document.querySelector("#m1-objects-panel-content");const r=document.querySelector("#m1-objects-panel-rail");if(!p||p.dataset.mode!=="autohide"||c?.getAttribute("aria-hidden")!=="true"||r?.getAttribute("aria-expanded")!=="false")throw new Error(`Auto-hide panel did not hide after pointer/focus left for >280ms: mode=${p?.dataset.mode}; revealed=${p?.dataset.revealed}; content aria-hidden=${c?.getAttribute("aria-hidden")}; rail aria-expanded=${r?.getAttribute("aria-expanded")}`);return "PASS"})()'
