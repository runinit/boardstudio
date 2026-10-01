#!/usr/bin/env bash
set -euo pipefail

url="${1:?Usage: focus-scroll-regression.sh <prototype-url>}"
export AGENT_BROWSER_SESSION="p2-focus-regression-$$"

cleanup() {
  agent-browser close >/dev/null 2>&1 || true
}
trap cleanup EXIT

agent-browser set viewport 1280 577
agent-browser open "$url"
agent-browser wait '.layout-canvas'
agent-browser eval '(() => {
  const svg = document.querySelector("svg.layout-canvas");
  const target = document.querySelector("[data-part-id=\"matrix/main-right-keys/r0c0\"]");
  if (!svg || !target) throw new Error("P2 gesture surface or repro target is missing");
  const rect = svg.getBoundingClientRect();
  if (window.scrollY !== 0 || Math.abs(rect.top - 273.125) > 1) {
    throw new Error(`Unexpected initial frame: scrollY=${window.scrollY}, svgTop=${rect.top}`);
  }
  if (document.elementFromPoint(393, 442)?.closest("[data-part-id]") !== target) {
    throw new Error("Captured pointer start does not hit the repro target");
  }
})()'

# Replay the trusted coordinates captured at 1280x577.
agent-browser mouse move 393 442
agent-browser mouse down
agent-browser eval '(() => {
  const svg = document.querySelector("svg.layout-canvas");
  const rect = svg.getBoundingClientRect();
  if (document.activeElement !== svg) throw new Error("Pointer interaction did not preserve SVG keyboard focus");
  if (window.scrollY !== 0 || Math.abs(rect.top - 273.125) > 1) {
    throw new Error(`Pointer-down focus shifted the coordinate frame: scrollY=${window.scrollY}, svgTop=${rect.top}`);
  }
})()'
agent-browser mouse move 423 466
agent-browser mouse up
agent-browser eval '(() => {
  const svg = document.querySelector("svg.layout-canvas");
  const rect = svg.getBoundingClientRect();
  if (window.scrollY !== 0 || Math.abs(rect.top - 273.125) > 1) {
    throw new Error(`Pointer gesture changed the coordinate frame: scrollY=${window.scrollY}, svgTop=${rect.top}`);
  }
  if (document.activeElement !== svg) throw new Error("Pointer interaction did not leave keyboard focus on the SVG");
})()'

echo "PASS: pointer focus keeps the 1280x577 SVG coordinate frame stable and preserves focus"
