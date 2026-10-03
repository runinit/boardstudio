# Combined frontend candidate — Standards review

**Standards HOLD at `a201a96a76c0d908580793e36e4c7d315155fbb3`: one newly identified P2 composition finding and one previously known P2 Parts finding.** Review compares served baseline `a8fd8988f649c70c11393066314095447535dce5` using `git diff a8fd8988...a201a96a` in `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`. Reviewer `/root/sol_case_final_review`, independent Sol6.1 High; 2026-10-03. This is the consolidated Standards axis; parallel Spec and paired browser qualification remain separate.

## Standards findings

1. **[P2] Admit the joined explicit Part context to its Layout Inspector.** `web/src/presentation.rs:986–990` rejects a selected Component context unless it exactly equals `objects::context_for_part`. The joined Keycaps repair at `:3651–3654` intentionally calls `component_context_for_finding_part`, which converts a primary matrix switch's ordinary Key context to `Component { part_id, matrix_id: None, ... }` (`objects/tree.rs:1195–1218`). The selected part is valid, but the ordinary-context equality remains false. The route therefore falls back to the generic position Inspector instead of the joined Properties/Relations component Inspector. This breaches CONSTRAINTS “preserve ... contextual panes and behavior” and default TypeScript parity: the retained Keycaps paired discrepancy explicitly required Part selection and the component Inspector for that switch. Accept the independently validated explicit Part context at the composition boundary while preserving scope, board membership, exact accepted identity, selected-part and owner-lifetime guards. Verify the actual finding route reaches the Inspector for a matrix switch; helper-only context tests currently stop before this consumer.

2. **[P2, previously known] Keep unchanged pad drafts through accepted ID rename.** `web/src/parts_custom_definition.rs:155–159` still keys PadFields by `scope/definition/pad.id`. Accepting an ID rename remounts the row and discards unrelated dirty coordinate drafts, contrary to reference per-value draft synchronization. This is the existing Parts04 HOLD from `/home/chris/.local/share/boardstudio/reviews/parts04-custom-authoring-source-review-1a0128ab-sol-20261003.md`, already under active repair. Retain its required consistent accepted-refresh → draft blur proof; do not open another child approval sequence.

No additional documented-standard source breach or separate actionable smell was identified. The skill's naming, duplication, ownership, data-clump, primitive, repeated-switch, surgery, divergent-change, speculative-generalization, message-chain, middle-man and inheritance heuristics were considered as judgement calls. Existing RF ownership covers root composition/adapter duplication; these heuristics do not add blocking gates.

## Cross-slice assessment

The eight joined source areas inspected are Core persisted-lock regression coverage; Project name; PCB current-plan Apply; Case live lifecycle/current diagnostic repair; Keycaps focused marker/navigation/fit; standalone Layout component Inspector; Parts custom-definition fields; and Keymap Layers disclosure.

- Project rename uses the latest accepted document under its stable document/session and mounted owner, one normal Session edit and persistence/history path. Its nonphysical accepted revision can safely reuse completed Case output; Preview remains captured-token owned. Root Runtime changes for test Core/persistence, injected Case cache and Layout Inspector event probes remain test-only.
- Case completed same-scope stale geometry is admitted by Objects and mechanical layer projection. Findings remain strict scope/token/exact scene sourced, with production Show callback token/scope/finding revalidation. Source mount remains byte-identical to the cleared Case85 repair. The prior Case HOLD/repair history is preserved.
- PCB Apply is composed with existing mode operations and the current plan resolver. Both UI eligibility and dispatch require exact accepted plan identity, board, revision, canonical scope, no errors and accepted electrical mode. Materialization uses one strict-revision ReplaceDocument request and normal terminal/persistence feedback; it adds no operation, Session, worker or cache authority. Core09 only adds persisted lock coverage.
- Marker geometry comes from current accepted Core finding contours under the existing Layout transform; marker retirement and destination-fit ownership remain guarded. Full marker contours now drive final fit bounds. The finding's explicit Part context reveals the Inspector consumer mismatch above.
- Layout Inspector lifetime now advances for context/view/accepted-owner changes, and edit callbacks check it alongside scope generation/token/revision/selected-part identity. The margin Enter/Escape repair uses actual blur/reset. The combined Part route is a distinct join bug, not a reopening of repaired standalone-owner behavior.
- Parts name/geometry composition uses the latest accepted command capture, owner-qualified controls, imported KiCad pad protection, reference net-pin remapping/removal and the normal edit path. The known pad-ID remount defect remains current correctness work.
- Keymap disclosure wraps the existing layer controls in native details/summary. The constant initial open attribute is left to browser disclosure state between renders; active-layer/source behavior and edits remain in their prior owners. Relevant styles reuse existing theme tokens and stay component scoped.

