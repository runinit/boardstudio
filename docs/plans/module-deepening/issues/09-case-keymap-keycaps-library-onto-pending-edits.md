# 09: Case, Keymap, Keycaps and Library settle through `PendingEdits`

Status: resolved
Type: build
Blocked by: 05
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Move the remaining settlement sites onto `PendingEdits` (files at `915d305c0`):

- `web/crates/case/src/`: `case_controller.rs`, `mechanical_settings_controller.rs`
  (only its settlement and `begin_edit` port; the patch logic moves in
  [typed Core edits 02](../../typed-core-edits/issues/02-mechanical-settings-patch.md));
- `web/crates/keymap/src/keymap/`: `binding_controller`, `layer_controller`,
  `macro_controller`;
- `web/crates/keycaps/src/keycaps_settings.rs`;
- `web/crates/library/src/library.rs`.

## Migration rules (same for 07, 08 and 09)

- Every settlement site uses `PendingEdits` and the `ui-shared` helpers from
  [ticket 05](05-pending-edits-module.md); the panel keeps only its projection, its
  resolvers and its own owner predicate.
- Retirement is silent and there is no "Saved" status
  ([ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)).
  Update mounted tests that assert a removed message or status.
- The panel chooses where a failure message appears; default inline at the field.
- Tests that call `EditResolver::resolve` by hand move to the native Runtime
  ([ticket 03](03-native-test-runtime.md)); native tests that only restated settlement
  are deleted.

## Acceptance criteria

- [x] No per-panel settle loop or feedback enum duplicating the module remains
  (`CaseBodyEditState` and the like). The shared modules own ticket settlement,
  latest-per-key observation and bound-field restoration/failure writes. Callers
  may project drained results for domain metadata, error placement and precise
  Landed follow-ups; those projections must not restore bound drafts or implement
  a second settlement policy.
- [x] `MechanicalSettingsController`'s commit order across catalogue loads
  (`flush_prepared`) and its native tests are unchanged.
- [x] Keymap and Library gain native tests through the native Runtime where they had
  none.
- [x] Mounted tests pass.

## Verification

```sh
cargo test -p boardstudio-web-case -p boardstudio-web-keymap -p boardstudio-web-keycaps -p boardstudio-web-library --locked
python3 scripts/check.py lint typecheck test browser
```

## Outcome

Root integrated the preserved external branch at `72ba713b7` with merge
`d72e7e55c`, then completed the consumer bindings in `ed92e9d9`. Shared typed
bindings are isolated in `9cf6a268`; the coordinated lifecycle extension and its
binding epochs from the external branch are retained. No history was rewritten.
The old hold and checkpoints below are historical and superseded by this Outcome.

All named sites use PendingEdits/PendingEditSignals: Case body and mechanical
fields, Keymap layer/macro/binding controllers, Keycaps settings, and Library name
editing. Case's submitted-draft/saved-ack layer and Library's extra settlement reset
are removed. Mechanical child components bind their actual draft/failure Signals
and unbind on drop. Stable UI keys distinguish profile-volume siblings even where
the domain patch field is shared. Catalogue preparation retains bounded admission
metadata, exact input and latest request identity only until submission; failure
before a ticket exists remains an admission concern. `flush_prepared` preserves
its prefix order and existing native queue/Undo assertions.

Macro fields submit exact text before parsing, including keycodes and delays;
accepted projection handles canonical values. Ordinary dirty/accepted projection
preserves newer text, including a return to the old accepted value, after an older
outcome drains. Layer, Macro and Binding owners observe workspace departure before
settlement; editor-lifetime hooks retain Session execution while UI observations
retire. Keycaps retry payloads and Library submitted-owner/dirty state are domain
admission and projection, not another ticket restoration policy. Feedback projections
may place errors or perform precise Landed follow-ups. Retirement is silent and
Saved messages are gone.

### Verification

Final source is pinned to `ed92e9d9d05de7c9ff3d1ad6e696e631e819091c`.
Root executed the checks in the integration worktree with `CARGO_BUILD_JOBS=2`
and exclusive Chrome ownership. Behavioral regressions failed at real mounted
Runtime gates before their fixes; interface compilation RED covered the typed helper.
Fixture compile errors and incorrect failure injections are excluded from RED evidence.

