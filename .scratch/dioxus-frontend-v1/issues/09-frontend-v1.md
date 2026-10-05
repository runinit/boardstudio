# F9 — Frontend qualification, default entrypoint and React retirement

> Scope update (2026-10-05): accessibility/axe/semantic-only and assistive-technology qualification are removed by user instruction. Ordinary keyboard, focus, Escape and functional controls remain in scope. Historical results are retained without counting excluded checks as passes.

**Triage:** ready-for-agent for planning and independent qualification preparation; release depends on all required workflow slices and explicit cutover approval.
**Planning baseline:** integration c827c4e6, executable f44a3d1b, React reference 5a472a94. F1 and F3a are verified increments; full F2–F8 remain open.

## Problem Statement

A recognizable Dioxus shell and working layer controls do not establish that the complete React application can be replaced. Required screens, UI hooks, visual states, and browser interactions must all have tested replacements before the default frontend changes.

## Solution

Qualify every completed workflow continuously against the same React fixtures and interaction traces, then join them into complete project-to-export journeys. Prepare a concrete entrypoint/rollback patch only after the inventory and applicable gates are complete. Keep the reference recoverable and the wider backend migration separately accounted for.

## User Stories

1. As a designer, I want every existing workspace action accounted for, so that migrating does not remove a control I rely on.
2. As a designer, I want the same fonts, colors, geometry, spacing and focus behavior in both themes, so that the application remains familiar.
3. As a designer, I want full workflows at compact widths, short heights and supported zoom, so that controls stay reachable.
4. As a keyboard user, I want menus, trees, inspectors and canvases to preserve their shortcuts and focus return, so that I can edit without a pointer.
6. As a designer, I want my existing archive and its assets to round-trip between the frontends, so that my work remains portable.
7. As a designer, I want saved projects and supported cached workflows to reopen offline, so that local work is dependable.
8. As a designer, I want cancel, navigation and delayed replies to respect the current project, so that old work cannot overwrite a new view or deliver the wrong file.
9. As a designer, I want interaction and rendering to meet existing budgets, so that the frontend port remains usable on the tested hardware.
10. As a designer, I want generation and export to retain correct scope and readiness, so that previews are never mistaken for manufacturing output.
11. As a maintainer, I want each production TSX, UI hook, stylesheet and visual asset mapped to a verified Dioxus owner, so that hidden presentation dependencies are not forgotten.
12. As a maintainer, I want valuable React tests replaced with public Dioxus behavior tests before removal, so that regressions stay observable.
13. As a maintainer, I want exact source/build/evidence provenance, so that a passing report identifies the delivered artifact.
14. As a maintainer, I want retained executable generator/host dependencies documented honestly, so that frontend completion is not confused with completing the broader full-Rust runtime migration.
15. As the project owner, I want to review the final default-entrypoint and rollback changes, so that adoption is a deliberate decision.

## Implementation Decisions

- Qualify vertical workflows as they land; the release milestone aggregates their evidence and tests their composition. It is not the first time visual or keyboard verification happens.
- Preserve the pinned React reference and original projects. Re-inventory new reference work before accepting it into this migration.
- Count behavior, not filename deletion. Shared components can remain partially ported until all their owning workflows pass.
- Dioxus owns all required presentation and UI controllers at frontend v1. Retained generators and browser/provider bridges have explicit ownership and disposition; they do not count as React islands and are not claimed to be Rust implementations.
- Keep the established session, accepted-snapshot, save, worker and delivery authorities. Qualification does not authorize a schema, API, budget or persistence contract change.
- Rebuild any changed provider inputs. The current page-only build helper can reuse maintained providers only while its source/hash guard proves they are unchanged.
- Preserve the existing supported browser claim. Chromium is the current established acceptance target; wider browser support needs actual evidence before a broader claim.
- Prepare the production entrypoint/data-writer transition and rollback together. Apply only after the user approves that concrete result. Until then keep the working isolated candidate available.

## Testing Decisions

Use the highest existing public seams: actual Dioxus UI, archive/file round trips, accepted application/provider contracts and real browser worker/storage behavior. Avoid private test hooks or assertions that simply mirror implementation. Keep one scenario/evidence manifest naming source, fixture, scope, route, browser, viewport/DPR, actions, expected outcome, actual result and limitations.

Pair React/Dioxus dark/light desktop and compact captures for each completed workspace, with selected, open, loading, empty, invalid and failed states. Use targeted affected checks after changes; reuse unchanged, attributable evidence. Run shared end-to-end journeys after merges. Keep CAD-heavy functional checks serial under the existing memory policy. If a bug appears, retain its failing public reproduction, fix it, then rerun affected checks.

Accessibility, axe and actual assistive-technology qualification are removed from this frontend migration by user instruction on 2026-10-05. Ordinary keyboard/focus/Escape checks remain functional requirements. Keep frozen UI/live failures, ineligible CAD comparisons and material/resource attribution gaps separate; identify their applicability to the changed frontend path rather than silently deleting or reopening unrelated backend projects.

