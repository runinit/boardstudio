# Parts pad-row review and repair — served19525578 / frozen08fcf1b2

Reviewer/repair author: Sol 6.1 High, 2026-10-03. Consolidated Standards/Spec/actual paired browser finding and authorized isolated bug fix. No extra source handoff or permission gate; root owns integration and the next served candidate.

**Served19525578: Parts row-lifetime HOLD. Frozen repair08fcf1b2: bounded source qualification CLEAR; repaired public paired acceptance pending.** The completed actual reuse package remains separately CLEAR within its recorded browser-environment bound; its pinned report is `frontend-reuse-19525578-package-review-sol-20261003.md`, SHA256 `59a2f646e30c8e4611f37ba3a963eb0189075742b1fb38e61e2beef9168d20a6`.

## Actual paired P2 finding

Source: `19525578207ff66bfb51a6c7a6483a0bb7905029`, build `frontend-parts-outline-keycaps-reuse-20261003`, root URL `http://127.0.0.1:34742/`. Pinned React reference: `5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5173/`.

Both legs start from original5b archive (`5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) and use their own profiles. The final matched row-deletion comparison is at1280×960, selected newly created **Custom component16**, with two ordinary default circle pads (numbers1/2, X/Y0, width/height2). Clear the first pad's Width, then use its actual Remove pad control. Wait for one remaining row, then compare visible fields against the read-only durable `projects` store.

| State after accepted deletion | Dioxus19525578 | Pinned React |
|---|---|---|
| Surviving pad number | 2 | 2 |
| Durable accepted width | 2 | 2 |
| Visible surviving Width draft | blank, copied from deleted row | 2 |
| Durable revision at capture | 20 | 18 |

The extra earlier probes account for the revision difference; the final selected definition's two-pad defaults and deletion actions match. Random generated IDs are recorded, not assumed equal: candidate definition `ui-764046a5-2d97-43cc-bd28-770ddd39b7ef`, survivor `pad-47`; React definition `ui-4f59f3c7-0c22-4db0-ae63-709e24879d4e`, survivor `ui-1c053fc0-5a5e-4383-85ef-4ca48ce65877`. Both project IDs are `m1-sofle-v2-copy`. The screenshot visibly confirms the blank candidate Width and accepted-looking React Width2. Browser error captures are empty.

Cause: a29fb53a's `{owner_key}:{index}` key preserves an ID-renamed row, but reuses the deleted first pad's component when the next pad moves into index0. Accepted-value effects only reset fields whose scalar value changes; identical accepted width2 leaves the deleted row's blank local draft attached to the survivor. This is a pad ownership regression, not an accepted geometry mutation.

The prior rename-retention leg is separately recorded: candidate195 retains an empty dirty Number after an accepted ID rename while the durable pad remains number1. The full8af baseline and settled React reset that Number to1. An immediate React read initially looked retained; the settled capture corrected that observation. Thus rename retention is a deliberate correctness improvement under existing user design-change authority, not an assertion of identical React behavior. Newly selecting Custom component16 also retired prior definition drafts/errors in both apps.

## Small private repair

Frozen source **`08fcf1b257d32400099000e1ffee7771ce8789fa`**, based on19525578, branch `codex/parts-pad-row-lifetime-repair-20261003`, worktree `/home/chris/.local/share/boardstudio/worktrees/parts-pad-row-lifetime-repair-20261003`. Worktree is clean. Four files: the existing Parts custom-definition leaf, the existing mounted Parts name test module, the Parts04 issue, and its existing historical source handoff.

Private `PadRowKeys` keeps unique accepted pad IDs attached to stable local component keys. One unmatched ID pair at the same index with unchanged row count is treated as an accepted rename, retaining that row's unrelated drafts. Surviving IDs keep their keys across insertion, removal and reorder; new rows get fresh keys, including reinsertion after deletion. Owner changes, duplicate IDs and ambiguous multi-row replacements retire unmatched keys. No public API/visibility, document format, geometry operation, commit boundary, provider or Runtime change was introduced. Existing admission and field synchronization remain intact.

This ordinary-action inference does not supply a new domain immutable-pad identifier: an arbitrary atomic remove-plus-add with identical row count and one unmatched pair at the same index cannot be distinguished from rename by accepted IDs alone. The shipped UI emits individual Add/Remove/ID actions, and the regression exercises those boundaries plus accepted field refresh. Broader document replacement semantics remain outside this bounded claim.

SHA256: `web/src/parts_custom_definition.rs` = `3599df00f336525ce102ddcb129ee9f0eb477d92d4359834a426d50714da8b0e`; `web/src/parts_definition_name.rs` = `1a5e28bca51bfa3f1f5fe97be7fbff4d767a5300ada4dc1deb751626d7f0165c`.

## Red / green and both review axes

The existing mounted rename/owner regression was extended through the **production Remove pad action**, actual Runtime event capture and Session/Core acceptance. The surviving pad intentionally has identical accepted scalar values, so scalar-change effects cannot mask component reuse. Before production repair, the mounted test fails exactly at `a removed pad's dirty draft cannot move into the surviving pad`, actual`9` versus expected`7`. Earlier rename/focus/accepted-edit assertions reached that point successfully.

After repair:

- The two mounted owner/rename/removal tests pass2/2 in headless Chrome. Rename retention, current accepted command target, deletion retirement and definition owner retirement are covered together.
- Focused native custom-definition tests pass9/9: all existing5 are retained; four additional cases cover accepted rename with scalar refresh, deletion/reinsertion/reorder, owner/ambiguous replacement retirement, and duplicate-ID isolation.
- **Actual strict command passed:** `cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets -- -D warnings`. The exact invocation and successful exit are recorded; no flags are inferred from compiler stdout.
- Formatting and diff checks pass. Source floor review found no removed assertions, skips, suppressions, threshold changes or new stubs.

Commands use Sol's existing isolated Case-repair target at `/home/chris/.local/share/boardstudio/worktrees/case-current-result-status-repair-20261003/web/target`, avoiding root/shared author targets. No full build or per-child release gate was added. A read-only graph recomputation on the frozen repair still reports page138/test14/Core3/CAD4/service-worker3; the provider graph remains outside this edit.

**Standards CLEAR for frozen repair:** row lifetime belongs to the Parts UI's private module; Core/Session remain authoritative for accepted edits. The new helper exposes no crate/public surface and the actual focused checks pass.

**Spec CLEAR for frozen repair's bounded source behavior:** accepted rename preserves unrelated drafts, deletion retires the removed row instead of transferring its draft, existing same-definition scalar updates and owner transitions remain covered, and old pad-reference/admission/history tests remain green. Actual repaired root/subpath public journeys await the coordinator's next integrated package. Source qualification does not close F4.1/F4.2 or other all62 parent criteria.

## Pinned evidence and preserved limits

Directory `/home/chris/.local/share/boardstudio/reviews/parts-row-19525578-sol-20261003/` contains the actual matched screenshots/snapshots/compact accepted-document receipts, prior rename and owner probes, expected mounted red, mounted/native green logs, strict Clippy/format/diff logs, exact command receipt, and frozen graph proof. Artifact index SHA256 **`501eef38ddd7c18343299c6d8d46fcafe453c29aa9d5bdfc814c075feb2e9078`**, 47 files.

Files prefixed `attempt1-` are preserved harness/transient evidence and excluded from acceptance: one removal click was covered at the earlier720px viewport, and an early baseline rename capture preceded acceptance. Two later wait expressions had malformed selector quoting; those are harness failures, not app failures. The final paired row-deletion captures use settled semantic predicates and independently read the durable accepted pad. The read-only store receipts omit binary asset stores.

RF001–015 and all62 parent history remain intact. RF009 receives exact-source evidence rather than a blanket completion claim. Prior8af/a201 reports and original Parts HOLD are retained. The immutable195 reuse package remains qualified; the source defect found in its mounted Parts workflow stays explicitly HOLD until the frozen repair is joined, packaged and passes the affected actual paired journey. Broader field/Undo/Redo/archive/compact/theme/geometry/export acceptance is not asserted here.
