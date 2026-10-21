# 001: Verify six application workflows through shared public seams

Status: ready-for-agent
Type: spec
Blocked by: None for agent characterization; operational prerequisites are listed below
Parent: [Application workflow verification](../map.md)
Related: [Workflow evidence](../../../research/2026-10-20-application-workflow-evidence.md), [ADR index](../../../adr/README.md)

## Problem Statement

BoardStudio has substantial unit, native, mounted and provider coverage, but that
does not establish that a designer's complete project, recovery, update, rendering
or handoff journey works. Some strong guarantees exist only at lower seams; older
browser evidence is not tied to current source. Hardware and package information
can also look complete while still needing downstream verification.

The designer needs their work preserved, usable failure explanations, pointer-free
access to a representative handoff, consistent assembly artifacts and portable
licence information. The maintainer needs evidence that names the actual source,
package, environment and limits instead of interpreting a test count as whole-app
assurance.

## Solution

Verify six bounded workflows through three approved existing public seams. Reuse
one supported synthetic assembly and shared package observations where useful;
retain isolated projects and browser storage so verification cannot endanger user
data. Record observed outcomes, regressions and remaining operational/human checks
separately. Implement the accepted root `license.md` package requirement where
missing, including safe import/re-export, without inventing distribution terms.

The six workflow outcomes are:

| Workflow | Representative outcome | Evidence/resource boundary |
| --- | --- | --- |
| Project changes and deletion during held/failed saving | Accepted/saved state, project records, pending-copy bytes, active editor identity and shared/Undo assets remain consistent through explicit recovery and reopen. | Agent workflow/mounted tests with real storage for transaction-specific cases; no simulated OS crash counted as process-crash evidence. |
| Old editor across a new release | Observe A with B waiting, locally available first-use CAD online/offline, a second A tab and subsequent B adoption without losing saved work or accepting stale results. | Two source-bound built artifacts and authorized switching of a disposable origin; real cache/network observations. |
| Renderer failure and recovery | Explain failure, retain forms/2D, reject stale picks and recover from the current accepted design: one automatic attempt when enabled, explicit Retry when paused or after failed recovery. Restore an available scoped viewpoint after recovery or same-session workspace return, otherwise fit an overview. | Controlled mounted failures plus genuine context loss where supported. Retention is session-only; no project/browser-reopen persistence is required under ADR-0014. |
| Keyboard-only source handoff | Create/name a project, add a small matrix and supported controller, configure wiring, bind one key and obtain the firmware repository template using trusted keyboard input. | Identified current packaged page, desktop and 390-pixel-wide navigation, visible focus and honest disabled/error states. |
| Supported assembly to downstream artifacts | One accepted MX/diode/nice!nano assembly has consistent pads, jumpers, effective GPIOs, key ordering and nominal PCB/model data across actual outputs. | Agent artifact comparison; compilation, routing/DRC, bench continuity and fit remain separately identified external/human checks. |
| Portable and downstream package notices | An offline recipient finds known applicable notices/source references in root `license.md`; information survives bounded project import/re-export. | Actual mixed-source package inventory and existing archive corruption/resource controls; not legal assurance. |

## User Stories

