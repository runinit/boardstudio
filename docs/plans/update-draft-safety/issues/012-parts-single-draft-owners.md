# 012: Parts single-draft owners

Status: resolved
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the Parts single-draft
editors sharing the one-Save/Apply pattern: generator parameters (including
its Upload admission key), the module profile editor, and the mechanical fit
profile editor. Their unapplied Draft values defer Update Now until resolved
through each editor's own controls.

## Acceptance criteria

- [x] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [x] Generator parameters are judged per admission key: submitting one key (Apply or Upload) does not clear an unresolved draft belonging to the other.
- [x] Each deferral message names its editing area and per-key remedy; resolution happens only through that editor's own controls.
- [x] Per-key behaviour follows the contract: landing updates the baseline, failure/refusal never clears, revert-to-baseline clears, newer typing re-raises, older settlement never clears newer input.
- [x] Hidden or keep-mounted instances keep blocking; unmount or owner change retires them.
- [x] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map). Starts only after the
approved verification-reliability preparation finishes
([execution](../execution.md#verification-reliability-preparation)). The rows restate the criteria
above per owner and add none. The shared gate, epoch and lifetime mechanics are proven at the 008
and 011 seams, so each row proves owner wiring only. Owners: generator parameters (Apply and Upload
keys), module profile, mechanical fit profile. Tier `mounted` (Update Now seam); every row
`planned`.

- P1 Deferral — a real valid unapplied draft, or a realistic invalid unsubmitted draft the owner's
  own submit would not accept → Update Now from the toast and from the header → the update defers
  and the draft value is untouched; the message names the editing area and the per-key remedy. Only
  the editor's own controls resolve it. Oracle: deferral state, message against the settled
  per-(owner, key) copy, unchanged draft. One scenario per owner; draft kind and entry point are
  table cases.
- P2 Generator admission keys — drafts unresolved under both Apply and Upload → submit one key and
  land it → the other key still defers with its own remedy. Oracle: the remaining deferral names the
  unsubmitted key only. Both directions (Apply→Upload, Upload→Apply).
- P3 Per-key lifecycle wiring — submission in flight → landing, failure or refusal, revert to
  baseline, newer typing before the older settlement → landing re-baselines, failure or refusal keeps
  deferring, revert clears, newer typing re-raises and the older settlement never clears it. Oracle:
  deferral state after each outcome. Owner specifics: module profile's one Save click admits two
  collections (profile and rotary), and either older settlement must leave newer input deferring;
  text an owner parses before submit compares by the value it would submit.
- P4 Retention — owner hidden but mounted → Update Now → still defers; owner unmounted or
  replaced → no deferral from the retired entry. Oracle: deferral present, then absent.
- P5 Adapter RED/GREEN — each adapter's rows run RED then GREEN through the mounted seam. A row the
  existing product already passes is a `proof gap` recorded GREEN, not a fabricated RED.

## Progress (first RED, 2026-11-08; NOT resolved — awaiting GREEN approval)

First vertical slice claimed: generator Apply valid draft at toast (P1 generator row, first case).
No production behavior changed; no GREEN implemented; no bulk tests written.

- Test: `mounted_generator_valid_apply_draft_defers_update_now_at_the_toast_012` in
  `web/crates/parts/src/parts/generator_settings.rs` tests. Reuses existing
  `mount_generator_editor`, `dispatch_width`, `take_request` fixtures; adds minimal
  `FakeUpdateSignals012` (no Update Now seam existed in parts crate) driving the real
  `Runtime::begin_update(UpdateOpener::Toast)` with a live mounted owner. Baseline
  keycap_width 18.0; draft "20" is real valid unapplied. Preview requests drained with
  Err (outcome irrelevant to gate); width request arrival proves input processed before
  the gate poll, so RED is missing-adapter, not race.
- Oracle (independent): deferral state Deferred, single `UnresolvedDraft` area
  "Generator settings" remedy "Apply the generator settings, or restore its saved
  values" (area per 004 N4; remedy follows 011 L15 own-controls pattern), zero
  adoptions, draft preserved "20".
- RED command (serialized): `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test
  --headless --chrome --mode no-install web/crates/parts --lib --
  mounted_generator_valid_apply_draft_defers_update_now_at_the_toast_012 --nocapture`.
  Log: `.evidence/update-draft-safety/012-red-generator-apply-toast.log`. Result: 1
  executed, 1 failed at `expected a deferred update dialog, got None`
  (`generator_settings.rs:2781`), not a compilation/setup failure. No adapter registered
  (verified: `register_draft_owner` only at 4 Geometry/Layout sites), so the gate adopts
  instead of deferring. P1 generator row status: `RED` (first case); all other rows remain
  `planned`.
- Gortex: pre-edit impact MEDIUM (lower-bound); post-edit detect maps the new test symbol;
  tests 0/1 graph coverage (graph blind to wasm_bindgen_test, as 008/011); guards no rules;
  contract allow LOW, test-only. `git diff --check` PASS. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked `docs/research/2026-11-07-ai-agent-test-economy.md`.
- Proposed smallest GREEN next (awaiting approval): register generator Apply key in
  `GeneratorSettingsEditor` with per-key unresolved predicate (live edits vs live accepted
  parameters, owner-canonical; in-flight Apply exemption via `submitted_draft`), area/remedy
  above, RAII guard + retire on unmount/scope change, poll-prune safe. Then rerun same test
  GREEN; do not expand to Upload/header/invalid/lifecycle until this slice is GREEN and reviewed.

## Progress (first GREEN slice, 2026-11-08; NOT resolved — awaiting review)

First vertical slice GREEN: same generator Apply valid draft at toast, no test changes.

- Production (`web/crates/parts/src/parts/generator_settings.rs`, +149/-0, all outside
  `mod tests`): new private `generator_apply_draft_differs` reusing `apply_input` coercion +
  `effective_value` baseline (invalid text and unknown keys defer fail-closed; empty map
  resolves), and an Apply admission-key registration in `GeneratorSettingsEditor`
  (scope/owner/schema-rotated effect, RAII guard + `retire_draft_owner` on unmount/scope
  change, poll-prune safe). Area "Generator settings", remedy "Apply the generator settings,
  or restore its saved values".
- Predicate refinement vs the RED proposal: one-shot `begin_one_shot` stores no submitted
  text, so `submitted_draft` cannot exempt Apply; the in-flight check is the Apply intent's
  draft sequence (pending + no newer typing means the live edits are exactly what was
  submitted, fail-closed). Terminal records never exempt (`is_pending` false);
  revert-to-baseline resolves by canonical equality; live accepted projection every poll with
  mount-base fallback only where no accepted definition exists. No Pair-exception expansion,
  no Upload/header/invalid/lifecycle change, no Apply policy change, 004 authority unchanged.
- GREEN command (serialized, identical to RED): `flock ~/.cache/tmp/boardstudio-build.lock
  wasm-pack test --headless --chrome --mode no-install web/crates/parts --lib --
  mounted_generator_valid_apply_draft_defers_update_now_at_the_toast_012 --nocapture`.
  Log: `.evidence/update-draft-safety/012-green-generator-apply-toast.log`. Result: 1 passed,
  0 failed, 72 filtered out, exit 0. `rustfmt --check --edition 2024` PASS on the touched
  file; `git diff --check` PASS. P1 generator row status: `GREEN` (first case only: valid
  Apply draft at toast); all other rows remain `planned`.
- Gortex: pre-edit impact MEDIUM (lower-bound); post-edit detect maps
  `generator_apply_draft_differs` + `GeneratorSettingsEditor`; tests 0/2 graph coverage
  (graph blind to wasm_bindgen_test, as 008/011 — behavioral proof is the mounted run
  above); guards no rules; contract WARN (behavioral, blast 12 lower-bound, no parse errors,
  no signature change). Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`.
- Proposed smallest next slice (awaiting approval): header entry-point case of the same P1
  generator row (same draft via `UpdateOpener::Header`), RED then GREEN with no production
  change expected; then invalid/unsubmitted draft, then P2 Upload key. Do not bulk-write tests.

## Progress (second slice header GREEN proof gap, 2026-11-08; NOT resolved — awaiting review)

Second P1 case GREEN as proof gap: same generator Apply valid draft at header, no production change.

- Test: `mounted_generator_valid_apply_draft_defers_update_now_at_the_header_012` in
  `web/crates/parts/src/parts/generator_settings.rs` tests. Mirrors the toast test with
  only the entry point parameterized (`UpdateOpener::Header` vs `Toast`); reuses
  `mount_generator_editor`, `dispatch_width`, `take_request`,
  `take_initial_default_request`, `FakeUpdateSignals012`, `waiting_update_012`.
  Baseline keycap_width 18.0; draft "20" is the same real valid unapplied Apply draft.
  Distinct failure story is header-checkpoint wiring.
- Oracle (independent, same shape as toast): deferral state Deferred, single
  `UnresolvedDraft` area "Generator settings" remedy "Apply the generator settings,
  or restore its saved values", zero adoptions, draft preserved "20". No existing
  oracle weakened.
- GREEN-gap command (serialized, test-first pre-prod run): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_generator_valid_apply_draft_defers_update_now_at_the_header_012 --nocapture`.
  Log: `.evidence/update-draft-safety/012-header-generator-apply.log`. Result: 1 passed,
  0 failed, 73 filtered out, exit 0. No production edit made; this passing run is the
  gap evidence, never a fabricated RED.
- RED-or-gap reason: both Update Now entry points poll the same draft registry via
  `Runtime::update_safety_inputs` (verified: `runtime.rs:974` polled at `1010`
  inside `begin_update` and at `1069` in `reload_when_safe`; `begin_update(opener)`
  at `997` stores opener at `1012`/`1017` for focus return only; `update_safety.rs`
  `UpdateOpener::{Toast,Header}` doc is focus-only and `update_safety` is
  opener-independent). Header defers identically to toast on the existing adapter,
  so no adapter gap exists for this entry point — a proof gap, recorded GREEN.
- Gortex: pre-edit impact file_not_indexed lower-bound (x2, as first slice);
  post-edit detect maps new header test at `2952`; tests 0/1 graph coverage
  (graph blind to wasm_bindgen_test, as 008/011/first slice — behavioral proof is
  the mounted run above); guards no rules; contract allow LOW, test-only, blast 0,
  no parse errors, no signature change. `rustfmt --check --edition 2024` PASS on the
  touched file; `git diff --check` PASS. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority
  unchanged; no Pair-exception expansion, no Apply policy change.
- P1 generator row status: `GREEN` (two cases: valid Apply draft at toast + header);
  invalid/unsubmitted draft, Upload key, lifecycle, retention, and other owners remain
  `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): invalid/unsubmitted draft case of the
  same P1 generator row (realistic invalid text the Apply would not accept, at toast),
  RED then GREEN; then P2 Upload key. Do not bulk-write tests.

## Progress (third slice invalid GREEN proof gap, 2026-11-08; NOT resolved — awaiting review)

Third P1 case GREEN as proof gap: realistic invalid unsubmitted generator Apply
draft at toast, no production change.

- Test: `mounted_generator_invalid_apply_draft_defers_update_now_at_the_toast_012` in
  `web/crates/parts/src/parts/generator_settings.rs` tests. Mirrors the toast valid
  case; reuses `mount_generator_editor`, `dispatch_width`, `take_initial_default_request`,
  `FakeUpdateSignals012`, `waiting_update_012`. Adds minimal `take_raw_width_request`
  (raw-string variant of `take_request`): the width control is a native number input,
  which sanitizes non-numeric text, so the realistic invalid draft here is the cleared
  field ("") and the f64-parsing wait cannot observe it. No submit is attempted.
  Distinct failure story is invalid-unsubmitted deferral (fail-closed comparison,
  never-submit path).
- Invalid value: `""` (cleared width field; baseline keycap_width 18.0). The owner's
  own Apply rejects it: `apply_input` number branch yields `None` for empty text and
  returns `Err("keycap width must be a finite number.")`, so the preview fails and
  Apply stays disabled (`on_apply` early-returns without a Ready candidate). A
  distinctive non-numeric literal cannot exist in this control: run #1 dispatched
  "1e999" and the standard wait found no inf-parseable preview request within 150
  polls (see `012-invalid-generator-apply-toast.log`), confirming sanitization.
- Oracle (independent): deferral state Deferred, single `UnresolvedDraft` area
  "Generator settings" remedy "Apply the generator settings, or restore its saved
  values", zero adoptions, cleared draft preserved ("", not the "18" baseline), and
  `aria-invalid != "true"` on the width field at gate time — deferral does not depend
  on any invalid-field marker (generator fields carry none, census A1). No existing
  oracle weakened.
- GREEN-gap commands (serialized, test-first pre-prod runs, no production edit):
  `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_generator_invalid_apply_draft_defers_update_now_at_the_toast_012 --nocapture`.
  Run #1 log `.evidence/update-draft-safety/012-invalid-generator-apply-toast.log`:
  test-mechanism FAIL (inf wait timeout, gate never reached), exit 1 — not a product
  RED, so no production edit. Runs #2 (cleared-field rewrite) and #3 (post-rustfmt
  final text) log `.evidence/update-draft-safety/012-invalid-generator-apply-toast-cleared.log`:
  1 passed, 0 failed, 74 filtered out, exit 0 each. Trio check
  `.evidence/update-draft-safety/012-generator-apply-trio.log` (filter `012`): 3 passed,
  0 failed, 72 filtered out, exit 0 — both prior 012 oracles still GREEN.
- RED-or-gap reason: `generator_apply_draft_differs` already fail-closes on `apply_input`
  error (`generator_settings.rs:323-325` returns true), and "" is rejected, so the
  existing adapter defers with no gap — a proof gap, recorded GREEN. Verified by running,
  never assumed; never a fabricated RED.
- Gortex: pre-edit impact file_not_indexed lower-bound (as prior slices); post-edit
  detect maps the new test (line 3032) + `take_raw_width_request` (line 1883); tests 0/1
  graph coverage each (graph blind to wasm_bindgen_test, as 008/011/prior slices —
  behavioral proof is the mounted runs above); guards no rules; contract allow LOW
  test-only blast 0 for the test, WARN (new-helper caution, blast 3 lower-bound) for
  the helper, no parse errors, no signature change. `rustfmt --check --edition 2024`
  PASS on the touched file (one formatting fix applied, rerun GREEN); `git diff --check`
  PASS. Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority unchanged;
  no Pair-exception expansion, no Apply policy change.
- P1 generator row status: `GREEN` (three cases: valid Apply draft at toast + header,
  invalid unsubmitted Apply draft at toast); invalid at header, Upload key, lifecycle,
  retention, and other owners remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): P2 Upload admission key
  (Apply→Upload direction first), RED then GREEN; then P3 lifecycle. Do not bulk-write tests.

## Progress (fourth slice P2 Apply→Upload GREEN, 2026-11-08; NOT resolved — awaiting review)

Fourth slice GREEN: generator Apply→Upload per-admission-key independence. Apply
landing preserves a newer Upload draft; the remaining deferral names Upload only.

- Test: `mounted_generator_apply_landing_preserves_upload_draft_012` in
  `web/crates/parts/src/parts/generator_settings.rs` tests. Reuses
  `mount_generator_editor`, `dispatch_width`, `take_request`,
  `take_initial_default_request`, `candidate`, `wait_for_text`, `mounted_value`,
  `FakeUpdateSignals012`, `waiting_update_012`, `support::gate_next_core_reply` /
  `drive_pending` / `run_pending`. Steps: valid Apply width "20" (baseline 18.0)
  made Ready, Apply submitted and held in-flight, Upload draft created while
  in-flight via production Remove path (readonly `switch_3dmodel_filename` cleared
  to ""; preview-request arrival proves processing), older Apply landed (accepted
  width 20.0 asserted), drafts preserved ("20" + ""), then Update Now at toast.
  Distinct failure story is Apply→Upload independence (one key in flight never
  exempts the other; older settlement never clears newer input).
- Oracle (independent): deferral Deferred, single `UnresolvedDraft` area
  "Generator settings" (per 004 N4, same area for both keys) remedy "Upload the
  generator model, or restore its saved values" (per-key copy per 004 B2/B5;
  follows Apply own-controls pattern, names Upload admission), zero adoptions,
  width "20" + filename "" preserved. No existing oracle weakened.
- RED command (serialized, test-first pre-prod): `flock ~/.cache/tmp/boardstudio-build.lock
  wasm-pack test --headless --chrome --mode no-install web/crates/parts --lib --
  mounted_generator_apply_landing_preserves_upload_draft_012 --nocapture`.
  Log: `.evidence/update-draft-safety/012-p2-apply-upload-red.log`. Result: 0
  passed, 1 failed, 75 filtered out, exit 1 at remedy mismatch (got Apply remedy,
  want Upload remedy) — no Upload registration exists, so Apply (judging all keys)
  defers instead of Upload. Not setup: Apply landed, drafts preserved, gate
  deferred with 1 blocker, only the key was wrong.
- Production (`web/crates/parts/src/parts/generator_settings.rs`, outside `mod
  tests`): partitioned per-key predicates + Upload registration. New
  `generator_is_upload_key` (`3dmodel_filename` suffix, matching Upload admission);
  `generator_apply_draft_differs` now judges non-Upload keys only (filtered
  `apply_input` to avoid Upload-invalid coupling); new `generator_upload_draft_differs`
  mirrors it for Upload keys only; new `generator_values_equal` compares number
  params as f64 (saved ints vs coerced floats hold the same value; strict `Value`
  equality false-deferred unedited `keycap_height` 18 vs 18.0 after every Apply —
  exposed by the first GREEN attempt yielding 2 blockers, see
  `012-p2-apply-upload-debug.log`, fixed here, debug removed). Upload registration
  in `GeneratorSettingsEditor` (same scope/owner/schema effect, second RAII guard +
  retire on unmount/scope change, poll-prune safe): live accepted projection per
  poll, mount fallback only where no accepted definition exists, Upload in-flight
  check via `is_pending(Upload)` + intent sequence (sequence 0 never matches a live
  draft generation, so fail-closed — an Upload submits an asset id, never the
  cleared text; terminal never exempts, newer typing re-raises). Area/remedy above.
  No Apply policy change (Apply still submits all), no Pair-exception expansion,
  004 authority unchanged, no Upload→Apply/P3/retention/other-owner change.
- GREEN command (serialized, identical to RED): same flock wasm-pack line.
  Log: `.evidence/update-draft-safety/012-p2-apply-upload-green.log`. Result: 1
  passed, 0 failed, 75 filtered out, exit 0. `rustfmt --check --edition 2024` PASS
  (two formatting fixes via native edits); `git diff --check` PASS.
- Regression (3 prior P1 oracles): `flock ~/.cache/tmp/boardstudio-build.lock
  wasm-pack test --headless --chrome --mode no-install web/crates/parts --lib --
  defers_update_now --nocapture`.
  Log: `.evidence/update-draft-safety/012-p2-apply-upload-regression.log`. Result:
  3 passed, 0 failed, 73 filtered out, exit 0 — toast + header + invalid still GREEN.
- RED-or-gap reason: RED observed (Upload remedy missing, Apply deferring instead),
  never fabricated; GREEN required the partitioned Upload registration plus the
  numeric-equality correction above.
- Gortex: pre-edit impact file_not_indexed lower-bound (test) and MEDIUM/HIGH
  lower-bound (prod symbol, HIGH only while temporary debug called the predicate);
  post-edit detect maps new test + `generator_is_upload_key` +
  `generator_upload_draft_differs` + `generator_values_equal` +
  `GeneratorSettingsEditor`; tests 0/1 (test) and 0/2 (prod) graph coverage (graph
  blind to wasm_bindgen_test, as 008/011/prior — behavioral proof is the mounted
  runs above); guards no rules; contract allow LOW test-only + WARN behavioral
  blast 12 lower-bound prod, no parse errors, no signature change. Preserved
  unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`.
- P2 generator row status: `GREEN` (Apply→Upload direction only); Upload→Apply,
  P3 lifecycle, P4 retention, and other owners remain `planned`. P1 generator row
  stays `GREEN` (three cases). No premature resolved.
- Proposed smallest next slice (awaiting approval): P2 Upload→Apply direction
  (Upload landing preserves an Apply draft; remaining deferral names Apply only),
  RED then GREEN; then P3 lifecycle. Do not bulk-write tests.

## Progress (fifth slice consolidation + P2 Upload→Apply GREEN proof gap, 2026-11-08; NOT resolved — awaiting review)

Fifth slice GREEN in two ordered steps: guarded consolidation of the per-key
predicates, then Upload→Apply independence as a proof gap (no production change
needed beyond step 1).

- Step 1 consolidation (`web/crates/parts/src/parts/generator_settings.rs`,
  outside `mod tests`): `generator_apply_draft_differs` +
  `generator_upload_draft_differs` (~30 identical lines, partition direction
  only) now delegate to one private
  `generator_partitioned_draft_differs(..., key: GeneratorKey)` reusing the
  existing `GeneratorKey::{Apply,Upload}` (avoids bool flag per
  rust handbook ch01.8; keeps `generator_is_upload_key` +
  `generator_values_equal`). Per-key doc comments preserved on the thin wrappers;
  both call sites' behavior IDENTICAL (same filter, same
  `apply_input`/`effective_value`/`generator_values_equal` comparison,
  fail-closed on invalid/unknown, empty partition resolves). No oracle changes.
- Step 1 regression: `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test
  --headless --chrome --mode no-install web/crates/parts --lib --
  012 --nocapture`.
  Log: `.evidence/update-draft-safety/012-fifth-slice-step1-consolidation-regression.log`.
  Result: 4 passed, 0 failed, 72 filtered out, exit 0 — toast + header + invalid
  + Apply→Upload still GREEN. `rustfmt --check --edition 2024` PASS;
  `git diff --check` PASS.
- Step 2 test: `mounted_generator_upload_landing_preserves_apply_draft_012` in
  `web/crates/parts/src/parts/generator_settings.rs` tests. Reuses
  `mount_generator_editor`, `dispatch_width`, `take_request`,
  `take_initial_default_request`, `mounted_value`, `FakeUpdateSignals012`,
  `waiting_update_012`, `support::take_held_effects`/`run_effect`. Steps: valid
  Apply width "20" (baseline 18.0, unapplied, Err-drained), Upload draft via
  production Remove (`switch_3dmodel_filename` "" with preview arrival proof),
  then REAL Upload submit via file input `test.stl` (as in
  `uploaded_model_projection`) driven to landing, drafts checked (width "20"
  preserved, filename resolved to accepted asset), refreshed preview drained,
  then Update Now at toast. Distinct failure story is Upload→Apply independence
  (Upload resolution does not clear Apply; one key in flight never exempts the
  other).
- Oracle (independent): deferral Deferred, single `UnresolvedDraft` area
  "Generator settings" remedy "Apply the generator settings, or restore its saved
  values" (Apply only, per 004 N4/B2/B5), zero adoptions, width "20" preserved,
  filename equals landed `boardstudio-asset:...`. Upload landed asserted before
  the gate poll (accepted filename + `test.stl` asset). No existing oracle
  weakened.
- test-first pre-prod run (serialized, no step-2 production edit): same flock wasm-pack
  line with `mounted_generator_upload_landing_preserves_apply_draft_012
  --nocapture`.
  Log: `.evidence/update-draft-safety/012-p2-upload-apply-pre.log`. Result: 1
  passed, 0 failed, 76 filtered out, exit 0 — GREEN on existing product, never a
  fabricated RED. No production fix made; this passing run is the gap evidence.
- RED-or-gap reason: the partitioned Upload registration from the fourth slice
  already clears only its own param on landing
  (`edits.remove(uploaded_parameter)`, preserving Apply), and
  `generator_apply_draft_differs` still judges width "20" vs 18.0 as unresolved,
  so the gate defers Apply only with no gap — a proof gap, recorded GREEN.
  Verified by running, never assumed.
- Full 012 regression: `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test
  --headless --chrome --mode no-install web/crates/parts --lib --
  012 --nocapture`.
  Log: `.evidence/update-draft-safety/012-p2-upload-apply-regression.log`.
  Result: 5 passed, 0 failed, 72 filtered out, exit 0 — all prior oracles still
  GREEN. `rustfmt --check --edition 2024` PASS; `git diff --check` PASS.
- Gortex: pre-edit impact file_not_indexed lower-bound (x2, prod+test, as prior
  slices); post-edit detect maps `generator_partitioned_draft_differs` (371) +
  new test (3379) + wrappers; tests 0/1 each (graph blind to wasm_bindgen_test,
  as 008/011/prior — behavioral proof is the mounted runs above); guards no
  rules; contract WARN behavioral blast 18 (consolidation, no parse errors, no
  signature change) + allow blast 0 (new test). Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority
  unchanged; no Pair-exception expansion, no Apply/Upload policy change, no
  P3/retention/other-owner change.
- P2 generator row status: `GREEN` (both directions: Apply→Upload +
  Upload→Apply). P1 generator row stays `GREEN` (three cases). P3 lifecycle, P4
  retention, other owners remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): P3 per-key lifecycle wiring
  (landing/failure/refusal/revert/newer-typing), RED then GREEN; then P4
  retention. Do not bulk-write tests.

