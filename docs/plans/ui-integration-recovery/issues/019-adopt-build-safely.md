# 019: Adopt the waiting build through protected Refresh

Status: resolved
Type: build
Blocked by: [018: Waiting-build notification](018-notify-waiting-build.md)
Parent: [013: Build update umbrella](013-surface-build-updates.md)
Related: [017: Specification and inspected sources](017-build-update-spec.md), [ADR-0022](../../../adr/0022-static-local-first-rust-application-with-worker-execution.md)

## What to build

Complete the chosen notification with a keyboard-accessible Refresh action that
actually adopts the waiting build while preserving work and other open editors'
assets. This is one safety-bounded vertical action: exposing reload separately
from its guards would violate 013. It depends on 018's real waiting state,
notification and A/B fixture; it does not depend on unrelated recovery tickets.

## Acceptance criteria

- [x] The action is labelled **"Update Now"**, a working clickable control in the Toast
  and in a persistent **header-bar notice** that appears once the Toast is dismissed and
  stays while the update waits (user, 2026-11-06: the header notice needs a working
  Update Now, not a state-only label). This ticket delivers both, because the header
  notice is useless without the action. Add them through the existing shared
  control/header seam; mounted tests cover both entry points and that the header
  notice survives Toast dismissal and workspace changes.
  Refresh uses real waiting state and rechecks safety at activation. Avoid
  duplicate adoption from repeated activation or stale notifications.
- [x] Inventory the relevant Draft value, Pending edit and local-save owners.
  Warn before reload and offer cancellation when work is at risk. Cancellation
  preserves the editor and values; pending or failed save never counts as safe.
  A warning acknowledgment alone is not permission to discard work. The user
  decided on 2026-11-06 that unsaved drafts that cannot yet be saved **defer the
  update** until the user resolves them; explain that and record any gap in detecting
  them.
- [x] Demonstrate safe refresh after work settles and saves, with the same saved
  project reopened. Do not equate the header save label with complete draft safety.
- [x] Preserve ordinary delayed activation and other open editors' offline assets.
  Document the adoption/cache mechanism before applying it; the current activation
  cleanup removes old scoped caches, so forced activation with unchanged cleanup
  is insufficient. Add a dated ADR-0022 amendment for any policy refinement.