| Gate | Executed result |
| --- | --- |
| `check.py lint typecheck` | Pass, including the WASM page presentation |
| Workspace native tests | Pass; Case 17, Keymap 8, Keycaps 7, Library 5; Runtime 110, UI-shared 8, Core 303 |
| `check.py browser` | Pass: all crate groups and page presentation 43/43; no missing or duplicate terminal outcomes |
| Full native `test` step | Fails only at the reproduced CAD gasket-volume baseline: 49 passed, 1 failed, 4 ignored |

Keymap's whole suite passes in one batch with the 120-second budget; no test is
excluded to avoid driver cleanup. Native Library has five Runtime tests; the native
Keymap layer and macro resolver suites have three each. Binding resolution still
uses its WASM view projection and is verified by mounted tests; extracting its
native seam is deferred. All new child-field and Signal-lifecycle behavior executes
in Chrome, not under native cfg.

Representative consumer evidence, in addition to the complete suite:

| Consumer path | Mounted Runtime evidence |
| --- | --- |
| Case body | `a_failed_unchanged_body_draft_restores_the_accepted_value_with_an_inline_failure`; `a_newer_body_draft_survives_an_older_failure_that_reports_inline`; `a_second_edit_to_one_body_field_replaces_the_observation_and_both_edits_undo`; `a_body_field_that_leaves_with_its_body_tab_keeps_its_held_edit` |
| Mechanical number/text | `unchanged_failed_mechanical_fields_restore_accepted_text_and_report_inline`; `older_mechanical_failures_report_beside_newer_drafts_and_original_accepted_text`; `mounted_mechanical_latest_commit_keeps_both_edits_and_undo_steps`; `presentation_departure_retires_field_observation_without_canceling_session_edit` |
| Mechanical preparation | `preparation_from_an_unmounted_field_cannot_bind_a_replacement_field`; `profile_volume_siblings_bind_distinct_fields_for_the_same_domain_request`; `newer_catalogue_failure_and_draft_survive_an_older_actual_ticket_landing`; both latest-preparation order tests |
| Macro step fields | `a_failed_keycode_restores_unchanged_submitted_text`; `a_failed_delay_restores_unchanged_submitted_text`; newer-draft inline-failure cases; `the_latest_step_keycode_observation_preserves_both_undo_steps`; generation and workspace departure cases; four raw whitespace/leading-zero cases |
| Keymap layer/binding | unchanged layer-name restore, newer layer-name failure, silent owner departure, `failed_binding_restores_accepted_field_and_explains_failure`, `two_binding_fields_queue_and_undo_in_commit_order` |
| Keycaps | unchanged legend restore, newer legend inline failure, `a_second_legend_edit_replaces_the_observation_and_both_edits_undo`, `leaving_the_keycaps_workspace_retires_a_pending_edit_silently` |
| Library | unchanged project-name restore, newer project-name inline failure, `two_project_name_commits_keep_the_latest_observation_and_both_undo_steps`, retained action after unmount and delayed persistence after owner replacement |

### Standards review

Final review of `974e367e7c8fc61ac94751295854ab6727b3590d` through
`ed92e9d9d05de7c9ff3d1ad6e696e631e819091c`: zero documented breaches.
One nonblocking duplication smell remains in the Case/Keymap OwnedEdits wrappers,
already assigned to [cleanup](13-cleanup.md). No visibility approval gap remains.

### Spec review

The same pinned range has no blocking implementation findings. Shared helpers own
submitted-value restoration and failure writes; pre-ticket preparation, domain retry
intent and precise follow-ups remain with callers. Actual tests prove newer drafts,
child epochs and departed presentation owners. No decision or ADR was weakened.

### Graph coverage and remaining limits

Root force-indexed the actual integration worktree under its own alias and verified
new helper/preparation symbols. Precommit analysis observed 20 files / 164 symbols /
15 flows (HIGH); the integrated pinned-base comparison observed 52 files / 636
symbols / 59 flows (CRITICAL), with no partial/truncated response. Source commits
were followed by a refresh at the final pin. Graph flow ranking and generic/dynamic
Rust receiver inference still have limits; current-source inspection and executed
interface tests supplement them. The [worktree CLI workflow](../gitnexus-worktree-coverage.md)
now gives each external app its own alias instead of querying root dev for absent
branch definitions. Root retains the canonical integrated-dev refresh.

