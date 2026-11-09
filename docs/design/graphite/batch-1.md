# Graphite Batch 1 — implementation and evidence

Mode: Operate. Scope: shared tokens/controls, single-header shell, docked panels, contextual toolbar and Layout presentation. The user approved the shaped brief and authorized implementation on 2026-10-12. [Migration brief](../../plans/ui-design/graphite-migration.md).

## Direction contract

THESIS: A compact keyboard workbench keeps the canvas dominant and exposes frequent edits beside current selection; unrelated specialist controls remain behind named sections.

OWN-WORLD: Graphite charcoal canvas and docks, periwinkle actions/selection, violet parts and green board geometry, Source Sans 3 and Source Code Pro, flat surfaces, crisp boundaries, one readable opaque toolstrip and docked panels.

STORY: Open a real project, select an object, find applicable actions, edit precise values and see accepted save feedback. Existing field drafts and undo remain dependable while the presentation changes.

FIRST VIEWPORT: One header with brand/project, history, save state and Export. Workspace navigation lives in the shared Objects dock. Objects at left with visible snapping at its foot; canvas in the center with compact contextual tools and view controls; Inspector at right; camera/status controls below. Narrow screens retain existing explicit panel switches and a single header.

FORM: Selected Graphite Layout comp [layout-approved.png](layout-approved.png), derived from direction seed `c452a8de`; user-pinned visual direction with no new concept round. Geometry, labels and generated key counts are illustrative. All shipping interface/content regions are semantic Dioxus/native controls or existing product-rendered geometry; this batch introduces no raster UI assets.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.

## Boundaries and adaptations

- Batch 2's new direct geometry handles and editable canvas dimensions remain deferred. Existing transform tools and overlays continue to work; exposing Splay directly is presentation of an existing action.
- Keep workspace navigation in the shared Objects dock; the browser feedback removes the duplicate header tabs and compact selector. Real document geometry is authoritative over generated keyboard shapes and counts.
- Grid increment and geometry snapping are separate existing preferences. Both are visible in the dock; specialist snap options retain the existing controlled popover.
- Retain current panel resize, collapsed/autohide choices and persisted widths; Graphite sizes are defaults. Keep existing Light/System preferences and all non-Layout workflows operable.
- User requested solo execution. Finish review and documentation use the skill's inline fallback workflow rather than spawning agents.

## Implemented

- Shared dark Graphite surface/field/toolbar roles, control sizing, spacing and typography roles extend `--wb-*` and Tailwind's existing inline theme. Light/System preferences remain intact.
- BoardToolbar/BoardToolbarButton wrap the pinned Dioxus Components toolbar primitive; existing controlled popovers, native fields and disclosures retain their owners. Direct Splay exposes an existing tool.
- A single header presents project identity, history, real save status and Export. The shared Objects sidebar owns workspace switching. History remains reachable in the footer at compact widths.
- The Objects dock keeps grid increment and geometry snapping visible, including while the existing setup guide is open. Advanced snap settings retain their popover and Escape/focus behavior.
- Layout context actions have one contrasting opaque backing and filled active states. Transform fields precede Key size; Key size uses a native disclosure whose children stay mounted.
- Narrow Layout views use full-width Objects/Inspector panels, independent scrolling and a canvas that fills the workspace. Footer actions remain reachable; no horizontal document overflow was observed.

## Executed checks

| Check | Result |
| --- | --- |
| Affected native tests | 205 passed |
| Shared controls/panels and dependent workspace browser suites | 323 passed across ui-shared, Keycaps, Library, Keymap, Case, Parts and PCB |
| Layout browser suite | 171 passed; separate setup-guide test passed (1) |
| Page browser suite | 72 passed, including isolated Case workspace coverage |
| Rustfmt, strict workspace Clippy and WASM page compilation | Passed |
| Documentation links and whitespace | Passed |
| Real application task journeys | Field edit/Undo/Redo, Escape draft cancellation, inspector tabs, snapping, popover Escape/focus, all workspace routes, Export and save/reopen passed |
| Compact layout | 390×844 document has no horizontal overflow; inspector scrolls within the workspace; snapping dock meets the footer at y=804 |