No new public API/schema/format, untracked persisted geometry, second domain owner, lint suppression, test skip or reduced threshold was introduced by this batch. Necessary API/design changes are authorized by the current CONSTRAINTS update and reviewed at candidate level; no superseded per-child approval rule was applied.

## Evidence and limits

No root source edits, repeated heavy tests/builds, extra subagents or per-child gates were performed. Inspected production join diffs, retained source reviews, repair tests and pinned reference semantics. CodeGraph is absent in this worktree, so targeted source reads/searches were used. The integration checkout contains unrelated tracked/untracked documentation/evidence/assets; these were left intact. This verdict applies to exact committed a201, not the mutable overlay.

Independent source-scoped `git diff --check a8fd8988...a201a96a -- web/src web/assets/m1.css core/tests CONSTRAINTS.md` passes. Whole-range diff whitespace exits2 on preserved raw red/green log whitespace; raw historical logs were not normalized or discarded to obtain a pass.

Read root retained strict receipt `/home/chris/.local/share/boardstudio/retained-tmp/20261002/frontend-batch7-root-clippy-final.json`: exact source `61ed76965ec2faa117813477b3277265abef6603`, `cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets -- -D warnings`, exit0. Its retained stdout completes successfully. Root also reports strict all-target WASM Clippy passed at a201; that second invocation was not independently rerun. The combined check supplies the previously pending integrated Case strict-check coverage; it does not clear the two behavioral findings or prove public acceptance.

Frozen blob SHA-256:

| File at a201 | SHA-256 |
| --- | --- |
| `web/src/presentation.rs` | `8954b9f24b498b937352bfaa179c1942233cda80c09d98958043d1c1094c0887` |
| `web/src/presentation/objects/tree.rs` | `08d4c1b1b2af5afd7851ee59383b362d258c4adaca12740fa95dfeee91690278` |
| `web/src/parts_custom_definition.rs` | `5886549f568dd8895217e5a140a44cc1985d70c13ae71c39bffdc89380cbb323` |
| `web/src/presentation/mechanical_settings_mount.rs` | `d920d1d9ae923f97fea713730f87c5c158d2341d44a7e3178ede7a86ce1fadc0` |
| `CONSTRAINTS.md` | `c7bcf1b9746b9b76759e2c941263572998ccd07afc7a9908433a75143c490439` |
| `refactor-findings.json` | `3ce72da51907f3db99727b3c2737236fb8f4bf09b9244d8a114e2ab0526ce8f6` |

No distinct new RF identifier or RF resolution is claimed. The Keycaps→Inspector mismatch extends the existing RF-001/RF-010 composition observation; current correctness must be repaired rather than deferred as refactoring. Preserve all RF-001–015 records, six streams and 62 canonical parents.

The root full build `frontend-candidate-batch-20261003` and new served paired journeys are coordinator-owned and pending at this source-review checkpoint. Public edits, Undo/Redo, save/reopen, theme/compact/accessibility and each parent acceptance join remain required; no child or parent is accepted by this report. Actual Case paired journeys will be recorded separately against the served build identity.