## Progress (sixth slice P3 generator lifecycle — failure RED blocked + revert GREEN gap, 2026-11-08; NOT resolved — awaiting decision)

Sixth slice: P3 per-key lifecycle wiring for the generator owner only — failure
keeps deferring + revert clears — at the toast entry with generator Apply drafts.
Landing/newer-typing/older-settlement NOT re-proved; cite P2 reuse:
`mounted_generator_apply_landing_preserves_upload_draft_012` (older Apply landing
never clears newer Upload, per-key independence) +
`mounted_generator_upload_landing_preserves_apply_draft_012` (reverse). No P4
retention, no other owners, no Apply/Upload policy change (fix reverted to avoid
it), 004 authority unchanged.

- Test (a) `mounted_generator_failed_apply_keeps_deferring_update_now_at_the_toast_012`
  (3546): valid width "20" (baseline 18.0) made Ready, Apply submitted, driven to
  terminal FAILURE via `support::fail_next_core_reply` (ExecutorFailed → Failed when
  owner live, realistic per edit-ticket `settlement_of`; resolver Retire also becomes
  Failed when live, same path). Oracle: terminal failure observed (alert +
  accepted 18.0 unchanged), draft preserved "20", Update Now defers with Apply remedy
  ("Generator settings" / "Apply the generator settings, or restore its saved values"),
  zero adoptions. Distinct story: terminal records never exempt. Gate assertion allows
  additive `RecoveryRequired` (Session::core_failed latches it after failed core; P3
  claim is Apply still defers among them), never weakens Apply remedy.
- Test (a) RED (serialized, test-first pre-prod, no production edit):
  `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome
  --mode no-install web/crates/parts --lib --
  mounted_generator_failed_apply_keeps_deferring_update_now_at_the_toast_012 --nocapture`.
  Logs: `.evidence/update-draft-safety/012-p3-failure-keeps-deferring-red.log` (masked
  exit) + `-red2.log` (REAL_EXIT:1). Result: 0 passed, 1 failed at `terminal failure
  preserves the draft for retry` left "18.0" right "20" (3587), not setup (alert
  observed, accepted 18.0 asserted). Cause: `GeneratorSettingsEditor` Failed arm clears
  edits/store on matching draft_sequence, so no draft remains to defer.
- Fix attempted + verified GREEN, then REVERTED to preserve existing oracles (no
  production diff remains): removing Failed clearing preserves "20" and defers with
  Apply (+ RecoveryRequired). GREEN log
  `.evidence/update-draft-safety/012-p3-failure-keeps-deferring-green2.log` (1 passed,
  0 failed, 78 filtered, REAL_EXIT:0). First GREEN attempt
  `-green.log` (REAL_EXIT:1) exposed 2 blockers [RecoveryRequired, UnresolvedDraft];
  test corrected to assert Apply present among them (not exactly 1), never weakening
  Apply remedy. Reverted because the fix breaks TWO existing oracles asserting failure
  restores accepted (per GLOSSARY Landed edit "field shows accepted value again"):
  `failed_apply_restores_accepted_parameters_and_reports_failure` (expects "18.0", got
  "20", REAL_EXIT:1, log `012-p3-existing-failed-apply-check.log`) and
  `upload_preview_refresh_does_not_keep_a_later_failed_apply_draft` via
  `uploaded_model_projection(true)` (expects "18.0" + store None). Changing them + the
  failure display is an Apply behavior decision, not a unilateral slice fix per "Do NOT
  weaken" + "No Apply policy change". Production restored verbatim; existing GREEN
  reconfirmed post-revert (both 1 passed, REAL_EXIT:0, logs
  `012-p3-existing-failed-apply-restored.log` +
  `012-p3-existing-upload-refresh-restored.log`). RED-or-gap reason: RED observed,
  never fabricated; GREEN achievable only with oracle-correction approval.
- Test (b) `mounted_generator_revert_to_baseline_clears_apply_draft_012` (3632):
  valid "20" defers at toast (asserted, 1 blocker Apply, 0 adoptions), `cancel_update_dialog`,
  revert to "18.0" via production `dispatch_width` (preview arrival proof), Update Now
  proceeds. Oracle: no Deferred from this owner (`update_dialog().is_none()`), adoptions
  ==1, field "18.0". Distinct story: revert clears. No existing oracle weakened.
- Test (b) GREEN gap (serialized, test-first pre-prod, no production edit): same flock line with
  `mounted_generator_revert_to_baseline_clears_apply_draft_012 --nocapture`.
  Logs: `.evidence/update-draft-safety/012-p3-revert-clears-pre.log` (1 passed, 0 failed,
  78 filtered, REAL_EXIT:0) + `-post.log` (same, post-revert). RED-or-gap reason:
  `generator_partitioned_draft_differs` already resolves via canonical f64 equality
  (18.0 == 18.0), so no gap — proof gap, recorded GREEN. Verified by running, never assumed.
