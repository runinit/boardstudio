# Q5 — Historical rationale

Fresh historian; backward-only from ad9db473012a4a4201217bd5094a4f24e7801932. Ten historical commits followed, code/diffs checked; no PR evidence retrieved. Commit subjects are author-intent claims, not independent rationale.

## Load-bearing history claims

H1: `1c5de957f "Extract private workbench compositions"` moved workspace composition, not all controller/input ownership. Keymap SVG moved to module while Editor assembled view/selection/viewbox/handlers. PCB was initially placeholder; `9271ade53 "Mount accepted PCB scene with guarded selection"` replaced it with typed accepted snapshot/scope/selection/generation/handlers and local SVG. Source: `web/src/presentation.rs:8269-8295 (Editor) @ ad9db4730`; `web/src/presentation/pcb_workspace.rs:93-130 (canvas) @ ad9db4730`; `web/src/presentation/keymap_workspace.rs:179-212 (canvas) @ ad9db4730`. Extraction introduced exact exception: "Layout's pointer-transaction SVG stays inline with its existing Editor-owned handlers." `web/src/presentation/workspace_composition.rs:94-105 (canvas) @ ad9db4730`. Records chosen boundary, not deeper reason for deferring Layout ownership.

H2: Retained controller versus visible observation became explicit. `1413685e6 "Add private Keymap layer edit controller"` documented Editor-lifetime pending work so hiding Keymap does not abandon single-flight guard/Session terminal observer, and "A scope/workspace switch never cancels or replays it." Historical path `web/src/presentation/keymap/layer_controller.rs:69-108 (use_layer_operations) @ 1413685e6`. `96874be6b "Add private keymap macro editor"` likewise documented retained submission state. `ed92e9d9d "Complete panel field bindings and retire departed presentation observations"` added workspace input, None owner outside Keymap, follow_owner before settle, cleared/filtered old feedback while retaining terminal path. Macro wording became: "Keeps controller Signals for the Editor lifetime; UI observations belong to the visible Keymap panel and retire when another workspace hides it." Snapshot: `web/crates/keymap/src/keymap/layer_controller.rs:137-201 (use_layer_operations) @ ad9db4730`; `web/crates/keymap/src/keymap/macro_controller.rs:242-244 (use_macro_operations) @ ad9db4730`; `web/crates/keymap/src/keymap/macro_controller.rs:270-330 (use_macro_operations) @ ad9db4730`. No specific production incident established as cause.

H3: `b29c3abfb "Add private PCB wiring preview adapter"` introduced unconditional Editor-lifetime query regardless of selected component/visible workspace. Its source says normalize physical-instance selection so Case/Layout instance changes do not churn board query. Identity/deduplication/owner-death/request supersession guards implemented that. `924ba2820 "Qualify PCB resolver error retry and stale board ownership"` extracted injected current-input/resolver seam for controlled replies "without changing production admission or settlement" and retained guards. Snapshot `web/crates/pcb/src/pcb_wiring/controller.rs:934-998 (use_pcb_wiring_resolution_owner) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1023-1054 (current_input) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1090-1109 (start_resolution) @ ad9db4730`. Supports test controllability, not changed visible ownership.

## Dates

Author dates, last-touch means supplied line ranges, not whole files.

- Parent-input/module-SVG Keymap introduced1c5de957f, 2026-10-02; PCB introduced9271ade53, 2026-10-02. Parent inputs last touched784dcb079, 2026-10-07, "Compose editor workspace state handles"; callbacks changed, split retained.
- PCB canvas last touched26f543d1d, 2026-11-05, "Migrate workbench and workspace presentation"; relevant changes CSS classes.
- Keymap canvas last touchedb100ac59f, 2026-11-05, "Migrate keymap workspace layouts"; relevant changes CSS classes.
- Visibility-retirement mechanism in both layer/macro supplied ranges introduced and last toucheded92e9d9d, 2026-10-08. Retained layer/macro foundations1413685e6/96874be6b, both2026-10-02.

## History boundary

10/10 commits:1c5de957f,9271ade53,784dcb079,26f543d1d,b100ac59f,ed92e9d9d,b29c3abfb,924ba2820,1413685e6,96874be6b. Other predecessor SHAs exposed by blame were not followed. Intermediate pending-helper evolution, broader alternatives and originating PR/issues untraced. No working-tree source/status, plans or execution evidence used. No new ADR recommended from these bounded findings.