The full native CAD failure remains a failing gate: rotated-concave/bottom expected
80481.2399, actual 80579.55733514718, tolerance 0.1. Root reproduced it independently;
this migration changes no CAD/Core geometry. The former 13 KiCad AppImage failures
are resolved by root's child-process APPDIR cleanup and did not recur. Duplicate
owner wrappers and the binding resolver's native seam remain later cleanup work;
no additional helper framework or unbind_all was introduced.

## Orchestrator checkpoint: f8f684959 (superseded)

Reviewed base `8ce860a0950304ddfae34f12d5891b8c6f267f2f` through
`f8f684959b592d819796056444f6cc2a4e94e88c`. The branch is clean and the migration
uses PendingEditSignals throughout. This is a completion checkpoint, not an
Outcome: the ticket stays claimed until the remaining fixes, checks and integration.

The fresh Standards review read the complete changed macro, layer and binding
bodies and Keycaps file. It found no hard documented-standard or visibility
violation. The duplicated Case/Keymap OwnedEdits holder is a nonblocking cleanup
finding. The Spec review confirmed two macro presentation defects: field failures
also appear in the panel status, and TextDraft/NumberDraft hide an older inline
failure while a newer draft is dirty. Repair both through the real mounted fields.

Complete the focused coverage in the
[final completion pass](../handoff-09-case-keymap-keycaps-library.md#final-completion-pass):
same-field mechanical requests behind held Core, a one-shot lifecycle regression
that actually fails without the fix, and representative panel tests for submitted
draft restoration and latest-per-key observation. Preserve existing domain and
catalogue-order assertions. Generic helper coverage alone does not prove panel
bindings and accepted-projection effects are wired correctly.

Reported browser results cover all suites, with Keymap passing only in separate
module/test invocations because the whole-suite driver is killed on the clean base
as well. Earlier passing suites remain evidence for unchanged code; record their
tested commits and rerun the paths affected by the final changes. The final Keycaps
commit includes a production Signal fallback, so do not describe both last commits
as fixture-only. Root still requires complete coverage at the integrated tree.

The root worktree compare found 29 changed files, 201 indexed symbols and 28 flows
with critical aggregate risk; no partial/truncated flags were reported. New branch
symbols still require current-source confirmation because the canonical graph is
indexed from dev. Root owns the final integrated-tree analysis.

## Comments

### 2026-10-08: completion pass held until PCB finishes

The agent's latest report is pinned to `72ba713b7` on
`deepening/09-case-keymap-keycaps-library-pending-edits`. Root confirmed that HEAD
and a clean worktree. The user asked to hold this stream and defer any spec decision
until the PCB work is done. Status remains claimed; this is not an accepted Outcome.

The agent reports the macro inline-failure fixes, a held-Core mechanical same-field
queue regression, stronger one-shot unbind regressions, and additional mounted
consumer coverage as committed. It reports a complete `check.py browser` pass at
the final HEAD and release of its Chrome lease, plus passing lint/typecheck. These
are reported results, not checks rerun by root. Keymap's 29 tests took about 28
seconds, exceeding the old 20-second default; the report's "under" wording is a typo.

The final reported Standards review found no hard violations. The Spec review still
flags Case's local submitted-draft/saved-ack layer, Keycaps' retry drafts/dirty state,
and Library's submitted owner/dirty state. The agent proposed an `is_awaiting` query
or settlement callback for Case but made neither change. After PCB, review these
remaining responsibilities together and decide whether the implementation, shared
contract or spec needs changing. This checkpoint authorizes no new helper API.

Native evidence remains limited: the CAD volume failure was reproduced at
`0a66f7172`, a workspace rerun reported no failures, and an isolated invocation
produced no output. The CAD gate is unresolved. Typecheck reported an unused Case
`is_pending` warning; Library's latest-per-key coverage remains partial. Final-HEAD
GitNexus analysis was not run by the agent; root still owns the integrated-tree gate.

The agent's timeout commit `72ba713b7` overlaps root's already reviewed fix
`f837bc82a`. Reconcile the overlap during integration and retain both branches'
useful verification. Leave source, history and acceptance criteria unchanged while
waiting for PCB. Root's browser-test pause remains in effect.