- Regression (post-revert, production unchanged): P1 trio `defers_update_now` 3 passed
  (REAL_EXIT:0, `012-p3-p1-trio-regression.log`); P2 pair `landing_preserves` 2 passed
  (REAL_EXIT:0, `012-p3-p2-pair-regression.log`); full `012` filter 5 passed + 2 failed
  (REAL_EXIT:1, `012-p3-full-012-regression.log`) — 1 expected RED (a) + 1 cascade
  pollution (`apply_landing...` mount timeout after (a)'s abort, passes alone above).
  Prior 5 + (b) = 6 GREEN, zero regressions. `rustfmt --check --edition 2024` PASS (one
  formatting fix via native edit, rerun PASS); `git diff --check` PASS.
- Gortex: pre-edit impact file_not_indexed lower-bound (tests x3 + docs, as prior) and
  HIGH lower-bound (prod GeneratorSettingsEditor, blast 9); post-edit detect maps new
  tests (3546 + 3632) + GeneratorSettingsEditor; tests 0/1 each (graph blind to
  wasm_bindgen_test, as 008/011/prior — behavioral proof is mounted runs above); guards
  no rules; contract allow blast 0 (tests) + WARN behavioral blast 9 (prod fix, reverted,
  no parse errors, no signature change). Preserved unrelated dirty `CODING_STANDARDS.md`
  and untracked `docs/research/2026-11-07-ai-agent-test-economy.md`.
- P3 generator row status: `RED` (failure keeps deferring, blocked on oracle decision) +
  `GREEN` (revert clears, proof gap). P1 generator stays `GREEN` (three cases). P2
  generator stays `GREEN` (both directions, cited reuse, not re-proved). P4 retention,
  other owners remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): coordinator/human decision on Apply
  failure semantics — (1) approve preserve-on-failure + correct the two existing oracles
  with spec story 12 + 004 "failure never clears" + 011 Remove/Pair precedent (then land
  the reverted one-block fix + GREEN (a)), or (2) revise P3 (a) to an already-preserving
  path (e.g., Upload failure, or failure+newer-typing if newer-typing reuse waived).
  Then P4 retention. Do not bulk-write tests.

## Progress (seventh slice P3 failure preserve + 2 oracle corrections GREEN, 2026-11-08; NOT resolved — awaiting review)

Seventh slice: land preserve-on-failure + correct 2 oracles + GREEN the failure
test, then STOP. No other behavior change, no other oracle weakened.

- Authorization (record pointer: this section): explicit user authorization relay
  via coordinator, 2026-11-08, option 1. Narrow product behavior change APPROVED:
  generator Apply terminal failure preserves the typed edits/store for retry
  instead of restoring accepted. Correction of EXACTLY these 2 existing oracles
  APPROVED (fail-case expectations only):
  `failed_apply_restores_accepted_parameters_and_reports_failure` and
  `upload_preview_refresh_does_not_keep_a_later_failed_apply_draft`. No other
  oracle may be weakened; no other behavior change authorized.
- Fix (re-derived from the settle path, not memory): `GeneratorSettingsEditor`
  `PendingEditResult::Failed` arm previously cleared on matching draft_sequence
  (`sequence` bump + `store.set(None)` + `edits`/`changed_parameters` reset),
  so no draft remained to defer. Minimal fix deletes that clearing block and
  keeps only the failure `feedback` Alert/reporting; added comment cites spec
  story 12 + 004. No baseline advance on failure; Upload/other arms untouched
  (Upload failure already preserved; Landed arms unchanged).
- Oracle 1 corrected + renamed (fail-case only; alert kept):
  `failed_apply_restores_accepted_parameters_and_reports_failure` →
  `failed_apply_preserves_typed_parameters_and_reports_failure` (old name pinned
  restore). Before: `assert_eq!(input.value(), "18.0")` after "controlled Apply
  failure" alert. After: `assert_eq!(input.value(), "20")` (typed draft
  preserved). Evidence: spec story 12 (failed work must keep deferring); 004
  "failure or refusal never clears" (contract state rule; contract-check N1/S6);
  011 preserve-on-failure precedent (Remove failure drops observation without
  resetting; Pair refusal preserves retry intent).
- Oracle 2 corrected + renamed (fail-case only; alert kept; else branch untouched):
  `upload_preview_refresh_does_not_keep_a_later_failed_apply_draft` →
  `upload_preview_refresh_keeps_a_later_failed_apply_draft` (old name pinned
  discard; helper `uploaded_model_projection(true)` only). Before: width
  `"18.0"` + `store.read().is_none()` ("late preview cannot restore the rejected
  draft"). After: width `"21"` (typed draft preserved) + `store.read().as_ref()
  .is_some_and(|draft| draft.definition.is_some())` ("late preview keeps the
  preserved failed draft for retry"); alert "Apply after upload failed" kept.
  Same three evidence citations as oracle 1. Rationale: with edits preserved,
  the refreshed 21.0 preview matches the live draft, so no sequence bump on
  failure keeps it current and it restores store for retry; bumping would strand
  retry behind a stale preview.
- Failure test verified, not weakened, no change needed: current tree
  `mounted_generator_failed_apply_keeps_deferring_update_now_at_the_toast_012`
  already asserts the Apply draft blocker (exact "Generator settings" /
  "Apply the generator settings, or restore its saved values") among
  [RecoveryRequired, UnresolvedDraft] (Session::core_failed latches recovery —
  specified behavior), plus draft-preserved "20", accepted 18.0 unchanged, zero
  adoptions. Conforms to specified recovery; not weakening.
- Validation (serialized flock; REAL exits):
  - Failure GREEN: `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test
    --headless --chrome --mode no-install web/crates/parts --lib --
    mounted_generator_failed_apply_keeps_deferring_update_now_at_the_toast_012
    --nocapture`. Log `012-p3-failure-keeps-deferring-green7.log`. 1 passed, 0
    failed, 78 filtered, REAL_EXIT:0.
  - Oracle 1 GREEN: same line with
    `failed_apply_preserves_typed_parameters_and_reports_failure --nocapture`.
    Log `012-p3-corrected-failed-apply-preserves.log`. 1 passed, 78 filtered,
    REAL_EXIT:0.
  - Oracle 2 GREEN: same line with
    `upload_preview_refresh_keeps_a_later_failed_apply_draft --nocapture`. Log
    `012-p3-corrected-upload-refresh-keeps.log`. 1 passed, 78 filtered, REAL_EXIT:0.
  - Full 012 regression: same line with `012 --nocapture`. Log
    `012-p3-full-012-regression-green7.log`. 7 passed, 0 failed, 72 filtered,
    REAL_EXIT:0 (P1 trio + P2 pair + P3 failure + P3 revert).
  - `rustfmt --check --edition 2024` PASS (one chain-wrap fix via native edit,
    rerun PASS); `git diff --check` PASS.
  - No other test changed outcome: `fail_next_core_reply` in generator_settings
    only in these 3 tests (all GREEN above); other parts failure tests are
    different owners with own settle logic (component_model, module_profile,
    module_attachment), untouched. No newly-failing test observed.
- Gortex: pre-edit impact file_not_indexed lower-bound (x2, prod+docs);
  refactor read-only in this view (renames via guarded edit); post-edit detect
  maps GeneratorSettingsEditor + renamed oracles + helper + 012 tests; tests 0/5
  graph blind to wasm_bindgen_test (as 008/011/prior — behavioral proof is the
  mounted runs above); guards no rules; contract WARN behavioral blast 10
  lower-bound, no parse errors, no signature change. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. 004 authority unchanged
  except the approved preserve; no Pair-exception expansion, no P4/other-owner
  change.
- P3 generator row status: `GREEN` (failure keeps deferring + revert clears).
  P1 generator stays `GREEN` (three cases). P2 generator stays `GREEN` (both
  directions, cited reuse). P4 retention, other owners remain `planned`. No
  premature resolved.
- Proposed smallest next slice (awaiting approval): P4 retention for the
  generator owner (hidden/keep-mounted still defers; unmount/owner-change
  retires), RED then GREEN; then module profile + mechanical fit profile owners.
  Do not bulk-write tests.

## Progress (eighth slice P4 generator retention GREEN proof gaps, 2026-11-08; NOT resolved — awaiting review)

Eighth slice: P4 retention/lifetime for the generator owner only — hidden still
defers + owner-change retires — at toast, then STOP. No production behavior
change, no oracle changes at all, no other owners.

- Test (a) `mounted_generator_hidden_apply_draft_still_defers_update_now_at_the_toast_012`
  (3703): valid "20" (baseline 18.0) unapplied, hide via `display:none` on root
  (established 011 hidden-owner pattern; generator has no own hide path, reuse,
  do not invent UI), Update Now toast. Oracle: Deferred 1 blocker Apply
  ("Generator settings" / "Apply the generator settings, or restore its saved
  values"), 0 adoptions, "20" preserved, stays mounted. Distinct story: hidden
  keeps blocking.
- Test (a) GREEN gap (serialized, test-first pre-prod, no production edit): flock
  wasm-pack ...
  `mounted_generator_hidden_apply_draft_still_defers_update_now_at_the_toast_012
  --nocapture`. Log
  `.evidence/update-draft-safety/012-p4-hidden-still-defers-pre.log`. 1 passed,
  0 failed, 79 filtered, REAL_EXIT:0. RED-or-gap: display:none keeps
  signals/guards alive; is_unresolved still sees "20" vs 18.0 — proof gap,
  recorded GREEN. Verified by running, never fabricated RED.
- Test (b) `mounted_generator_owner_change_retires_apply_draft_012` (3760):
  valid "20" defers asserted, cancel, owner-change to second definition via
  production selection signals (definition + selected + selection_generation
  bump, as existing selection test), switched preview arrival proof (second.id),
  drain, Update Now toast proceeds. Oracle: adoptions==1, update_dialog None
  (no UnresolvedDraft from retired entry). Distinct story: retire wiring. No
  existing oracle weakened.
- Test (b) GREEN gap (serialized, test-first pre-prod, no production edit): same flock
  line with `mounted_generator_owner_change_retires_apply_draft_012 --nocapture`.
  Log `.evidence/update-draft-safety/012-p4-owner-change-retires-pre.log`. 1
  passed, 0 failed, 80 filtered, REAL_EXIT:0. RED-or-gap: draft_owner effect
  retires old guards + owner effect clears edits; new owner initial_parameters
  == baseline resolves — proof gap, recorded GREEN. Verified by running.
  Root.remove() alone does not run Dioxus use_drop (VirtualDom still alive), so
  owner-change is the faithful retire path; no product UI invented.
- Regression: full `012` filter 9 passed, 0 failed, 72 filtered, REAL_EXIT:0,
  log `012-p4-full-012-regression.log` (7 prior + 2 new). Pinned oracles still
  GREEN: `failed_apply_preserves...` 1 passed REAL_EXIT:0
  (`012-p4-pinned-oracle1-preserves.log`); `upload_preview_refresh_keeps...` 1
  passed REAL_EXIT:0 (`012-p4-pinned-oracle2-keeps.log`). `rustfmt --check
  --edition 2024` PASS (one chain-wrap fix via native edit, rerun PASS); `git
  diff --check` PASS.
- Gortex: pre-edit impact file_not_indexed lower-bound (x3: test a, test b,
  format); post-edit detect maps new tests (3703 + 3760); tests 0/1 each (graph
  blind to wasm_bindgen_test, as 008/011/prior — behavioral proof is mounted
  runs above); guards no rules; contract allow blast 0 (a LOW, b MEDIUM
  lower-bound, verdict allow, no parse errors, no signature change), test-only.
  Detect attempt1 pending, attempt2 after independent work mapped. Preserved
  unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority +
  user option-1 authorization unchanged; no behavior change, no oracle changes,
  no Pair-exception expansion, no other-owner change.
- P4 generator row status: `GREEN` (hidden still defers + owner-change retires,
  proof gaps). P1 generator stays `GREEN` (three cases). P2 generator stays
  `GREEN` (both directions). P3 generator stays `GREEN` (failure + revert).
  Other owners remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): module profile owner P1
  deferral (valid Save draft at toast, RED then GREEN); then mechanical fit
  profile. Do not bulk-write tests.

## Progress (ninth slice module profile P1 Save valid deferral RED then GREEN, 2026-11-08; NOT resolved — awaiting review)

Ninth slice: module profile owner P1 deferral — valid unapplied profile Save
draft at toast, RED then GREEN — then STOP. Profile Save only; rotary,
reviewed and staged-shape-alone follow separately. No other behavior change,
no oracle changes at all, no other owners.

- Test `mounted_module_profile_valid_save_draft_defers_update_now_at_the_toast_012`
  (1298): real valid measured volume via production inputs (width/depth/Z/height
  10/10/1/2 + evidence measured-012 + Add measured volume; wait for measured-012
  proves processing before gate poll), Update Now toast. Oracle: Deferred 1 blocker
  Save (area Module profile per 004 N4 / remedy Save the module profile, or restore
  its saved values per own-controls pattern, naming Save project module profile),
  0 adoptions, measured-012 + 1 volumes preserved. Distinct story: profile Save
  deferral wiring. Reuses mounted_host, type_value, by_role_name, wait_until;
  adds minimal FakeUpdateSignals012 + waiting_update_012 (same shape as generator,
  local to this file per slice ownership). No existing oracle weakened.
- RED (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_module_profile_valid_save_draft_defers_update_now_at_the_toast_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-p1-red.log`.
  Result: 0 passed, 1 failed, 81 filtered, REAL_EXIT:1 at expected a deferred update
  dialog, got None (1297). Volume Add proof passed, gate reached — missing-adapter
  RED, not setup. No module-profile adapter exists (verified: register_draft_owner
  only at Geometry/Layout/generator sites).
- Production (`web/crates/parts/src/parts/module_profile_editor.rs`, outside `mod
  settlement_tests`): ONE profile Save admission-key registration in
  `ModuleProfileEditor` (scope/owner effect, RAII guard + retire_draft_owner on
  unmount/scope/owner change, poll-prune safe). Judges live ModuleProfileDraft
  (the value Save would submit) vs live-projected accepted definition
  (ModuleProfileDraft::from_definition per poll; mount baseline only where no
  accepted definition exists) under PartialEq (owner-canonical, matching dirty
  markers; parse-then-compare is P3). Same-key in-flight exemption only via
  profile_edits.submitted_draft(Save) == live; terminal records never exempt
  (None unless pending); newer typing re-raises by mismatch. Scope check inside
  closure (stale scope returns false). Area/remedy above; owner module-profile,
  admission save, workspace Parts. Rotary/reviewed/staged-shape-alone untouched
  (Save submits draft, not shape); no Save policy change, 004 authority unchanged.
- GREEN (serialized, identical to RED): same flock wasm-pack line. Log
  `.evidence/update-draft-safety/012-module-profile-p1-green.log`. Result: 1 passed,
  0 failed, 81 filtered, REAL_EXIT:0. `rustfmt --check --edition 2024` PASS (three
  formatting fixes via native edits, rerun PASS); `git diff --check` PASS.
- Regression (all 012): same flock line with `012 --nocapture`. Log
  `.evidence/update-draft-safety/012-module-profile-p1-regression.log`. Result: 10
  passed, 0 failed, 72 filtered, REAL_EXIT:0 (9 prior generator + 1 new module
  profile). Zero regressions.
- RED-or-gap reason: RED observed (None instead of Deferred, adapter missing),
  never fabricated; GREEN required the Save registration above.
- Gortex: pre-edit impact HIGH lower-bound (file) + HIGH lower-bound
  (ModuleProfileEditor symbol) + LOW (docs); post-edit detect maps ModuleProfileEditor
  (295) + new test (1298) + Fake (1249) + waiting (1289); tests 0/1 each (graph blind
  to wasm_bindgen_test, as 008/011/prior — behavioral proof is the mounted runs above);
  guards no rules; contract allow blast 0 test-only + WARN behavioral blast 10
  lower-bound prod, no parse errors, no signature change. Detect attempt1 pending,
  attempt2 after independent work mapped. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority +
  user option-1 authorization (generator failure only) unchanged; no behavior change
  beyond adapter registration, no oracle changes, no Pair-exception expansion, no
  fit-profile/other-owner change.
- P1 module profile row status: `GREEN` (first case only: valid profile Save draft
  at toast). P1 generator stays `GREEN` (three cases). P2 generator stays `GREEN`
  (both directions). P3 generator stays `GREEN` (failure + revert). P4 generator
  stays `GREEN` (hidden + owner-change). Invalid/header/rotary/lifecycle/retention,
  fit profile remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): module profile rotary Save P1
  (valid rotary draft at toast, RED then GREEN, second admission with rotary remedy);
  then invalid/header, then P3 lifecycle. Do not bulk-write tests.

