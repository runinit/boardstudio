#!/usr/bin/env bash
set -euo pipefail
session="${1:?already-imported task-owned session}"
kind="${2:?candidate or reference}"
agent-browser --session "$session" find role tab click --name Layout
if [[ "$kind" == candidate ]]; then
  agent-browser --session "$session" find role button click --name 'keys Independent' --exact
else
  agent-browser --session "$session" find role treeitem click --name 'keys Independent' --exact
fi
agent-browser --session "$session" find role tab click --name Keycaps
agent-browser --session "$session" find role button click --name 'Edit key left-keys-SW7' --exact
agent-browser --session "$session" wait 400
agent-browser --session "$session" eval '(() => { const actual = document.querySelector("select[aria-label=\"Selected key\"]")?.value; const expected = "matrix/left-keys/r0c0"; if (actual !== expected) throw new Error(`Expected retained matrix selection first key ${expected}; got ${actual}`); return {expected,actual}; })()'