Browser suites executed 567 tests in total, without double-counting the three additional focused Layout page checks. The initial affected check run stalled during Chrome startup; later isolated runs completed the remaining checks. One legacy visual assertion was updated from the old 6px radius to the approved shared 5px control radius. Regressions in toolbar placement and guide-state snapping were reproduced before their fixes and verified afterward.

Measured opaque dark text contrasts: primary/panel 13.89:1; muted/panel 8.16:1; toolbar text 9.34:1; action 8.36:1; action hover 10.53:1; selected text 7.77:1. These measurements do not certify all geometry, disabled opacity or 3D materials.

## Screenshot comparison

Inspect the [approved comp](layout-approved.png), [implemented desktop](implemented-desktop.png), [compact canvas](implemented-mobile.png), [compact Inspector](implemented-mobile-inspector.png) and [compact Objects](implemented-mobile-objects.png). The source captures and structured observations are in `.scratch/graphite-batch1/`; canonical copies are under `.impeccable/review/`.

The comparison keeps the selected charcoal/periwinkle system, dock topology, dominant canvas and opaque labeled toolstrip. Adaptations are explicit: navigation is restored, a 56px header replaces the generated 64px header, real document geometry/IDs replace the generated split keyboard, grid and geometry snapping stay separate, and existing inspector fields remain available. New handles/dimension labels belong to Batch 2.

The automatic pixel comparison reports drift; it is not a passing pixel-fidelity gate. Its region scoring marks even the visibly present header as missing because of differing text/content/metrics. The actual paired crops and viewport captures are the basis of the manual comparison. Do not use the numeric score to claim pixel-perfect reproduction or remove working product content to improve it.

## Finish review at Batch 1 scope

Solo execution uses the skill's inline review/documentation fallback; no agents were spawned.

Disposition: **fix — workflow closure pending**. Implementation checks and manual visual comparison are complete at Batch 1 scope; the automated Impeccable review checkpoint remains unclosed.

- Persistence: approved comp and exact prompt/provenance, direction contract, implemented captures, DESIGN.md and the extension sidecar are recorded.
- Fidelity: header, docks, backed toolbar, active state, field styling and snapping match the selected direction. Document geometry, restored navigation, preserved advanced fields and compact view arrangements are adaptations supported by the approved brief. Batch 2 handles are explicitly excluded.
- Ceiling: desktop, 1024px and 390px states were inspected together, material findings corrected, and final states confirmed. Additional captures verified identified functional regressions; no further visual alternatives or polish loop was opened.
- Material fixes: resolved toolbar bounds, active-button contrast, field hierarchy, compact panel scrolling, footer access, save-status overlap and guide-state snapping. No further Batch 1 implementation fix is identified by the recorded checks. Complete the required human review receipt and subsequent phase gates before reporting the Impeccable workflow as shipped.
- Keep: controlled edit/draft settlement, accepted-state history and save/recovery, workspace-specific owners and semantic geometry roles.

## First submitted review — implemented revisions

The user submitted changes requested for Objects, tree and contextual toolbar;
the other nine plan items were approved. The original [submitted feedback](feedback-1.json)
is preserved without rewriting its decisions.

- PCB, Case and Keycaps now appear as native workspace navigation leaves at the
  same tree level as Board. They retain current document selection and use the
  existing workspace state; their callbacks reject a departed scope/generation.
  These entries do not add CAD visibility toggles or fabricate geometry children.
- Matrix, Row, Column and Key selection each have a direct button; Part remains
  directly available too. Existing selection callbacks and the pinned shared
  toolbar's arrow-key behavior remain authoritative.