## Progress (tenth slice Part A staged-shape classification CLOSED, 2026-11-08; NOT resolved — Part B rotary P1 follows in same slice)

Part A determination FIRST, per slice brief: ninth slice reports staged
volume/shape typing without Add does not defer, claiming Save submits the
draft list not the staged field. VERIFIED from the module profile Save submit
path in source: staged state is NOT included in what Save submits. No
predicate extension, no test (would test non-contract behavior). Question
closed with evidence below, never silently dropped.

- Verdict: NO. `submit_profile` (`ModuleProfileEditor` 646-718,
  `web/crates/parts/src/parts/module_profile_editor.rs::ModuleProfileEditor`
  295-902) reads only `rotary()` (671, rotary path) or `draft()`/`reviewed()`
  (686, profile path), branching on `rotary_override` (658/670/681/683/685/
  688/696/705) to call exactly one of `rotary_edits.begin_value` (697) or
  `profile_edits.begin_value` (706). It never reads `shape()`. CFG stmt 51
  uses confirm: runtime/owner/definition/baseline/draft/rotary/profile_edits/
  rotary_edits/failure/error/selected, NO shape. Save buttons call only this
  path: rotary Save 777 → `submit_profile.call(Some(None))`, profile Save 896
  → `submit_profile.call(None)`.
- Staged path is separate: `shape` signal 321 holds `ModuleVolumeDraft`;
  `add_volume` 541-644 parses shape 560-565/574/581, constructs volume
  629-631, sets `profile_dirty` true 633, pushes to `draft`, resets shape 641.
  Shape inputs 808-815/820-821 do `shape.with_mut` only, NO dirty set; all
  draft mutations 633/729/784/794/870/880 set `profile_dirty` true. Owner-
  canonical dirty = draft changed, not shape typed. Resolver 169+ takes
  draft+rotary+reviewed, updates volumes/openings/models OR rotary_profile,
  never shape. Ninth-slice registration 431-508 judges draft vs accepted,
  matching dirty markers.
- Glossary reasoning: Draft value = typed into field, not yet in accepted doc
  (broad). Unresolved draft = differs from accepted value it would replace +
  not submitted as Pending edit. Staged-unadded text would NOT replace any
  accepted value via Save — Save would submit draft unchanged, ignoring staged
  text. Per 004 state rule, unresolved judged per (owner, admission key) by
  owner-canonical quote the value that would be submitted endquote. Staged shape
  consumed only by Add (local staging, no `begin_value`, not an admission),
  never by `submit_profile`. Owner itself does not mark it dirty. Thus pre-Add
  staging is not quote value that would be submitted endquote for any key, not
  an Unresolved draft, no deferral required. Protection boundary: Add enters
  draft → Save submits draft.
- Current admission shape (verified for P3 slice, P3 proof map NOT reshaped):
  separate Save buttons, separate `begin_value` calls — B3 quote one Save two
  begin_values endquote OUTDATED. Profile Save 896 (None) → 681/686/705-706
  submits (`draft()`, `reviewed()`) via `profile_edits` (`PendingEditSignals<
  ModuleProfileKey, ModuleProfileDraft>` 323/326), resolver rotary=None updates
  volumes/openings/models. Rotary Save 777 (Some(None)) → 670-671/683-684/688/
  696-697 submits `rotary()` parsed via `rotary_profile_from_draft` 266/671 via
  `rotary_edits` (`PendingEditSignals<ModuleProfileKey, ModuleRotaryDraft>`
  324/327), resolver rotary=Some updates rotary_profile, profile_draft=baseline
  684 ignored. Keys exist: (module-profile, profile-collection, Save) +
  (module-profile, rotary-collection, Save); same enum `ModuleProfileKey::Save`,
  different collections. Reviewed submitted only by profile Save (686/692-693);
  rotary Save uses false 684.
- Gortex: pre-edit impact file_not_indexed lower-bound (docs, as prior).
  Post-edit detect/tests/guards/contract follow after Part B combined (single
  slice). Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. No production change,
  no oracle change in Part A.
- P1 module profile row status: still `GREEN` (first case: valid profile Save
  draft at toast; staged-shape-alone explicitly NOT a case — classification
  above). Part B rotary P1 next in same slice, then STOP.

## Progress (tenth slice Part B rotary P1 Save valid deferral RED then GREEN, 2026-11-08; NOT resolved — awaiting review)

Tenth slice Part B: module profile owner P1 deferral — valid unapplied
rotary Save draft at toast, RED then GREEN — then STOP. Rotary Save only;
reviewed, invalid/header, P3 lifecycle, retention follow separately. Part A
staged-shape classification above stands (NO, closed with evidence). No other
behavior change, no oracle changes at all, no other owners.

- Test `mounted_module_profile_valid_rotary_draft_defers_update_now_at_the_toast_012`
  (1502): real valid rotary draft via production input (pulses 20→24,
  baseline A/B/C steps 20 triggers 20 driver ec11; wait for pulses 24 proves
  processing before gate poll — same timing GREEN passes, so RED is not race),
  Update Now toast. Oracle: Deferred 1 blocker Save-rotary (area Module profile
  per 004 N4 / remedy Save the rotary profile, or restore its saved values per
  own-controls pattern, naming Save rotary profile), 0 adoptions, pulses 24
  preserved, profile draft clean (0 volumes, profile predicate resolves — only
  rotary defers, proving per-key). Distinct story: rotary Save deferral wiring.
  Reuses mounted_host, type_value, by_role_name, wait_until, FakeUpdateSignals012
  + waiting_update_012 (ninth-slice locals, no new helpers). No existing oracle
  weakened.
- RED (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_module_profile_valid_rotary_draft_defers_update_now_at_the_toast_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-rotary-p1-red.log`.
  Result: 0 passed, 1 failed, 82 filtered, REAL_EXIT:1 at expected a deferred
  update dialog, got None (1469). Pulses 24 proof passed, gate reached —
  missing-adapter RED, not setup. No rotary adapter exists (verified:
  register_draft_owner only at Geometry/Layout/generator/profile sites, no
  save-rotary).
- Production (`web/crates/parts/src/parts/module_profile_editor.rs`, outside
  `mod settlement_tests`): ONE rotary Save admission-key registration in
  `ModuleProfileEditor` (520-610, mirroring profile 431-518; separate guard +
  effect + use_drop, same scope/owner deps, RAII + retire_draft_owner on
  unmount/scope/owner change, poll-prune safe). Judges live ModuleRotaryDraft
  (the value Save rotary profile would submit) vs live-projected accepted
  rotary (`ModuleRotaryDraft::from_profile` per poll; mount baseline only where
  no accepted definition exists) under PartialEq (owner-canonical, matching
  rotary_dirty; parse-then-compare is P3). Same-key in-flight exemption only
  via `rotary_edits.submitted_draft(Save) == live` (rotary collection — same
  enum Save, different PendingEditSignals instance, no cross-talk); terminal
  records never exempt (None unless pending); newer typing re-raises by
  mismatch. Scope check inside closure (stale scope returns false). Admission
  `save-rotary` (profile keeps `save` — distinct registry keys, same owner
  module-profile/workspace Parts/area Module profile); remedy above. Reviewed/
  staged-shape/invalid/header untouched; no Save policy change, 004 authority
  unchanged.
- GREEN (serialized, identical to RED): same flock wasm-pack line. Log
  `.evidence/update-draft-safety/012-module-profile-rotary-p1-green.log`.
  Result: 1 passed, 0 failed, 82 filtered, REAL_EXIT:0. `rustfmt --check
  --edition 2024` PASS (three formatting fixes via native edits — single-line
  if/remedy/fn-sig mirroring profile — rerun PASS, logs
  `012-module-profile-rotary-fmt.log` RED 1 + `012-module-profile-rotary-fmt2.log`
  GREEN 0); `git diff --check` PASS.
- Regression (all 012): same flock line with `012 --nocapture`. Log
  `.evidence/update-draft-safety/012-module-profile-rotary-p1-regression.log`.
  Result: 11 passed, 0 failed, 72 filtered, REAL_EXIT:0 (10 prior: 9 generator +
  1 profile + 1 new rotary). Zero regressions.
- RED-or-gap reason: RED observed (None instead of Deferred, rotary adapter
  missing), never fabricated; GREEN required the save-rotary registration
  above. Same timing GREEN passes, so RED is not input race.
- Gortex: pre-edit impact file_not_indexed lower-bound (file) + HIGH
  lower-bound (ModuleProfileEditor symbol, 10→11 transitively incl new test) +
  LOW (docs Part A); test-only contract allow MEDIUM lower-bound blast 0 +
  prod contract WARN behavioral blast 11 lower-bound, no parse errors, no
  signature change; post-edit detect maps ModuleProfileEditor (295) + new test
  (1502, shifted from 1411 by +91 prod lines) + Fake (1343) + waiting (1383);
  tests 0/1 each (graph blind to wasm_bindgen_test, as 008/011/prior —
  behavioral proof is mounted runs above); guards no rules. Detect attempt1
  invalid-arg (target.file refused — correct: detect takes no target),
  attempt2 mapped. Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority +
  user option-1 authorization (generator failure only) unchanged; no behavior
  change beyond adapter registration, no oracle changes, no Pair-exception
  expansion, no fit-profile/other-owner change.
- Current admission shape (recorded, P3 map NOT reshaped): two registry entries
  — (module-profile, save) profile + (module-profile, save-rotary) rotary;
  two Save buttons (777 rotary, 896 profile) → one `submit_profile` 646-718
  branching to exactly one `begin_value` (697 rotary OR 706 profile). P3 slice
  must prove per-key lifecycle for both keys (landing/failure/revert/newer/
  older-settlement) — B3 two-collection note now refined to two-admission
  shape.
- P1 module profile row status: `GREEN` (two cases: valid profile Save draft
  at toast + valid rotary Save draft at toast). P1 generator stays `GREEN`
  (three cases). P2 generator stays `GREEN` (both directions). P3 generator
  stays `GREEN` (failure + revert). P4 generator stays `GREEN` (hidden +
  owner-change). Part A staged-shape-alone explicitly NOT a case (closed NO
  above). Invalid/header/reviewed/lifecycle/retention, fit profile remain
  `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): module profile invalid draft
  P1 (realistic invalid unsubmitted rotary or profile draft at toast, RED then
  GREEN, fail-closed deferral); then header, then P3 lifecycle (both keys).
  Do not bulk-write tests.

## Progress (eleventh slice module profile invalid + header P1 GREEN proof gaps, 2026-11-08; NOT resolved — awaiting review)

Eleventh slice: module profile owner P1 deferral — (a) realistic invalid
unsubmitted rotary draft at toast + (b) valid profile Save draft at header —
both GREEN as proof gaps, then STOP. No production behavior change, no oracle
changes at all, no other owners. test-first order per test (write, run, LOG before
any production edit); both passed on existing product, never fabricated RED.

- Test (a) `mounted_module_profile_invalid_rotary_draft_defers_update_now_at_the_toast_012`
  (1582): realistic invalid rotary draft via production input (pulses 20→"0",
  baseline A/B/C steps 20 triggers 20 driver ec11; wait for "0" proves
  processing before gate poll — same timing valid-rotary GREEN passes, so a
  RED would not be race), NO submit attempt, Update Now toast. Invalid value
  "0": owner's own Save rotary profile refuses it — `rotary_profile_from_draft`
  parses steps "0" → Some(0) → `is_none_or(==0)` → Err("Enter distinct A, B
  and common terminals and positive whole-number pulses and actions per
  rotation."); `submit_profile` rotary arm calls that parser and on Err sets
  `ModuleProfileError` with NO `begin_value`. Where invalid text lives: IN
  the typed draft itself — rotary `EditorNumberField oninput` does
  `rotary.with_mut draft.steps = event.value()` + `rotary_dirty=true`;
  `ModuleRotaryDraft` holds raw Strings, so "0" enters live draft; predicate
  compares live vs `from_profile` "20" under PartialEq → differs → unresolved.
  No extra raw-text comparison needed; draft IS raw text. Staged-shape
  contrast (not silently skipped): shape signal never enters draft and Save
  ignores shape (tenth Part A NO with `submit_profile`/`add_volume`/dirty
  evidence, still closed) — staged invalid correctly NOT a case. Census A2:
  "Add-volume validates at click (ModuleProfileError Alert, no field-level
  marker); submit-time resolver errors → visible_error Alert; no aria-invalid
  on any profile input" — full-file read confirms zero aria-invalid.
  Oracle: Deferred 1 blocker save-rotary (area Module profile per 004 N4 /
  remedy Save the rotary profile, or restore its saved values per
  own-controls, naming Save rotary profile), 0 adoptions, "0" preserved,
  `aria-invalid != "true"` on pulses at gate time (asserted absent, so the
  DOM query cannot be the cause). Distinct story: invalid-unsubmitted
  fail-closed. Reuses mounted_host, type_value, by_role_name, wait_until,
  FakeUpdateSignals012 + waiting_update_012 (ninth-slice locals, no new
  helpers). No existing oracle weakened.
