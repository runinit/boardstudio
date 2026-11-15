# Proposed architecture and interaction contract

Parent: [map.md](../../toolbar-led-workbench/map.md). **Planning, not a shipped interface.** Confirmed constraints are
separated from concrete proposals needing approval in 001. The seam should make shared
behaviour local without turning workspace document models into toolbar primitives.

## Shared module and adapters

Use the existing `workspace_composition.rs::{objects,toolbar,inspector}` seam. Put the
shared host in a private page module, proposed `web/src/presentation/workbench_shell.rs`,
so its real adapter wiring and mounted tests share the page crate without widening domain
interfaces. Its typed workspace inputs already vary; retain those adapters rather than
introduce a generic entity or command registry. The shared host owns one 42px row, the
single fitting/clip implementation, top-level surface visibility/anchoring, inertness and
dismissal/focus. Ticket 009 owns those generic mechanics; 002 supplies Layout actions and
owns their fitting/motion acceptance, not another fitting engine. It receives workspace tools and
content, not runtime edit authority. Keep it private to its current crate where possible;
obtain approval before widening any cross-crate interface. Existing `ui-shared` controls
and semantic Theme roles remain the visual vocabulary.

- Layout/PCB keep `ScopedTreeContext` + `TreeSelectRequest` → `ContextRequest` /
  `submit_context`. Layout's `selection_kind_for_tree_context` remains the scope conversion
  owner; no invented scope for Outline, Bridge, Board or MountedModule.
- Keymap retains layer/input identities, edit tokens and binding/macro/encoder controllers.
- Keycaps retains board/matrix/key settings and owner-checked finding navigation.
- Case retains `TreeAction`, `CaseSelection` and physical-instance display requests.
- Parts retains definition/module/assembly selection, generation, preview and draft owners.

Only Layout exposes its five accordion sections. PCB uses its existing applicable Layout
mounts, not new operations. Keymap/Keycaps relocate existing `DesignViewToolbar` controls;
Case/Parts acquire shared utilities without fabricated Transform/Align sections. Domain
status, legend, Layers and error controls are not extra copies of Layout tools. Existing
2D/3D and Footprints toggles move into the applicable View section; Case's existing Live
preview/Update/Cancel/Export geometry controls move into its Preview section with their
Case-owned callbacks. Camera controls stay in the footer. Do not remount expensive viewers
to open a panel.

## Panel state and transitions

The private host stores only `Option<ToolPanel>`: Objects or Inspector (utility label
**Inspect**). Task/tool-detail visibility is read from its existing typed workspace owner,
not copied into a four-variant host enum. The composition asks that owner to hide/park a
surface before opening another; refusal leaves both owners unchanged. Hiding is not Cancel:
retain draft signals and their registration together, or keep the actual editor mounted.
The owner is also the only writer of its own visible/closed state. No mirrored flags,
global task registry, or new shell-owned admission. Layout's `Option<ToolbarSection>` and
nested pickers are separate. See [the corrected refactor seam](../../toolbar-led-workbench/refactoring.md).

Until 008, Layout may use the new host while other workspaces use their old one. Each
workspace has exactly one active panel authority; commands/reveals route by their TARGET
workspace and generation, not by a global new-host boolean. Route first, then reveal the
requested destination if still current. This development candidate stays on its branch;
a mixed-shell merge/rollout is not authorized.

| Event | Panel result | Selection / camera / drafts |
|---|---|---|
| Activate same utility again | None | unchanged |
| Activate other utility | requested panel only, if current close guard permits | retain drafts, focus enters requested content |
| Request tool detail / larger task | close compact panel, let its domain owner reveal the surface | retain controller, preview and pending guard; no copied host task state |
| Current task refuses a hide request | task remains; compact panel stays closed | no cancelled/replayed edit; do not invent a refusal where the current owner permits hiding |
| Domain's own Close/Cancel | derive closed state from that owner; no automatic old-panel reopen | existing explicit Cancel/settlement, not a shell cancellation |
| Workspace/board request while task cannot hide | existing domain navigation admission decides; veto keeps current route/task, accepted route invalidates old callbacks normally | no forced cancellation or stale task reveal in the next owner |
| Ordinary canvas or Objects pick | Objects closes; closed Inspect stays closed; already-open Inspect can update context | existing domain selection/focus path only |
| Explicit Inspect / Show Inspector / edit-details request | Inspect | existing selected owner; no automatic fit |
| Findings / Relationships / outline lifecycle request | Inspect with requested content | preserve existing scoped navigation, not implicit activation |
| Workspace change | None, close secondary popups | preserve workspace selection/tools/drafts for this editor session; never reopen a previous panel automatically |
| Board/instance change | close panels and transient tools; recompute availability | key drafts by actual owner; old callbacks cannot target the new owner |
| Project replacement / editor-session end | close and invalidate former owners | no draft replay into the new session |

