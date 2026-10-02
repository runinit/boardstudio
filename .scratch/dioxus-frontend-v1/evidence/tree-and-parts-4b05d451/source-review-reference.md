## Standards: bounded tree corrections

Reviewed copy fix `8ea71a84...96811f92`, hierarchy repair `96811f92...b51353bb` (Objects and tree projection only), and integration `515f390d`. Applied the existing Standards/constraint/domain/ownership contracts. No application edits, compiler, builds or browser actions; both narrow diff checks passed.

**No new documented-standard violation or material regression found in these narrow changes. The previous P2 range-copy finding is resolved at source level.**

- `presentation.rs:1358` now captures `visible_ids.clone()` as Rc. The handler creates a Vec<String> only for SelectionMode::Range; ordinary/toggle selection passes an empty vector. This removes the per-cell O(cells × visible parts) render-time string copies while retaining the scoped adapter and existing Range semantics. No allocation benchmark or rectangular-selection acceptance is claimed.
- `objects.rs` initializes board/Layout/matrix disclosure defaults and extends them on the relevant document/session/board/default-set changes. This mutates only local disclosure state. Existing independent toggles, grouping storage, callbacks and Session selection remain separate.
- `objects/tree.rs` now emits the matrix header only for unowned matrices, adjusts descendant levels consistently, places owned standalone components directly below nonsplit layouts, retains split Components groups, and preserves unowned rows under Layout even when layouts exist. Helpers still borrow accepted document/scene data; no alternative writable model, core geometry implementation, API or schema change appears.

Reviewed retained React `Workbench.tsx:514–520` and `useWorkbenchTree.ts:256–323`, plus supplied diagnosis and paired public red/reference evidence. The scoped pair uses archive SHA-256 `672d5f58…64a8f`, with candidate collapsed/default hierarchy mismatch and reference match. The unowned pair uses the same variant archive and equal complete stored records: candidate pass=false, reference pass=true. These support the bounded correction; this reviewer did not rerun them, and repaired-artifact public green is pending.

Integration `515f390d` preserves the repaired tree files; its additional Parts files are unmounted (`presentation.rs` has no Parts module/mount), so this review attributes no Parts behavior or acceptance to the build.

Existing nonblocking duplicated pointer-guard RF-001/010 observations remain. No new refactoring takeaway observed for these repairs. Full T1-10 scope/anchor/pointer-reuse/nudge/Undo, keyboard/focus/axe/assistive-technology and public acceptance gates remain open. The separate minimum-context UI proposal and ticket 11/12 responsibilities are not approved or closed here.