1. As a designer, I want a held edit to remain pending while saving is incomplete, so that switching projects cannot falsely mark it saved.
2. As a designer, I want a failed save retained for save-only retry, so that recovery does not replay my edit or discard its assets.
3. As a designer, I want to obtain a pending copy before explicitly discarding failed work, so that I have a recoverable record.
4. As a designer, I want cancelling project deletion to leave records and editing context intact, so that a confirmation dialog is not itself destructive.
5. As a designer, I want current-project deletion to wait for a usable saved replacement, so that I am not left with neither project available.
6. As a designer, I want failed replacement or deletion to leave a truthful recoverable state, so that I can retry without deleting the wrong record.
7. As a designer, I want deleting another project to preserve my current document and Undo history, so that unrelated work remains usable.
8. As a designer, I want unreadable project data recoverable without hiding healthy projects, so that one damaged record does not remove all access.
9. As a designer, I want shared and Undo-reachable assets retained during project deletion, so that later editing or recovery can still load them.
10. As a designer, I want reopening to establish a new editor session, so that old callbacks cannot update the new project view.
11. As a designer, I want a new release to wait while an old editor is active, so that an update does not remove its cached resources prematurely.
12. As a designer, I want locally available CAD to work when first opened offline, so that lazy worker creation is not confused with missing cached bytes.
13. As a designer, I want unavailable resources explained explicitly, so that a failed request does not look like success or endless generation.
14. As a designer, I want a second old tab to remain safe when the first closes, so that update activation respects all active clients.
15. As a designer, I want the new release to reopen saved work after old clients close, so that adopting an update preserves my project.
16. As a maintainer, I want the release experiment to identify actual page, worker, provider, cache and source versions, so that request freshness is not mistaken for release compatibility.
17. As a designer, I want initialization, render and context-loss failures to be visible, so that broken geometry is not presented as healthy output.
18. As a designer, I want forms and a usable 2D route after renderer failure, so that I can continue work without editing the document merely to repair the view.
19. As a designer, I want stale picks, handles and finding-focus events rejected, so that an old scene cannot select or change a replacement object.
20. As a designer, I want reopening a preview to use the current accepted source or show an explicit failure, so that I know which design is visible.
21. As a designer, I want successful recovery in the same project/view to restore my last usable viewpoint when available, otherwise fit an overview, so that renderer recovery preserves context without restoring stale geometry.
22. As a designer, I want to create and name a project without a pointer, so that the workflow is accessible from the keyboard.
23. As a designer, I want matrix creation, cancellation and placement reachable through keyboard controls, so that canvas interaction is not a pointer-only dependency.
24. As a designer, I want a supported controller and wiring configuration reachable without programmatic shortcuts, so that a successful handoff demonstrates a real user journey.
25. As a designer, I want to assign a representative key and navigate to Export using the keyboard, so that the generated handoff contains my intended input.
26. As a designer, I want invalid values and missing-controller reasons associated with usable controls, so that I can understand and correct readiness problems.
27. As a designer, I want a failed action retry not to duplicate edits, so that keyboard recovery has the same ownership guarantees as pointer interaction.
28. As a designer, I want focus visible and in a usable current destination when content disappears, so that creating or cancelling a form does not strand navigation.
29. As a designer, I want the representative navigation and handoff usable at desktop and narrow widths, so that a compact layout does not hide required actions.
30. As a designer, I want the firmware handoff to be an editable repository template, so that external compilation is not confused with a ready-to-flash download.
31. As a hardware designer, I want named controller, switch and diode variants preserved across the representative artifacts, so that the verification refers to one actual assembly.
32. As an assembler, I want jumper bridge/open instructions to agree with emitted pad nets and the selected face, so that silk labels are not substituted for connectivity evidence.
33. As a PCB designer, I want routing obligations disclosed when local traces are omitted, so that an unrouted handoff is not described as fabrication-ready.
34. As a firmware author, I want effective MCU GPIOs and key order to agree with the electrical plan, so that the source template describes the assembled connections.
35. As a mechanical designer, I want nominal PCB thickness, drills, model datum and placement cross-checked, so that exact representation is not mistaken for measured fit.
36. As a designer, I want handed-off assignments protected after Undo or cancelled delivery, so that verification respects the accepted external-handoff boundary.
37. As a reviewer, I want target builds, physical continuity and fit checks separated from synthetic tests, so that a passing artifact comparison is not called hardware validation.
38. As an offline collaborator, I want a root `license.md` with the known notices for packaged contents, so that I do not need this checkout to discover those terms.
39. As a contributor, I want retained vendor bytes and their embedded notices unchanged, so that packaging does not destroy source evidence.
40. As a recipient, I want known source references and supplied customization context associated with the relevant contents, so that different vendors and authored changes are not conflated.
41. As a designer, I want unknown authorship, revisions or terms identified honestly, so that generated notices do not invent permission or attribution.
42. As a designer, I want valid notice information preserved through project import and re-export, so that a round-trip does not silently strip the sidecar.
43. As a designer, I want unsafe paths, duplicates, oversize entries and bad asset hashes rejected after notice support is added, so that a new metadata file does not weaken package integrity.
44. As a recipient, I want the inner KiCad ZIP to carry its own root `license.md` and the outer handoff to retain its own clearly scoped notice, so that either distributed package remains understandable independently.
45. As a maintainer, I want every outcome classified as executed, source-inspected, policy or pending, so that no lower-seam test is overclaimed as end-to-end acceptance.
46. As a designer, I want one automatic preview-recovery attempt when automatic preview is enabled, so that a transient renderer failure can recover without reloading my project.
47. As a designer, I want paused automatic preview to wait for Retry preview, so that failure recovery respects my existing preview preference.
48. As a designer, I want failed recovery to retain an explicit error and Retry action without an automatic loop, so that repeated failures do not monopolize the application or hide their cause.
49. As a designer, I want returning to the same project/board/view after workspace navigation within an editor session to restore its prior usable viewpoint, otherwise fit an overview, so that visiting another workspace does not discard my viewing context.
50. As a designer, I want export to proceed with an explicit licence-unknown notice for content whose distribution terms are unavailable, so that missing information is disclosed without an acknowledgement gate or a claim of permission.
51. As a designer, I want keyboard focus to move to the new matrix's entry in Objects after saved creation succeeds and its form closes, so that I can continue navigating from the object I just created.