Panel opening alone never changes canvas bounds/camera. An explicit Fit or existing finding
navigation that intentionally fits remains a separate action. Setup guide, project library
and Export keep their existing route/lifetime semantics and cannot leave hidden panel focus.
Show/hide commands request the destination workspace's sole current host. Legacy state may
remain for unmigrated workspaces, never as a second authority for migrated Layout.

### Reveal-site migration

Native source search confirms **13 call sites**, excluding the helper definition at
`presentation.rs:2886`. The coordinator owns this single disposition table and shared-file
hunks; each ticket proves its own observable route. Also audit non-helper reveals when
removing the old fields. Base-line anchors below refer to `cafaf430`.

| Source anchor / event | Destination / preserved intent | Ticket |
|---|---|---|
| `presentation.rs:1340`, Keycaps finding navigation | Explicit scoped Inspector reveal and intentional finding fit; retain captured owner | 012 |
| `presentation.rs:3544`, mirrored pair created | Existing explicit flow-completion selection/Inspector; no second creation/Undo | 005 |
| `presentation.rs:4142`, outline/bridge route | Ordinary active-outline pick does not open; explicit lifecycle/bridge intent keeps its current action/fit | 003/004 |
| `presentation.rs:4212`, Layout selection-kind projection | Select scope without forcing Inspector; keep context/anchor updates | 002/003 |
| `presentation.rs:4304`, PCB selection-kind projection | Same PCB scoped selection, no implicit Inspector | 010 |
| `presentation.rs:5637`, Keycaps finding RouteAction::PinInspector | Explicit finding destination through current workspace authority, no pin | 012 |
| `presentation.rs:5679`, on_toggle_layout_findings | Explicit Inspector findings with existing owner/highlight/return | 003 |
| `presentation.rs:7461`, open_geometry_scripts | Existing legacy editor as Layout-owned larger task, not Inspector | 005 |
| `presentation.rs:7869`, Layout on_show_properties | Inspect utility/retained explicit command; no duplicate Position editor | 002/003 |
| `presentation.rs:8009`, PCB on_show_properties | Explicit current-owner Inspector request with same availability | 010 |
| `export_workspace.rs:109`, review_wiring | Route to PCB, retain selection clearing, reveal its current Inspector authority | 006 |
| `export_workspace.rs:118`, review_case | Route to Case, reveal its current Inspector authority | 006 |
| `workspace_navigation.rs:42`, InspectorActions | Explicit shortcut; retain generation checks, disclosure and field focus | 006 |

Ordinary object/key/catalog picks never open Inspector. Do not remove selection/context
updates just to remove pinning. During M1, 006 keeps Export/shortcut requests working with
both migrated Layout and unmigrated targets; later adapters replace only their own route.

## Draft owners, not merely hidden DOM

Retain existing edit controllers above visibility and conditional-content switches; keep
closed content hidden/inert where feasible. Audit every draft-reset effect and keyed editor,
including Outline `use_outline_lifecycle`, Keymap's conditional Keys/Macros/Encoders branches
and Keycaps' board/matrix/key changes. Heavy canvas viewers need not all remain mounted.

Proposed retention lifetime: the current editor session, across close/reopen, Objects↔Inspect,
selection changes and workspace round trips. Retain unresolved text, errors and disclosure
state under existing identity: session + document + board/physical instance where applicable
+ entity/version/layer/input identity + editor kind and existing identity generation. Do not
key a catalog draft by a board selection or a half link by a display name. No local-storage
cache, document schema change or undo entry for UI visibility/focus.

Use typed draft ownership in each domain, not a global string-to-value bag. Reuse settlement
and resolver rules; pending edits retain their captured owner and are not replayed on remount.
Visibility switching must not manufacture a commit or reset. Existing real blur/explicit
triggers keep their semantics: exactly one admitted commit to the captured owner, or (when
that trigger has not happened) its full draft remains retained. See the per-path table in
[refactoring.md](../../toolbar-led-workbench/refactoring.md); no extra visibility/focus Undo or text rebound to the next selection. Deleted/recreated inputs, removed versions and expired sessions
invalidate admission; a retained draft cannot resurrect them. Preserve unrelated valid drafts
and show the existing unavailable/stale-target explanation. Validate this timing separately
from A→B→A restoration. No wider test-only interface without approval.

