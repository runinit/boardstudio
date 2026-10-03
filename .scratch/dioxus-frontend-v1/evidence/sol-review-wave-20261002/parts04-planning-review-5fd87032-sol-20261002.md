# Parts04 custom footprint planning review — 5fd87032

**Standards HOLD; Spec HOLD for bounded documentation correction only.** Existing empty-create and scoped Name capabilities are sufficient to start the remaining authoring feature once the exact corrected packet is rereviewed. Full F4.1 acceptance, a new API/schema or an additional user permission request is not required.

## Exact packet

- Frozen docs commit: `5fd8703255864090ef83e5459e4dd5b15e10f2ca`, clean isolated worktree `/home/chris/.local/share/boardstudio/worktrees/parts04-authoring-planning-20261002` at inspection.
- Draft: `.scratch/dioxus-parts-catalogue/drafts/04-custom-footprint-authoring.md`, SHA-256 `de0faeba3ecc877fa0a7d92fcc166a703d5c03da8c7865325de6b388fd958242`.
- Issue04: `.scratch/dioxus-parts-catalogue/issues/04-custom-footprint-authoring.md`, SHA-256 `228a34ee5ee46d7f342fc2f26a9a167aaf658fd908255ab305e69a3e9b518be7`.
- Inventory: `.scratch/dioxus-parts-catalogue/evidence/04-custom-footprint-source-inventory-20261002.md`, SHA-256 `581f2e748b4aadd73b254ffb79f9c8b9062e4b8770d63080a48f0652adcbc462`.
- Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`. Independently compared the seven relevant source files against immutable Git blobs; all equal this packet's local source. The action/geometry helpers are at `app/src/ui/createLibraryActions.ts` and `app/src/ui/workbenchGeometry.ts`.
- Standards: supplied project AGENTS, `CONSTRAINTS.md`, accepted ownership/context and issue-tracker instructions, `docs/hardware/component-onboarding.md`, existing F4 parent and code-review smell baseline. No `.codegraph/`, root AGENTS.md or CONTEXT-MAP.md exists in this isolated checkout. No independent smell finding.

## Standards

**HOLD — one P2 packet-provenance correction.**

The inventory's Parts composition SHA is mistyped. It records `4bea702b5bd4d42c896fe15123643849981c035976f1b1e5a448321a5625b81a7f`; actual `web/src/presentation/parts.rs` is **`4bea702b5bd4d42c896fe15123643849981c035976e1e5a448321a5625b81a7f`**. Correct and freeze the inventory so its claimed capability evidence is reproducible. The other two cited Rust source hashes match. This does not negate the independently inspected capability or impose a parent-wide start lock.

The intended private field/definition editor preserves ownership: local drafts in Parts; one normal Core/Session edit, accepted revision, history and durability; imported KiCad pad geometry read-only; no placement, generator, asset or public API work. Multi-instance ID rename/removal requires coupled net-pin changes inside the same edit, rather than unrelated mutation or a second writable authority. The packet correctly retains actual save/reopen and archive evidence instead of presenting simulated persistence/JSON round trip as browser qualification. No new RF takeaway; existing RF-001/RF-002/RF-006/RF-009 remain appropriate where confirmed.

## Spec

**HOLD — one P2 unresolved acceptance-oracle correction in the frozen text.**

The frozen issue requires both reference Add pad defaults and unique nonempty pad numbers, then leaves the discrepancy undispositioned. React `ui/createLibraryActions.ts:121–124` computes `String(pads.length + 1)` without collision checking; manual Number edit rejects duplicates at lines 81–83. With two existing pads numbered `1` and `3`, Add pad produces duplicate `3`. This is a confirmed authored-validation defect relative to existing parent story23, not desired reference fidelity.

The coordinator has confirmed the user-approved fidelity contract permits this bounded defect correction. **Concrete approved oracle:** start at `pads.length + 1`; preserve that default if unused, otherwise increment to the next unused positive integer. Existing `1,3` therefore adds `4`; ordinary `1,2` adds `3`. Preserve stable new pad identity, defaults `(0,0)`, `2×2 mm`, circle and no drill, existing nets/pins, and exactly one normal edit/Undo unit. Record the deliberate divergence in draft/issue/inventory and require the duplicate-number fixture to fail for the expected old behavior before the correction passes. No React-source edit or broader validation rewrite is authorized. Rereview the corrected exact docs before dispatch; do not leave an implementer to choose between duplicating the bug and an unspecified allocator.

Everything else is source-faithful and coherent: raw Name commit with advisory blank-Name issue; six Kind options; local text drafts and accepted-field refresh, Enter blur/Escape reset; positive finite courtyard/pad dimensions and optional drill, finite coordinates, trimmed required pad IDs/numbers; immediate Kind/Shape/Add/Remove versus blur commits; one history unit per action; existing selected project/session and latest-document guards; preservation of all matching instances' ID-based net references on rename and targeted removal; number edits do not change net identity. No rotation or imported-pad authoring controls are invented.

## Verified source facts and implementation cautions

- Creation: `Workbench.tsx:759–774` immediately adds default custom definition (10×6 mm origin-centered courtyard, empty pads), selects it and reveals Inspector. The private create action is mounted independently of source loading and reconciles selection through its exact terminal outcome/current owner.
- Editor: `PartsInspectorPanel.tsx:71–101` supplies the actual disclosure, controls, accessibility labels, shape options and imported-source disable behavior. `InspectorControls.tsx:7–22` ties dirty draft refresh to accepted field values, not every accepted revision.
- Geometry/numbers: courtyard resize preserves its existing bounding-box centre (`createLibraryActions.ts:52–67`); blank Drill removes `drill`, while blank coordinate follows JavaScript `Number('') == 0`. Preserve these observed field semantics rather than introducing blanket empty-number rejection. Invalid sizes remain rejected/recoverable.
- Net remap/removal: lines 93–97 and 127–131 identify every placed part by definition ID and change/filter only its corresponding pad-ID pins, preserving unrelated nets and placements. A multiple-instance fixture must exercise ID rename, removal, number-only edit and Undo/Redo/reopen.

Executed checks: exact packet SHA-256 comparison; immutable React-source equality; current create/Name/composition and Core validation inspection; frozen docs diff whitespace check (passed). No heavy tests, app source changes or delegation. Planning/source/public acceptance remain separate; this review adds no global or full-parent gate. Root and Parts author received the exact oracle and hash correction.

Summary: Standards 1 P2 source-hash correction; Spec 1 P2 oracle-publication correction. **HOLD only until corrected exact packet rereview; narrow capability start otherwise CLEAR.**