- Test (a) GREEN gap (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_module_profile_invalid_rotary_draft_defers_update_now_at_the_toast_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-invalid-p1-pre.log`.
  Result: 1 passed, 0 failed, 83 filtered, REAL_EXIT:0. RED-or-gap: live "0"
  vs accepted "20" already differs under the save-rotary PartialEq predicate
  with no in-flight exemption (no submit → `submitted_draft` None), so the
  existing adapter defers fail-closed with no gap — proof gap, recorded GREEN.
  Verified by running, never assumed.
- Test (b) `mounted_module_profile_valid_save_draft_defers_update_now_at_the_header_012`
  (1668): mirrors ninth-slice toast test with ONLY the entry point changed
  (`UpdateOpener::Header` vs Toast); same real valid measured volume via
  production inputs (width/depth/Z/height 10/10/1/2 + evidence measured-012 +
  Add measured volume; wait for measured-012 proves processing). Oracle
  (independent, same shape as toast): Deferred 1 blocker Save (area Module
  profile / remedy Save the module profile, or restore its saved values), 0
  adoptions, measured-012 + 1 volumes preserved. Distinct story:
  header-checkpoint wiring for this owner. No existing oracle weakened.
- Test (b) GREEN gap (serialized, test-first pre-prod, no production edit): same flock
  line with `mounted_module_profile_valid_save_draft_defers_update_now_at_the_header_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-header-p1-pre.log`.
  Result: 1 passed, 0 failed, 84 filtered, REAL_EXIT:0. RED-or-gap: both entry
  points poll the same draft registry (`Runtime::update_safety_inputs`;
  opener focus-only per second-slice verification), so header defers
  identically to toast on the existing profile Save adapter — proof gap,
  recorded GREEN. Verified by running, never assumed.
- Regression (all 012): same flock line with `012 --nocapture`. Log
  `.evidence/update-draft-safety/012-module-profile-eleventh-regression.log`.
  Result: 13 passed, 0 failed, 72 filtered, REAL_EXIT:0 (11 prior: 9 generator +
  1 profile toast + 1 rotary toast + 2 new). Zero regressions. `rustfmt --check
  --edition 2024` PASS (no fixes needed, log
  `012-module-profile-eleventh-fmt.log` REAL_EXIT:0); `git diff --check` PASS.
- Gortex: pre-edit impact HIGH lower-bound (file, x2, 9 direct/9 transitive,
  as prior) + LOW (docs); post-edit detect maps new tests (1582 + 1668) +
  ModuleProfileEditor (295) + Fake (1343) + waiting (1383) + prior 012 tests;
  tests 0/1 each (graph blind to wasm_bindgen_test, as 008/011/prior —
  behavioral proof is the mounted runs above); guards no rules; contract allow
  MEDIUM lower-bound blast 0 each, test-only, no parse errors, no signature
  change. Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority +
  user option-1 authorization (generator failure only) unchanged; no behavior
  change at all, no oracle changes, no Pair-exception expansion, no
  fit-profile/other-owner change.
- P1 module profile row status: `GREEN` (four cases: valid profile Save draft
  at toast + valid rotary Save draft at toast + invalid rotary draft at toast +
  valid profile Save draft at header). P1 generator stays `GREEN` (three
  cases). P2 generator stays `GREEN` (both directions). P3 generator stays
  `GREEN` (failure + revert). P4 generator stays `GREEN` (hidden +
  owner-change). Part A staged-shape-alone explicitly NOT a case (closed NO,
  still stands). Reviewed-checkbox/P3 lifecycle/P4 retention, fit profile
  remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): module profile P3 per-key
  lifecycle wiring for both keys (save + save-rotary: landing/failure/revert/
  newer-typing/older-settlement), RED then GREEN; then P4 retention; then fit
  profile. Do not bulk-write tests.

## Progress (twelfth slice module profile P3 lifecycle — 3 GREEN gaps + 1 RED blocker, 2026-11-08; NOT resolved — awaiting decision)

Twelfth slice: module profile owner P3 per-key lifecycle wiring for both keys
(save + save-rotary) at toast — (a) profile landing re-baselines, (b) rotary
failure keeps deferring, (c) profile revert clears, (d) older rotary settlement
never clears newer profile — then STOP. No production behavior change at all
(3 GREEN gaps on existing product, 1 RED blocker with no fix per
sixth-slice precedent), no oracle changes at all, no other owners. test-first order
per scenario (write, run, LOG before any production edit); GREENs verified by
running, never assumed; RED observed, never fabricated.

- Test (a) `mounted_module_profile_save_landing_rebaselines_update_now_at_the_toast_012`
  (1782): real valid profile volume measured-012-p3a via production Add,
  Save project module profile submitted and landed (observe_next Completed +
  accepted volumes 1/source asserted before gate poll), Update Now toast
  PROCEEDS. Oracle: adoptions==1, dialog None. Distinct story: landing
  re-baselines. Admission shape recorded (two registry save+save-rotary, two
  Save buttons → one submit_profile → exactly one begin_value). No existing
  oracle weakened.
- Test (a) GREEN gap (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_module_profile_save_landing_rebaselines_update_now_at_the_toast_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-p3a-pre.log`.
  Result: 1 passed, 0 failed, 85 filtered, REAL_EXIT:0. Fmt RED 1 → 2 native
  fixes → GREEN 0 (logs `012-module-profile-p3a-fmt.log` +
  `012-module-profile-p3a-fmt2.log`); post-fmt GREEN
  `012-module-profile-p3a-green.log` 1 passed, 85 filtered, REAL_EXIT:0.
  RED-or-gap: live accepted projection each poll; after landing live==new
  accepted → resolves. Proof gap, recorded GREEN.
- Test (b) `mounted_module_profile_failed_rotary_keeps_deferring_update_now_at_the_toast_012`
  (1914): valid rotary 24 (baseline 20) submitted, driven to terminal FAILURE
  via `support::fail_next_core_reply` (ExecutorFailed → Failed when owner live,
  realistic per sixth-slice; rotary chosen: same path as generator P3 +
  existing mounted_rotary_save_... controlled failure proves it exists, simple
  text draft, no resolver complexity). Oracle: terminal observed (alert +
  accepted 20 unchanged), draft 24 preserved, Update Now still defers with
  save-rotary remedy (RecoveryRequired additive allowed, as generator P3).
  Distinct story: terminal never exempts. Admission shape + key choice recorded.
  No existing oracle weakened.
- Test (b) RED BLOCKER (serialized, test-first pre-prod, no production edit): same
  flock line with `mounted_module_profile_failed_rotary_keeps_deferring_update_now_at_the_toast_012
  --nocapture`. Logs `.evidence/update-draft-safety/012-module-profile-p3b-pre.log`
  + `-red2.log` (post-fmt). Result: 0 passed, 1 failed at `terminal failure
  preserves the draft for retry` left "20" right "24" (1967), REAL_EXIT:1 both.
  Alert observed, accepted 20 asserted — missing-preserve RED, not setup. Fmt
  RED 1 (panic line) → native fix → GREEN 0 (logs `012-module-profile-p3b-fmt.log`
  + `012-module-profile-p3b-fmt2.log`); RED2 same reason. Cause:
  `PendingEditSignals::apply_to_field` restores accepted on Failed when
  submitted==current (24==24, 24!=20 → draft.set(20)), same as generator Apply
  originally; newer typing would preserve (draft_may_restore false), but plain
  failure discards. NO FIX per slice brief + sixth-slice precedent (needs
  explicit user approval; option-1 was generator-only). Existing 004 authority
  unchanged.
- Test (c) `mounted_module_profile_revert_to_baseline_clears_save_draft_012`
  (2009): valid profile volume measured-012-p3c defers asserted (1 blocker
  Save, 0 adoptions), cancel, revert via production Remove measured volume 1
  (wait 0 volumes + evidence gone proves processing), Update Now proceeds.
  Oracle: adoptions==1, dialog None. Distinct story: revert clears. Admission
  shape recorded. No existing oracle weakened.
- Test (c) GREEN gap (serialized, test-first pre-prod, no production edit): same flock
  line with `mounted_module_profile_revert_to_baseline_clears_save_draft_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-p3c-pre.log`.
  Result: 1 passed, 0 failed, 87 filtered, REAL_EXIT:0. Fmt GREEN 0 (log
  `012-module-profile-p3c-fmt.log`), no fix. RED-or-gap: live==accepted after
  Remove → resolves via canonical PartialEq. Proof gap, recorded GREEN.
- Test (d) `mounted_module_profile_rotary_landing_preserves_profile_draft_012`
  (2142): rotary 24 submitted + held via gate (drive_pending + entered.await),
  newer profile volume measured-012-p3d via Add while in flight (arrival proves
  processing), land older (Completed + accepted 24), assert landing + surviving
  newer (pulses 24 + measured-012-p3d + 1 volumes) before gate, Update Now
  still defers 1 blocker Save only. This is the recorded two-collection case
  adapted to separate Save buttons. Distinct story: older settlement never
  clears newer input (per-key independence); covers rotary landing (with (a)
  profile landing, both landings proven). Admission shape + adaptation recorded.
  No existing oracle weakened.
- Test (d) GREEN gap (serialized, test-first pre-prod, no production edit): same flock
  line with `mounted_module_profile_rotary_landing_preserves_profile_draft_012
  --nocapture`. Log `.evidence/update-draft-safety/012-module-profile-p3d-pre.log`.
  Result: 1 passed, 0 failed, 88 filtered, REAL_EXIT:0. Fmt RED 1 (panic) →
  native fix → GREEN 0 (logs `012-module-profile-p3d-fmt.log` +
  `012-module-profile-p3d-fmt2.log`); post-fmt GREEN
  `012-module-profile-p3d-green.log` 1 passed, 88 filtered, REAL_EXIT:0.
  RED-or-gap: separate PendingEditSignals collections, no cross-talk; rotary
  landing updates accepted rotary only, profile live vs accepted still differs
  → defers. Proof gap, recorded GREEN.
- Regression (all 012): same flock line with `012 --nocapture`. Log
  `.evidence/update-draft-safety/012-module-profile-p3-regression.log`.
  Result: 16 passed, 1 failed, 72 filtered, REAL_EXIT:1 (13 prior: 9 generator +
  4 module P1, all still GREEN + 3 new GREEN (a,c,d) + 1 expected RED (b)).
  Zero regressions. `git diff --check` PASS (log
  `012-module-profile-p3-diffcheck.log` REAL_EXIT:0).
- Reviewed checkbox determination (from submit path, per slice brief): RIDES
  with profile Save, covered by profile-draft comparison (recorded, no separate
  admission). Evidence: `submit_profile` 775-776 profile path uses
  `(draft(), reviewed())` vs rotary path `(baseline.clone(), false)`; 686/
  692-693 reviewed only in profile resolver; 684 rotary ignores; tenth Part A
  same. Resolver 169+ uses reviewed only to retain assembled-envelope gate when
  volumes non-empty + all qualified; predicate judges ModuleProfileDraft vs
  accepted (owner-canonical, matching dirty markers). No scope expansion.
- Gortex: pre-edit impact HIGH lower-bound (file, (a)) + MEDIUM lower-bound
  (file x5: (b)(c)(d) + 2 fmt) + LOW (docs); post-edit detect maps new tests
  (1782 + 1914 + 2009 + 2142) + ModuleProfileEditor (295) + Fake (1343) +
  waiting (1383) + prior 012 tests; tests 0/1 each (graph blind to
  wasm_bindgen_test, as 008/011/prior — behavioral proof is mounted runs above);
  guards no rules; contract allow MEDIUM lower-bound blast 0 each, test-only,
  no parse errors, no signature change. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority +
  user option-1 authorization (generator failure only) unchanged; no behavior
  change at all, no oracle changes, no Pair-exception expansion, no
  fit-profile/other-owner change.
- P3 module profile row status: `GREEN` (landing re-baselines + revert clears +
  older-settlement-never-clears-newer, proof gaps) + `RED` (failure keeps
  deferring, blocked on preserve decision). P1 module profile stays `GREEN`
  (four cases). P1 generator stays `GREEN` (three cases). P2 generator stays
  `GREEN` (both directions). P3 generator stays `GREEN` (failure + revert). P4
  generator stays `GREEN` (hidden + owner-change). Part A staged-shape-alone
  explicitly NOT a case (closed NO, still stands). Reviewed covered above (no
  separate case). P4 retention, fit profile remain `planned`. No premature
  resolved.
- Proposed smallest next slice (awaiting approval): coordinator/human decision
  on module failure semantics — (1) approve preserve-on-failure + correct
  affected existing oracles with spec story 12 + 004 "failure never clears" +
  011/generator precedent (then land minimal preserve fix + GREEN (b)), or (2)
  revise P3 (b) to an already-preserving path (e.g., failure+newer-typing if
  newer-typing reuse waived). Then P4 retention; then fit profile. Do not
  bulk-write tests.

## Progress (thirteenth slice module profile P3 failure preserve + zero oracle corrections GREEN, 2026-11-08; NOT resolved — awaiting review)

Thirteenth slice: land scoped preserve-on-failure for module profile Save keys
+ correct affected oracles + GREEN the failure test + full crate suite proof,
then STOP. No other behavior change, no helper touch, no other file.

- Authorization (record pointer: this section): explicit user authorization
  relay via coordinator, 2026-11-08, option 1 for module profile. Narrow
  product behavior change APPROVED: module profile Save keys (save +
  save-rotary) terminal failure preserves the typed draft for retry instead of
  restoring accepted. Correction of oracles pinning the discard behavior
  APPROVED, each with cited independent evidence. SCOPING RULE (hard, from
  brief): fix must NOT change any other owner's settle behavior; owner-scoped
  mechanism required, shared default untouched; if only viable fix changes
  shared behavior, STOP with blast evidence, do NOT land.
- Fix (re-derived from the settle path, not memory): `ModuleProfileEditor`
  settle effect previously let `PendingEditSignals::apply_to_field` restore
  accepted on Failed when submitted==current (24==24, 24!=20 → draft.set(20)),
  per twelfth-slice cause. Minimal owner-scoped fix snapshots `pre_profile` /
  `pre_rotary` before `settle`, then after Failed restores the typed draft when
  pre!=accepted (Landed/Retired keep helper restoration; failure Alert/reporting
  untouched; baseline never advances on failure; preserved draft stays dirty
  since draft!=accepted). Fully-qualified `PendingEditResult::Failed`, no import
  change, no helper change, no other owner. Comment cites spec story 12 + 004.
- Scoping argument (why no other owner changes behavior): single owner-local
  edit in `ModuleProfileEditor` settle effect only (receipt commit-124);
  `pending_edit_helpers.rs` untouched (zero diff); `generator_settings.rs`
  frozen, untouched; no other file touched. Shared `apply_to_field` default
  (restore on Failed when submitted==current) byte-identical for every other
  caller; module opts in by restoring after, others never execute this code.
  Full parts suite 89/0 proves no cross-talk (all other owners GREEN unchanged).
- Failure test GREEN without weakening, no change needed: current tree
  `mounted_module_profile_failed_rotary_keeps_deferring_update_now_at_the_toast_012`
  already asserts the rotary blocker present with EXACT area "Module profile" /
  remedy "Save the rotary profile, or restore its saved values" among
  [RecoveryRequired, UnresolvedDraft] (Session::core_failed latches recovery
  after failed core — specified behavior, same as generator P3 seventh slice),
  plus draft-preserved "24", accepted 20 unchanged, zero adoptions. Why
  RecoveryRequired alongside is truthful, not weakening: failed core raises
  recovery as a separate blocker; P3 claim is the typed draft still defers
  among them with exact copy; allowing additive recovery preserves the exact
  rotary assertion while conforming to specified recovery. Draft-preserved +
  zero-adoptions kept verbatim.
- Oracle inventory: ZERO affected, ZERO corrections, ZERO renames. Full parts
  suite (no filter) 89 passed, 0 failed proves no existing oracle pins plain
  module-profile failure discard. The sole existing module failure test
  `mounted_rotary_save_preserves_newer_baseline_typing_and_resumes_history_projection`
  checks newer typing 26 preserves + alert (already-preserving path via
  draft_may_restore false), still GREEN unchanged; it never asserted plain
  discard. `fail_next_core_reply` in module file only in these 2 tests (both
  GREEN). No before/after text: nothing pinned the old discard, so nothing to
  correct. Independent evidence for preserve (had corrections been needed):
  spec story 12 (failed work must keep deferring); 004 "failure or refusal
  never clears" (contract state rule; contract-check N1/S6); 011 precedent
  (Remove failure drops observation without resetting; Pair refusal preserves
  retry intent); generator P3 seventh-slice precedent (same preserve + 2
  oracle corrections). No other newly-failing test: full suite GREEN, no
  BLOCKER.
- Validation (serialized flock; REAL exits):
  - Fix GREEN: `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test
    --headless --chrome --mode no-install web/crates/parts --lib --
    mounted_module_profile_failed_rotary_keeps_deferring_update_now_at_the_toast_012
    --nocapture`. Log `012-module-profile-p3b-fix-green.log`. 1 passed, 0
    failed, 88 filtered, REAL_EXIT:0.
  - Full parts (no filter): same line with no filter. Log
    `012-module-profile-p3b-fix-full-parts.log`. 89 passed, 0 failed,
    REAL_EXIT:0.
  - Full 012 filter: same line with `012 --nocapture`. Log
    `012-module-profile-p3b-fix-012-regression.log`. 17 passed, 0 failed, 72
    filtered, REAL_EXIT:0 (9 generator + 8 module).
  - `rustfmt --check --edition 2024` PASS REAL_EXIT:0 (log
    `012-module-profile-p3b-fix-fmt.log`, no fix); `git diff --check` PASS
    REAL_EXIT:0 (log `012-module-profile-p3b-fix-diffcheck.log`).
  - ui-shared suite not run: helper untouched, per brief (only if touched).
- Gortex: pre-edit impact MEDIUM lower-bound (file); post-edit detect maps
  ModuleProfileEditor (295) + 012 tests; tests 0/1 graph blind to
  wasm_bindgen_test (as 008/011/prior — behavioral proof is mounted runs above);
  guards no rules; contract WARN behavioral blast 17 lower-bound, no parse
  errors, no signature change. Preserved unrelated dirty `CODING_STANDARDS.md`
  and untracked `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing
  004 authority unchanged except approved preserve; no Pair-exception expansion,
  no P4/other-owner change, no generator change.
- P3 module profile row status: `GREEN` (failure keeps deferring + landing
  re-baselines + revert clears + older-settlement-never-clears-newer). P1
  module profile stays `GREEN` (four cases). P1 generator stays `GREEN` (three
  cases). P2 generator stays `GREEN` (both directions). P3 generator stays
  `GREEN` (failure + revert). P4 generator stays `GREEN` (hidden +
  owner-change). Part A staged-shape-alone explicitly NOT a case (closed NO,
  still stands). Reviewed rides with profile Save (no separate case, still
  stands). P4 retention, fit profile remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): P4 retention for module
  profile (hidden/keep-mounted still defers; unmount/owner-change retires), RED
  then GREEN; then mechanical fit profile. Do not bulk-write tests.

## Progress (fourteenth slice P4 module profile retention GREEN proof gaps, 2026-11-08; NOT resolved — awaiting review)

Fourteenth slice: P4 retention/lifetime for the module profile owner only —
hidden still defers + owner-change retires — at toast, then STOP. No production
behavior change, no oracle changes at all, no other owners.

- Test (a) `mounted_module_profile_hidden_save_draft_still_defers_update_now_at_the_toast_012`
  (2329): valid measured volume "measured-012-p4a" (baseline 0 volumes)
  unapplied via production inputs, hide via `display:none` on root (established
  011 hidden-owner pattern, same as generator P4; module Collapsible
  keep_mounted would also qualify, reuse, do not invent UI), Update Now toast.
  Oracle: Deferred 1 blocker Save ("Module profile" / "Save the module profile,
  or restore its saved values"), 0 adoptions, "measured-012-p4a" + "1 volumes"
  preserved, stays-mounted assert. Distinct story: hidden keeps blocking.
- Test (a) GREEN gap (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_module_profile_hidden_save_draft_still_defers_update_now_at_the_toast_012
  --nocapture`. Log
  `.evidence/update-draft-safety/012-module-profile-p4a-pre.log`. 1 passed, 0
  failed, 89 filtered, REAL_EXIT:0. RED-or-gap: display:none keeps
  signals/guards alive; is_unresolved still sees measured vs 0 baseline — proof
  gap, recorded GREEN. Verified by running, never fabricated RED.
- Test (b) `mounted_module_profile_owner_change_retires_save_draft_012` (2511):
  valid "measured-012-p4b" defers asserted, cancel, owner-change to second
  module definition ("module-second") via production selection signals
  (definition + selected + selection_generation bump, as generator P4), switch
  proof = new owner's clean baseline renders ("0 volumes", measured gone),
  drain, Update Now toast proceeds. Oracle: adoptions==1, update_dialog None
  (no UnresolvedDraft from retired entry). Distinct story: retire wiring. No
  existing oracle weakened. New test-only host
  `mounted_host_with_selection_012` (2452) + `ModuleProfileMountedSignals012`
  (2446) justified: existing mounted_host hardcodes generation 1, selected
  module:module and definition[0], cannot drive owner change; mirrors generator
  P4 selection-signal shape, no product UI invented.
- Test (b) GREEN gap (serialized, test-first pre-prod, no production edit): same flock
  line with `mounted_module_profile_owner_change_retires_save_draft_012
  --nocapture`. Log
  `.evidence/update-draft-safety/012-module-profile-p4b-pre.log`. 1 passed, 0
  failed, 90 filtered, REAL_EXIT:0. RED-or-gap: owner effect retires old guards
  + draft_owner effect clears edits/resets drafts; new owner clean
  (live==accepted) resolves — proof gap, recorded GREEN. Verified by running.
  Root.remove() alone does not run Dioxus use_drop (VirtualDom still alive), so
  owner-change is the faithful retire path; no product UI invented.
- Key coverage (one key proves wiring; rotary inherits): both registrations
  share the identical effect/drop shape — same deps (save_owner_scope, owner),
  same retire-then-register rotation, same use_drop retire, same project_scope
  for poll-prune (verified ModuleProfileEditor 295-1023: save 466-555,
  rotary 557-647). They differ only in guard/draft/edits/baseline/projection/
  admission_key/remedy. Retire drops guards independent of draft content;
  is_unresolved differs only in which draft/baseline it compares, not in
  lifetime. Profile save key (a)+(b) proves the wiring; save-rotary inherits.
- Regression: full `012` filter 19 passed, 0 failed, 72 filtered, REAL_EXIT:0,
  log `012-module-profile-p4-012-regression.log` (17 prior + 2 new: 9 generator
  + 10 module). Full parts (no filter) 91 passed, 0 failed, 0 filtered,
  REAL_EXIT:0, log `012-module-profile-p4-full-parts.log` (89 prior + 2 new).
  Zero regressions. `rustfmt --check --edition 2024` initial FAIL 2 chain-wraps
  (log `012-module-profile-p4-fmt.log` REAL_EXIT:1), fixed via native edits,
  rerun PASS (log `012-module-profile-p4-fmt-rerun.log` REAL_EXIT:0); `git diff
  --check` PASS (log `012-module-profile-p4-diffcheck.log` REAL_EXIT:0).
- Gortex: pre-edit impact MEDIUM lower-bound (file x3: test a, test b+host,
  2 fmt) + LOW (docs); post-edit detect commit-126 maps new test (2329),
  commit-127 attempt1 pending, attempt2 after independent work maps host+test
  (2446 + 2452 + 2511), commit-128/129 fmt maps same; tests 0/1 each (graph
  blind to wasm_bindgen_test, as 008/011/prior — behavioral proof is mounted
  runs above); guards no rules; contract allow MEDIUM lower-bound blast 0 each,
  test-only, no parse errors, no signature change. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority +
  both user authorizations (generator failure preserve + module preserve)
  unchanged; no behavior change, no oracle changes, no Pair-exception expansion,
  no other-owner change.
- P4 module profile row status: `GREEN` (hidden still defers + owner-change
  retires, proof gaps). P1 module profile stays `GREEN` (four cases). P1
  generator stays `GREEN` (three cases). P2 generator stays `GREEN` (both
  directions). P3 generator stays `GREEN` (failure + revert). P4 generator stays
  `GREEN` (hidden + owner-change). P3 module profile stays `GREEN` (failure +
  landing + revert + older-never-clears). Part A staged-shape-alone explicitly
  NOT a case (closed NO, still stands). Reviewed rides with profile Save (no
  separate case, still stands). Fit profile remains `planned`. No premature
  resolved.
- Proposed smallest next slice (awaiting approval): mechanical fit profile P1
  deferral (valid Save draft at toast, RED then GREEN); then P1 header/invalid,
  P3 lifecycle, P4 retention. Do not bulk-write tests.

## Progress (fifteenth slice fit profile P1 Save valid deferral RED then GREEN, 2026-11-08; NOT resolved — awaiting review)

Fifteenth slice: mechanical fit profile owner P1 deferral — valid unapplied
Save draft at toast, RED then GREEN — then STOP. Save only; header, invalid,
lifecycle and retention follow separately. No other behavior change, no oracle
changes at all, no other owners.

- Test `mounted_fit_profile_valid_save_draft_defers_update_now_at_the_toast_012`
  (622): real valid cutout via production Add cutout button (baseline 0
  cutouts; wait for Contour 1 proves processing before gate poll), Update Now
  toast. Oracle: Deferred 1 blocker Save (area Fit profile per 004 N4 / remedy
  Save the fit profile, or restore its saved values per own-controls pattern,
  naming Save fit profile), 0 adoptions, Contour 1 + Vertex 1 preserved.
  Distinct story: fit Save deferral wiring. Reuses mount, open_editor; adds
  minimal FakeUpdateSignals012 + waiting_update_012 (same shape as
  generator/module, local to this file per slice ownership). No existing oracle
  weakened.
- RED (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome --mode
  no-install web/crates/parts --lib --
  mounted_fit_profile_valid_save_draft_defers_update_now_at_the_toast_012
  --nocapture`. Log `.evidence/update-draft-safety/012-fit-profile-p1-red.log`.
  Result: 0 passed, 1 failed, 91 filtered, REAL_EXIT:1 at expected a deferred
  update dialog, got None (641). Contour 1 proof passed, gate reached —
  missing-adapter RED, not setup. No fit adapter exists (verified:
  register_draft_owner only at Geometry/Layout/generator/module sites).
- Domicile: ManualProfileEditor (editor), NOT shell. Evidence: draft signal
  editor.rs:109 `use_signal(initial_profile)`; all inputs mutate it (plate-gap
  parse-guard finite>=0 keeps old, vertex Ok+finite else return with no
  aria-invalid verified full-file, Add cutout/clearance push empty_square,
  Remove/Add vertex, merge_standard_profile, extraction apply); Save submits
  draft() via on_save + on_close. Shell holds ProfileKey::Save one-shot via
  save_profile (ui.rs: is_pending + workspace/selection/generations/scope/
  epoch/doc/definition_id guards → begin_one_shot + SubmittedProfile pending).
  Shell `if editing()` mounts editor else entry; editing=false unmount drops
  the draft, shell stays. Pair observable with correct lifetime only in editor
  (draft present) + submission via ports (shell owns one-shot, editor reads
  it); shell domicile would require lifting draft (invasive, Cancel/drop
  risk) — rejected. Lifetime: editor use_drop retires, scope/owner effect
  retires, poll-prune safe via project_scope.
- Production (fit files only): editor ports +2 closures (save_is_pending +
  submitted_save, Rc ptr_eq) + Runtime context + mount baseline +
  registration effect + use_drop in ManualProfileEditor; shell constructs
  closures from pending_edits + pending (hiding private ProfileKey/
  SubmittedProfile). Judges live MechanicalPartProfile (PartialEq, the value
  Save would submit) vs live-projected accepted (mount fallback only where no
  accepted value exists); same-key in-flight exemption only via ports
  (pending + submitted==live); terminal never exempt (is_pending false +
  pending cleared on settle); newer typing re-raises by mismatch. Scope check
  inside closure (stale scope returns false). Area/remedy above; owner
  fit-profile, admission save, workspace Parts. Test fixture stubs +
  Runtime provide in editor test host (existing extraction test still GREEN).
  No Save policy change, 004 authority unchanged, no mechanical_profile.rs
  change needed.
- GREEN (serialized, identical to RED): same flock wasm-pack line. Log
  `.evidence/update-draft-safety/012-fit-profile-p1-green.log`. Result: 1
  passed, 0 failed, 91 filtered, REAL_EXIT:0. `rustfmt --check --edition 2024`
  PASS REAL_EXIT:0 (log `012-fit-profile-p1-fmt.log`, no fix); `git diff
  --check` PASS REAL_EXIT:0 (log `012-fit-profile-p1-diffcheck.log`).
- Regression: full `012` filter 20 passed, 0 failed, 72 filtered, REAL_EXIT:0,
  log `012-fit-profile-p1-012-regression.log` (19 prior + 1 new: 9 generator +
  10 module + 1 fit). Full parts (no filter) 92 passed, 0 failed, REAL_EXIT:0,
  log `012-fit-profile-p1-full-parts.log` (91 prior + 1 new; includes existing
  extraction + 3 save tests, still GREEN). Zero regressions.
- RED-or-gap reason: RED observed (None instead of Deferred, adapter missing),
  never fabricated; GREEN required the Save registration above.
- Gortex: pre-edit impact file_not_indexed lower-bound (fit files, as prior
  generator slices); post-edit detect maps ManualProfileEditor (91) + Ports
  (58/64/65) + Workspace (43) + new test (622, shifted +11 by shell wiring) +
  Fake (573) + waiting (613); tests 0/1 each (graph blind to
  wasm_bindgen_test, as 008/011/prior — behavioral proof is the mounted runs
  above); guards no rules; contract allow LOW test-only blast 0 + WARN
  behavioral blast 13/18 lower-bound prod, no parse errors, no signature
  change. Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority
  + both user authorizations (generator + module failure only) unchanged; no
  behavior change beyond adapter registration, no oracle changes, no
  Pair-exception expansion, no other-owner change.
- P1 fit profile row status: `GREEN` (first case only: valid Save draft at
  toast). P1 generator stays `GREEN` (three cases). P2 generator stays `GREEN`
  (both directions). P3 generator stays `GREEN` (failure + revert). P4
  generator stays `GREEN` (hidden + owner-change). P1 module profile stays
  `GREEN` (four cases). P3 module profile stays `GREEN` (failure + landing +
  revert + older-never-clears). P4 module profile stays `GREEN` (hidden +
  owner-change). Part A staged-shape-alone explicitly NOT a case (closed NO,
  still stands). Reviewed rides with profile Save (no separate case, still
  stands). Invalid/header/lifecycle/retention, fit P1 remaining cases remain
  `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval): fit profile P1 header +
  invalid (valid Save draft at header as proof gap; realistic invalid
  unsubmitted at toast with input-path evidence for the parse-guard question —
  parse-guard inputs silently keep old values, so the invalid-text path needs
  its own evidence), RED then GREEN; then P3 lifecycle, P4 retention. Do not
  bulk-write tests.

## Progress (sixteenth slice fit profile P1 header GREEN proof gap + invalid input-path BLOCKER, 2026-11-08; NOT resolved — awaiting review/decision)

Sixteenth slice: mechanical fit profile owner P1 — (a) HEADER valid Save
draft at header GREEN proof gap, then STOP for (a); (b) INVALID unsubmitted
at toast BLOCKED with full input-path evidence (no invalid-into-draft path
exists in the fit files; writing an invalid-text deferral test would test a
non-draft, refused per owner-canonical + task). No production change at all,
no oracle changes at all, no other owners. Header completes; invalid awaits
coordinator/human decision (close as vacuous like staged-shape-alone NO, or
direct an alternative).

- (a) HEADER test
  `mounted_fit_profile_valid_save_draft_defers_update_now_at_the_header_012`
  (675): mirrors the fifteenth-slice toast test with only the entry point
  parameterized (`UpdateOpener::Header` vs `Toast`); reuses mount,
  open_editor, FakeUpdateSignals012 (573) + waiting_update_012 (613).
  Baseline 0 cutouts; real valid cutout via production Add cutout button;
  wait for Contour 1 proves processing before gate poll. Oracle (same shape
  as toast): Deferred 1 blocker Save (area Fit profile / remedy Save the fit
  profile, or restore its saved values), 0 adoptions, Contour 1 + Vertex 1
  preserved. Distinct story: header-checkpoint wiring for this owner. No
  existing oracle weakened.
- (a) GREEN proof gap (serialized, test-first pre-prod, no production edit):
  `flock ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless
  --chrome --mode no-install web/crates/parts --lib --
  mounted_fit_profile_valid_save_draft_defers_update_now_at_the_header_012
  --nocapture`. Log
  `.evidence/update-draft-safety/012-fit-profile-p1-header-gap.log`. Result:
  1 passed, 0 failed, 92 filtered, REAL_EXIT:0. Existing Save registration
  (fifteenth slice, editor ManualProfileEditor + shell ports) polls the same
  is_unresolved at both checkpoints, so header defers identically with no
  fix. RED never observed; proof gap recorded GREEN, never fabricated.
  `rustfmt --check --edition 2024` PASS REAL_EXIT:0 (log
  `012-fit-profile-p1-header-fmt.log`); `git diff --check` PASS REAL_EXIT:0
  (log `012-fit-profile-p1-header-diffcheck.log`).
- (b) INVALID input-path analysis (REQUIRED parse-guard question, closed
  with exact evidence; NO test written, NO non-draft tested):
  - Plate-gap: `ManualProfileEditor` editor.rs:466-475
    `EditorNumberField { value: "{profile.plate_to_pcb}" oninput:
    if let Ok(value) = event.value().parse::<f64>() && value.is_finite() &&
    value >= 0.0 { draft.with_mut(|profile| profile.plate_to_pcb = value); }
    }`. Unparseable/non-finite/negative text never enters `draft`
    (MechanicalPartProfile f64); control re-renders old text from draft on
    next render. No `error` prop passed → no aria-invalid (EditorNumberField
    editor_fields.rs:67-110 sets aria_invalid=has_error only).
  - Vertex: `contour_editor` editor.rs:958-975 `value: "{axis_value}"
    oninput: let Ok(value) = event.value().parse::<f64>() else { return; };
    if !value.is_finite() { return; }; draft.with_mut(...)`. Same pure
    parse-guard; no error prop → no aria-invalid. Full-file
    `aria-invalid` search in mechanical_profile_editor.rs: 0 matches
    (confirms fifteenth-slice verification).
  - WHERE invalid text lives: only transiently in the DOM native number
    input's value (browser sanitizes non-numeric; e.g. cleared "" or "-5"
    shows until re-render), never in any Rust signal. Save submits
    `draft()` (old f64), ignoring DOM text: editor Save
    `onclick: on_save.call(draft())` (editor.rs Save button) → shell
    save_profile (ui.rs) guards + begin_one_shot with draft MechanicalPartProfile.
  - Draft-entering paths (all valid, Save accepts): 6 `draft.with_mut`
    sites — 392 extraction apply (Core geometry), 474 plate parsed valid,
    925 Add cutout/clearance empty_square, 943 Remove contour, 966 vertex
    parsed finite, 986 Add vertex duplicate-last — plus
    merge_standard_profile:308 from Core (valid). No String-in-draft user
    field exists (unlike module rotary Strings / generator raw-text edits);
    MechanicalPartProfile (core/src/model.rs:3173) user-editable fields are
    f64/Vec only.
  - Save refusal paths (none value-invalid via UI): save_profile guards
    (ui.rs) are scope/selection/generations/workspace/epoch + only
    value-dependent `profile.definition_id != definition.id`, unreachable
    via production UI (initial correct, merge filtered by
    standard_profile_reply_matches, extraction untouched); resolver
    (mechanical_profile.rs:214-260) refuses only PROFILE_GONE (definition
    deleted, :232) or Unchanged — no value validation, no Delete UI in
    parts src (search 0), contrived not realistic; out-of-range never
    enters (plate >=0 guard, vertex no range); Add paths always valid.
  - Classification: pure parse-guard, control restores old text = no draft
    = nothing to defer, consistent with 004 owner-canonical quote value
    that would be submitted endquote and staged-shape-alone NO precedent
    (012 tenth Part A: pre-Add text not submitted, no deferral). Invalid
    text is NOT an Unresolved draft for fit. No path DOES enter the draft
    but Save rejects was found in the fit files after bounded exhaustive
    search (all draft writes + guards + resolver + census A3 + 004 N3).
  - Specific invalid value: NONE EXISTS realistically via production UI.
    Why Save would reject: N/A (no invalid-into-draft to reject; Save
    accepts all draft values that UI can produce). Where invalid text
    lives: DOM input transient only, never submitted. Do not test a
    non-draft: no (b) test written, no RED fabricated, no production fix
    (a false deferral on no-draft would be wrong).
