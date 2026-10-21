# 001: Separate Editor workspace ownership without changing lifetimes

Status: ready-for-agent
Type: build
Blocked by: None (can start with composition characterization)
Related: [Ownership research](../../../research/editor-workspace-extraction.md), [plan map](../map.md)

## Problem Statement

People designing a keyboard expect workspace changes to be safe: a pending edit must
not disappear, a delayed result must not affect another board, and returning from a
guide or finding must restore the intended destination. Today a single large Editor
module assembles most workspace controllers, selection admission, canvas behavior,
guide orchestration and panel content. Even a local workspace change requires
understanding unrelated workflows, making improvements slower and regressions harder
to isolate.

Workspace state and render modules already exist, but much of their meaningful wiring
still belongs to Editor. Splitting only the markup would not solve this problem.

## Solution

Keep the application looking and behaving as it does now while assigning each
workspace its complete page-shell orchestration: controllers, accepted-state
projections, admitted actions and construction of its visible surfaces. Keep shared
selection, navigation, canvas ownership and panel composition in the common shell.

Make the actual production composition mountable with the existing test Runtime so
workspace transitions and lifetime behavior can be verified together, rather than
inferred from isolated helpers. Preserve existing feature tests and add focused
composition tests only for behavior they cannot establish.

## User Stories

1. As a keyboard designer, I want to switch among Layout, PCB, Parts, Case, Keymap and Keycaps, so that I can continue working without unintended document edits.
2. As a keyboard designer, I want selection to follow the existing workspace rules, so that switching views does not silently select a different object.
3. As a keyboard designer, I want board and physical board instance navigation to remain distinct, so that I edit the intended design or physical realization.
4. As a keyboard designer, I want returning to a board not to revive obsolete pending-edit feedback, so that an old interaction cannot overwrite my current work.
5. As a keyboard designer, I want a replacement project with reused object IDs to remain isolated from the previous session, so that delayed actions cannot affect it.
6. As a keyboard designer, I want pending field edits to keep their existing queueing behavior, so that rapid entry still applies in order.
7. As a keyboard designer, I want a newer draft value to survive an older result, so that asynchronous completion cannot erase what I am typing.
8. As a keyboard designer, I want failures to appear only on their current controls, so that another selection does not inherit an unrelated error.
9. As a keyboard designer, I want one-shot actions to remain unavailable while pending, so that double activation cannot create or remove items twice.
10. As a keyboard designer, I want leaving a workspace to retire obsolete UI observations without canceling Session-owned work, so that navigation does not lose a submitted edit.
11. As a keyboard designer, I want accepted values, silent retirement and inline failures to keep their current meaning, so that the refactor does not introduce misleading Saved messages or retries.
12. As a keyboard designer, I want Undo and Redo to keep their existing transaction boundaries, so that each action remains reversible in the same way.
13. As a keyboard designer, I want Layout creation and transform controls to keep their selection and ownership checks, so that a stale control cannot edit another target.
14. As a keyboard designer, I want Layout pointer and keyboard gestures to preserve their priorities, so that pan, selection, placement and editing do not compete.
15. As a keyboard designer, I want canceling a placement or gesture to keep its current effect, so that temporary previews do not become accidental accepted edits.
16. As a keyboard designer, I want only one interaction to own the canvas, so that matrix, pair, part and outline workflows cannot overlap incorrectly.
17. As a keyboard designer, I want PCB actions to keep their readiness and locked-item checks, so that reorganizing the UI does not bypass editing safeguards.
18. As a keyboard designer, I want PCB plan resolution to remain available outside PCB, so that Keymap, the setup guide and Export keep receiving the current plan.
19. As a keyboard designer, I want choosing a controller in PCB to retain its Parts-to-Layout workflow, so that I can place the intended component without losing the chooser context.
20. As a keyboard designer, I want placing a matrix assembly from Parts to enter Layout with the existing view and panel behavior, so that the placement is immediately usable.
21. As a keyboard designer, I want mounted-module browsing, attachment and return navigation to validate the original destination, so that a changed Parts selection cannot redirect an old result.
22. As a keyboard designer, I want Case assembly, body and layer selection to retain their distinct behavior, so that the correct Inspector and physical instance remain active.
23. As a keyboard designer, I want Case's previous display geometry to remain distinct from current editable results, so that stale geometry cannot authorize edits or findings.
24. As a keyboard designer, I want the guide and Case Inspector to share one physical-setup workflow, so that they cannot submit conflicting setup operations.
25. As a keyboard designer, I want guide stages to select the same workspaces and reveal the same panels, so that the guided workflow remains predictable.
26. As a keyboard designer, I want temporary guide hiding during placement not to destroy its workflow, so that I can continue the guide afterward.
27. As a keyboard designer, I want guide requests and preferences to stay associated with their project, so that another project does not inherit them.
28. As a keyboard designer, I want guide finish, dismiss and full Case settings return to retain their different navigation rules, so that each action does what it did before.
29. As a keyboard designer, I want Keymap layers, key bindings, encoder bindings and macros to retain their existing admission and feedback, so that reorganizing their wiring does not change firmware intent.
30. As a keyboard designer, I want Keymap firmware export to keep its current readiness and captured-source checks, so that exporting cannot use a stale board configuration.
31. As a keyboard designer, I want Keycaps assessment to stay current after changes in other workspaces, so that fit and legend information do not freeze while Keycaps is hidden.
32. As a keyboard designer, I want navigating a Keycaps finding to select and frame the intended Layout or Case target, so that I can inspect the actual problem.
33. As a keyboard designer, I want obsolete deferred fits and focus requests to be ignored, so that a newer selection is not interrupted.
34. As a keyboard designer, I want cross-board Layout findings and return navigation to retain their destination checks, so that the selected finding reaches the right board.
35. As a keyboard designer, I want camera, snap settings and shared 2D/3D preferences to retain their current sharing, so that workspace switching does not create unexpected resets.
36. As a keyboard designer, I want panel widths, compact behavior and explicit reveals to retain their priorities, so that guide and placement controls remain accessible.
37. As a keyboard designer, I want opening the Library to hide rather than unnecessarily recreate Editor, so that existing view state and permitted pending work survive.
38. As a keyboard designer, I want Export Back, Escape and the header toggle to return to the last non-Export workspace, so that I cannot become stuck returning to Export itself.
39. As a keyboard designer, I want Export's wiring and Case review links to keep their distinct selection behavior, so that review takes me to the intended context.
40. As a keyboard designer, I want genuine teardown to release listeners, pointer capture and observers without reviving UI after a reply, so that closing the editor does not leave stale interactions behind.
41. As a keyboard designer using a keyboard or assistive technology, I want labels, focus, shortcuts and control semantics preserved, so that the refactor does not reduce accessibility.
42. As a maintainer, I want workspace controllers, projections and surface construction behind a small private interface, so that a local change does not require reconstructing the workspace in Editor.
43. As a maintainer, I want the production subscriber and providers exercised by composition tests, so that tests do not accidentally validate a simplified second application.
44. As a maintainer, I want each extraction slice to keep the application and existing checks working, so that failures can be attributed to a bounded ownership change.

