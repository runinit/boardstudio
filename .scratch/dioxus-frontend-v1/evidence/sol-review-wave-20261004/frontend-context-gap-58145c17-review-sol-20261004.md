# Consolidated candidate review — 2026-10-04

Sol 6.1 High reviewed frozen **9dd2a412823b650d5b055733181b59ca724c68f1 → 58145c178156ee0a88ccaadc33dcae194c0f1639** (f399d8c2 and 58145c17). Candidate **frontend-context-gap-20261004**, `http://127.0.0.1:34804/` and `/boardstudio/`. Later source edits are excluded. No browser, application test, compiler or package was rerun; only this review and its audit were written.

## Package integrity

Proof: `.scratch/dioxus-frontend-v1/evidence/frontend-context-gap-20261004/package-proof.json`, SHA-256 **533b5944b420a97f94c96fbb358cfc9da2f1625f339c421624851add6b77d706**. Provenance: `web/target/builds/frontend-context-gap-20261004/provenance.json`, SHA-256 **8c6c3d52439370a0d85f95b6f7f09ea352af4735a983a37632b30b0d0f7b7773**.

Independent rehash matches all **1,393 frozen source inputs**, including the committed imported-modules symlink target; **190 local assets per route**; **170 inherited provider assets per route**; and **23 inherited command logs**. Base bb4f2216 provenance matches `ac4d3409f85c532d1d28e0f8ab275c4bae87ce42a934d95ccc45db581cae9334`. Nine fresh and 23 inherited recorded commands exit 0. The published proof records live HTTP 200, COOP/COEP and zero asset mismatches/release warnings on both routes; live HTTP was not repeated. Locked page-check retains three dead-code warnings; warning-free Clippy is not claimed. Frozen `git diff --check` passes.

## Standards

**No new blocking standards finding.** CONSTRAINTS.md, issue/domain guidance, architecture, ADR 0003 and acceptance requirements govern. Changes retain private presentation ownership and accepted selection/settings authority; no public API, format, provider or test-bar change. RF-001 records tab ownership, RF-026 delegated pointer capture, and RF-028 context/member liveness. Their bounded repairs are coherent. RF-026's pending-package wording is historical: the panel repair has public GREEN evidence. Structural proposals and RF-009 evidence limitations remain; unrelated RF-024/025/027 retain their owning workflows.

## Spec

**[P2] Finding Back has no focus destination for prior Board/Outline contexts.** `web/src/presentation.rs:3498–3507` consumes the return-focus flag and searches only a selected `.m1-layout-component-tabs` tab. Capture accepts current Board/Outline contexts at 5674–5685, and Back clears its target/removes its button at 5613. Those inspectors exclude tabs (`layout_workspace.rs:205–208`, 300–305). Navigating a finding from either context, then pressing Back, restores selection but cannot restore keyboard focus through this code. F3.5-C04 requires “Escape/back behavior and focus restoration”; use a current-owner focus fallback to an appropriate mounted Inspector control. This is source evidence, not a reviewer browser reproduction. The receipt's later focus-repair follow-up is outside the frozen range and does not clear this candidate. **This does not block F2.3.**

Retained receipt paths and hashes are pinned in the audit:

| Scoped action | Supported result and limits |
|---|---|
| Finding Show outline → Back to prior left-U1 | Public keyboard selection/Properties-focus GREEN on f399d8c2. Owner/snapshot/destination and removed-component guards are coherent; other return contexts remain open. |
| Matrix Relations → Key | Focused RED/GREEN and exact 34804 public retest pass. Full scoped context resets Properties; equal context retains the tab. Broader F3.5 contexts/drafts remain open. |
| Case empty fit profiles | Focused RED/GREEN and 34804 retest show configured 3.00 mm gap after plate 1.5→2.0 / foam 3.0→2.8 edit; supported profile range remains truthfully unavailable. This leg does not prove fresh generation, every dimension or archive behavior. |
| F2.3 panels | Paired TS resize, candidate pointer GREEN, keyboard bounds, pinned/auto-hide reload and desktop modes/nonmutation pass. Retained compact Close/Escape/scrim/focus/inert evidence is supplemented by the TS desktop/compact reference leg. Panel source/policy/CSS are unchanged f399d8c2→58145c17. |

## F2.3 recommendation and limits

**Recommend functional F2.3 acceptance after this single review.** All four canonical criteria are verified; INT.1 is accepted; no additional acceptance join/external gate is listed. The receipt now correctly labels its Dioxus C01/C04 leg and supplies the matching TS pinned/auto-hide/collapsed, content-interaction and compact focus/inert leg. TS Saved status, Undo/Redo availability and 100% camera remain unchanged; its header exposes no numeric revision. Preserve that limit: numeric revision invariance is supported by Dioxus's unchanged Revision 3 plus source-side presentation-only panel actions, not an invented TS revision measurement. No remaining F2.3 source or acceptance blocker was found. The coordinator owns the acceptance decision and review/audit hash pinning.

F3.5-C04 stays open for the P2 and broader routes. Other F3.5 branches, broader F7.4 settings and F7.5 valid preview→release→Undo/lifecycle are not accepted here. Exact visual parity, actual assistive-technology coverage, performance/resources, compatibility corpus and retirement/cutover gates retain their owning milestones.