## Execution slices

| Slice | Visible outcome | Hard prerequisites | Priority / effort | Acceptance |
| --- | --- | --- | --- | --- |
| F9.1 — Inventory and transferable tests | Maintainer can identify the owner and evidence for every existing frontend behavior | None; baseline exists | P0 / M | All 79 TSX rows, 18 CSS files and inventoried hooks/assets have one lead workflow, explicit shared consumers, state and test disposition; no omission or premature completion |
| F9.2 — Paired workflow verification | Every delivered screen matches the reference and works through its main/error paths | Each scenario waits only for its owning slice; full milestone aggregation waits for F2–F8 | P0 / L | Per-workflow public UI, themes, keyboard, desktop short-height/zoom and source reviews pass; screenshots include source/fixture identity |
| F9.4 — Compatibility and offline journeys | Existing work survives exchange/reload/offline/navigation throughout the port | Completed project and relevant export/asset slices | P0 / L | React↔Dioxus round trip retains supported fields/assets; actual saved writes, root/subpath scoped offline update, stale opens/replies and missing lazy asset recovery pass |
| F9.5 — Affected performance and resource gates | Editing, workspace changes and preview/export remain responsive and release resources | Corresponding changed interaction/renderer/provider slice; qualification of final source after integrations | P1 / L | Existing budgets and endpoint eligibility retained; relevant paired pointer/frame, crossing/size/startup, renderer lifecycle and worker/URL/resource observations recorded; failed/ineligible gates stay open |
| F9.6 — Release candidate and adoption patch | Owner receives a complete Dioxus candidate with a concrete default-entrypoint and rollback diff | F9.1, F9.2, F9.4, F9.5 plus all required F2–F8 exits | P0 / M | No required placeholder/React UI island; exact build/check/review provenance; cutover and rollback dry-run using copied data; request approval only for the finished patch |
| F9.7 — Approved adoption and retirement | Approved default frontend runs the verified workflows and can roll back | F9.6 and explicit approval for actual cutover | P1 / M | Apply only approved changes; verify launch, compatibility and critical journey on adopted path; retire only covered React UI/test entrypoints while keeping recoverable reference |

## Out of Scope

Generator/CAD/kernel/engine rewrites, inventing new supported features, replacing file formats, native hosting, loosening performance budgets, deleting unexplained failures, unattended deployment, or changing production writers without approval.

## Further Notes

**Lead owner:** qualification/release agent. **Independent reviewers:** interaction and cross-workflow Spec review; release author cannot approve its own acceptance evidence. Coordinator owns the final adoption wiring and ledger. F9.1 preparation can proceed beside feature work; aggregate gates are release joins, not artificial blockers on independent implementation.

Source/evidence map: `.scratch/dioxus-frontend-v1/evidence/tsx-inventory.json`, `docs/migration/dioxus-frontend-v1-run.json`, `.scratch/m1-production/ACCEPTANCE.md`, existing application/core/renderer tests, `app/e2e`, the frontend browser drivers and maintained root/subpath build helper. Proposed maintained frontend scenario manifest and Dioxus browser harness belong under the existing migration evidence/testing convention, with their final repository home selected when implementation makes that concrete.

The final lane must resolve the existing M1 acceptance applicability explicitly. It cannot claim full M1 or wider full-Rust runtime completion merely because the frontend is finished.


### Authoritative execution dependencies

The milestone-level prerequisites above describe integration context. The refined rows below replace whole-milestone or symbolic dependencies. Preparation/fixture work may start after `Start after`; completion also requires `Acceptance joins`. Existing F1/F3a source and evidence are baseline prerequisites, not tasks to repeat. Full workflow qualification also joins F2 shared panels/controls under F9.

| Slice | Start after | Acceptance joins |
| --- | --- | --- |
| F9.1 | Baseline; preparation may start | Own slice acceptance |
| F9.2 | Baseline; preparation may start | F2.2, F2.4, F3.7, F4.5, F4.6, F5.8, F6.6, F7.8, F8.6 |
| F9.4 | Baseline; preparation may start | F8.6 |
| F9.5 | Baseline; preparation may start | F9.2 |
| F9.6 | F9.1, F9.2, F9.4, F9.5 | Own slice acceptance |
| F9.7 | F9.6 | Own slice acceptance |

### Refactoring observation handoff

Update the [living RF register](../refactor-findings.json) and [post-port takeaways](../../../docs/migration/POST-PORT-REFACTOR.md) for architectural, design, theoretical or quality issues discovered in this slice, or record “No new refactoring takeaway observed” with reviewed scope. Distinguish confirmed findings from hypotheses; include evidence, impact, current mitigation, later proposal and validation. This does not authorize unrelated refactoring or defer required parity fixes.
