# 010: Packaged A→B draft-safety proof

Status: claimed
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Assignee: Pi Spark (010 implementation)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md), [Diagnostic gated-persistence adapter for packaged builds](009-diagnostic-gated-persistence-adapter.md)

## What to build

The packaged tracer proof: the confirmed Geometry scripts A→B draft loss
becomes a permanent regression that passes in real packaged builds. Build A
runs with a real waiting build B; a valid unapplied Geometry scripts draft
defers Update Now at the toast entry point and, after toast dismissal, at the
header entry point. After the draft is resolved and the save settles, Update
Now loads actual build B and reopens the exact saved project at the intended
value and revision. Report at the packaged-GREEN stopping point.

## Ownership (010 implementation, 2026-11-07, base `21d3e7a`)

Exact owned paths (verified natively before edits):

- `scripts/verify-update-adoption.py` (extend with 010 legs + `pre010` RED mode; reuse existing helpers/server/stamping; no parallel harness)
- `scripts/verify-update-adoption-payloads/geometry-draft.js` (new, Geometry editor values)
- `scripts/verify-update-adoption-payloads/save-state.js` (new, Saved/Saving/Failed via header labels)
- `scripts/verify-update-adoption-payloads/persist-gate.js` (new, gate snapshot)
- `scripts/verify-update-adoption-payloads/tab-prep-header.js` (new, header Tab order)
- `scripts/verify-update-adoption-payloads/update-check.js` (new, quick no-build check)
- `scripts/verify-update-adoption-payloads/layout-constraint.js` (new, minimal Layout probe; leg blocked, see Results)
- Existing payloads reused unchanged: `state.js`, `update.js`, `loaded-binary.js`, `cachecontent.js`, `marker.js`, `focus-info.js`, `tab-prep.js`
- `docs/plans/update-draft-safety/issues/010-packaged-a-to-b-proof.md` (this ticket; progress/handoff record)
- Minimal focused driver tests if needed (none planned; driver is exercised via packaged runs, not unit tests)

Not owned: production Rust, registry/owner adapters, other tickets, root rules, spec/map, coordinator-owned `execution.md`, unrelated build tooling. Diagnostic adapter (`window.__boardstudioPersistGate`) is reused, not modified. Historical RED artifacts (`web/target/site-002-ab/a|b`) are read-only.

## Implementation progress (Pi Spark, 2026-11-07; CLAIMED, not resolved)

- RED artifacts: immutable `web/target/site-002-ab/a` (root `boardstudio-9ef37ed793933307-1794026098578752454`) and `/b` (root `boardstudio-8b2afa895e232839-1794026099409516407`); verified has 019 adoption (`skipWaiting` in worker), lacks tracer (`unapplied edit` absent from page wasm) and gate (`__boardstudioPersistGate` absent), stamped meta + `boardstudio-build<A|B>` wasm section. No rebuild of old code.
- GREEN build: fresh diagnostic src `web/target/site-010-src` from `21d3e7a` via `scripts/build-web.py --fast --skip-providers --routes root,subpath --diagnostic-persist-gate` (log `010-build-diag.log`); page wasm contains gate (1 occurrence) + tracer (`unapplied edit`) + adoption (`skipWaiting`). Stamped to A/B via driver per run (same code, versions differ only in `time_ns`; representative tracer run root A `boardstudio-75d2e21c0706a3fb-1794071406316968517` B `boardstudio-c3b7f987b3af4ed4-1794071407218938008`, subpath A `boardstudio-d57769c07ecd3ca9-1794071406763542674` B `boardstudio-329382aff8b86738-1794071407644626062`; packaged diff exactly `index.html` + `service-worker.js` + page wasm per route, all runs). Unarmed gate transparent per 009. Fast-vs-optimized limitation: fast build (debug, no wasm-opt) used for speed; shipped build is release + wasm-opt. Same Rust code, behavior identical; optimization-only difference.
- Legs implemented: `geometry-tracer`, `persist-hold`, `persist-fail`, `persist-hold-fail`, `takeover-draft` (exactly one), `no-build`; inherited `adopt`, `second-editor`, `subpath`, `failed-install`, `unavailable-target` reused unchanged. `pre010` mode runs tracer RED vs legacy 002 A/B. `layout-constraint` payload created (`layout-constraint.js`) but leg BLOCKED (see below); no leg added yet.
- New payloads (all owned): `geometry-draft.js`, `save-state.js` (Saved/Saving/Failed via header labels; `state.js` misses Saved/Saving), `persist-gate.js` (gate snapshot), `tab-prep-header.js` (header Tab order), `update-check.js` (quick no-build check), `layout-constraint.js` (minimal, unused until Layout unblocked).
- Driver helpers (all reused across legs): `find_ref_retry`, `geo_values`, `save_state`, `gate_snapshot`, `gate_arm/release/disarm`, `wait_gate_entered/acked` (deadline polling, rejects pre-release ack), `wait_save/stable`, `dialog_accept`, `open_geometry_editor`, `close_all_toasts` (synthetic cleanup of non-update toasts only; preserves update toast; Update Now/Cancel always trusted), `create_baseline_script`, `set_geometry_draft` (Tab, no blur commit), `expect_draft_deferral`, `cancel_preserves_draft`, `expect/cancel_save_deferral`, `select_baseline_script`.

