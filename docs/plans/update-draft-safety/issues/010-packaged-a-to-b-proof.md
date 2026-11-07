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
- `scripts/verify-update-adoption-payloads/layout-constraint.js` (real standalone-part constraint observation; packaged leg added in focused follow-up below)
- Existing payloads reused unchanged: `state.js`, `update.js`, `loaded-binary.js`, `cachecontent.js`, `marker.js`, `focus-info.js`, `tab-prep.js`
- `docs/plans/update-draft-safety/issues/010-packaged-a-to-b-proof.md` (this ticket; progress/handoff record)
- Minimal focused driver tests if needed (none planned; driver is exercised via packaged runs, not unit tests)

Not owned: production Rust, registry/owner adapters, other tickets, root rules, spec/map, coordinator-owned `execution.md`, unrelated build tooling. Diagnostic adapter (`window.__boardstudioPersistGate`) is reused, not modified. Historical RED artifacts (`web/target/site-002-ab/a|b`) are read-only.

## Implementation progress (Pi Spark, 2026-11-07; CLAIMED, not resolved)

- RED artifacts: immutable `web/target/site-002-ab/a` (root `boardstudio-9ef37ed793933307-1794026098578752454`) and `/b` (root `boardstudio-8b2afa895e232839-1794026099409516407`); verified has 019 adoption (`skipWaiting` in worker), lacks tracer (`unapplied edit` absent from page wasm) and gate (`__boardstudioPersistGate` absent), stamped meta + `boardstudio-build<A|B>` wasm section. No rebuild of old code.
- GREEN build: fresh diagnostic src `web/target/site-010-src` from `21d3e7a` via `scripts/build-web.py --fast --skip-providers --routes root,subpath --diagnostic-persist-gate` (log `010-build-diag.log`); page wasm contains gate (1 occurrence) + tracer (`unapplied edit`) + adoption (`skipWaiting`). Stamped to A/B via driver per run (same code, versions differ only in `time_ns`; representative tracer run root A `boardstudio-75d2e21c0706a3fb-1794071406316968517` B `boardstudio-c3b7f987b3af4ed4-1794071407218938008`, subpath A `boardstudio-d57769c07ecd3ca9-1794071406763542674` B `boardstudio-329382aff8b86738-1794071407644626062`; packaged diff exactly `index.html` + `service-worker.js` + page wasm per route, all runs). Unarmed gate transparent per 009. Fast-vs-optimized limitation: fast build (debug, no wasm-opt) used for speed; shipped build is release + wasm-opt. Same Rust source only; optimized packaged behavior has NOT been run or established. Release + wasm-opt validation remains DUE; no unrelated build/dependency repairs are authorized.
- Legs implemented: `geometry-tracer`, `persist-hold`, `persist-fail`, `persist-hold-fail`, `takeover-draft` (exactly one), `no-build`; inherited `adopt`, `second-editor`, `subpath`, `failed-install`, `unavailable-target` reused unchanged. `pre010` mode runs tracer RED vs legacy 002 A/B. At `c899b0f6`, `layout-constraint` was only a blocked probe; the focused follow-up below adds its real imported-fixture leg.
- New payloads (all owned): `geometry-draft.js`, `save-state.js` (Saved/Saving/Failed via header labels; `state.js` misses Saved/Saving), `persist-gate.js` (gate snapshot), `tab-prep-header.js` (header Tab order), `update-check.js` (quick no-build check), `layout-constraint.js` (now the real-form observation used by the follow-up leg).
- Driver helpers (all reused across legs): `find_ref_retry`, `geo_values`, `save_state`, `gate_snapshot`, `gate_arm/release/disarm`, `wait_gate_entered/acked` (deadline polling, rejects pre-release ack), `wait_save/stable`, `dialog_accept`, `open_geometry_editor`, `close_all_toasts` (synthetic cleanup of non-update toasts only; preserves update toast; Update Now/Cancel always trusted), `create_baseline_script`, `set_geometry_draft` (Tab, no blur commit), `expect_draft_deferral`, `cancel_preserves_draft`, `expect/cancel_save_deferral`, `select_baseline_script`.

## Results (2026-11-07; CLAIMED, awaiting fresh reviews + coordinator verification)