## Implementation Decisions

- Use the existing application/session/Core authority, Runtime effects, storage, renderer-host and package boundaries. Do not add a second application implementation, universal recovery service, asset registry or new browser/testing framework.
- Keep failed-save candidates and their staged bytes available until an explicit existing recovery choice. Retry persistence without replaying Core work; refuse stale writes and preserve shared/Undo assets under conservative reclamation.
- Whole-project deletion is a Library/store workflow and is distinct from deleting a physical half inside a project. Do not reinterpret the latter's undoable exclusive-design/accessory removal as whole-project deletion policy.
- Make observations and any reproducible failures visible at the owning workflow. Existing accepted safety requirements provide test oracles; where recovery wording or a focus destination is unspecified, characterize it and obtain a narrow decision rather than selecting new behavior from a test's convenience.
- After successful saved matrix creation closes its form, focus the new matrix's entry in Objects once that entry is rendered, provided the originating project/workspace is still current. Do not focus it merely on submission or steal focus after navigation/owner replacement. This is ordinary workflow acceptance, not a new focus framework or ADR; preserve existing Cancel return-to-opener behavior.
- Treat release A and B as actual packaged software artifacts. Retain delayed activation and scope-local caches; observe page/provider/cache origin rather than treating revision or executor identities as a build handshake. No new handshake, immutable asset-addressing scheme, forced reload or rollback strategy is selected without evidence.
- Renderer failure must not become document or manufacturing authority. Preserve forms/2D, captured-source checks, cleanup and stale-event rejection. Apply ADR-0014's accepted recovery policy using the existing affected-view automatic-preview setting: one automatic recovery attempt when enabled, explicit Retry when disabled, and explicit Retry after recovery fails. Rebuild from the current accepted source without project reload or edit replay; rerenders must not create an automatic retry loop. Recovery and ordinary same-session workspace return restore an available usable viewpoint for the matching project/board/physical-instance/view, otherwise fit the current design. Do not borrow another scope's camera, restore stale geometry or override deliberate Fit/navigation. Retention is session-only; project reopening/browser restart need not restore it. Do not introduce a saved camera schema or global preference for cross-session retention.
- Keyboard workflow evidence uses trusted keyboard input and the visible production controls. Do not seed completed wiring through a hidden document replacement or programmatically focus/click controls and call it keyboard traversal. Use disposable synthetic projects and isolated storage.
- Use one small supported MX solder/diode/nice!nano assembly as the base consistency case, with a bounded face/jumper variation where applicable. Compare actual emitted plans, packages and nominal geometry; do not require every split-policy feature or peripheral class to finish this representative case.
- Firmware delivery remains an editable repository template built externally. Unsupported optional peripherals may be disclosed where the base configuration is valid; the simple baseline need not depend on delivering that entire separate support policy. PCB handoff protection remains saved before delivery and survives Undo and later cancelled/failed download.
- Every exported package covered by the accepted notice policy includes root `license.md`. Preserve known actual terms and source references. If distribution terms are unavailable, proceed with an explicit licence-unknown notice for the affected content; do not require acknowledgement or block solely for unknown terms. This is not permission to waive known restrictions, invent terms or weaken unrelated integrity/readiness checks.
- Support the notice sidecar deliberately through bounded project archive admission and retained information for re-export. Preserve existing path, duplicate, resource and asset-hash checks. Reuse known catalogue/asset/source metadata and retained file notices; do not build a universal history-reconstruction system for old unknown assets.
- Check notice coverage for the package actually distributed. User-authored modification notes are optional: include supplied commentary, disclose unavailable history honestly and do not block export solely for a missing note; known applicable vendor notices remain required. The inner KiCad ZIP is independently forwardable and must carry its own root `license.md` covering its contents; the outer handoff also retains a root notice with clear coverage. Verify both inventories and do not rely on the outer notice alone or invent a common licence for nested contents.
- Existing source-preservation, forgiving vendor-preview, current-generator, replacement-input, hardware-topology and output-readiness policies remain authoritative. This spec neither weakens them nor claims all their implementation debt is in this scope.
- A necessary fix must be tied to an accepted requirement and a demonstrated failing scenario. New recovery/product policies, broad refactors and cross-boundary mechanisms need separate explicit scope; this is not permission to fix every concern found by research.
- No persisted-schema or public-interface redesign is assumed. If the accepted notice requirement needs additional retained metadata or an admission-contract change, carry it end to end in the owning slice and review it under the existing boundary rules.

