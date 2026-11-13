# 010: Packaged A→B draft-safety proof

Status: resolved
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Assignee: Coordinator (010 verification)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md), [Diagnostic gated-persistence adapter for packaged builds](009-diagnostic-gated-persistence-adapter.md)

## Focused Retry escalation ownership (2026-11-07; CLAIMED)

Sole implementation/required diagnostic packaged-build/browser owner from
`99eafb6447ef389fc651f8200addf7abcf2171b5`. Own existing
`scripts/verify-update-adoption.py`, directly necessary existing 010 payloads
(none changed yet), this ticket and 008, plus Geometry production/local tests
as recorded in 008. Literal Retry save is mandatory: replace the two existing
Reload+re-apply routes, not an added duplicate matrix. Pre-fix
`web/target/site-010-src` and A/B were preserved for RED/coordinator reruns;
corrected output was separate. These disposable outputs were removed only after
verification completed, as attested in the final Answer. No other owner/Runtime/
registry changes or subagents. Preserve dirty
`CODING_STANDARDS.md` and coordinator untracked research.

The Retry correction is committed as `be39738b`; coordinator verifier follow-ups
through `c5a876ec`, corrected packaged checks and complete independent code reviews
pass as recorded below. Historical Reload + re-apply controls still do not count
as Retry acceptance. Required cleanup is now complete, all eight criteria are
accepted, and the final Answer supersedes historical CLAIMED/blocker statements.
Optimized validation and final 015 obligations remain due, not waived.

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
- `docs/plans/update-draft-safety/issues/010-packaged-a-to-b-proof.md` (this ticket; progress/transfer record)
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
`agent-browser guides get core --full` and `--help`; custom `--profile` persists
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
transfer; no production Rust or other payload changed, and coordinator-owned
untracked research remains untouched/uncommitted.

That focused follow-up stopped at `99eafb644` with Retry blocked. The later
correction and coordinator evidence below supersede that blocker, not the
optimized/cleanup/final-gate debts.

## Coordinator verification and proof follow-up (2026-11-07; CLAIMED)

- Source correction: `be39738b10b985d0529b6ee53a1677f513502b4d` reconciles the
  still-clean Geometry form after literal Retry while preserving newer typing.
  Existing fail/hold-fail legs now perform **Retry save → direct Update Now**,
  without Export/navigation/reselection/re-apply repairing the form first.
- Fresh independent Pi Contributor medium Spec and Standards reviews of
  `21d3e7a706868afbe269266cd03e37d7dcbb6143...be39738b10b985d0529b6ee53a1677f513502b4d`
  report **PASS**, no required changes. Spec task `26ee2d43-6db3-45fc-a273-3fef65d6b7d0`;
  Standards `112978f3-03d1-429c-b267-dec2c3319e6e`. Standards explicitly reviewed
  only part of the large driver; neither report certifies structural freshness.
  Coordinator subsequently read the **complete combined changed source**.
- Independent mounted verification: Geometry **6/6**, pending helpers **16/16**,
  notifications **30 executed / 0 failed**, logs
  `008-coordinator-retry-{geometry,helpers,notifications}.log`.
- Independent package run: `post --src-dir web/target/site-008-retry-src
  --work-dir web/target/site-010-parent-retry-ab.6OMkRE --port 4744
  --legs persist-fail,persist-hold-fail,geometry-tracer,persist-hold,layout-constraint`,
  `010-coordinator-retry.log`: **five legs, zero failures**. Root A
  `boardstudio-f8d3dd85e0a765d4-1794079543893225514`, B
  `boardstudio-39e57057593af741-1794079544682455057`. Literal Retry's reopened normal
  exports independently prove projects `cb00e22a-1935-49d9-8b00-cab6f9fd570e` and
  `cbdecba3-f40d-4972-ade4-9adfe4b0d7bf`, both r3 with their exact intended source.
  Layout's A/B documents are equal at `layout-010`, r1, U2→U1 offset (7.25,0).
  Owned Layout profile `/tmp/boardstudio-010-layout-1jsnu1h_` was observed with
  IndexedDB, its Chrome process exited, and its directory was deleted; sessions
  ended and port freed. This does not prove deletion of unidentified old profiles.