## Implementation Decisions

### Ownership and interfaces

- Keep all workspace containers in the page-shell binary. Existing workspace crates
  remain feature owners; Runtime remains presentation-independent. No new crate,
  dependency direction, public cross-crate contract or persistent schema is required.
- Extend the existing Layout, PCB, Parts, Case, Keymap, Keycaps and setup modules.
  Each workspace module owns its controller calls, source identities, projections,
  admitted callbacks and construction of its Objects, toolbar, canvas and Inspector
  surfaces. Moving only markup, or exporting individual controllers so Editor can
  rebuild its former large input structures, does not satisfy this specification.
- Preserve the distinction between code ownership and mount lifetime. Initially use
  ordinary, unconditional workspace hooks in the existing Editor scope. Do not add
  active-workspace-only controller mounts or new conditional hook sequences. Retain
  the existing accepted-scope guard and provider initialization dependencies.
- Preserve three lifetimes: App owns application composition and reconciliation;
  Editor owns long-lived workspace orchestration; visible children retain their
  existing conditional resource, draft-binding and observation lifetimes. Do not
  introduce project-keyed remounts or blanket workspace draft retention.
- The common shell owns common-ancestor providers and outlets, shared selection and
  instance authority, workspace routing, panel composition and cross-workspace ports.
  A provider must remain an ancestor of every consuming slot, even when those slots
  render in different panels. Do not replace stable provider identity reactively.
- Workspace interfaces consume existing authorities and only the peer capabilities
  they actually need. Keep controller details private; return rendered slots or
  existing typed surface inputs constructed by the module, plus genuinely shared
  projections. The prototype demonstrated this shape for Keymap: consume the one
  PCB resolution signal, retain controller hooks privately, and expose view/contours
  needed by common camera handling. Do not mandate that exact prototype type shape
  for unrelated workspaces or introduce a universal workspace trait or registry.