## Testing Decisions

The user approved **three shared validation seams on 2026-10-20**. Test external
behavior through the highest existing seam that actually reaches the condition;
retain useful lower tests for store, device and protocol boundaries. There are no
six separate test systems and no expectation to duplicate the full matrix in-browser.

### S-A: real application workflows and mounted composition

Use the real Session/Core Runtime with existing held/failing-effect controls,
targeted production Library/Editor/Parts/Export mounts and actual disposable
IndexedDB only for storage-specific crossings. Reuse the production App composition
seam from existing Editor work rather than a fake Session or a second App.

Minimum representative coverage:

- One asset-bearing project A and project B, with a held/failing A edit/save crossing the real Library switch/delete controls, explicit recovery and reopen. Assert accepted owner, stored records, retained pending-copy bytes and Undo-reachable/shared asset lifetime.
- Current deletion Cancel, successful saved replacement, failed replacement/deletion and noncurrent deletion without resetting active Undo. Reuse existing cases instead of copying them where they already cover the boundary.
- One current renderer failure with forms/2D remaining usable. Cover automatic mode's single recovery attempt, paused mode waiting for Retry, and failed recovery remaining terminal until explicit Retry; ordinary rerenders must not retrigger attempts. Check successful same-view recovery with a known available viewpoint and separately with unavailable/unusable view state requiring fitted overview. Recover only the current source and reject another scope's camera and late picks/focus/handles. Do not use DOM-root removal as proof the production route unmounted.
- One actual workspace departure and return within the same editor session: restore the remembered matching viewpoint despite renderer remount where applicable, reject another board/instance's state, retain current geometry and honor a deliberate Fit action. Keep the no-usable-state fitted fallback. Do not confuse view remount with project reopen or require a new persisted camera schema.
- One keyboard form-error/cancel/retry slice checking visible errors, usable focus, accepted/saved state and once-only settlement. Successful saved Create matrix must close its form and focus the new Objects entry; failure must not take that success-focus path, and late completion after navigation must not steal focus. Verify this through actual composition and trusted keyboard traversal as appropriate, not selection-only assertions or programmatic focus counted as user input. Existing Cancel focus behavior remains unchanged.

Prior art includes the durable Session retry/unknown-outcome tests, actual Library
delete/replacement tests, storage lifetime/conflict tests, Runtime pending-archive
tests, Parts initialization-failure test, SharedViewer identity checks and matrix
setup/control focus tests. Their existing fixture and mock boundaries must remain
explicit; a scripted renderer event is not genuine device loss.

### S-B: source-bound packaged-page journeys

