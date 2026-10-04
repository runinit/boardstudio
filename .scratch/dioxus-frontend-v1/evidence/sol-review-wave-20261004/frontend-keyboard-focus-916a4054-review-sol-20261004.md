# Consolidated keyboard-focus candidate review — 2026-10-04

Sol 6.1 High reviewed frozen **58145c178156ee0a88ccaadc33dcae194c0f1639 → 916a40549444e7ac73c144e422f632a129fd2eaa**, one commit. Candidate **frontend-keyboard-focus-20261004**, `http://127.0.0.1:34805/` and `/boardstudio/`. Concurrent source/worktree edits are excluded. No browser, application test, compiler or package was rerun; only this review/audit were written.

## Integrity and standards

Proof: `.scratch/dioxus-frontend-v1/evidence/frontend-keyboard-focus-20261004/package-proof.json`, SHA-256 **b6378306b986ba175324108798ad41f070f1c97a38f796f15de29630dc755d9f**. Provenance: `web/target/builds/frontend-keyboard-focus-20261004/provenance.json`, SHA-256 **ff599c745e92629dd6357ac20e04d9d637d5f0a70f164cb26bbabe38c353b2f3**.

Independent rehash matches **1,393 frozen source inputs** with committed symlink dereference, **190 local assets** and **170 inherited provider assets per route**, **23 inherited command-log hashes**, and base bb4f2216 provenance `ac4d3409f85c532d1d28e0f8ab275c4bae87ce42a934d95ccc45db581cae9334`. Nine fresh and 23 inherited recorded commands exit 0. The existing publication proof records HTTP 200/COOP/COEP with no mismatches or release warnings; live HTTP was not repeated. The locked page-check retains three existing dead-code warnings. Frozen `git diff --check` passes.

**Standards: no new material finding.** Existing operating contract, architecture, ADR 0003 and acceptance requirements are preserved. Focus remains presentation-owned; camera/selection use existing owners. The private window listener removes both callbacks on drop, rolls back partial installation, excludes the reference's input/textarea/select targets, and releases Space on key-up; existing workspace/scope cleanup remains. Decorative marker correction removes its obsolete local focus/visibility override without changing module selection or the mounted Findings-list action. No public API/format/provider change, lowered assertion or new suppression.

## Spec and scoped verdicts

**Spec: no new material source defect. The prior SOL-58145-P2-01 is resolved on this candidate.** Retained source/regression and packaged receipts support:

| Scoped journey | Verdict and limit |
|---|---|
| F3.5 Board/Outline finding Back | Explicitly selected Board returns to focused Board name; explicitly selected Outline returns to focused Active outline. Stale board switch removes Back. New focus fallback coherently handles inspectors without tabs and disabled-control fallback. Default presentation with no selected tree context has no saved return target; the receipt distinguishes it. Broader C04/other Inspector branches remain open. |
| F3.6 unfocused Space-pan | BODY-focused Space plus drag pans; key-up prevents the next pan; Board name input Space remains unprevented and does not arm pan. Reuse the paired pointer-centered zoom and rotated-key/visible-outline Fit evidence. Broader limits, view-switch and renderer/resource lifecycle remain open. |
| F5.4 finding semantics/action | All 44 marker groups are decorative, with no tab stops and pointer events disabled. Actual SW2 Findings-list pointer/Enter action routes to Layout and its affected target; return preserves hidden PCB layers and host selection still works. Existing module pointer/Enter/Space routing is retained. Broader face/flip/every-instance, stale-route and nonempty-clearance branches remain open. |

Exact receipt paths/hashes are pinned in the audit. Public retests are focused Dioxus legs against already retained reference behavior, not newly repeated broad paired suites.

## RF disposition and acceptance limits

**RF-024's v1 decorative-marker correction is source-reviewed and publicly verified; RF-029's narrow unfocused Space-pan correction is source-reviewed and publicly verified.** Their frozen ledger statuses still say packaged pass pending / correction in progress. The coordinator should reconcile those narrow statuses and evidence pointers once; retain both post-port shared overlay/keyboard ownership proposals and historical observations. These stale progress phrases are record maintenance, not product blockers.

Approve the three scoped repairs. This does not accept full F3.5, F3.6 or F5.4 parents, waive remaining criterion/final joins, or reopen accepted F2.3. Existing actual AT, visual, performance/resource, compatibility corpus, Case RF-025 and retirement/cutover limits remain with their owning milestones.