- Preserve one shared canvas interaction arbiter, one instance preference authority
  and the existing shared selection-kind, snap and 3D-mode identities. Source-module
  relocation must not create a second copy or change reset policy.
- Keep PCB plan resolution and Keycaps fit assessment alive for Editor lifetime;
  their consumers are not limited to their visible workspace. Keep one physical-setup
  controller serving both Project guide and Case Inspector, with their existing
  distinct admission predicates and shared busy gate.
- Setup ownership includes guide requests/preferences, project-name editing, stage
  actions, visibility, wiring/auto-open effects and drawer composition. Shared
  instance preference is not guide-owned; geometry-script visibility belongs to
  Layout presentation. Relocate these coherently without altering their lifetimes.
- Layout owns its transaction canvas, overlays, controller/projection assembly,
  Inspector priority and feature-specific gesture admission. PCB owns PCB-specific
  hit and action admission. Shared pan, wheel, surface measurement and camera
  coordination use the existing navigation modules. Move remaining camera wiring
  behind that responsibility without normalizing intentionally different fit formulas.
- Preserve routing-slot refresh and initialization-before-use, captured owner
  admission, post-Landed follow-ups and destination checks. Keycaps finding admission,
  source-camera capture, pending destination fit and deferred focus move as one
  Editor-lifetime workflow. Cross-board Layout findings retain their resume path.

### Application composition seam

- Separate Runtime construction from the existing production App composition through
  a private, mountable interface accepting the same Runtime handle. Normal startup
  still constructs the production Runtime. Tests supply the existing real browser
  test-support Runtime with in-process Session/Core and memory persistence.
- Reuse the actual production subscriber, scope transition/selection reconciliation,
  context providers, header navigation, Export-return tracking and Library/Editor
  mount logic. Do not copy that logic into a test-only host. Preserve the single
  Runtime notifier; no workspace may install a competing subscriber.
- The prototype's ordinary composition function demonstrates a viable seam without
  adding a Dioxus scope. Production implementation must preserve hook ownership and
  teardown while keeping test controls, diagnostics and fixtures out of normal startup.
- Establish a scope-correct effect-driving fixture before extracting workspaces. The
  prototype's parent-side driver produced cross-scope Signal warnings; reproducing
  that warning-prone driver is not an accepted test contract. Characterize warnings
  against unchanged production composition, fix fixture ownership rather than silence
  diagnostics, and stop ownership migration if safe driving/unmount cannot be shown.

### Behavior and delivery invariants

- Preserve Core as the only writable document/history authority and Session ownership
  of submitted work. Preserve operation-specific identity checks rather than imposing
  one universal owner tuple on every workflow. Workspace departure retires obsolete
  observations; returning does not revive them. It does not cancel authoritative work.
- Preserve queued field edits, one-shot disabling, newer-draft protection, binding
  replacement and unbinding, silent retirement, failure placement, save-before-publication
  and strict gesture/export ownership. Keep existing async execution primitives,
  cancellation checks and alive checks before any access to possibly dropped Signals.
- Preserve DOM keys, IDs, accessibility, CSS ancestry, event bubbling and existing
  panel/camera behavior unless a narrowly required structural change is explicitly
  verified as equivalent. Do not combine extraction with UI polish or semantic fixes.
- Keep production resource cleanup attached to its current owner: selection cleanup
  registration, pointer capture, active SVG/surface, ResizeObserver, layer-dock owner,
  document listeners and view-lifetime collision cancellation. Hiding Library,
  submitting Close and actually unmounting a scope are different events.
- Begin with a recorded baseline and the composition seam, then extract Keymap as the
  first coherent ownership slice. Move PCB and the shared plan capability next;
  Parts/Case and guide follow with one setup workflow; move Keycaps navigation as a
  whole; finish Layout and shared canvas coordination. Middle slices may reorder
  for demonstrated dependencies, but each must land working composition and checks.
- Refresh callers, tests, fixtures, exports and dependency assumptions before each
  slice. Remove obsolete forwarding/input plumbing after its consumers migrate.
  Do not copy unrelated work from the changing checkout or merge the prototype wholesale.

## Testing Decisions

### Primary seam and prior art

- The primary new test surface is mounted production App composition with the real
  test-support Runtime. Exercise user actions and assert visible results plus the
  relevant accepted document, saved result or Session view state. Do not substitute
  manually incremented generation, fabricated route effects or a fake workspace controller
  for production reconciliation.