RED (`010-red-tracer.log`, exit 1, 2 failures as expected): pre-tracer A/B adopt instead of deferring (`RED: toast defers` → `reloaded`), draft lost (`source` `// 010 baseline`, draft line gone). Baseline/draft/waiting setup all passed on old builds; failures are exactly the regression assertions. Fresh RED via `pre010` mode + immutable 002 artifacts, not a citation of 002.

GREEN (all exit 0, 0 failures):

- `geometry-tracer` (`010-green-tracer.log`): toast pointer + Tab/Enter/Space defer with `Geometry scripts has an unapplied edit. Apply the script edit, or cancel the edit in the form…`, Cancel preserves exact draft + focus to Update Now, stays A; header pointer + Tab/Enter/Space same after dismissal; Apply via hold captures revision 3 (`Split 42`), release → `completed` → Saved → header adoption to B; loaded doc B + served binary B (`requestedByThisDocument`, `reload`, B cache only); reopened `Tracer 010` at draft value, rev 3.
- `persist-hold` (`010-green-hold2.log`): hold v1 entered rev 3, Saving + no ack; B waiting; Update Now defers `A save is still in progress`, Cancel preserves hold; release → `completed` → Saved v1 → toast adoption to B, reopened v1; gate reused in B (second hold entered/completed/Saved v2).
- `persist-fail` + `persist-hold-fail` + `takeover-draft` (`010-green-fail.log`): fail → Failed + `aborted:diagnostic quota`, Update Now defers `The last save did not complete` (+ additive `RecoveryRequired`), Cancel, explicit recovery via `Reload saved version` (accept confirm) to baseline, re-apply v2 → Saved → adopt → reopen v2. Hold-then-fail: held → save-in-progress deferral → release atomically (`released` + `aborted`, no re-arm) → Failed → did-not-complete deferral → Reload + re-apply → adopt. Takeover: clean B waiting, new draft while waiting defers, Cancel, Apply+stable-Saved → adopt → reopen takeover value.
- `no-build` (`010-green-hold2.log`): A only, no waiting/Update Now, quick update check finds nothing, no reload, editor alive.
- Inherited (`010-green-inherit.log`): `adopt`, `second-editor` (other-tab defer + offline A-asset bytes), `subpath` (isolation), `failed-install` (script/asset, repair recovers), `unavailable-target` (report + retry, repaired B adopts) all pass on diagnostic builds (unarmed gate transparent).

Retry acceptance is **not satisfied** by those historical GREEN controls: their recovery uses `Reload saved version` (confirm `Discard your pending edits…`) then re-apply, whereas approved 003/006 require literal `Retry save`. The focused follow-up below resolves the durable-value ambiguity but exposes a Geometry owner settlement blocker. No product clarification/policy change is needed or claimed.

Original Layout blocker at `c899b0f6` (**superseded by the successful focused follow-up below**, retained as negative-selection evidence): disposable diagnostic A (site-010-ab/a) on isolated profiles/ports 4742–4743; Split 42 + Studio 60 demo copies; selections Main board / Left half / Left main / Column 1 / Key 1.1 / Outline; DOM queries for `[aria-label="Layout constraint"]`, `Apply position`, `Save constraint`, `Add constraint`, `Companion placement` all absent (`[]`); key/matrix/half inspectors show Position & rotation / Key size / Selected keys, never the 011 adapters. No Layout draft form found in demo via tree or workspace navigation. Proposed smallest next change: 011 owner supplies packaged UI steps for ONE Layout owner (L1 constraint, L8 position, or L15 companion) in a demo project (or a minimal fixture project with constraints), reusing the tracer leg shape; 010 then adds `layout-constraint` leg (payload `layout-constraint.js` already owned). Parent may assign a fresh issue agent; no scope expansion done here.

Historical cleanup evidence: sessions were closed and servers stopped; ports 4733/4740–4743 were reported free. Those observations alone **do not prove disk-profile/IndexedDB deletion**; the earlier stronger attestation is withdrawn. No live browser/profile/project was intended or authorized to be touched, and no cache/worker clearing was used for GREEN. Fresh explicit-path Layout cleanup proof is below; historical-profile cleanup remains a gate, not permission to delete unidentified state. Build outputs `web/target/site-010-src` + `site-010-ab` are retained (git-ignored) for coordinator reruns; diagnostic-output deletion remains DUE after acceptance. `.evidence/` contains logs, not source copies/targets/custom manifests.