- [x] When another editor prevents safe adoption, preserve it and tell the user to
  close other open tabs (user, 2026-11-06: wording like "Please close any other open
  tabs"). Never silently force, close or reload other tabs. Any further interaction
  that needs a new product decision is recorded as an exact blocker.
- [x] Mounted production tests drive Refresh, warning/cancel and pending/failure
  states. Shared Toast tests cover action reachability, Tab/Enter/Space, focus
  after cancellation, normal/reduced motion and Theme/Appearance/narrow layouts.
  Preserve existing transient notifications and persistent inline reports.
- [x] Extend 018's real A-then-B scenario: A remains controlled and open while B
  installs and waits; the notification appears; cancel with a draft and pending
  save; settle/save, activate Refresh and prove the loaded application is B and
  the saved project is intact. A worker label or server manifest alone is not proof.
- [x] Repeat with a second A editor kept open and an offline request for a cached
  A asset it had not yet used. Prove continuity through adoption or explicit
  deferral, then complete B adoption once the obstacle is resolved. Verify
  root/subpath scope isolation and no reload on failed installation.
- [x] Capture RED on the missing adoption behavior and GREEN on the same real
  scenario after implementation, plus mounted safety controls. Do not clear caches,
  unregister workers or replace real lifecycle with mocks to obtain GREEN.
- [x] Record whether optional running version/time display was included or omitted
  (user: drop it if it needs plumbing). Any shown identity belongs to the loaded page.
  Reconcile 013's acceptance list with evidence without closing 010/011 or claiming
  historical stale-build causation.

## Verification and completion

Follow 017's full testing protocol and cited prior art (S3–S9). Run focused
mounted/shared-control tests, the dedicated packaged A/B browser scenarios and
python3 scripts/check.py quick for touched crates. Record commands, counts,
source revisions, both build identities and .evidence/ log paths. Keep builds
serialized per target and leave the live preview alone. A blocked real scenario
is an explicit incomplete criterion, not a mounted-test substitute.

## Evidence (2026-11-07)

Historical receipts below are retained, not current implementation state. **Resolved
2026-11-08** after combined review and the AC7 delta recheck; see the Completion
record and Answer below. `.evidence/` is git-ignored, so these paths are local to
the shared checkout. Implementation was committed in `398c7a45c`, with registry
follow-ups `10df36141` / `02ec0b636` and the combined AC7 driver proof in `118e60f1`.
The original HEAD `366d52100d9dbf6686a9cd9dfdf02f0262a13eed` plus then-uncommitted
019 work (tracked-019-diff sha256 `58aa9326…a151`;
`019-adoption-identity-pre-build2.txt`) is a historical build receipt, not current HEAD.
Page-probe-era mechanism and safety limitations below are **superseded** by the
worker-authoritative protocol and draft-safety 008–015 where explicitly reconciled
in the completion record; they are not erased or represented as current gaps.

**Slices A-D** (RED/GREEN observed in-session; no per-slice log was saved, so
the in-session results below are the coordinator's record, and the slice E reruns
re-exercise C and D):

- A: `Session::has_pending_work`; `application/tests/session_pending_work.rs`, 1 test
  (`pending_work_covers_queued_active_and_unanswered_save`).
- B: pure gate `web/crates/runtime/src/update_safety.rs`, 6 native unit tests.
- C: generic `ToastActionSlot` in ui-shared toast; 2 mounted `toast_action` tests.
- D: Header `update_available`/`on_update` notice; 2 mounted `header_update` tests.

**Slice E** (Runtime orchestration, bridge, mounted safety tests; notes
`.evidence/ui-integration-recovery/019-efix-notes.log`):

| Check | Result | Log |
|---|---|---|
| Cancel-focus reproducer | 1/1 | `019-efix-focus1b.log` |
| `notifications::tests` (7 new + all 018) | 14/14 | `019-efix-5.log` |
| `header_update` | 2/2 | `019-efix-6-header.log` |
| ui-shared `toast_action` | 2/2 | `019-efix-7-toast.log` |
| full-page `mounted_` | 41/41, no cascade | `019-efix-8-mounted.log` |

Pre-fix failures kept as history, not evidence: `019-efix-notifications.log` and
`019-efix-notifications2.log` (8 passed / 6 failed) and `019-efix-focus1.log`.
Coordinator reran the reproducer and the save-failed deferral guardrail (both ok).

**Packaged A/B** (`scripts/verify-update-adoption.py` + `scripts/verify-update-adoption-payloads/`;
notes `019-adoption-notes.log`, build `019-adoption-build-a.log`):

| Run | Result | Log |
|---|---|---|
| GREEN `post`, run 2 | 194 PASS / 0 FAIL | `019-adoption-green2.log` |
| GREEN `post`, run 3 | 196 PASS / 0 FAIL | `019-adoption-green3.log` |
| RED `pre` (018 pair, no 019) | 6 PASS / 3 FAIL, rc=1: toast Update Now, header Update Now, adoption | `019-adoption-red1.log` |
| RED `pre`, coordinator rerun | exit 1, same 3 FAILs | `019-adoption-red-coord.log` |

Legs: adopt, second-editor (cancel-only "Please close any other open tabs", offline
A asset served from A's cache, adoption after the second editor closes), subpath
isolation, failed-install (script-error and missing-asset), unavailable-target.
Earlier runs are harness history: `019-adoption-smoke1.log` (44/2), `-smoke2.log` (0
failures), `-green1.log` (128/23) and `-iter2.log` (64/11).

A/B identities (stamped copies of one `--fast` build; packaged diff asserted to be
exactly `index.html` + `service-worker.js` per route):

- root A `boardstudio-7a42a81855d780f7-<ns>`, B `boardstudio-fa8cd6cd0283ac83-<ns>`
- subpath A `boardstudio-544117cdac5e969f-<ns>`, B `boardstudio-1397d93191557955-<ns>`

**Carried risks (historical 2026-11-07; final dispositions below supersede this list):**

- `--fast` builds only; non-fast fails in wasm-opt (wasm-pack 0.15.0 TOML error), so
  the shipped optimized build is unexercised.
- The packaged run omits the draft + pending-save cancel case and settle/save under a
  pending-save gate; mounted tests cover them.
- Deferral scope is any same-origin window, including a subpath editor (conservative).
- Draft detection gap: component-local signals, no global registry; recorded in
  `update_safety.rs` and the ADR-0022 amendment.
- Pre-existing and out of scope, unchanged: a heuristically fresh HTTP cache can make
  the precache fetch a stale `index.html`; a failed asset install leaves its own
  incomplete cache until the next activation.
- The two Open-PCB mounted tests' pre-change status is unconfirmed (no clean-HEAD baseline).
- RED is a control against pre-019 builds, not a revert in the same tree.
- Running version/time display is **omitted**, as in 018: the notice carries no identity.
- `scripts/check-native-tests-baseline.json` is **not yet updated** for the new native
  tests (`session_pending_work` binary, `boardstudio_web_runtime` `update_safety::tests`);
  `check.py test --update-native-baseline` after the full gate must record them.
- ADR-0022 amendment dated 2026-11-07 records the mechanism; the full `check.py` gate
  and the review are outstanding.

## Comments

The following 2026-11-06/07 entries are historical checkpoints. Their pending-review,
no-closure and page-probe-era statements are superseded by the 2026-11-08 Completion
record; failure receipts and attribution caveats remain evidence history.

- 2026-11-07: Independent Standards and Spec review identified two standards
  concerns and five spec concerns. Review fixes replace the page-side client
  probe with a worker-authoritative check, recheck safety before reload, detect
  visible `aria-invalid` drafts, narrow toast matching, and expand keyboard and
  packaged coverage. The standards probe duplication was removed; title-based
  toast matching remains a constrained heuristic. **Review remains incomplete:**
  drafts without blur commit or `aria-invalid` remain undetectable; the packaged
  pending-save scenario uses mounted coverage instead of a real held save; the
  invalid field is injected; worker restart and navigating-client cache races
  remain unproven. See `.evidence/ui-integration-recovery/019-rev-notes.log` for
  full dispositions and further limitations. The delegate's final packaged run
  reported 339/339 passing (`019-rev-green-final.log`), and its quick gate passed
  (`019-rev-check-quick4.log`). Coordinator independently reran the pre-fix
  late-adoption rejection: exit 1 with seven expected failures
  (`019-review-coordinator-red.log`), then the full `scripts/check.py` gate:
  all six steps passed (`019-review-coordinator-full.log`). Other suggested
  mutation/rejection reruns remain outstanding; no closure is claimed. Earlier
  evidence below describes the superseded page-probe implementation.

- 2026-11-07: FULL GATE GREEN, implementation phase complete (review not started,
  no closure claimed). `scripts/check.py` exit 0, all steps passed
  (repo/tooling/build/lint/test/browser, 440s; log
  `.evidence/ui-integration-recovery/019-check-full5.log`). Gate fallout fixed:
  (1) stale `test-build-web.py` skipWaiting assertion rewritten to the approved
  contract (install/activate skip-free, one gated occurrence; 8/8 + negative
  control); (2) rustfmt applied to owned 019 files only; (3) slice-D header
  fixture given the 3 theme providers (full-shard viewport pollution exposed
  the missing AppearanceState; isolation had passed). Native baseline updated
  with ONLY 019's 7 test names (full regen reverted: it swept ~40 unrelated
  entries from other lanes' uncommitted tests); `check.py test` green. Wasm
  baseline needs nothing (debt ratchet; 019 adds only `wasm_bindgen` tests).
  Coordinator direct changes this phase: header fixture, fmt, baseline JSON.
  Stopping at the phase boundary for independent review + approval.

- 2026-11-07: Packaged A/B adoption proof accepted GREEN. Driver
  `scripts/verify-update-adoption.py` + 7 payloads: GREEN `post` run twice (194
  and 196 PASS, 0 FAIL) across adopt / second-editor / subpath / failed-install
  / unavailable-target legs; RED `pre` control against the 018 pair fails only on
  the 3 missing-019 behaviors (toast Update Now, header Update Now, adoption).
  A/B are stamped copies of one `--fast` packaged build (root+subpath); packaged
  diff asserted exactly `index.html` + `service-worker.js` per route; identities
  and hashes in `.evidence/ui-integration-recovery/019-adoption-notes.log`.
  Coordinator independently reran RED `pre` (exit 1, same 3 FAILs;
  `019-adoption-red-coord.log`). Carried risks: `--fast` builds only (non-fast
  fails in wasm-opt, wasm-pack 0.15.0 TOML error); packaged run omits the
  draft+pending-save cancel case (mounted tests cover it); deferral scope is any
  same-origin window; pre-existing stale-shell + failed-asset-cache observations
  out of 019 scope and unchanged. Next: docs slice (ADR-0022, evidence, 013
  reconciliation, native baseline), then full gate + phase-boundary report.

- 2026-11-07: Slice E accepted GREEN. Sonnet coding delegate fixed the Cancel-focus
  failure: the `UpdateWarningDialog` effect captured its first-render closure, so the
  open->close transition never fired; it now reads `version()` plus live runtime
  state. Same-file test fixes: fixture `version` read, `switch_to_pcb`
  compact-header helper (headless Chrome is 780px, below the 1250px breakpoint),
  dismissal check on `[data-slot=toast]`. Results: reproducer 1/1,
  `notifications::tests` 14/14 (7 new + all 018), `header_update` 2/2, ui-shared
  `toast_action` 2/2, full-page `mounted_` 41/41 with no cascade. Evidence:
  `.evidence/ui-integration-recovery/019-efix-*.log` (notes: `019-efix-notes.log`).
  Coordinator independently reran the reproducer and the save-failed deferral
  guardrail (both ok) and verified only `notifications.rs` changed in that run.
  Open risk: the two Open-PCB tests' pre-change status is unconfirmed (no
  clean-HEAD baseline). Next slice: packaged A/B adoption driver.

- 2026-11-07: Implementation checkpoint, coordinator transfer (status unchanged:
  no closure claimed, no commit/push; independent spec+standards review still pending).
  Slices A-D GREEN with RED/GREEN evidence in-session: (A) `Session::has_pending_work`
  + `application/tests/session_pending_work.rs`; (B) pure gate
  `web/crates/runtime/src/update_safety.rs` + 6 native unit tests; (C) generic
  `ToastActionSlot` in ui-shared toast wrapper + 2 mounted tests; (D) Header
  `update_available`/`on_update` notice + 2 mounted tests + `web/assets/header.css`.
  Slice E (Runtime orchestration + bridge + mounted safety tests) is IN PROGRESS and
  BLOCKED on one focus failure: `mounted_update_now_warns_and_cancels_while_an_edit_is_pending`
  fails at the cancel-focus assertion (`notifications.rs:964`): after Cancel,
  `document.activeElement` is `body`, not the toast Update Now. The toast action
  Button was missing `aria-label` (header's had it); adding it did NOT fix the
  failure, so the cause is elsewhere (effect timing vs AlertDialog unmount focus
  handling, or the focus query). Dialog open/warn/cancel-preserve behavior itself
  works. Combined `mounted_` page-suite run also showed a 22-test cascade (render
  waits timing out in unrelated modules) after that failure; unresolved whether the
  failing test's leftover wedges the page or it is load flake; rerun the
  notifications-only subset after the focus fix to separate the causes.
  Owned 019 files (uncommitted, shared checkout): `application/src/session.rs`,
  `application/tests/session_pending_work.rs`, `web/crates/runtime/src/update_safety.rs`,
  `web/crates/runtime/src/{lib,runtime}.rs`, `web/crates/host/src/{offline,host/offline}.rs`,
  `web/crates/ui-shared/src/ui/toast.rs`,
  `web/crates/ui-shared/src/toolbar_toggle_slider_toast_tests.rs`,
  `web/src/{service_worker.rs,presentation.rs,presentation/header.rs,presentation/notifications.rs}`,
  `web/assets/header.css`. Untouched unrelated work preserved: 026-merge-gate.md,
  022 ticket, plan map, `drag_smoke.rs`, untracked `ui-status.md`.
  Remaining slices: E completion (7 mounted tests + 018 regressions green),
  packaged A/B adoption driver (pre-019 RED / post GREEN, second-editor deferral +
  offline continuity, subpath isolation, failed-install control), ADR-0022 dated
  amendment, native test baseline update, full `check.py` gate, 013 reconciliation.
  Design record (mechanism documented before applying): opt-in sole-client-gated
  `skipWaiting` + `controllerchange` reload via `ADOPT_WAITING_MESSAGE`; ordinary
  delayed activation untouched; `STORAGE_WINDOW_QUERY` reuse for the sole-client
  probe; cancel-only warnings (no discard path); draft-detection gap recorded in
  `update_safety.rs` docs (component-local draft signals, no global registry).
  Next: coordinator delegates disjoint coding/testing tasks to fresh-context agents.

- 2026-11-06: Published through ticket splitting at the user's explicit request.
  The two slices are intentionally notification first, then the complete guarded
  action; safety is not deferred to a later hardening ticket. No implementation,
  RED/GREEN execution or acceptance completion has occurred in this planning task.

## Completion record (coordinator, 2026-11-08)

HEAD `118e60f18c9d0749f69506aee6f64e78f8696490`; Status `resolved`, READY.
Combined Sol review on `366d5210..5a45a0e39`: Standards PASS, Spec CHANGES REQUIRED
with one Required finding — AC7 needed combined packaged draft + held-save proof.
The driver fix committed in `118e60f1` (`scripts/verify-update-adoption.py:2123–2132`)
adds that conjunction. Delta recheck: Spec PASS / Standards PASS carried, READY;
review checkpoints `review-019` and `review-019-delta` hold the review records.

**Executed for 019 closure (not rerun by this documentation reconciliation):**

- `019-ac7-legs.log` in `.evidence/ui-integration-recovery/`: takeover combined
  draft + genuinely unanswered save, Cancel preserving both, later settle/save,
  adopted B and exact reopened project `e711302e-c95e-4cdd-b899-f0d9f968fef3`, revision 4,
  all PASS. The run exited 1: **layout in 019-ac7-legs.log never passed import setup**;
  that failed leg is not a layout pass or an all-green run.
- **Layout flake:** rerun `019-ac7b-layout.log` behavioral assertions PASSED (both
  Cancel 7.25, adopted B, exact reopened) despite stale preview-click failure, exit 1.
  Clean layout receipt reused from draft-safety: `015-p5s-optimized-layout-leg.log`,
  57 PASS / 0 FAIL. Neither exit-1 run is relabelled green.
- **Tooling flake:** `019-ac7-tooling.log` empty-PID interrupt failure → rerun
  `019-ac7-tooling2.log` all passed (34 driver tests). The initial failed invocation
  is retained, not counted as a pass.
- Rejection reruns (2026-11-08, `019-rejection-rerun-*`): 4/4 killed,
  `restored_exact` — invalid-draft, reload bypass, Space (two receipts), toast-identity.
  Owned packaged cleanup: `019-ac7-cleanup.log` and `019-ac7b-cleanup.log`.

**Reused checks and acceptance reconciliation:**

- AC1/5: mounted Toast/header controls, cancellation/focus and persistent notice
  receipts above; packaged second-editor/offline continuity, scope isolation and
  failed-install controls in `019-rev-green-final.log` (339/339). Sole-client
  authority is in the worker, not a page probe; blocked adoption is cancel-only.
- AC2/3: [draft-safety 008–015](../../update-draft-safety/map.md#decisions-so-far),
  especially [010 packaged proof](../../update-draft-safety/issues/010-packaged-a-to-b-proof.md#answer)
  and [015 closure](../../update-draft-safety/issues/015-coverage-mutation-and-closure.md#answer):
  Runtime registry, all 14 Class A owners / 17 registrations, held-save/failure/retry,
  exact saved-project reopening. AC7 is now additionally proven **combined** above.
- AC4/8: worker-authoritative adoption and guarded cache cleanup, ordinary delayed
  activation unchanged; second-editor/offline/root-subpath/failed-install receipts
  above; dated ADR-0022 amendments 2026-11-07 and 2026-11-08.
- AC6: review-accepted packaged keyboard-matrix **89 PASS / 0 FAIL** satisfies
  the control/reduced-motion/Theme/Appearance/narrow-layout intent; mounted fixtures
  have no app CSS and are not offered as packaged layout proof. Own Update Now and
  Cancel controls are protected, with the Space rejection proof above.
- AC9: genuine pre-019 real-boundary RED and post-019 GREEN receipts above, plus
  draft-safety 010/015 protection proof and rejection controls; no lifecycle mocking
  or cache clearing substituted for adoption.
- AC10: version/time display omitted (no identity in the notice); 013/017 reconciled
  in this same pass, without closing 010/011 of the UI recovery plan or attributing
  the historical stale-build incident to a proven cause.
- Full gate **reused**, not newly claimed: `.evidence/update-draft-safety/015-coordinator-full-gate-fdlock.log`,
  all steps passed on the 015 work based at `240b0634` (its four production Rust
  lint fixes were inside that run; no later production Rust delta). Driver deltas
  re-receipted by `015-p5s-tooling.log` and `019-ac7-tooling2.log`. No new full gate
  is claimed for these documentation edits.

**Final risk dispositions (supersede the historical carried-risk list):**

- `--fast`-only: CLOSED-BY 015's optimized build and 57/0 layout leg. Packaged
  pending-save omission: CLOSED-BY draft-safety 010 and AC7's combined proof.
- Same-origin deferral wording: STALE — isolation is per registration scope.
  Draft-detection gap: SUPERSEDED for Class A by registry 008–015; Class B remains
  explicit revocable risk acceptance, not proven safe. Its trigger is any observed
  Class B draft loss, new Class B owner, or blur-ordering counterexample (Enter→blur,
  portal focus-out, unmount-without-blur); see the 2026-11-08 ADR amendment.
- Worker-restart/ADOPTER lifetime and navigating-client/cache races: explicitly
  **retained risks**, not proven safe; ADR residuals remain. Stale precache and
  failed-install incomplete cache: **STILL-OPEN, out of scope**, not fixed by closure;
  see draft-safety [map](../../update-draft-safety/map.md#out-of-scope) /
  [spec](../../update-draft-safety/spec.md#out-of-scope) boundaries.
- Open-PCB pre-change baseline: permanent attribution caveat; RED-as-control
  (pre-019 build, not same-tree revert) caveat retained. Native test baseline: CLOSED
  by the recorded 019 baseline update/full gate. Toast title heuristic: constrained,
  out of scope, not a general identity guarantee.
- **Open product question, outside 019 scope:** should Layout Space-to-pan apply
  on all buttons? Own update controls are protected and rejection-tested; this
  closure makes no global button policy decision and creates no ticket.

Documentation-only checks newly executed in this reconciliation: `python3
scripts/check-doc-links.py` PASS (719 Markdown files;
`.evidence/ui-integration-recovery/019-doc-reconciliation-links.log`),
`git diff --check` PASS, and final edited-section/full-diff read. No new full gate.
Unmet mandatory gates: none for 019 under the review-accepted dispositions above.
Retained cleanup: none from the owned packaged runs (cleanup receipts above).
Next: reconcile 013/017 in this same pass, then parent plan 020/021; no parent-plan,
UI recovery 010/011 or merge acceptance is implied.

## Answer

Protected Update Now is complete and reviewed: all ten criteria are reconciled,
including combined packaged draft + held-save cancellation, loaded B and exact
saved-project reopening. Spec PASS / Standards PASS, READY. The completion record
separates executed/reused proof, failed runs and retained risks; 019 is resolved,
not a claim that worker restart, navigating clients or every Class B race is safe.
