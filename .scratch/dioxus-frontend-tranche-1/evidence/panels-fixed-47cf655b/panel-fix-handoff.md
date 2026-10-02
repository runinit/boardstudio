# Panel repair handoff — phase 1

Commit a3a0224f1b4e4694a431725016e6f0c72a1c6f6e. Parent edf59161. Own changes only web/src/presentation/panels.rs and web/assets/m1.css. Two inserted expressions/four removed lines. Checkout clean at handoff. Root owns build and final source/evidence integration; different reviewers must review the repair.

## Executed public red

`/tmp/frontend-run/panel-regressions/compact-resize-red.sh http://127.0.0.1:34647/ frontend-panel-fixer /tmp/frontend-run/panel-fixer-red` exited1 against frozen2030c9a3 candidate. Actual browser: at720x650 after public Objects open and at1280x650 after expansion, shell inertAttr="false" but DOM inert=true. This prevented real options interactions. The script's extra assertion against presence of class compact-open is not a behavioral defect; root/verifier agreed to remove that acceptance assertion while retaining diagnostic class data. No change to class policy was made.

`python3 /tmp/frontend-run/panel-grid-red.py http://127.0.0.1:34647/ frontend-panel-grid-fixer /tmp/frontend-run/panel-grid-red.json` exited1. Real persisted optional preference inputs pinned widths320/360 (existing keys), then real app reload and browser resize. At1280x650 tracks320/600/360; at1000x650 incorrectly220/480/300; at1280x500 incorrectly220/760/300. One command independently reproduces each media override. Fixtures are isolated named-browser storage; no DOM style/handler modification. Reference980/820 compact drawers remain ticket09; this repair preserves candidate760 compact fallback.

## Hypotheses shown before probes

1. inert is rendered as a presence-based HTML attribute, so false string still disables the panel.
2. Retained compact_open may separately suppress desktop auto-hide.
3. Old media grid declarations override parent variable tracks.

First and third are confirmed. Second remains source-supported but masked by unreachable inert controls; source timer was not changed in phase1.

## Cause and narrow repair

Installed Dioxus0.7.10 dioxus-web mutations.rs serializes AttributeValue::Bool(false) as string false. Its interpreter set_attribute.ts only removes falsy known boolean attrs; inert is missing from its isBoolAttr list. Panel shell/content therefore retained inert="false". Conditional Option<&str> emits empty presence when hidden and AttributeValue::None/removal when visible, preserving reactive media changes and hidden accessibility behavior. No dependency/framework patch or imperative DOM synchronization added.

Two older CSS declarations under <=1050 desktop and <=520 height replaced the variable grid-template-columns rule with fixed220/300 tracks. Removing only those declarations allows current variables, mode32px rail tracks, persisted widths, bounds and responsive default CSS to determine layout. Other tab/canvas/short-height rules unchanged.

## Local checks / remaining gates

`rustfmt --edition2024 web/src/presentation/panels.rs` and `git diff --check` pass. No local cargo/build/compiler by fixer. Root reports strict WASM Clippy passed and production rebuild started for a3a0224f. Await fresh build URL and verifier rerun. Timer actual red, fix and green still outstanding; phase1 is not final07 acceptance. Required independent standards/spec review and other acceptance evidence remain blocking.

RF: framework boolean attribute assumptions need real browser semantic checks (actual DOM property/accessibility), and new layout ownership must remove stale overriding rules. These are evidenced current repair lessons for root's existing RF register; no broad refactor proposed.

## Unmasked timer diagnosis and second repair

Fresh a3a0224f candidate: http://127.0.0.1:34649/.

Actual public red command:

```sh
python3 /tmp/frontend-run/panel-autohide-regression.py http://127.0.0.1:34649/ frontend-panel-timer-red /tmp/frontend-run/panel-timer-red.json
```

Exited 1. Real compact Objects click at 720×650, resize to 1280×650, real Options → Auto-hide objects → Layout click, wait 450 ms. Shell/content inert prerequisites pass; after idle, mode=autohide, revealed=true, contentInert=false, focus=m1-tab-Layout. No DOM/handler instrumentation.

Ranked hypotheses before the control: retained compact-open flag blocks desktop timeout; stale hover keeps reveal; timer not scheduled after mode change. Identical desktop-only control with compact-open step omitted passes on the same candidate, hiding content after idle. `/tmp/frontend-run/panel-autohide-control.py` and `/tmp/frontend-run/panel-timer-control.json` retain that control.

Corrective commit: `47cf655b` (`Allow desktop panels to auto-hide after compact opening`). Only `web/src/presentation/panels.rs`, removing the irrelevant compact_open guard from desktop timeout and its three private call sites. Current viewport, mode, hover, focus, 280ms delay, and drop cancellation retained. No compact class/state changes, API changes, CSS edits, or instrumentation.

Owned-file rustfmt and git diff --check passed. Root owns compiler/build and independent review. Final artifact green still pending; do not claim complete verification yet.

The original custom-width grid red command was rerun on a3a0224f and passed all three viewports; result `/tmp/frontend-run/panel-grid-green.json`. The final built artifact must rerun this and the original timer regression.

## Final built artifact verification

Root rebuilt source `47cf655b`, candidate http://127.0.0.1:34651/.

All original red-capable loops passed (exit 0):

```sh
python3 /tmp/frontend-run/panel-autohide-regression.py http://127.0.0.1:34651/ frontend-panel-timer-green /tmp/frontend-run/panel-timer-green.json
python3 /tmp/frontend-run/panel-grid-red.py http://127.0.0.1:34651/ frontend-panel-grid-final /tmp/frontend-run/panel-grid-final.json
python3 /tmp/frontend-run/panel-autohide-control.py http://127.0.0.1:34651/ frontend-panel-control-final /tmp/frontend-run/panel-timer-control-final.json
```

Timer final state after 450ms: mode=autohide, revealed=false, contentInert=true, shellInert=false, activeId=m1-tab-Layout. Original compact→desktop sequence and desktop-only control both pass. Stored pinned widths 320/360 retained at 1280×650, 1000×650, 1280×500. No further source change or debug instrumentation. Independent standards/spec review and verifier checks remain separate root-owned gates.