- Full read found two small **verifier gaps**, not production failures: header
  Space activated Cancel but not Update Now; tracer/hold printed the captured
  revision without independently reading B's reopened revision. Coordinator added
  the missing header Space → deferral → Enter Cancel steps and reused the existing
  export oracle (renamed `check_reopened_geometry_save`) for tracer and hold as well
  as Retry. All four establish expected r3 separately, then check exact project,
  source and revision. Hold still consumes the gate again in B. No new matrix,
  production API or app build. The export oracle rejects wrong project, revision
  and source in `010-coordinator-export-oracle.log`.
- Tightened affected rerun: same src, `--work-dir
  web/target/site-010-parent-oracles-ab.BDadHu --port 4744 --legs
  geometry-tracer,persist-hold,persist-fail,persist-hold-fail`,
  `010-coordinator-oracles.log`: **four existing legs, zero failures**, including
  header Space deferral and all four exact B exports at r3. Root A
  `boardstudio-f8d3dd85e0a765d4-1794080101343591108`, B
  `boardstudio-39e57057593af741-1794080102196326775`. Python compile/diff checks pass.
  Native impact/detect/tests/guards/contract run; no graph-covered tests or guards,
  contract WARN (behavioral lower-bound blast). Signature verification's claimed
  old arity 1 is lossy Python extraction: actual old and renamed helper both have
  the same four parameters; both original callers were updated and executed.
- **Timing adjudication and correction:** fresh Pi Sol/high task
  `e48884a1-159e-41c0-a074-a2989bcf07cb` confirmed that the original takeover leg
  typed before its first Update Now attempt and missed story 23's held-save
  timing. It made no edits: three bounded native impacts returned `view_building`.
  Coordinator's later native impact/edit was admitted; the existing single leg
  is now strengthened as described below. No production defect is inferred.

### Held-save waiting-intent proof (coordinator, 2026-11-07)

The single `takeover-draft` now holds a real Geometry Apply at r3, attempts
Update Now and observes save-in-progress deferral, cancels, then enters newer
valid text **while that earlier save remains held and unacknowledged**. Releasing
r3 preserves the newer text; after Saved, a second Update Now attempt names the
Geometry draft and defers again, so Saving cannot mask a broken draft guard.
Cancel preserves exact text/focus/build. Applying the newer text captures the
same project at independently expected **r4**, completes, then directly adopts B
and checks the normal reopened export's exact project/source/revision.
This is the specified held-save/waiting-intent story, **not** a post-permit
worker-probe race. S1 remains primary for the true takeover window; existing
`mounted_work_begun_during_the_takeover_delays_the_reload_until_it_settles`
(`notifications.rs:1262`) covers pending Core work, not specifically Geometry.
No new production timing hook, registry/API change or duplicate leg was added.
Current behavior passes; no artificial production RED is claimed.

- `010-coordinator-takeover.log`: `--src-dir web/target/site-008-retry-src
  --work-dir web/target/site-010-parent-takeover-ab.Rag4QM --port 4744 --legs
  takeover-draft,layout-constraint`, **two legs, zero failures**. Root A
  `boardstudio-f8d3dd85e0a765d4-1794081453072804692`, B
  `boardstudio-39e57057593af741-1794081453862773779`. Reopened project
  `ab51d143-82ac-40cb-9e0f-1efd8d820706`, r4, exact `// TAKEOVER-HELD-010`
  plus `// TAKEOVER-NEWER-010` source; actual document/requested binary B, reload,
  B-only cache. Layout still has equal A/B r1 documents.
- Both legs use explicit owned profiles. The existing Layout cleanup is shared
  rather than copied: close once, await the exact owned profile's Chrome exit,
  then delete only its fixture/profile directory. `start_session` accepts an
  optional private-harness profile path; existing three-argument calls retain
  their behavior. Both `/tmp/boardstudio-010-takeover-yi6h_ze3` and
  `/tmp/boardstudio-010-layout-v9bpr3pq` contained IndexedDB and were removed after
  no matching Chrome remained. `010-coordinator-profile-controls.log` verifies
  default/explicit startup compatibility and rejection of directory deletion
  when close is unconfirmed or the owned Chrome is still live.
