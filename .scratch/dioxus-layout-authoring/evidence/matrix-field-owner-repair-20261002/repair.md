# Matrix field owner boundary repair

Isolated worktree `matrix-field-owner-repair-20261002`, branch `codex/matrix-field-owner-repair-20261002`, base `014febc1579edd52da12f0e2fe27c77c30a0e25b`. Coordinated with layout_tree_author: their MatrixSetup, shared joins and worktree remain untouched. Root integration sources/build freeze were not modified.

Source-backed trigger and framework details: `/tmp/frontend-release-key-identity-review-20261002.md`. Dioxus-rsx0.7.10 only assigns a VNode key from the first root of a template. The five nested MatrixFieldEditor keys were evaluated as debug literals but did not create identity boundaries. A's dirty draft could therefore survive into B with equal baseline and submit under B's newly captured owner.

The 12-line production correction wraps each actual MatrixFieldEditor in its own rsx template root. Existing owner_key and all typed props/admission logic remain intact. No renaming/suppression, new API, config or dependency change. Each field remounts only when its existing owner key changes; unrelated token/revision changes preserve typing.

## Actual mounted regression

The native harness imports the real production MatrixInspector module, mounts it in VirtualDom, dispatches actual HTML input and blur events through its generated handlers, records DOM value mutations and observes emitted MatrixEditRequests. The only adapter is deterministic platform event data; no copied draft/admission implementation. It selects actual inputs by accessible label.

- Original source: two expected failures (same-baseline matrix A→B and Matrix→Layout name target). The captured request explicitly contains B's owner and A's draft. Two positive regressions already pass.
- Fixed source: all four pass, including each of five fields, unchanged-owner typing across token/revision updates, and same-owner Pending/Saved feedback settlement without duplicate submission.
- Executed the same harness with `cargo rustc --locked --manifest-path web/Cargo.toml --no-default-features --features page --test matrix_field_lifecycle -- -C debug-assertions=no`, then ran its emitted executable. This compiles the imported production component and rsx macros using the release static-key branch. Restoring original production source gives the same two expected failures; restoring the fix gives four green. This is targeted release-branch execution, not a claim of a fully optimized browser build.

Evidence is in this directory: normal red/green, static red/green and static compile logs. The original static build logs also show the five unused owner-key variables; the fixed static build does not. Existing native main unused portable-provider warnings remain outside this patch.

`cargo fmt --check` and `git diff --check` pass. Actual page all-target WASM strict Clippy attempted; it is blocked only by the two original New17 collapsible-if diagnostics in setup_guide.rs/presentation.rs on this base. Those are independently repaired in New17 `7fdccd4a` and must be joined and checked by root. No unrelated source fix or checker allowance was added.

Source SHA256: matrix_inspector.rs `df5bbd01ee8a125dc3d81a8598f779b496238394b48b1a1653e6d4b3d8b65cea`; harness.rs `678d9da0a9eaf35007a4b48591b0d5d182e8fa2f5b283c2330ef42985ad98dd4`.

## Handoff

This author cannot self-clear the repair; independent Spec/Standards reack required. Root serial integration must preserve MatrixSetup/numeric/source joins and run strict checks after the New17 repair. Public release browser target-switch/draft behavior still needs its gate; no numeric/ticket/demo completion claim. Encoder nested-key warnings were audited separately: its effective outer EncoderEditor and actual KeycodeField roots already carry identity, and this repair changes no encoder source. Reuse RF-001/RF-006/RF-009 for scoped draft and release-evidence findings; no new RF ID proposed.