Use the existing packaged-page runner/receipt family, coordinated with the shared
browser-harness work. Each page/artifact result names its actual source/build and
browser/environment identity. An older unbound package is useful observation, not
current-source acceptance.

- **Trusted keyboard journey:** create/name, small matrix, supported controller placement and wiring, one binding and an actual firmware source download through visible controls. Include invalid input, Escape/cancel, failed-action retry and desktop/narrow navigation. Inspect downloaded contents through S-C. Read-only DOM/focus observations are allowed; programmatic focus/edit or pointer shortcuts are not keyboard evidence.
- **Genuine context-loss journey:** record document, selection and camera, induce loss through the actual supported GPU/context mechanism, and observe the automatic attempt or explicit Retry according to existing preview mode. Confirm recovery uses current source without project reload/edit replay, restores the last same-view viewpoint when available or fits an overview otherwise, and rejects old picks. Use controlled mounted failures for repeat-failure/loop prevention and the unavailable-viewpoint fallback where needed; do not label those actual GPU loss. If real loss is unsupported, mark device evidence pending; same-session workspace retention is tested separately and does not imply project/browser-reopen persistence.
- **Two-release journey:** use already-built source-bound A/B artifacts at a disposable origin/subpath. Let A finish installing/controlling and leave CAD uninstantiated; do not assume its provider bytes are uncached. Keep two A tabs, switch serving to B, observe waiting/asset origin, exercise locally available first-use CAD online/offline, close one A tab and finally all A clients, then adopt B and reopen saved work. A separate cold/interrupted-install case may establish cache-miss behavior without deleting a live editor's cache.

Expected release outcomes are saved-work preservation, delayed activation and
honest usable/unavailable results, not an invented guarantee of uncached remote
access or universal A/B interoperability. Record actual hosting response/cache
behavior; local server headers do not prove hosted CDN behavior.

### S-C: accepted source to actual package round-trip

Drive existing Runtime export/handoff and Core artifact/archive interfaces with one
accepted supported assembly plus representative mixed-source materials. Inventory
the actual project package, outer/inner PCB package and firmware template. Compare
part/pad/net identities, bridge/open recipes, effective GPIOs, key order/binding,
nominal thickness/drills/model placement, retained vendor bytes and known notices.

Exercise root `license.md` export, bounded import and re-export with existing
unsafe-path, duplicate-entry, oversize/decompression and bad-hash negative controls.
An import that accepts a sidecar and then loses its information is not a successful
round-trip. Known vendor notices inside original bytes remain preserved; unknown
terms are not manufactured. Do not inject caller-owned claims of source accuracy
or fabricate physical measurements to make artifact consistency pass.

Include one locally supplied asset with unavailable distribution terms: export
must proceed without a mandatory acknowledgement, identify that content as licence
unknown, preserve its original bytes and retain all other integrity checks. This
case is not evidence that redistribution is legally permitted.

### Evidence, execution and safety

- Distinguish inspected source/tests, fresh executed checks, historical receipts, accepted policy, static concern, demonstrated failure and pending external verification.
- New nontrivial behavior changes require a genuine public failing regression and GREEN evidence, not merely inverted source-string assertions. In areas being characterized rather than changed, report the observation and applicable policy instead of inventing an expected product answer.
- Reuse supported check scheduling and existing evidence/receipt formats. Do not overlap builds against a target, copy source trees, create target directories under evidence or produce custom pre/post source manifests.
- Use owned disposable browser storage and synthetic data. Do not clear personal profiles, alter production projects, switch an unapproved hosted deployment, publish repositories, flash devices or change hardware as a testing shortcut.
- Long logs belong in gitignored evidence storage; the durable result states source, commands/conditions, outcome and limits. Passing a subset is not whole-application acceptance.

## Out of Scope