- Regression: full `012` filter 21 passed, 0 failed, 72 filtered,
  REAL_EXIT:0, log `012-fit-profile-p1-header-012-regression.log` (20 prior
  + 1 new header: 9 generator + 10 module + 2 fit). Full parts (no filter)
  93 passed, 0 failed, REAL_EXIT:0, log
  `012-fit-profile-p1-header-full-parts.log` (92 prior + 1 new; includes
  existing extraction + 3 save tests, still GREEN). Zero regressions.
- RED-or-gap reasons: (a) GREEN proof gap observed (existing adapter covers
  header, same timing GREEN passes, no fix); (b) BLOCKED with evidence
  above (pure parse-guard, no invalid-into-draft path, no test per
  do-not-test-non-draft). Never fabricated RED.
- Gortex: pre-edit impact file_not_indexed lower-bound (fit file, as prior
  slices); post-edit detect commit-131 maps new header test (675) + toast
  (622) + Fake (573) + waiting (613); tests 0/1 (graph blind to
  wasm_bindgen_test, as 008/011/prior — behavioral proof is the mounted
  runs above); guards no rules; contract allow LOW test-only blast 0, no
  parse errors, no signature change. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004
  authority + both user authorizations (generator + module failure only)
  unchanged; no behavior change, no oracle changes, no Pair-exception
  expansion, no other-owner change. Changed files owned: ui.rs test only
  (+ docs progress); editor.rs, mechanical_profile.rs untouched (no change
  needed); frozen generator/module untouched.