## Results (2026-11-07; CLAIMED, awaiting fresh reviews + coordinator verification)

RED (`010-red-tracer.log`, exit 1, 2 failures as expected): pre-tracer A/B adopt instead of deferring (`RED: toast defers` → `reloaded`), draft lost (`source` `// 010 baseline`, draft line gone). Baseline/draft/waiting setup all passed on old builds; failures are exactly the regression assertions. Fresh RED via `pre010` mode + immutable 002 artifacts, not a citation of 002.

GREEN (all exit 0, 0 failures):

- `geometry-tracer` (`010-green-tracer.log`): toast pointer + Tab/Enter/Space defer with `Geometry scripts has an unapplied edit. Apply the script edit, or cancel the edit in the form…`, Cancel preserves exact draft + focus to Update Now, stays A; header pointer + Tab/Enter/Space same after dismissal; Apply via hold captures revision 3 (`Split 42`), release → `completed` → Saved → header adoption to B; loaded doc B + served binary B (`requestedByThisDocument`, `reload`, B cache only); reopened `Tracer 010` at draft value, rev 3.
- `persist-hold` (`010-green-hold2.log`): hold v1 entered rev 3, Saving + no ack; B waiting; Update Now defers `A save is still in progress`, Cancel preserves hold; release → `completed` → Saved v1 → toast adoption to B, reopened v1; gate reused in B (second hold entered/completed/Saved v2).
- `persist-fail` + `persist-hold-fail` + `takeover-draft` (`010-green-fail.log`): fail → Failed + `aborted:diagnostic quota`, Update Now defers `The last save did not complete` (+ additive `RecoveryRequired`), Cancel, explicit recovery via `Reload saved version` (accept confirm) to baseline, re-apply v2 → Saved → adopt → reopen v2. Hold-then-fail: held → save-in-progress deferral → release atomically (`released` + `aborted`, no re-arm) → Failed → did-not-complete deferral → Reload + re-apply → adopt. Takeover: clean B waiting, new draft while waiting defers, Cancel, Apply+stable-Saved → adopt → reopen takeover value.
- `no-build` (`010-green-hold2.log`): A only, no waiting/Update Now, quick update check finds nothing, no reload, editor alive.
- Inherited (`010-green-inherit.log`): `adopt`, `second-editor` (other-tab defer + offline A-asset bytes), `subpath` (isolation), `failed-install` (script/asset, repair recovers), `unavailable-target` (report + retry, repaired B adopts) all pass on diagnostic builds (unarmed gate transparent).