- Compact toolbar mode hides labels while retaining icons, accessible names,
  pressed states and existing actions. The density preference survives reload.
  Align now has an explicit icon. At intermediate and narrow widths the toolbar
  wraps to retain reachable controls; it is not forced into one row.

Latest captures: [desktop compact](feedback-1-desktop.png),
[desktop with labels](feedback-1-labeled.png), [1024px](feedback-1-intermediate.png),
[390px canvas](feedback-1-mobile.png) and [390px Objects](feedback-1-mobile-objects.png).
Compared with the approved comp and the first implementation, the fixed Graphite
palette, single header, docks and canvas remain; the tree leaves and selection
buttons are the user's requested extensions. This is a manual composition
comparison, not a claim of pixel fidelity.

Revision checks: 77 affected native tests passed (one existing ignored test),
172 Layout browser tests passed plus the isolated guide test (173 total), strict
Clippy/rustfmt and WASM page compilation passed. The mounted direct-choice test
covers pointer activation, pressed state, menu closure and arrow-key focus.
Real-app checks cover all four requested selection modes, keyboard navigation,
compact-mode reload, all three tree routes, Escape draft cancellation and
accepted-field reload. Desktop/intermediate/narrow toolbar bounds remained
inside the canvas, with no horizontal document overflow or page errors.

The first run exposed stale generated Tailwind output (old 6px button radius),
then poisoned later async browser tests after that assertion panicked. After
refreshing the asset, the complete Layout run passed. A separate wasm-pack
Chrome startup stall was bypassed using the same compiled test binary's direct
browser runner. Source fixes were limited to the shared button's supported
attribute syntax and the icon/row presentation identified during review.
Raw evidence is in `.scratch/graphite-feedback/`.

## Second submitted review — implemented revisions

The [second submitted receipt](feedback-2.json) requests shared sidebar reuse and
smaller subfunction popovers. The accompanying chat also requests right-hand
accordion sections. These changes supersede the first revision's Layout-only
navigation leaves and always-expanded context actions.

- All six editor workspaces now mount the same Objects navigation from the shell:
  Layout, PCB, Keymap, Keycaps, Case and Parts. The current workspace is marked.
  Existing domain tree children, visibility, scope and selection behavior remain
  authoritative; onboarding does not become an Objects pane.
- The main Layout strip retains direct scope choices, Transform, Align and density.
  Selection-aware Transform subfunctions move into a 220px popover. Labels remain
  visible inside the popover in compact mode. Existing tool callbacks, Escape,
  focus restoration, outside dismissal and selection conversion stay in place.
- Layout's right inspector uses independently collapsible Position/orientation,
  row offsets, column Position/rotation, Splay/origin and key Geometry/Assembly
  groups. They start open. Shared native `BoardDisclosure` retains child nodes,
  drafts and edit owners through collapse/reopen and unrelated renders. The pinned
  Dioxus `AccordionContent` was checked: it unmounts closed children, so it is not
  used for draft-owning fields. Existing Key size behavior remains.

Verified captures: [desktop](feedback-2-desktop.png),
[labeled toolbar](feedback-2-labeled.png), [Transform popover](feedback-2-popover.png),
[1024px](feedback-2-intermediate.png), [390px canvas](feedback-2-mobile.png),
[390px inspector](feedback-2-mobile-inspector.png),
[390px Objects](feedback-2-mobile-objects.png). Shared navigation was also captured
on [PCB](feedback-2-pcb.png), [Keymap](feedback-2-keymap.png),
[Keycaps](feedback-2-keycaps.png), [Case](feedback-2-case.png) and
[Parts](feedback-2-parts.png). The fixed Graphite palette, single header, docks
and real CAD content remain the comparison baseline; these are requested UX
revisions, not a claim of pixel fidelity to the illustrative keyboard.