Gortex: pre-edit impact (MEDIUM→CRITICAL lower bound, additive only, 0 existing callers changed); post-edit detect maps 2 files + 106 new symbols (helpers/legs/`pre010`/`LEGS`/`main`); tests 0/4 graph-tracked (driver exercised via packaged runs, logs above); guards no rules; contract `warn` (behavioral, blast reviewed, no parse errors). Payload JS files are untracked until commit (detect shows tracked modifications only).

Status stays CLAIMED: acceptance checkboxes unchanged pending fresh Spec+Standards reviews and coordinator independent reruns. Full gate/mutation belong to 015.

## Focused Layout follow-up (2026-11-07; CLAIMED)

Sole follow-up owner: implementation sub-agent (no delegates). Exact ownership
remains `scripts/verify-update-adoption.py`, its existing
`verify-update-adoption-payloads/layout-constraint.js`, and this ticket. No other
payload or production changes planned. Coordinator-owned untracked research is
untouched. Approved seam S2: normal `.boardstudio` import, production selection,
real Update Now, diagnostic persistence acknowledgement and normal reopen/export.

Reproduced absent form in packaged A, Split 42 → Main board:
`010-layout-missing-repro.log` reports `layout:0, buttons:[]`. Source then explains
eligibility: `presentation.rs::layout_component_inspector_projection` (1400+)
requires a current selected accepted part; board/half/matrix contexts cannot
mount it. Non-mirrored generated assembly members return None. Production
`WorkbenchApp` (6166+) passes the projection to `layout_workspace::inspector`
(942+), which mounts `LayoutComponentInspector`; the Layout constraint section
(2743+) is collapsed/keep-mounted and requires another board part to submit.

Material experiment succeeded: a ZIP containing only `project.json`, format v7,
one board, one controller definition and two standalone parts U1/U2; normal
`#library-import-file` upload → click U1's **Objects button ref** → Layout
constraint. No projected state, registry seed or synthetic mount. The first two
probe attempts used a canvas-text locator / unsupported text CSS and did not
select U1; the ref-driven attempt (`010-layout-mount-probe.log`) mounts the real
Component inspector and Add constraint. Disposable fixture and explicit profile
under a newly owned `/tmp/boardstudio-010-layout-*` directory removed after close;
actual profile contained `Default/IndexedDB`. Port 4744 free afterwards.

Layout leg added; initial `010-layout-red.log` fails exactly on toast deferral
against disposable *copies* of immutable 002 A/B. Current source-assets run
`010-layout-green2.log` is 0 failures: toast/header defer and Cancel preserves
7.25 + focus; Add constraint held at r1; release completed; normal portable
exports A/B both contain project `layout-010`, r1, framed source→target offset
(7.25, 0), rotation 0. Actual loaded document and requested application binary
are B, reload, B-only cache. Initial `010-layout-green.log` had one harness
assertion mistake (looked for legacy top-level offset instead of the production
framed relation); corrected to the actual source schema, no app change.

Cleanup follow-up exposed a real **driver** issue: `main` calls `end_session`
after the leg has already closed; agent-browser 0.39.1 can launch another daemon
on a second close, recreating the explicit disk profile after its first removal.
Observed the earlier GREEN paths `...-lmzgdk29` and `...-mmwbz50r` reappear. The
owned driver now records session closure idempotently, reports close errors,
and Layout waits for the exact owned `--user-data-dir` process to exit before
deleting its directory. No original leg is removed/changed; their shared
session lifecycle is affected; the required reruns are recorded below. Historical cleanup attestation
above is superseded: closing alone did not prove disk deletion.