- P1 fit profile row status: `GREEN` (two cases: valid Save draft at toast
  + valid Save draft at header proof gap). (b) invalid at toast remains
  `planned` + BLOCKER (no invalid-into-draft path; propose close as vacuous
  N/A like staged-shape-alone NO, awaiting coordinator/human decision — no
  unilateral close, no premature resolved). P1 generator stays `GREEN`
  (three cases). P2 generator stays `GREEN` (both directions). P3 generator
  stays `GREEN` (failure + revert). P4 generator stays `GREEN` (hidden +
  owner-change). P1 module profile stays `GREEN` (four cases). P3 module
  profile stays `GREEN` (failure + landing + revert + older-never-clears).
  P4 module profile stays `GREEN` (hidden + owner-change). Part A
  staged-shape-alone explicitly NOT a case (closed NO, still stands).
  Reviewed rides with profile Save (no separate case, still stands). Fit P3
  lifecycle/P4 retention remain `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval/decision): coordinator/
  human rules on fit P1 invalid vacuity (close N/A with this evidence, or
  directs an alternative realistic invalid path if one is identified
  outside the fit files with ownership); then fit P3 lifecycle wiring
  (Save landing re-baselines, failure/refusal preserves, revert clears,
  newer typing re-raises, older never clears) RED then GREEN; then P4
  retention. Do not bulk-write tests.

## Ruling (coordinator, 2026-11-08): fit P1 invalid table cell N/A — vacuous, not waived

The sixteenth slice's (b) analysis is accepted: no realistic invalid
unsubmitted draft exists for the fit profile owner, so the invalid table
cell of P1 has no inhabitant. This is a vacuity finding with evidence,
not a criterion waiver: the proof map phrases P1 per owner with "or" (a
real valid unapplied draft, or a realistic invalid unsubmitted draft),
one scenario per owner with draft kind as table cases, and the fit owner
already carries valid-toast + valid-header GREEN. Evidence: (1) the
sixteenth-slice exhaustive input-path analysis (plate-gap
editor.rs:466-475 and vertex :958-975 pure parse-guards restoring old
text; zero aria-invalid; all six draft.with_mut sites + merge:308
produce typed valid values; no String-in-draft); (2) coordinator-verified
`mechanical_profile_resolver` (mechanical_profile.rs:211-260) performs
zero value validation — it stores any submitted profile, retiring only
on PROFILE_GONE; (3) census A3 corroboration of the parse-guard inputs.
Under the 004 owner-canonical rule (the value Save would submit), every
draft value the fit UI can produce is acceptable, so invalid text is
never an Unresolved draft here — the same category as the tenth-slice
staged-shape-alone NO. Reversible: this is a ticket-record
classification; the 012 Spec review must independently judge it, and the
user may reopen it at any time before closure. Fit P3/P4 proceed.

## Progress (seventeenth slice fit profile P3 lifecycle — 2 GREEN gaps + 2 BLOCKERs, 2026-11-08; NOT resolved — awaiting review/decision)

Seventeenth slice: mechanical fit profile owner P3 per-key lifecycle wiring
for the single Save key at toast — (a) landing re-baselines, (b) failure
keeps deferring, (c) revert clears, (d) older-never-clears-newer — then STOP.
test-first order per scenario (write, run, LOG before production edit); GREENs
verified by running, never assumed; REDs observed, never fabricated. No
production behavior change at all (2 GREEN gaps on existing product, 2
BLOCKERs with no fix per sixth/twelfth-slice precedent), no oracle changes
at all, no other owners.

- Test (a) `mounted_fit_profile_save_landing_rebaselines_update_now_at_the_toast_012`:
  real valid cutout via production Add, Save submitted and landed
  (observe_next Completed + accepted 1 cutout asserted before gate poll),
  editor closed by Save (asserted), reopened (Contour 1 from accepted),
  Update Now toast PROCEEDS. Oracle: adoptions==1, dialog None. Distinct
  story: landing re-baselines. Adaptation recorded: Save closes editor
  (on_save + on_close) retiring registration, so reopen proves re-baseline
  (draft==accepted) not just retirement. No existing oracle weakened.
- Test (a) GREEN gap (serialized, test-first pre-prod, no production edit): `flock
  ~/.cache/tmp/boardstudio-build.lock wasm-pack test --headless --chrome
  --mode no-install web/crates/parts --lib --
  mounted_fit_profile_save_landing_rebaselines_update_now_at_the_toast_012
  --nocapture`. Log
  `.evidence/update-draft-safety/012-fit-profile-p3a-pre.log` (1 passed, 0
  failed, 93 filtered, REAL_EXIT:0). Fmt RED 1 → 2 native fixes → GREEN 0
  (logs `012-fit-profile-p3a-fmt.log` + `-fmt2.log`); post-fmt GREEN
  `012-fit-profile-p3a-green.log` 1 passed, 93 filtered, REAL_EXIT:0.
  RED-or-gap: live accepted projection each poll; after landing live==new
  accepted → resolves. Proof gap, recorded GREEN.
- Test (b) `mounted_fit_profile_failed_save_keeps_deferring_update_now_at_the_toast_012`:
  valid cutout submitted, driven to terminal FAILURE via
  `support::fail_next_core_reply` (ExecutorFailed → Failed when owner live,
  realistic per sixth/twelfth-slice; chosen over PROFILE_GONE retire which
  has no Delete UI and is contrived; shell settle DOES support core failure
  — terminal alert + accepted unchanged + entry re-enabled observed).
  Oracle: terminal observed, draft Contour 1 preserved on reopen, Update Now
  still defers with Save remedy (RecoveryRequired additive allowed, as
  generator/module P3). Distinct story: terminal never exempts. No existing
  oracle weakened.
- Test (b) RED BLOCKER (serialized, test-first pre-prod, no production edit): same
  flock line with `mounted_fit_profile_failed_save_keeps_deferring_update_now_at_the_toast_012
  --nocapture`. Logs `.evidence/update-draft-safety/012-fit-profile-p3b-pre.log`
  + `-red2.log` (post-fmt). Result: 0 passed, 1 failed at `terminal failure
  preserves the draft for retry` with reopened text showing `No plate cutouts
  defined` not `Contour 1` (ui.rs:846), REAL_EXIT:1 both. Alert observed,
  accepted None asserted — missing-preserve RED, not setup. Fmt RED 1 →
  native fix → GREEN 0 (logs `012-fit-profile-p3b-fmt.log` +
  `012-fit-profile-p3b-fmt2.log`); RED2 same reason. Cause: Save closes
  editor (draft signal dropped) + shell settle clears `pending` on Failed
  → reopen starts from accepted (no cutouts), draft discarded. NO FIX per
  slice brief + sixth/twelfth-slice precedent (needs explicit user approval;
  prior option-1 authorizations were generator-only + module-only). Existing
  004 authority unchanged.
- Test (c) `mounted_fit_profile_revert_to_baseline_clears_save_draft_012`:
  valid cutout defers asserted (1 blocker Save, 0 adoptions), cancel, revert
  via production Remove contour 1 (wait `No plate cutouts defined` + no
  Contour 1 proves processing), Update Now proceeds. Oracle: adoptions==1,
  dialog None. Distinct story: revert clears. No existing oracle weakened.
- Test (c) GREEN gap (serialized, test-first pre-prod, no production edit): same
  flock line with `mounted_fit_profile_revert_to_baseline_clears_save_draft_012
  --nocapture`. Log `.evidence/update-draft-safety/012-fit-profile-p3c-pre.log`.
  Result: 1 passed, 0 failed, 95 filtered, REAL_EXIT:0. Fmt GREEN 0 (log
  `012-fit-profile-p3c-fmt.log`), no fix. RED-or-gap: live==accepted after
  Remove → resolves via canonical PartialEq. Proof gap, recorded GREEN.
- (d) NEWER + OLDER: BLOCKER with analysis evidence, NO test (removed hanging
  attempt; do NOT test non-production paths). Desired same-key story
  (single-key owner, unlike module cross-key): Save + hold → newer typing
  while in flight → land older → newer STILL DEFERS. Impossibility evidence:
  (1) Editor Save button `onclick: on_save.call(draft()); on_close.call(())`
  (editor.rs) always closes, dropping draft signal; (2) Shell
  `pending_edits.bind_one_shot(ProfileKey::Save, save_disabled)` + entry
  Button `disabled: save_disabled()` (ui.rs) disables reopen while pending;
  (3) Existing GREEN `mechanical_profile_save_disables_entry_silently_and_rejects_retained_duplicate`
  proves in-flight editor-closed + entry-disabled. No production input exists
  while Save pending (all fit inputs live in editor). Attempted (d) test hung
  2x (SIGKILL/timeout, logs `012-fit-profile-p3d-pre.log` + `-red2.log` +
  `-fmt.log` GREEN + `-infra-check.log` proving infra OK via (a) GREEN),
  removed to avoid hanging tree. Single-key note recorded. Awaits
  coordinator/human ruling (vacuous N/A like invalid, or product change to
  allow reopen while pending) — no unilateral close, no premature resolved.
- Regressions: Fit filter `mounted_fit_profile` 4 passed + 1 failed
  (expected b), 91 filtered, REAL_EXIT:1, log
  `012-fit-profile-p3-fit-regression.log`. Module filter
  `mounted_module_profile` 10 passed, 0 failed, REAL_EXIT:0, log
  `012-fit-profile-p3-module-regression.log`. Generator filter
  `mounted_generator` 12 passed (9 012 + 3 other), 0 failed, REAL_EXIT:0,
  log `012-fit-profile-p3-generator-regression.log`. Full `012` filter 13
  passed + 11 failed (1 expected b + 10 cascade module mount-timeouts after
  b abort, passes alone above — same as sixth-slice precedent), REAL_EXIT:1,
  log `012-fit-profile-p3-012-regression.log`. Full parts (no filter) same
  cascade + SIGKILL, REAL_EXIT:1, log `012-fit-profile-p3-full-parts.log`.
  Zero regressions (all prior GREEN pass alone; failures are expected b +
  cascade pollution). `git diff --check` PASS REAL_EXIT:0 (log
  `012-fit-profile-p3-diffcheck.log`).
- Gortex: pre-edit impact CRITICAL lower-bound (file, x4 tests + x2 fmt + x1
  removal) + LOW (docs); post-edit detect maps new tests (a/b/c) +
  PartsMechanicalProfileWorkspace + Fake012 + waiting012; tests 0/1 each
  (graph blind to wasm_bindgen_test, as 008/011/prior — behavioral proof is
  mounted runs above); guards no rules; contract allow test-only blast 0
  (a/b/c/d), no parse errors, no signature change. Preserved unrelated dirty
  `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority
  + both user authorizations (generator + module failure only) + fit invalid
  N/A ruling unchanged; no behavior change at all, no oracle changes, no
  Pair-exception expansion, no other-owner change. Changed files owned: ui.rs
  test only (+ docs progress); editor.rs, mechanical_profile.rs untouched (no
  change needed); frozen generator/module untouched by this slice.