Explicit retry semantics (packaged): failed saves disable the editor (`RecoveryRequired`); `Retry save` also succeeds but its saved value is ambiguous in UI (baseline shown); legs use `Reload saved version` (confirm `Discard your pending edits…`) to recover to last saved (v0), then re-apply (v2) → Saved → adopt. Both are explicit user actions; Reload is unambiguous. Recorded for reviewers; no product change.

Layout follow-up BLOCKED (focused reproducer, not silently skipped): disposable diagnostic A (site-010-ab/a) on isolated profiles/ports 4742–4743; Split 42 + Studio 60 demo copies; selections Main board / Left half / Left main / Column 1 / Key 1.1 / Outline; DOM queries for `[aria-label="Layout constraint"]`, `Apply position`, `Save constraint`, `Add constraint`, `Companion placement` all absent (`[]`); key/matrix/half inspectors show Position & rotation / Key size / Selected keys, never the 011 adapters. No Layout draft form found in demo via tree or workspace navigation. Proposed smallest next change: 011 owner supplies packaged UI steps for ONE Layout owner (L1 constraint, L8 position, or L15 companion) in a demo project (or a minimal fixture project with constraints), reusing the tracer leg shape; 010 then adds `layout-constraint` leg (payload `layout-constraint.js` already owned). Parent may assign a fresh issue agent; no scope expansion done here.

Cleanup attested: all `agent-browser` sessions closed (`close`), all `serve-web.py` servers stopped, ports 4733/4740–4743 verified free after each run; disposable profiles/databases were session-isolated (fresh `agent-browser session id` per leg) and discarded on close; no live browser/profile/project touched (all runs on `127.0.0.1` disposable ports + stamped copies); no caches/workers unregistered to obtain GREEN. Build outputs `web/target/site-010-src` + `site-010-ab` (git-ignored) retained for coordinator reruns; no targets/sources/manifests under `.evidence/` (logs only). `git diff --check` clean (see commit).

Gortex: pre-edit impact (MEDIUM→CRITICAL lower bound, additive only, 0 existing callers changed); post-edit detect maps 2 files + 106 new symbols (helpers/legs/`pre010`/`LEGS`/`main`); tests 0/4 graph-tracked (driver exercised via packaged runs, logs above); guards no rules; contract `warn` (behavioral, blast reviewed, no parse errors). Payload JS files are untracked until commit (detect shows tracked modifications only).

Status stays CLAIMED: acceptance checkboxes unchanged pending fresh Spec+Standards reviews and coordinator independent reruns. Full gate/mutation belong to 015.

## Acceptance criteria

- [ ] The packaged regression assertion from [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md) is RED before the tracer fix is present in the packaged build and GREEN after, with A/B build identities recorded.
- [ ] Toast entry point: trusted pointer and trusted Tab/Enter/Space defer; Cancel preserves the exact draft text and the running build stays A.
- [ ] Header entry point after toast dismissal defers the same draft the same way.
- [ ] Held-persistence legs: with the diagnostic adapter armed in A, Update Now defers as save-in-progress with no durable change; after release and acknowledgement, adoption proceeds; the same gate is consumed again after takeover in B. Fail and hold-then-fail legs defer as did-not-complete and recover via explicit retry.
- [ ] Exactly one packaged new-draft-during-takeover leg, reusing the tracer scenario's shape: a draft created while adoption waits defers Update Now again.
- [ ] Loaded-B proof: the loaded document and the served binary both identify B (not manifests or labels), the navigation is the adoption reload, and the reopened project holds the intended value at the intended revision.
- [ ] Sole-client/other-tab, offline-asset continuity and subpath-isolation legs inherited from the parent adoption acceptance pass; failed-install and no-new-build negative controls do not adopt.
- [ ] Disposable fixtures, profiles and databases are cleaned up; evidence logs are stored per the plan's evidence conventions. This ticket does not close the parent adoption umbrella.