Retention also preserves the existing `DraftOwnerRegistry` admission guard and synchronous
unresolved predicate. A cache of text is not enough if unmounting drops the guard and makes
Update Now miss unresolved work. Keep one live domain owner/registration, not a second
registry. Prove hide/workspace-switch deferral and normal retirement on session replacement.
Retain the signals read by the guard's predicate as well as the guard itself; a dropped
component scope cannot supply a live predicate. M1 uses real registered Layout owners in
005. M2 must classify currently unregistered forms before choosing update-safety treatment;
absence of a registration does not establish an explicit-submit draft. That human gate is
not permission for silent reload loss. [Commit-trigger rules](../../toolbar-led-workbench/refactoring.md#commit-triggers-need-a-decision-not-an-accidental-refactor)
preserve existing field semantics rather than the earlier ambiguous blanket no-revision claim.

## Responsive and keyboard contract (proposed)

- **42px exactly**, one row at 320, 390, 728 and desktop CSS widths, including coarse pointer.
  No wrapping, second permanent action row, clipped utility or page-level horizontal scroll.
- Keep all five Layout section triggers and Objects/Inspect visible in fixed trigger banks.
  Icon hit regions at least 32×32px, separated by at least 4px with no overlap; compact
  captions disappear before action labels. Reserve room for at least one complete 32px
  action in the open section at 320px; the remaining actions scroll horizontally inside it.
  Other workspace trigger banks only contain applicable sections. Other touch controls keep
  the existing 44px floor; 42px strip approval does not waive it elsewhere.
- Native horizontal touch/trackpad scrolling for the action lane; focusing an offscreen action
  scrolls it into view without moving the trigger banks or page. Never intercept canvas pan
  or numeric-field arrow keys. At 200% zoom/long names measure again rather than wrap.
- Measure the final post-toggle layout with actual Theme/font/content before movement.
  Probe is noninteractive/aria-hidden. Font readiness, resize and content changes remeasure;
  cancel stale transitions, then use one stable label allocation for the next movement.
  Approved 2026-11-13: width-band density replaces this measurement once
  [002 Step 3](../../toolbar-led-workbench/issues/002-toolbar-accordion-label-fitting.md#2026-11-12--revised-step-3-design-replace-the-toolbar-fitter)
  passes its gates; until then this clause stands.
- Toolbar has an accessible name, one roving tab stop, Left/Right and Home/End across enabled
  visible controls (including action-lane controls); Enter/Space activate. No hidden probe or
  closed content in the focus order. Detailed references/settings use secondary surfaces,
  not text inputs fighting the toolbar's arrow navigation.
- Objects: focus search on open; Tab reaches grouping/disclosure/results; arrows navigate
  results, Enter/Space pick; modifiers retain current multi-selection intentions. A pick
  closes the popover. Historical component-arrow nudge remains available on the focused
  canvas selection after dismissal; no silent loss of keyboard movement.
- Inspect: focus heading/first appropriate control, no focus trap. Tab follows normal field
  order, then leaves to canvas/footer; hidden panel content is inert. Workspace toolbar →
  opened panel → focusable canvas → footer follows DOM order, not positive tabindex.
- Escape priority: nested picker/menu first, armed canvas pick/relationship return next,
  panel last. Each Escape does one thing. Close/toggle/Escape return focus to the utility;
  an outside pointer dismisses without stealing focus back from its valid target. If the
  opener disappears on a route change, use the new workspace toolbar or route heading.
- Objects width `min(320px, viewport−16px)`; Inspect `min(400px, viewport−16px)`. Anchor below
  its utility, flip/clamp horizontally to an 8px safe inset, bound height by the remaining
  visual viewport and scroll content vertically. Recompute on viewport/keyboard changes.
  No backdrop, body scroll lock, modal ARIA or canvas reframe. Close remains reachable at
  short heights; opening Inspect is not a desktop pin operation.

These values are proposed acceptance literals, not accessibility certification. If the 32px
spacing/42px constraint cannot meet the applicable floor in the real app, stop at 001/007
with evidence; do not enlarge the toolbar or waive a requirement silently.

## Motion and layering

Allocate the final density and clip before movement. Icon+label action units translate in as a
solid section under a stationary clip (280ms entry; close is immediate, approved 2026-11-13);
no icon/text relative drift,
opacity animation, fades, stagger or delayed pop-in. Apply the same no-opacity rule to
panel transitions. Reduced motion sets final state immediately with zero running
workbench toolbar/panel animations. This governs the new workbench composition; do not
change unrelated shared-menu/tooltip animations throughout the app under this ticket.

Only one transient tool-detail surface is active; opening it dismisses Objects/Inspect,
and a utility dismisses the detail popup. Larger Add/script flows close compact panels,
retain the existing controller/preview and get a bounded dedicated task surface, not a new
permanent rail or backdrop. Keep canvas pick overlays/toasts in their existing owners;
layer them without covering the toolbar or trapping the visible canvas.