- P3 fit profile row status: `GREEN` (landing re-baselines + revert clears,
  proof gaps) + `RED` (failure keeps deferring, blocked on preserve decision)
  + `BLOCKED` (older-never-clears-newer, no production path, awaits ruling).
  P1 fit profile stays `GREEN` (valid toast + header; invalid N/A vacuous per
  ruling). P1 generator stays `GREEN` (three cases). P2 generator stays
  `GREEN` (both directions). P3 generator stays `GREEN` (failure + revert).
  P4 generator stays `GREEN` (hidden + owner-change). P1 module profile stays
  `GREEN` (four cases). P3 module profile stays `GREEN` (failure + landing +
  revert + older-never-clears). P4 module profile stays `GREEN` (hidden +
  owner-change). Part A staged-shape-alone explicitly NOT a case (closed NO,
  still stands). Reviewed rides with profile Save (no separate case, still
  stands). Fit P4 retention remains `planned`. No premature resolved.
- Proposed smallest next slice (awaiting approval/decision): coordinator/human
  rules on (b) fit failure preserve (approve preserve-on-failure + correct
  affected oracles with spec story 12 + 004 "failure never clears" +
  generator/module precedent, then land minimal preserve fix + GREEN (b); or
  revise P3 (b) to already-preserving path) AND on (d) vacuity (close N/A
  with this evidence like invalid, or direct product change to allow reopen
  while pending + full (d) test); then fit P4 retention (hidden still defers
  + unmount/owner-change retires) RED then GREEN. Do not bulk-write tests.

## Ruling (user, 2026-11-08, option 2): fit P3 (b) failure + (d) newer-typing draft scenarios N/A — structural, not waived

The user chose structural N/A for both seventeenth-slice blockers, on the
coordinator's recommendation. The fit draft lifetime ends at SUBMIT by
design (close-on-submit), so failure clears nothing that submit did not
already retire, and no production path admits typing during pending:
- (b) Save button `onclick: on_save.call(draft()); on_close.call(())`
  always closes, dropping the draft signal; shell settle clears `pending`
  on Failed, so reopen starts from accepted. The draft leg therefore has
  no post-failure draft to judge. Safety is covered by the recovery latch
  (Session::core_failed → RecoveryRequired, observed in the (b) RED run's
  terminal state), which defers Update Now independently of any draft.
- (d) Reopen while pending is impossible: the editor is closed and the
  entry Button carries `disabled: save_disabled()` bound to the Save
  one-shot (ui.rs:273); existing GREEN
  `mechanical_profile_save_disables_entry_silently_and_rejects_retained_duplicate`
  proves in-flight closed+disabled. During pending, the save-in-progress
  leg (Session pending work) defers Update Now; no draft input exists to
  protect.
This differs from generator/module (editors stay open; their discard was
gratuitous and user-approved preserve fixes landed). No submit-flow
redesign, no oracle changes, no product change for fit. The (b) RED test
`mounted_fit_profile_failed_save_keeps_deferring_update_now_at_the_toast_012`
is REMOVED as testing a structural non-scenario (not weakened: the
behavior it pins contradicts the designed submit flow); its RED log
remains as the break evidence. Reversible: ticket-record classification;
the 012 Spec review must independently judge it. Fit P4 proceeds; P3 fit
is GREEN (landing + revert) with (b)/(d) N/A.

## Answer

012 complete and coordinator-accepted 2026-11-08: registry adapters + per-key
deferral copy for generator parameters (Apply + Upload keys), module profile
(save + save-rotary keys) and mechanical fit profile (Save key). 25 mounted
Update Now tests GREEN (9 generator + 10 module + 6 fit), full parts crate
suite 97/97 GREEN. Fresh independent Standards and Spec reviews PASS with
full four-file source coverage and no Required findings; both acceptance
readiness lines READY on their axes (Spec's BLOCKED items were closure
mechanics discharged here, except the full project gate which belongs to
015 per the execution plan and 008/009/011 precedent). Row status: P1 GREEN
all owners (fit invalid N/A vacuous, ruled); P2 GREEN both directions; P3
GREEN (generator + module failure preserved per explicit user option-1
authorizations with cited evidence; fit (b)/(d) structural N/A per user
option-2 ruling); P4 GREEN all owners (fit P4b owner-change draft-reset
fix, module precedent); P5 every adapter change carries RED-then-GREEN or
recorded proof-gap evidence. No scope expansion. Ticket resolved; umbrella
019 stays open.

## Completion record

- HEAD: 012 commit on branch fix/ui-integration-recovery (5 owned files:
  4 source + this ticket; source byte-identical to the reviewed frozen
  worktree).
- Checks newly executed: every slice RED-or-gap + GREEN run by delegate
  with REAL exits under .evidence/update-draft-safety/012-*.log, every
  slice independently rerun by the coordinator (012-coordinator-*.log),
  full parts suite 97/97 twice, doc-links 718 PASS. Reused unchanged:
  ABC preparation + 008/009/010/011 acceptance.
- Unmet gates: full project check gate + optimized validation + mutation
  controls + packaged legs (all 015 scope, explicitly deferred per plan,
  not waived). No Class B change. No live delegates/builds at transfer.
- Retained cleanup: none owed (mounted tests only; no browser/packaged
  runs in 012).
- Pointers: proof map + 18 slice progress sections + 3 rulings above;
  coordinator notes update-draft-safety/coordinator.md; Spec review task
  89e1f826, Standards review task b6672485.
- Next action: 013 PCB owners — BLOCKED on the explicit human
  origin-baseline decision (existing re-deferral vs narrow baseline
  amendment); 013 decision section + 004 Pair-only amendment govern. 014
  queued after 013; 015 closes verification.

## Progress (eighteenth slice fit P4 retention — step-1 ruled removal + hidden gap + owner-change RED→GREEN, 2026-11-08; NOT resolved — awaiting review)

Eighteenth vertical slice, two ordered steps, owner wiring only. This completes ALL
012 owner implementations (generator 9 + module 10 + fit 6 = 25 mounted 012 tests GREEN).

- STEP 1 (FIRST, under recorded user ruling authority): REMOVED test
  `mounted_fit_profile_failed_save_keeps_deferring_update_now_at_the_toast_012`
  from `mechanical_profile_ui.rs` (comment block + fn, exact match, nothing else).
  Authority: user option-2 ruling 2026-11-08 recorded above — the test pins a
  structural non-scenario (draft lifetime ends at submit by design; failure clears
  nothing submit didn't retire; recovery latch covers Update Now). Executing the
  ruling, not weakening: its RED log stays as break evidence
  (`.evidence/update-draft-safety/012-fit-profile-p3b-pre.log` + `-red2.log`).
  Post-removal fit filter GREEN before proceeding (see regressions).
- STEP 2 P4 (a) HIDDEN BUT MOUNTED STILL DEFERS: test
  `mounted_fit_profile_hidden_save_draft_still_defers_update_now_at_the_toast_012`.
  Valid unapplied cutout → hide via display:none (established generator/module P4
  pattern; production mechanism is workspace switch keeping signals alive) →
  stays-mounted assert (Save fit profile present) → Update Now toast → STILL DEFERS
  with Save remedy, draft preserved (Contour 1 + Vertex 1). Distinct story: retention
  wiring. test-first pre-prod (no production edit): 1 passed, 0 failed, 95 filtered,
  REAL_EXIT:0, log `012-fit-profile-p4a-pre.log`. Fmt GREEN 0 (log `012-fit-profile-p4a-fmt.log`).
  RED-or-gap: GREEN proof gap — registration reads live draft signal each poll;
  display:none keeps component mounted → still unresolved. No fix needed.
- STEP 2 P4 (b) OWNER CHANGE RETIRES: test
  `mounted_fit_profile_owner_change_retires_save_draft_012` + helpers
  (`FitProfileMountedSignals012`, `mounted_profile_host_with_selection_012`,
  `mount_with_selection_012` with two definitions, module P4 precedent). Valid
  unapplied cutout → defers asserted (1 blocker Save, 0 adoptions) → cancel →
  owner change to second definition via production selection signals (definition +
  selection + generation) → switch proof (new baseline "No plate cutouts defined",
  no Contour 1) → Update Now PROCEEDS (adoptions==1, dialog None, no UnresolvedDraft
  from fit). Distinct story: retire wiring (effect rotation + use_drop on remount).
  test-first pre-prod RED (expected owner-wiring reason, no production edit): 0 passed,
  1 failed at `wait_until: owner change resets to the new baseline` (5s timeout),
  REAL_EXIT:1, log `012-fit-profile-p4b-pre.log`. Fmt RED 1 → 3 native fixes →
  GREEN 0 (logs `012-fit-profile-p4b-fmt.log` + `-fmt2.log`).
- P4b diagnosis (debug log `012-fit-profile-p4b-debug.log`, reverted): after owner
  change, text shows new title "Mechanical second part fit" BUT old draft "Contour 1"
  persists. Root cause: ManualProfileEditor stays mounted across definition switches
  (Dioxus key does not remount single children); `draft` use_signal initializer runs
  once with no reset effect (unlike module's draft_owner effect). Registration effect
  DOES rotate (old retired), but new registration sees old draft vs new empty baseline
  → still unresolved. Expected retire/owner-wiring gap.
- P4b smallest fix (editor.rs only, module precedent, +22/-1): draft_owner effect with
  processed guard resetting `draft` to new baseline on (session_epoch, document_id,
  scope, definition_id) change. Only draft resets; async currency already owner-guarded;
  purposes/errors stay owner-local (caveat: not reset, out of P4 scope). Post-fix GREEN:
  P4b filter 1 passed, 96 filtered, REAL_EXIT:0, log `012-fit-profile-p4b-green.log`.
  Warning fix (remove unneeded mut) → GREEN2 same, no warnings, log `-green2.log`.
  Fmt editor GREEN 0 (logs `012-fit-profile-p4b-editor-fmt.log` + `-fmt2.log`).
- Regressions (all serialized, REAL_EXIT:0): Fit filter `mounted_fit_profile` 6 passed
  (valid toast/header, landing, revert, hidden, owner-change), 91 filtered, log
  `012-fit-profile-p4-fit-regression.log`. Module filter `mounted_module_profile`
  10 passed, log `-module-regression.log`. Generator filter `mounted_generator`
  12 passed (9 012 + 3 other), log `-generator-regression.log`. Full `012` filter
  25 passed (9+6+10), 0 failed, 72 filtered, log `-012-regression.log` (no cascade;
  b removed). Full parts (no filter) 97 passed, 0 failed, log `-full-parts.log`.
  Step-1 post-removal fit 4 passed, log `012-fit-profile-step1-post-removal.log`.
  `git diff --check` PASS REAL_EXIT:0, log `012-fit-profile-p4-diffcheck.log`.
- Gortex: pre-edit impacts CRITICAL lower-bound (ui.rs file, x removal + x2 tests +
  helpers + fmt/debug) + HIGH (editor.rs, 8 dependents) + LOW (docs); post-edit detect
  maps ManualProfileEditor + new P4 symbols (155 changed symbols across 6 files incl.
  5 pre-existing uncommitted); tests 0/2 graph coverage (graph blind to wasm_bindgen_test,
  as 008/011/prior — behavioral proof is mounted runs above); guards no rules; contract
  WARN behavioral blast 13 lower-bound, no parse errors, no signature change (P4b fix).
  Preserved unrelated dirty `CODING_STANDARDS.md` and untracked
  `docs/research/2026-11-07-ai-agent-test-economy.md`. Existing 004 authority + both user
  failure authorizations (generator + module only) + both N/A rulings (fit invalid + fit
  P3 (b)/(d)) unchanged; step-1 is ruled removal, not weakening; no other oracle changes,
  no Pair-exception expansion, no other-owner change. Changed files owned: ui.rs tests +
  helpers, editor.rs draft-reset effect (+ docs progress); mechanical_profile.rs untouched
  (no change needed); frozen generator/module untouched by this slice.
- P4 fit profile row status: `GREEN` (hidden gap + owner-change RED→GREEN). P1 fit profile
  stays `GREEN` (valid toast + header; invalid N/A vacuous per ruling). P3 fit profile stays
  `GREEN` (landing + revert; (b)/(d) N/A structural per ruling; (b) test removed, RED log kept).
  P1 generator stays `GREEN` (three cases). P2 generator stays `GREEN` (both directions).
  P3 generator stays `GREEN` (failure + revert). P4 generator stays `GREEN` (hidden + owner-change).
  P1 module profile stays `GREEN` (four cases). P3 module profile stays `GREEN` (failure + landing +
  revert + older-never-clears). P4 module profile stays `GREEN` (hidden + owner-change).
  Part A staged-shape-alone explicitly NOT a case (closed NO, still stands). Reviewed rides
  with profile Save (no separate case, still stands). ALL 012 owner implementations COMPLETE
  (25/25 GREEN). No premature resolved — closure needs coordinator Spec/Standards reviews
  + full-suite/build gates + commit (coordinator-owned).
