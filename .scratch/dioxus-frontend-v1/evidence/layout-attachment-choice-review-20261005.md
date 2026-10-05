# Layout attachment-choice repair review — 2026-10-05

Verdict: **CLEAR. No concrete defects found in the frozen three-file diff.** This clears the source repair for integration and the planned packaged replay; it does not establish criterion or parent acceptance.

Reviewed against HEAD `7f3165b497d135eafccbe8c3335cb48024fd62c1` in `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`.

| File | Reviewed SHA-256 |
| --- | --- |
| `web/src/matrix_transform_operation.rs` | `8fd982251844b215913571d99fa9c27ac884616a5e7cae0aab38a28302bcda64` |
| `web/src/presentation/objects/matrix_transform_controller.rs` | `2fa5a644a8bbac86a31ce795413e246f3114ecce57b261fafcccbfe784cf0686` |
| `web/src/presentation/objects/matrix_transform_inspector.rs` | `1b45a1f972d7a46a808dacba0145aad94000bdf8c1288263a280f1d645132624` |

All three hashes match the coordinator's frozen packet. Only this report was written; no source edits, tests, builds, packages or commits were performed. Concurrent lifecycle changes were excluded and are not a gate for this review.

## Filtering and preservation

The native policy reproduces the relevant `app/src/ui/partsCatalog.ts::partChoices` filtering: retired `infused-kim/nice_nano_pretty` definitions are excluded even if currently selected; non-current assembly snapshots are excluded; the row's current snapshot remains selectable; imported definitions with KiCad source remain available. Snapshot recognition retains the reference's case-insensitive assembly-prefix/UUID and `/definition/` matching, including the nonempty prefix segment and JavaScript dot line-terminator exclusions. Order and definition identities are preserved.

The controller now retains the same deduplicated, document-first source definitions rather than prematurely reducing them to `(id, label)` pairs. The Inspector applies the policy separately for each attachment's `definition_id`, so one row's current snapshot does not enter sibling rows' lists. Actual option value/selected binding still uses the same IDs. Existing label selection is unchanged; this repair does not claim an unrelated label presentation correction.

The attachment replacement/removal callbacks retain their existing member IDs, full attached-list baseline, request sequencing, scope/token/revision, feedback and dispatch paths. No edit-operation builder, history behavior or canonical/mirrored locality policy changes in this diff.

## Native regression and limits

The new test calls the production filtering policy with 59 definitions: 51 available definitions, seven unrelated snapshots and one current snapshot. It requires the exact ordered 52 choices, current-member preservation, KiCad-backed snapshot-shaped definition retention, removal of unrelated snapshots, and retired-source exclusion even when current. This meaningfully targets the observed option-set defect; it does not substitute a test-only implementation. Coordinator evidence records actual browser RED (59 versus reference 52) and actual-policy native RED/GREEN with one executed test.

The native test does not mount Dioxus or prove per-row DOM behavior. The reviewed mounted call site computes choices from each row's definition ID; planned packaged replay should supply that public integration evidence. Existing edit/history/reload/locality receipts remain reusable because their paths are unchanged. No new exhaustive test matrix or unrelated qualification gate is needed. Mobile remains deferred; no parent acceptance is claimed.