- Because startup is shared, the five inherited adoption controls plus no-build
  were rerun on the corrected app: `010-coordinator-controls.log`, **six legs,
  zero failures**, `--work-dir web/target/site-010-parent-controls-ab.v8njsk
  --port 4744 --legs adopt,second-editor,subpath,failed-install,unavailable-target,no-build`.
  Root A `boardstudio-f8d3dd85e0a765d4-1794081522695393567`, B
  `boardstudio-39e57057593af741-1794081523474864779`. Other-editor deferral,
  offline A bytes, subpath isolation, failed-install/no-build non-adoption and
  repaired target adoption pass. No new app build or worker/cache cleanup.
- Native impact/verify/detect/tests/guards/contract ran. Coverage is actual
  packaged execution, not graph test edges (0/4); no guards, contract WARN,
  lower-bound shared-startup blast. The parser's old arity 1 is lossy; the real
  helper changes from three parameters to four with a default, and compatibility
  probes plus existing packaged three-argument callers pass. Complete independent
  combined rechecks through `c5a876ec` are recorded below.

### Independent combined rechecks (2026-11-07)

Pinned range: `21d3e7a706868afbe269266cd03e37d7dcbb6143` →
`c5a876eca89bca50f5bb72835b5c07c531ae5f93`, including both coordinator
verifier corrections. Fresh Pi Contributor/medium, read-only:

- **Standards PASS**, task `e7e3846c-37d3-47ed-b122-ae6d86ac2a77`:
  complete changed-hunk coverage, all six payloads, Geometry effect/two tests and
  receipt deltas; no Required changes. Driver size is an optional/FYI inspection
  signal, not authorization for cleanup or a framework extraction.
- Initial Spec task `606c173e-4055-4dcc-bf78-89eb3064527a` found no error in
  the latest follow-ups but disclosed unread driver/governing-ticket ranges.
  Its conditional result is **not** counted as the complete combined review.
- **Complete Spec code review: correct in-scope behavior; CHECKPOINT BLOCKED**,
  fresh task `6ec0a0e3-cf9e-44d7-b03c-b9dce5603cc4`. Read the entire added driver
  cluster/CLI, all six payloads, Geometry effect/two tests, and governing
  003/006/009/015. Stories 8/14/15/23/25 and S2 are implemented without scope creep.
  Required remaining gate: this ticket's cleanup criterion and 003's deletion
  protocol. Only the two explicit profiles above have complete removal evidence;
  unidentified historical/default profiles and retained diagnostic output do not.

All reviewers changed no files and ran no builds/browser checks/mutations;
coordinator experiments above remain the behavioral evidence. No structural
attachment freshness certification or post-permit Geometry race proof is inferred.
JS focus/update-check setup is not a substitute for trusted activation.

These runs used diagnostic **fast** assets, not proof of optimized/wasm-opt
equivalence. The sole cleanup blocker at the review cut is now discharged by the
coordinator's final Answer below, without a production correction or a fabricated
new reviewer verdict. Optimized validation, all-14 owner coverage, 015 mutation/
full gate and final closure review remain later obligations. No 012–014 owner
expansion or parent umbrella closure is claimed by this checkpoint.

## Acceptance criteria

- [x] The packaged regression assertion from [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md) is RED before the tracer fix is present in the packaged build and GREEN after, with A/B build identities recorded.
- [x] Toast entry point: trusted pointer and trusted Tab/Enter/Space defer; Cancel preserves the exact draft text and the running build stays A.
- [x] Header entry point after toast dismissal defers the same draft the same way.
- [x] Held-persistence legs: with the diagnostic adapter armed in A, Update Now defers as save-in-progress with no durable change; after release and acknowledgement, adoption proceeds; the same gate is consumed again after takeover in B. Fail and hold-then-fail legs defer as did-not-complete and recover via explicit retry.
- [x] Exactly one packaged new-draft-during-takeover leg, reusing the tracer scenario's shape: a draft created while adoption waits defers Update Now again.
- [x] Loaded-B proof: the loaded document and the served binary both identify B (not manifests or labels), the navigation is the adoption reload, and the reopened project holds the intended value at the intended revision.
- [x] Sole-client/other-tab, offline-asset continuity and subpath-isolation legs inherited from the parent adoption acceptance pass; failed-install and no-new-build negative controls do not adopt.
- [x] Disposable fixtures, profiles and databases are cleaned up; evidence logs are stored per the plan's evidence conventions. This ticket does not close the parent adoption umbrella.