- Existing mounted real-Editor layer-visibility and relation-highlight tests establish
  practical rendering prior art. Existing Keymap queued-edit, Layout Inspector and
  placement tests establish draft, gate and settlement patterns. Existing Keycaps
  navigation and Case conditional-Inspector tests establish deferred-fit and hook-scope
  hazards. Existing panel and guide tests supply policy coverage, not proof of the
  complete App flow.
- Keep lower-level native and mounted feature tests. Add focused composition cases
  for wiring and lifetime gaps; do not repeat every field edit in the page suite or
  replace existing tests with one enormous walkthrough. Native workflow tests remain
  useful but cannot establish browser mount, focus, pointer or observer behavior.
- Test external behavior rather than private field layout, function length, file
  counts, hook counts or source text. Review the ownership/interface change directly:
  Editor must no longer assemble each workspace's controller details and large
  Inspector inputs. A smaller function alone is not acceptance evidence.

### Required composition coverage

| Area | Observable acceptance |
| --- | --- |
| Workspace cycle | Navigate through all six workspaces and return using actual controls; correct surfaces, current selection/reset policy, panel defaults and shared view settings; no document revision change from switching alone. |
| Keymap ownership | Exercise layer, key binding, macro and encoder actions with an eligible PCB-plan fixture; existing providers work, accepted values and export readiness match the current board. |
| Pending work | Gate a field edit, leave/return using workspace controls, create a newer binding/draft where applicable, then deliver success and failure; Session work completes once and obsolete UI observation cannot overwrite or report against the current control. |
| Board and instance transition | Navigate through the production subscriber while a gesture or edit is pending; selection reconciles, owned gesture cleanup runs, physical instance preference resolves correctly and stale callbacks cannot retarget. |
| Project replacement | Replace with reused document/board/part IDs; advance the actual session identity and demonstrate stale retained callbacks cannot select, focus, report or edit the replacement. Exercise any deliverable late reply through the appropriate real Runtime adapter seam. |
| Parts and PCB routes | Drive a real Parts placement action through Layout canvas commit/cancel, matrix assembly placement, controller chooser and mounted-module attach/return; assert destination selection and accepted membership/pose, plus a stale destination rejection. |
| Guide and Case | Exercise Project physical setup, departure during preparation, accepted instance reassignment, stage/panel routing, placement hide/return, full Case settings return, finish and dismiss. Preserve one busy gate and the correct effective physical instance. |
| Findings | Navigate actual Keycaps findings to Layout and Case; frame the measured destination and focus its current control. Replace selection before deferred work and assert no obsolete fit/focus. Include a cross-board Layout finding and return route. |
| Camera and panels | Use a measured canvas to check fit/zoom/pan and shared 2D/3D transitions; preserve panel widths/modes and explicit guide/placement reveals. Include desktop and compact behavior. |
| Gesture dispatch | Exercise pointer drag/threshold/commit, modifier selection, Space pan priority, Escape, pointer cancellation/lost capture, keyboard placement and Undo/Redo without duplicate bubbling effects. Reuse existing geometry checks rather than rebuilding them here. |
| Export | Exercise entry from another workspace, Back, Escape, header toggle and distinct wiring/Case review routes. Return never targets Export itself; compact-panel policy holds; drop Export while mounted and verify its listener cannot act afterward. |
| Hide versus teardown | Show/hide Library while pending work settles without recreating Editor. Separately remove Editor while App remains and remove App itself; release held work and assert no stale DOM, focus or navigation writes, resource leaks or access to dropped Signals. Closing drains permitted authoritative work without making Close synonymous with unmount. |

### Fixture and evidence requirements

- Use minimal real documents with two boards, explicit physical instances, keys,
  layers, a relevant encoder and reused identities across replacement projects where
  the scenario requires them. Drive normal Runtime operations, not a second Session.
- Use bounded state-based waits and filtered Core/save gates so background plan and
  fit queries do not accidentally consume the intended hold. Establish gate entry
  before changing owners. Distinguish pending, successful save, accepted publication
  and view-only state in assertions.
- Serialized Open behind a held edit is not evidence of delivery after accepted
  replacement. Use the existing controlled adapter at the seam that can actually
  deliver stale output; keep its rejection coverage there and combine it with real
  mounted replacement/stale-callback coverage. Do not invent an impossible Session
  ordering merely to obtain an end-to-end test.
- Attach real DOM, provide nonzero canvas measurements and use production styles
  where layout is measured. Isolate and restore browser preferences. Avoid concurrent
  mounts sharing document IDs/listeners. Actual scope removal or VirtualDom teardown
  is required; removing only the root DOM node proves no lifecycle guarantee.