**Literal Retry save blocker (not a product-policy question):** 003/006 require
Retry save. Packaged experiments `010-persist-fail-retry2.log` and
`010-persist-hold-fail-retry2.log` prove Retry holds/retries the intended r3,
real store completes, Saved returns, and normal export contains the original
`// FAIL-010` / `// HOLDFAIL-010` source at r3 (not the form's displayed baseline).
But the still-mounted Geometry form rolls back to the old r2 text on failure;
Retry's success does not resync it. The tighter no-Export experiment
`010-retry-direct-probe.log` then clicks Update Now and gets **Geometry scripts
has an unapplied edit**, remains A/B waiting. Thus direct retry→adoption is not
GREEN. This is an adapter/settlement gap, not durable data loss and not authority
to patch Rust. The attempted Retry changes to the two legs were withdrawn;
original Reload+re-apply controls remain intact, explicitly insufficient for
this mandatory Retry acceptance. The export-before-adoption experiments also
saw a nonacting toast after workspace navigation; that separate Toast concern
is not diagnosed or patched here.

Smallest ownership request: 008/Geometry owner investigates
`web/crates/ui-shared/src/geometry_scripts.rs::GeometryScriptsEditor`:
`pending.settle` (158+) rolls draft back on failure; owner identity effect
(252–289) excludes accepted revision/value, so retry's accepted value alone
cannot refresh the rolled-back clean form; registry (195–232) correctly detects
old local text != new accepted source. Preserve newer typing and refusal/failure
protection while making successful retry follow the intended accepted value.
Direct Retry reproducer (all production UI; gate is the approved diagnostic):

1. Open a Split 42 demo copy; Add object → Geometry scripts → + New script.
   Name `Tracer 010`, source `// 010 baseline`; Apply script (r2).
2. `window.__boardstudioPersistGate.arm_fail("diagnostic quota")`; type source
   `// 010 baseline\n// FAIL-010`; Apply script → Local save failed. For the
   hold-then-fail variant, arm that single mode, await entry, then release.
3. Publish real B at the same origin; toast Update Now defers failed save;
   Cancel. Arm hold; click literal Retry save; entry is same project r3.
4. Release → ack `completed` → Saved locally. Rhai source still displays r2.
5. Without Export/navigation/reselection/re-apply, click toast Update Now:
   warning names Geometry scripts unapplied edit; marker stays, A still loaded,
   B still waiting (`010-retry-direct-probe.log`).

Material next experiment: same packaged direct Retry reproducer after that
owner's focused RED/GREEN, assert Update Now adopts and B reopens exact r3/value;
then replace the existing two recovery routes, not a duplicate matrix.

### Final focused verification and reproducible commands

No compile was needed: reused `web/target/site-010-src` (diagnostic fast build
from `21d3e7a`); `git diff --name-only 21d3e7a..HEAD -- web core application`
was empty before this scripts/docs-only follow-up. Existing `stamp_build`
protocol is reused unchanged. Historical 002 directories remain read-only.

Final RED command (exit 1, exactly deferral + abort failures; setup passes):

```sh
python scripts/verify-update-adoption.py post \
  --src-dir web/target/site-002-ab/a \
  --work-dir web/target/site-010-layout-red-ab --port 4744 \
  --legs layout-constraint \
  --serve-log .evidence/update-draft-safety/010-layout-red-final-serve.log \
  > .evidence/update-draft-safety/010-layout-red-final.log 2>&1
```

This stamps fresh disposable A/B copies of the pre-adapter artifact, never
mutates 002. Root identities A `boardstudio-efc2829b36b3ccdf-1794075406701824019`,
B `boardstudio-f2b31c1f204ab7e7-1794075407548529039`. Expected failed assertion
`layout toast: unresolved draft blocks adoption`; actual state: B loaded,
`navType=reload`, marker gone, waiting cleared, no warning. Valid 7.25 draft and
zero invalid markers were proven before the click.

Final affected-controls command (exit 0, **12 legs, 0 failures**):

```sh
python scripts/verify-update-adoption.py post \
  --src-dir web/target/site-010-src \
  --work-dir web/target/site-010-layout-ab --port 4744 \
  --legs layout-constraint,geometry-tracer,persist-hold,persist-fail,persist-hold-fail,takeover-draft,no-build,adopt,second-editor,subpath,failed-install,unavailable-target \
  --serve-log .evidence/update-draft-safety/010-followup-affected-serve.log \
  > .evidence/update-draft-safety/010-followup-affected.log 2>&1
```

Root A `boardstudio-75d2e21c0706a3fb-1794075066652228230`, B
`boardstudio-c3b7f987b3af4ed4-1794075067449559274`; subpath A
`boardstudio-d57769c07ecd3ca9-1794075067036111041`, B
`boardstudio-329382aff8b86738-1794075067852262243`. Package diff assertions pass
unchanged. Layout held Add captures `document_id=layout-010, revision=1`, no ack;
release→`completed`, Saved; normal A export and actual B reopen/export both have
exactly the same document at r1 and framed constraint source U2→target U1,
offset (7.25,0), rotation 0. Requested binary B + loaded-document B + reload +
B-only cache pass. Original Geometry trusted keyboard checks, hold/reuse in B,
existing recovery controls and inherited five legs remain intact. Their GREEN
is not literal Retry acceptance (blocker above).

CLI lifecycle reference: installed agent-browser **0.39.1**, loaded via
`agent-browser skills get core --full` and `--help`; custom `--profile` persists
IndexedDB/service workers/cache, and `close` is not disk deletion. Current Layout
uses its own absolute `mkdtemp` profile, confirms close, waits until no process
has that exact `--user-data-dir`, observes `Default/IndexedDB`, then removes
only its own directory. Final GREEN path `/tmp/boardstudio-010-layout-yoigjwr6`
was absent after the entire CLI returned; final RED path also removed. Earlier
recreated owned paths were individually removed after confirming no matching
process (`010-layout-cleanup-red-paths.log`). Retry experiments used independent
owned profiles; `retry2` logs confirm no matching processes and full deletion.
The first Retry probe's deletion raced Chrome shutdown; its exact owned path
`/tmp/boardstudio-010-retry-5fg85qlf` was separately observed idle and removed.
`agent-browser session list` then reported no active sessions; port 4744 was free
after every final run. No cache/worker clearing or production hooks added.
Historical unknown-profile/disk cleanup and diagnostic-output deletion remain
DUE, explicitly not inferred from session close.

After strengthening close-error reporting, a final one-leg rerun also passed:
`post --src-dir web/target/site-010-src --work-dir web/target/site-010-layout-final-ab
--port 4744 --legs layout-constraint`, log `010-layout-final.log`, 0 failures.
Root A `boardstudio-75d2e21c0706a3fb-1794075663701435131`, B
`boardstudio-c3b7f987b3af4ed4-1794075664440587691`; same exact r1 document/constraint,
actual B identity and full owned-profile cleanup pass.

Focused static checks: `python -m py_compile scripts/verify-update-adoption.py`,
`node --check scripts/verify-update-adoption-payloads/layout-constraint.js`
and `git diff --check` PASS. Native Gortex detect maps these three files only;
checks by changed symbol IDs: tests 0/8 graph-covered (actual packaged execution
above is the verification seam), guards no configured rules, contract WARN
(behavioral, high/lower-bound blast; no parse-error finding). Pre-edit contract
attachment was explicitly uncertified/rebuilding; no stronger contract freshness
claim is made. Existing signatures remain unchanged. Full diff self-read at
handoff; no production Rust or other payload changed, and coordinator-owned
untracked research remains untouched/uncommitted.

Optimized behavior is not yet validated. Fresh Spec+Standards review and
coordinator independent reject-control reruns/full 015 gate remain DUE. No
acceptance boxes advanced; 010 remains CLAIMED with the Retry blocker.

## Acceptance criteria

- [ ] The packaged regression assertion from [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md) is RED before the tracer fix is present in the packaged build and GREEN after, with A/B build identities recorded.
- [ ] Toast entry point: trusted pointer and trusted Tab/Enter/Space defer; Cancel preserves the exact draft text and the running build stays A.
- [ ] Header entry point after toast dismissal defers the same draft the same way.
- [ ] Held-persistence legs: with the diagnostic adapter armed in A, Update Now defers as save-in-progress with no durable change; after release and acknowledgement, adoption proceeds; the same gate is consumed again after takeover in B. Fail and hold-then-fail legs defer as did-not-complete and recover via explicit retry.
- [ ] Exactly one packaged new-draft-during-takeover leg, reusing the tracer scenario's shape: a draft created while adoption waits defers Update Now again.
- [ ] Loaded-B proof: the loaded document and the served binary both identify B (not manifests or labels), the navigation is the adoption reload, and the reopened project holds the intended value at the intended revision.
- [ ] Sole-client/other-tab, offline-asset continuity and subpath-isolation legs inherited from the parent adoption acceptance pass; failed-install and no-new-build negative controls do not adopt.
- [ ] Disposable fixtures, profiles and databases are cleaned up; evidence logs are stored per the plan's evidence conventions. This ticket does not close the parent adoption umbrella.