## Answer

Coordinator acceptance (2026-11-07): all eight criteria are met. The complete
independent reviews and coordinator functional evidence above stand; cleanup was
the sole checkpoint blocker. Resolving it does not turn a conditional review
snapshot into a new reviewer PASS or establish optimized behavior.

### Final cleanup attestation

- Installed package/binary report **agent-browser 0.39.1**. Its matching release
  [temporary-profile implementation](https://github.com/vercel-labs/agent-browser/blob/v0.39.1/cli/src/native/cdp/chrome.rs)
  creates `agent-browser-chrome-<UUID>` without a configured profile and removes
  it in `ChromeProcess::drop` after terminating/waiting for owned Chrome;
  [BrowserManager close](https://github.com/vercel-labs/agent-browser/blob/v0.39.1/cli/src/native/browser.rs)
  awaits that owned process lifecycle. Configured persistent paths are different
  and still require explicit owned-path cleanup. This is a source-contract check,
  not a binary-provenance certification.
- A bounded live check reused the existing normal Layout archive/import and
  server helpers, with a unique private session and **no profile/restore/CDP
  override or config**. The CLI CDP endpoint matched the new profile's
  `DevToolsActivePort`; `ps` observed the exact profile's live Chrome processes.
  Import reached Saved and created real `Default/IndexedDB`. After acknowledged
  close, every exact-profile Chrome process was gone and the whole default
  profile/IndexedDB directory was absent, with **no manual profile deletion**.
  The owned import fixture was removed and port 4744 was free.
  Log: `010-cleanup-lifecycle.log` (serve log alongside).
- Two earlier ownership-instrumentation setups failed before import because
  environment/procfs PID matching was insufficient; both closed and removed their
  fixture. They are retained in `010-cleanup-lifecycle-setup.log` and
  `010-cleanup-lifecycle-endpoint-setup.log`, not counted as behavioral RED or
  silently discarded. The successful check used the existing driver's `ps` seam.
- `010-cleanup-attestation.log` inventories **33 recorded temporary fixture,
  profile and export roots**, all independently absent, including the old
  duplicate-close Layout paths and the latest explicit takeover/Layout profiles.
  Historical default profile addresses were not logged per run; no retrospective
  PID claim is made. Read-only scans under both actual temp roots, `/tmp` and
  `/home/chris/.cache/tmp`, found **no remaining** version-specific default/copied
  browser profile directories or `boardstudio-010-*` roots. No unidentified or
  live profile was deleted. Session metadata reports no active sessions.
- The same log records original ownership receipts, ignored/untracked-source
  checks, expected package-directory shapes and absence of process-argv users
  **before** deleting these 15 exact disposable output roots under `web/target/`:
  `site-010-src`, `site-010-ab`, `site-010-layout-ab`, `site-010-retry-ab`,
  `site-010-layout-red-ab`, `site-010-layout-final-ab`, `site-008-retry-red-ab`,
  `site-008-retry-red-final-ab`, `site-008-retry-src`, `site-008-retry-green-ab`,
  `site-008-retry-layout-ab`, `site-010-parent-retry-ab.6OMkRE`,
  `site-010-parent-oracles-ab.BDadHu`, `site-010-parent-takeover-ab.Rag4QM`,
  `site-010-parent-controls-ab.v8njsk`. A separate post-cleanup check confirmed
  all 15 absent, immutable `site-002-ab` retained and port 4744 free.
  Cargo/provider caches and unrelated files were not touched. Evidence logs remain
  in `.evidence/update-draft-safety/`, with no source copies or custom manifests.

The 008/009 dependencies and reviewed 011 slice are resolved alongside this
composite acceptance. Next is 012 under the approved execution order, not another
packaged matrix or optional driver cleanup. The reproducible commands above now
require regenerating deleted disposable outputs; 015 must build fresh assets for
optimized/full verification. All-14 coverage, mutations, full gate/final reviews,
Class B risk tracking and umbrella 019 remain open.