- The composition fixture must exercise mounted edit, workspace transition, board
  reconciliation, Export return, gated completion and genuine unmount without the
  prototype's cross-scope Signal warnings or dropped-Signal errors. Record any
  independently reproduced pre-existing diagnostic separately, not as a silent waiver.
- Memory persistence and installed identity-matched CAD fixtures do not qualify real
  workers/GPU behavior. Retain packaged-page provider and drag-smoke acceptance for
  the final integrated result; do not use the explanatory HTML model as test evidence.

### Completion checks

- [ ] Production App composition accepts the existing test Runtime through one private seam; normal startup retains its real constructor and single subscriber.
- [ ] The scope-correct mounted fixture and initial characterization pass before workspace ownership moves.
- [ ] All six workspace modules own their coherent wiring and surface construction; shared shell authority and controller/child lifetimes remain unchanged.
- [ ] Guide and camera responsibilities are separated without duplicating physical setup, instance preference, view state, routing or canvas ownership.
- [ ] Required composition behaviors above have automated evidence at the highest practical existing seam, with any intentionally lower-seam coverage explained.
- [ ] Existing relevant native and workspace-mounted tests, the page-bin browser suite and page-only WASM compilation pass; tests are not moved into the worker library.
- [ ] Run the repository's documented quick checks per slice and full pre-PR checks for the integrated result, including the checked packaged build and browser/provider/drag smoke. Use supported scheduling and record verification receipts; do not overlap builds against one physical target.
- [ ] Review both specification conformance and repository standards, including dependency direction, ownership, teardown, accessibility and absence of obsolete forwarding plumbing. No prototype controls, features, fixtures or HTML are shipped.

## Out of Scope

- A visual redesign, new user workflows, changed panel defaults, new camera math,
  workspace-specific copies of shared preferences, or broad draft persistence.
- New document schemas, migrations, public cross-crate interfaces, dependencies,
  generic workspace registries, event buses, mega-state contexts or a second Runtime/Session.
- Changes to Core geometry, allowances, mirrored-half behavior, edit resolution,
  firmware semantics, export leases, persistence policy or worker protocols.
- Converting existing document-replacement debt, fixing unrelated UI/CSS findings or
  implementing the independent cleanup audit tickets.
- Merging the experimental branch, shipping its diagnostics or treating its manual
  observations as a completed regression suite.
- Performance claims, forced line-count/file-count targets, or duplicating every
  existing feature test at App level.

## Further Notes

The user accepted the real-composition test direction, then requested source research
and an isolated prototype before this specification. Research established the
lifetime constraints; the prototype established bounded feasibility, not full
equivalence. Remaining lifecycle and routing gaps are explicit implementation gates
above, rather than claims that those cases already pass.

Primary evidence:

- [Ownership research and behavior matrix](../../../research/editor-workspace-extraction.md).
- [Architecture](../../../architecture.md) and [glossary](../../../../GLOSSARY.md).
- [Queued edits and retirement](../../../adr/0005-resolve-queued-edits-at-execution.md),
  [save before publication](../../../adr/0013-save-before-publication-and-explicit-recovery.md),
  [document authority and captured lifetimes](../../../adr/0014-document-authority-and-captured-lifetimes.md),
  [export ownership](../../../adr/0016-export-ownership-is-separate-from-preview-currency.md),
  and [real workflow/mounted-browser testing](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md).
- Throwaway branch `t3/editor-keymap-lifecycle-prototype`, source commit
  `c9e60307feeec9bdb0d2dc6d21a30a4af7440f59` and verdict commit
  `10cb410a1104fbe28970eac6e7a97b755a08ecb6`, based on
  `9099b3ef7a72b0919b1572f116490ed57514d43a`. The committed verdict records the
  exact private interface, observed interactions, commands and unproven cases.

The prototype demonstrated pending rename across workspace leave/return, Export
return actions, production board reconciliation, serialized replacement with reused
IDs, Library hiding, real App removal, and binding/macro saves. Normal and experimental
page WASM checks passed. It did not demonstrate reply delivery after accepted
replacement, every routing/focus path, warning-free external effect driving, or
packaged worker/GPU behavior. The separate HTML walkthrough models an invented policy
and is explanatory only.

Research inspected a changing working tree; the prototype used a clean older base.
Before implementation, establish a fresh baseline and reconcile concurrent changes
without discarding them. This specification authorizes the bounded production seam,
ownership refactor and regression coverage, not wholesale adoption of the prototype.