- A whole-application, security, accessibility, legal or hardware certification audit.
- New ADRs or reopening already accepted topology, generator, input-identity, package or firmware-delivery decisions.
- Unbounded automatic renderer retries, recovery that ignores paused automatic-preview mode, camera-persistence guarantees beyond accepted recovery and same-session workspace retention, a new recovery journal, automatic merge, rollback/update-policy redesign or a generic compatibility handshake chosen without evidence.
- New browser infrastructure, duplicate deployment gating or duplicate Editor/CAD implementation tickets.
- User-authored geometry scripting, binary firmware generation, repository provisioning/publication or automatic flashing.
- All-catalogue/driver/transport qualification, general CAD/STEP certification, physical manufacturing guarantees or inferring measured fit from nominal solids.
- New cap/repair-allowance policies, a universal asset-version/provenance registry, new licence terms, blanket relicensing or a blanket unknown-licence export ban.
- Implementing every concern revealed by research, unrelated UI polish or broad source refactoring.

## Further Notes

### Research baseline is not acceptance

The [workflow research](../../../research/2026-10-20-application-workflow-evidence.md)
inspected connected paths in all six areas. It executed two small tooling tests and
one partial keyboard journey on an older, source-unbound package. That journey did
not configure a controller, download a handoff, invoke 3D or exercise two releases.
It is not a current-source focus regression or evidence that these six outcomes pass.

### Operational and human prerequisites

| Check | Required input/access | If unavailable |
| --- | --- | --- |
| Current packaged-page journeys | Identified built package, browser and isolated profile/origin | Continue source/mounted characterization; page acceptance stays pending. |
| Two-release transition | Source-bound built A/B artifacts, manifests/receipts and operator-authorized switching of one disposable origin | Publish the bounded recipe and precise missing inputs; do not count a source-only service-worker check as release acceptance. |
| Genuine GPU loss | Browser/device/context exposing a safe actual loss mechanism | Record capability and missing evidence; preserve scripted lifecycle tests without claiming actual device recovery. |
| Full firmware compilation | The exported target's exact external toolchain or authorized build runner | Keep source/template consistency results separate; fake build-tool tests do not produce a compiled image. |
| Downstream routing/manufacturing | KiCad reviewer, completed routing and applicable ERC/DRC/process checks | Report the routing handoff and unmet obligations, not fabrication readiness. |
| Physical continuity/fit | Qualified operator, exact hardware variant, safe unpowered continuity setup and representative manufactured/prototype parts | Retain a finite bench/fit checklist as pending; do not claim electrical operation or dimensional acceptance. |
| Licence interpretation assurance | Applicable primary terms and qualified interpretation where a legal conclusion is requested | Propagate known information without inventing terms or making a legal-compliance claim. |

The representative bench checklist compares emitted jumper instructions and pad
numbering with actual unpowered continuity/isolation, diode polarity and power-rail
short checks before any powered work; checks nominal model/socket/enclosure fit
separately; and records every human modification to the exported source or routed
board. Access to hardware or permission to power/flash it is not assumed by this spec.

### Existing work and decision boundaries

Reuse the [Editor production-composition work](../../editor-workspace-extraction/map.md),
[shared browser harness specification](../../cad-deepening/issues/016-rust-browser-harness.md),
[deploy-gating ticket](../../cleanup/issues/06-gate-pages-deploy-on-checks.md) and
[forgiving-preview successor work](../../cad-backend-replacement/spec.md). Refresh
their status and coordinate shared source before scheduling; publication is not delivery.
Do not edit or close those parent records as a side effect of this specification.

Renderer recovery mode is now settled by
[ADR-0014's amendment](../../../adr/0014-document-authority-and-captured-lifetimes.md#amendment-preview-recovery-respects-automatic-preview-2026-10-20).
Camera retention is session-only, not a remaining cross-session decision.
Modification notes are optional under ADR-0033, not a remaining mandatory-capture decision.
Independent nested-package notice coverage is now settled by ADR-0033.
Unknown-term admission is settled: proceed with explicit disclosure, not an acknowledgement gate.
Successful matrix creation now focuses the new matrix's Objects entry after saved
settlement and form closure. The researched decision queue is resolved; implementation
and source-bound verification remain. Do not reopen the queue merely for missing
operational resources. A genuinely new ambiguity found during characterization
needs a narrow decision gate rather than an invented regression oracle.

The spec is published with `ready-for-agent` status after verification-seam approval.
No child ticket breakdown, implementation, test run, operational experiment or human
verification is authorized as completed by publication. The six earlier proposed
investigation tickets remain unpublished; any later breakdown needs approval.
