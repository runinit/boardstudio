# 002: Portable isolated browser launch and prerequisite contract

Status: resolved
Type: research
Blocked by: none
Parent: [Chromiumoxide browser checks](../map.md)

## Question

What launch/discovery/isolation contract should a Rust browser harness share with the supported
human/CI test entry points, given the diagnosed dead profile mount, personal binary pin,
and missing Node-module setup?

Inventory current real-page consumers and CI launch/configuration owners. Reuse existing
diagnosis evidence where sufficient, verifying freshness before claims. Propose process-owned
writable profile/config directories, portable browser discovery, diagnostics, process cleanup,
and sandbox-policy handling without unrelated persistent configuration changes.
Distinguish requirements for the new harness from the retained WASM runner.
No production changes; preserve existing test assertions and supported receipts.

## Claim

Owned by this Wayfinder session; delegated research is shared with the protocol question.

## Answer

Inventoried all three real-page consumers (drag smoke, CAD capture, Graphite
acceptance), their launchers (`run-drag-browser.py`, `measure-cad.py`, manual),
the `browser_tools.locate()`/`check.py` preflight gate, CI's pinned
`playwright-core@1.62.1` + `NODE_PATH`, and the retained WASM/ChromeDriver path
(which keeps chromedriver discovery, `webdriver.json` and `wasm-pack` launches
untouched). Proposed harness contract from upstream `BrowserConfig`/`detection`
sources: own `CHROME_BIN`-first discovery (upstream's `CHROME`-var/Edge-inclusive
order must not replace it), process-owned temp profile under a shallow `TMPDIR`
plus child `XDG_CONFIG_HOME` override (covers the diagnosed dead-config crash
class; the psd symlink itself is gone today), shared SwiftShader/no-sandbox flags
and 1513x1043 viewport, version logging for receipts, child + profile cleanup, no
persistent config changes. Diagnosis evidence reused with freshness notes; full
default-env launch and build-lock probes deliberately unrun.

Full findings:
[chromiumoxide-migration-feasibility.md](../../../research/chromiumoxide-migration-feasibility.md)
(§2). Archived on local branch `research/chromiumoxide-feasibility` at
`74a623030a2e174ab5fb2e393800728af5ea838f` (not for merge, do not push).