Checks: 77 native tests passed (one existing ignored test), 173 Layout browser
tests plus the isolated setup-guide test passed (174 total), strict Clippy/rustfmt
and WASM page compilation passed. Mounted coverage verifies persistent draft
nodes, independent section collapse, no edit on toggle and retained state after
rerender. Real-app coverage verifies all six workspace routes, keyboard disclosure
activation, blank draft recovery, the 220px labeled popover, Escape/focus return,
existing tool activation/closure and compact preference reload. The compact strip
measured 48px tall at desktop/intermediate and 56px at 390px, with no horizontal
page overflow or browser page errors.

The first browser run embedded stale generated CSS, and a separate test initially
used arbitrary text in a native number input, which browsers sanitize to blank.
Refreshing the CSS and correcting the test to a valid blank intermediate draft
resolved both. The bounded screenshot pass identified compact-mode labels leaking
into the popover; the confirmation captures verify labeled subfunctions. Raw
observations, logs, source baselines and the actual receipt remain in
`.scratch/graphite-feedback-2/`.

## Browser feedback — remove duplicate header navigation

The user marked the header's Layout/PCB/Keymap/Keycaps/Case/Parts navigation for
removal. Both desktop tabs and the compact Workspace selector are removed. The
single header retains brand/project, history, real save feedback and Export;
workspace switching belongs to the shared Objects sidebar at every width. This
navigation is also reachable while the onboarding guide is displayed.

The workspace container is now a named region rather than a tabpanel referring
to deleted tab IDs. Sidebar buttons identify the region they control. Existing
workspace state, domain trees, editing, selection and Export-return behavior are
preserved. The action group stays right-aligned after the redundant navigation
is removed.

Verification: strict Clippy/rustfmt and WASM page compilation passed. Real-app
checks exercised keyboard activation for all six workspace routes, current-page
state, Export and return, and sidebar switching at 390px. Screenshots at 1359px,
1024px and 390px confirm one 56px header, no duplicate workspace navigation and
no horizontal overflow or page errors. Captures: [desktop](header-navigation-desktop.png),
[compact canvas](header-navigation-mobile.png) and
[compact sidebar](header-navigation-sidebar.png). Raw evidence is in
`.scratch/graphite-header-navigation/`. Earlier Batch 1 behavior suites remain
recorded above; they were not broadly rerun for this bounded shell change.

## Remaining boundaries and tool limitation

- New direct geometry interactions and editable canvas dimensions remain Batch 2. The other workspaces use the shared foundation but retain their existing content pending their planned migration batches.
- Generated reference counts, footprint geometry and pin assignments are not implementation requirements. The selected-context companion/key count wording remains existing product behavior, not an asserted new semantic rule.
- The Impeccable CLI preserves both submitted changes-requested receipts. Round 4 also incorporates the header-navigation browser feedback. Five plan items await review (header, Objects, tree, toolbar, inspector controls); seven unchanged approvals carry forward. The [component review checkpoint](http://127.0.0.1:42679/) uses the same session (`4e679bbd8c1af8579db49314895e90ba444a6767f9ae55ad2801750f7fad285f`), with a 24-hour idle timeout and local draft backups. This packet reviews the plan with original comp crops; the actual implementation is shown in the captures above and the frontend preview. No receipt was fabricated, and subsequent automated phase gates remain open. Static page-capture gates are not a substitute for this worker-backed Dioxus application.
- No production/offline or full-provider rebuild was performed; provider inputs were unchanged. No repository-wide migration-complete claim follows from this batch.

Development preview remains at port 8182; port 8184 serves the mockup gallery. The existing frontend process group and private target remain running, with fresh asset health recorded in `.scratch/ui-dev/runtime.json`. Generated Tailwind output was refreshed using the installed pinned compiler before inspection.

## Chat approval and Batch 2 handoff

On 2026-10-12 the user approved the current implementation in chat and asked to
move on. Batch 2 direct Layout geometry work is authorized. The native review
packet still has no new submitted receipt; this chat approval is recorded here
without manufacturing one.
