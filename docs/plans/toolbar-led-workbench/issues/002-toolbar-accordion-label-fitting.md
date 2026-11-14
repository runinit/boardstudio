# 002: Layout's measured 42px accordion and existing tools

Status: claimed
Type: build
Blocked by: [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Owner: Layout toolbar/transform/align modules and `web/assets/layout-toolbar.css`

## Boundary

Use 009's shared row/utility host, not new panel flags in LayoutWorkspaceState. This slice
owns Layout's `Option<ToolbarSection>` and Select/Transform/Align/Snap/View content. Reuse
LayoutSelectionKind/context conversion, LayoutContextActionToolbar activation/guards,
LayoutAlignMount, LayoutSnapSettings/Intent and existing visibility/view state. PCB may
reuse these mounts in 010; do not force them into other domains. Feed 009's single generic
fitting/clip implementation, not a second Layout engine. The coordinator serializes any
shared-host hunks; this ticket owns Layout action composition and its measured acceptance.

Measure the final post-toggle strip before motion; action labels outrank captions. Translate
icon+label units under a stationary clip, never opacity/stagger/pop-in; reduced motion is
instant. Follow [responsive/keyboard literals](../architecture.md#responsive-and-keyboard-contract-proposed).
Merge snap-in-dock/duplicate Footprints into one owner. View includes existing 2D/3D and
visibility/handles. Keep camera in footer, canvas manipulation rails/pick overlays intact.
Save/recovery alerts move to shell status in 006, not inside the bounded strip.

## Acceptance criteria

- [ ] All actions in the Layout tools inventory reachable with unchanged guards, reasons,
      reference selection (>12 combobox), Relationships and snap settings.
- [ ] One 42px row, all seven entry points visible with each section open; action overflow
      scrolls without page overflow, trigger movement or scope popups.
- [ ] No duplicate position, snap or footprint editor and no second permanent tool row.
- [ ] Existing tool assertions preserved; only uncovered interaction stories gain tests.

## Proof map

For the proposed fitter replacement, the [2026-11-12 Step 3 entry](#2026-11-12--revised-step-3-design-replace-the-toolbar-fitter)
refines bounded fitting/motion into rows D/L/F/K/M/R/O/B and maps retired assertions
and mutations. Those rows remain planned; approving this design and completing its
gates precede retirement. Availability and action parity below remain required.
Historical measured-fitting receipts do not establish replacement acceptance.

- Bounded fitting — 320/390/728/desktop, real built Theme/toolbar stylesheet and font loaded,
  actual Layout action/section controls mounted → open each section/resize →
  height 42px, seven entry points hit-testable, one full action available, action labels
  allocated before captions and before movement · mounted · planned
- Solid motion — labelled section moving, then rapid switch → sample initial/middle/final
  frames → constant icon/text offset and opacity 1, stationary clip, no stale transition · mounted · planned
- Reduced motion — preference reduce before open → toggle section/detail → final state
  immediate and zero running workbench animations · mounted · planned
- Availability — existing first-column / Follow-rotation-off / locked / empty fixtures →
  attempt tool activation → unchanged disabled reason and accepted state; legal tools still work · mounted · planned
- Action parity — valid selection with >12 references → Align, change reference, open Snap
  details and toggle View → named action/result matches existing behaviour, no duplicate entry · mounted · planned

## Checks and mutations

[Verification contract](../verification.md): `check.py quick`, `check.py lint`,
`check.py browser --only layout`; page tests selected with `run-wasm-tests.py --files` when
page wiring changes. Motion/measurement/reduced-motion/guard proof-gap mutations are named
there. Run one combined Spec+Standards review and affected gate before closure.

## Implementation progress

### 2026-11-12 — Revised Step 3 design: replace the toolbar fitter

**Design only; implementation not started.** Inspected `c418bb966` on
`cleanup/remove-dead-weight`, including CSS steps 1–2 at `656ab6011`. No new
runtime, RED/GREEN, mutation or browser evidence is claimed here. This entry
revises the earlier Step 3 proposal, not the historical receipts below.

#### Decision summary

Prefer a named inline-size query container on `.workbench-tool-row`, with CSS
density rules for its descendants. Remove clone/probe allocation, fitting
observers/font callbacks, `fit_key` and measured animation distance only after
the replacement passes the gates below. Keep retained lanes, native scrolling,
existing tools/guards, the shared Toolbar tab-stop observer, and domain state.
Production close-focus rescue belongs to Layout, not the synthetic shell section.

This is a deliberate change from greedy per-content fitting to predictable width
bands; it does **not** promise identical label counts at every old width. Numeric
density breakpoints are candidates, not measured safe limits. The proposed
entry-only motion also needs explicit approval: the parent architecture still
describes measured fitting and 280ms/170ms entry/exit. Before implementation, the
coordinator must record approval of this replacement contract and reconcile those
clauses and 009's probe obligations; this ticket alone does not silently waive them.
All surviving behavioral gates remain mandatory.

#### Behavior / density model

Adopted constraints carried from the existing contract: one 42px row, seven Layout
2D entry points, each hit region at least 32×32px with at least 4px separation, no
row/page horizontal overflow or wrapping, stationary triggers during lane scroll,
and at least one complete usable action in an enabled open section. Other touch
surfaces retain their existing 44px floor. All actions remain keyboard/pointer
reachable; no scope popups or duplicate editors. Guidance is not an enabled action.

Let **W** be the row's **content-box inline size in CSS pixels**, not viewport
width, border-box width, or lane width. Intervals below are inclusive at the lower
bound and exclusive at the upper bound. Adopt the following *density ordering*;
all five numeric boundaries **360 / 500 / 700 / 850 / 1200 remain provisional**
until the measurement gate accepts or revises them.

| State | W candidate | Headings and their chevrons | Captions | Utility labels |
|---|---:|---|---|---|
| Any section open | <850 | none | none | none |
| Any section open | 850≤W<1200 | open section only | none | full |
| Any section open | ≥1200 | all five | none | full |
| All sections closed | <360 | none | none | none |
| All sections closed | 360≤W<500 | none | none | full |
| All sections closed | 500≤W<700 | Select + Transform only | none | full |
| All sections closed | 700≤W<850 | all five | none | full |
| All sections closed | ≥850 | all five | captions that exist | full |

Closed captions have their **own candidate 850 threshold**, independent of the
500/700 heading bands. They never appear without their heading, nor when any
section is open. Chevron display follows the same predicate as its heading,
including closed partial-heading mode. There is no standalone chevron-only mode.
The Transform caption may be absent or name an active tool; do not budget it as
permanently empty. Caption removal precedes heading removal in closed density.

For marked inline icon+label actions: W≥850 shows every label; W<850 shows labels
only for `aria-pressed="true"` **or** `aria-checked="true"`, and always for
text-only actions. This is a visibility policy, not a promise that the whole
action fits the lane. Do not treat `data-state="on"` as a substitute for the
actual pressed/checked semantics; retain its existing visual styling where used.
Buttons without a meaningful action SVG must never turn blank. Per-size Snap
buttons, including unpressed `0.5 mm` / `0.1 mm`, remain text-bearing; Custom,
Reference, Handles, input values/units and other unmarked content are not swept
into a generic span-hiding rule. Existing `sr-only` labels stay screen-reader-only.

The 3D ToolRow branch has **no accordion sections**: apply the closed utility
policy, preserve its 2D/3D controls and Objects/Inspect, and do not invent five
missing triggers. Verify this branch separately from the seven-entry 2D oracle.

**Select invariant:** each shown label has intrinsic/auto button width sufficient
for its exact text, icon, gap and padding, without icon/text overlap or text
overflow *within the button*. Each compact action has a contained icon and a
target at least 32×32px. Both scope-trigger and toggle-group-item forms obey this.
Never leave a labelled Select button at fixed 32px. A pressed labelled button can
be wider than the lane; retain auto width and scroll access, never squeeze its
text into the icon. Report it as partially exposed, not fully visible. Such a
button cannot supply the “one complete usable action” proof; scroll to a fitting
enabled peer and prove actual activation. If a required control cannot be used,
or no complete action fits even with compact entry points, reject the candidate
layout rather than weaken that assertion. An oversized action needs readable
name access and a hit-testable activation region; complete text/button
containment in the lane is a separate, stricter measurement.

#### Component ownership and focus model

- `workbench_shell::ToolRow` owns row structure/density context, not Layout
  selection, section or popup state. Its fitting machinery may go; its shared
  `ui-shared/src/ui/toolbar.rs::ToolbarObserver` **must stay**. That observer repairs
  tab stops for retained inert content and is unrelated to width allocation.
- `layout_toolbar.rs::LayoutAccordionSection` owns production trigger/reveal,
  `inert`, `aria-hidden` and close-focus rescue. Use the actual section trigger
  reference: Align/Snap use `*-section-trigger`, distinct from detail triggers.
  Do not call the legacy Select restore helper that can select a scope action
  inside a now-inert lane.
- Implement the small rescue privately in Layout with mounted DOM references,
  previous-expanded state and a close-generation token. No new public shell
  component, cross-crate focus API or global section registry. The shell fixture's
  `ToolSection` may keep equivalent local behavior for its fixture, but its passing
  test is not evidence that production focus is wired.
- Do **not** rely on a post-render effect inspecting only `activeElement`: setting
  inert can already displace it to body. While expanded, capture focus ownership
  through focus-in/out events, retaining the last focused descendant for the
  impending close. A document focus-in to a valid outside element clears that
  claim; ordinary blur/window loss clears it rather than leaving stale ownership.
  If focus-out has no related target, resolve after the event/DOM turn: retain
  only when closure/inertness displaced the previously owned focus, not a
  voluntary departure while still open. Snapshot ownership before setting the
  section signal in local close paths; event tracking covers external signal
  changes too. Never read/mutate DOM during component render.
- The reconciliation effect explicitly depends on the mounted node and expansion.
  Read `open_menu()` inside the effect (and react to `menu` if captured as a prop),
  or use `use_reactive((&expanded,), ...)` for a computed bool; plain captured
  `expanded` is not reactive. Run only on true→false, not initial mount.
  Schedule the final check after the current pointer event/default focus and DOM
  update, with a generation check; do not focus synchronously on outside
  pointerdown. Rescue only if the old lane owned focus and it remains in that
  hidden lane or has fallen to body/no active element. Preserve any connected,
  visible, non-inert outside destination, including a newly opened section,
  Objects/Inspect or canvas. Reopening, a newer focus event, unmount or route
  change cancels the pending rescue. Disconnect listeners/cancel pending work on
  drop; route owners, not removed sections, choose a new-workspace destination.
- Keep nested popup dismissal in its existing owner. Align's local `detail_menu`
  must be cleared when its parent section closes (subscribe to parent
  `open_menu`), without restoring focus to the hidden Reference trigger or
  reopening on section return. Verify portalled choices, if present, as belonging
  to that popup; do not classify them as unrelated outside focus.

Required focused cases: close with a real lane action focused; focus already
displaced by inert; switch to another section; outside pointer selects a valid
focus destination; close then reopen before the deferred check; unmount/3D route.
Keep one roving stop, closed descendants excluded, and real arrows/Home/End plus
offscreen focus scrolling. Synthetic `.focus()` alone does not prove scrolling.

#### CSS / container / selector strategy

Use `container-name: workbench-tools; container-type: inline-size` on the row,
with a definite parent-assigned width and `min-width:0` along the flex chain.
Use `@container workbench-tools (...)`, not the nearest anonymous container;
avoid accidental capture by nested Select/Popover containers. Named containment
does not by itself guarantee overlay safety.

A query container cannot query-style its **own** padding/gap. Keep row padding
and gap fixed outside the queries, and query-style only descendants. The current
row has horizontal padding 2px, gap 3px, and the Layout child has gap 4px; record
both rather than subtract a fictional single gap. The real seven-entry bank is
inside `.layout-toolbar-accordion`. If acceptance needs different spacing, set
a fixed row/child rule or explicitly propose an outer container; do not write a
self-query that silently resolves against some ancestor. Preserve at least the
required 4px separation in the actual entry geometry, not just a CSS token.

Closed groups stay intrinsic-sized/non-shrinking only if the density budget fits.
An open group and reveal shrink with `min-width:0`; its trigger does not shrink.
Maintain a stationary reveal-inner clip and the native overflow-x action track.
Do not “solve” entry overflow with row clipping, `overflow:visible`, or shrinking
trigger hit regions. Check closed reveal descendants cannot paint, hit or take
focus despite retained nonzero intrinsic widths.

Replace, rather than layer over, the old `data-fit-headers`,
`data-closed-hide`, `data-action-labels`, `data-show-label` and
`data-utility-labels` selectors. Scope action rules to
`[data-workbench-action] > .workbench-action-label` (verified markup), exclude
popup descendants, and identify meaningful SVGs in the action itself; a nested
popup SVG is not an icon for its trigger. Use the same predicates for Select's
auto-width and label display. `@container` contributes **no specificity**:
explicitly replace the old fixed-width Select override with scoped equal-or-higher
specificity rules in the correct cascade order. Read computed width, padding,
display and bounds for pressed, checked, unchecked and text-only cases. Do not
use `!important` to hide an unresolved cascade conflict or change legacy PCB
non-accordion styling. Keep state/class styling separate from density.

#### Motion model

Propose **entry-only solid translation**, 280ms with the existing easing; close
immediately collapses to 0fr and becomes inert. Remove the ineffective closed
170ms track animation, not pretend it supplies visible exit motion. This is an
intentional contract revision requiring the approval noted above.

`translateX(100%)` refers to the track's transform reference box, **not**
`track.scrollWidth`. Set/verify `transform-box:border-box`; translate by one
allocated track-box width from the right to its rest position. Final clip and
density must already be allocated in the first animated frame. The scrollable
track's own overflow clip moves with it, so prove that earlier scrolled content
does not leak into the stationary outer clip at nonzero `scrollLeft`. Retain the
scroll offset on close/reopen unless native focus scroll adjusts it; do not reset
scroll to make a motion test pass. No promise that all overflow content travels
its full scroll extent.

Icon/text units keep their relative offset; enabled units keep effective opacity
1, disabled units keep their pre-existing constant disabled opacity (no fade,
stagger or label delay). Clip bounds stay fixed during an open segment with no
resize; a resize may allocate a new final clip immediately, not animate its size.
Rapid A→B→A cancels prior entry, immediately clips/inerts B on close, and ends
with only A open and zero stale animations. Test actual intermediate translation,
not only endpoints. Settled `transform` must be `none`, not a retained identity
transform; avoid `will-change:transform` that keeps a containing block alive.

A non-none transform changes fixed-descendant positioning. When a lane overlay
opens during entry, finish/cancel that lane's animation immediately and remove
the transform before displaying the overlay; do not delay access until 280ms.
Prefer a CSS rule keyed to the existing open overlay state on the containing
section; prove portal/state reach before selecting its exact selector. If that
cannot reliably remove the transform in the same rendering turn, stop this
motion slice for a different non-transform/overlay-host design, rather than
silently portal/reparent controls. Reduced motion disables all workbench row and
panel animations/transitions (including chevrons) from the first state change;
observe zero running animations, not “none left after waiting”.

#### Overlay model

Keep the step 1–2 anchor direction, subject to measured containment on the actual
supported browser/version set. Test named container on/off and track transform
on/off separately: neither “CQ always clips fixed popups” nor “fixed always
escapes” is an acceptable assumption. Current references:
[container queries](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_containment/Container_queries),
[transform](https://developer.mozilla.org/en-US/docs/Web/CSS/transform) and
[translateX](https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/translateX).
Context7's MDN containment summaries differ in detail; browser evidence must
resolve the effect of inline-size containment here. Dioxus
[effect dependencies](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/hooks/docs/side_effects.md)
likewise require explicit non-signal dependencies, not a guessed pre-inert effect.

| Surface / owner | Required observations |
|---|---|
| Tooltips, including Transform guidance and Handles | Hover and real keyboard focus expose exact full text; stable trigger/scroll offset, popup bounds and hit target outside lane clip; no clipped or ghost tooltip after section close |
| Layers / View | `#m1-layers-trigger` and fixed `#m1-layers-list`: visible choices, viewport-safe placement, scrolling, no page overflow; dismiss/hide on parent close, no stale interactive list |
| Alignment Reference / `LayoutCommandPopup` | Its independent `detail_menu`, dialog and Close stay usable; parent section closes detail; Escape restores to Reference only while its lane is open |
| Nested reference Select (≤12) / Combobox (>12) | Open from Reference, choose a literal reference (filter + ArrowDown + Enter for Combobox), confirm accepted reference/result; first Escape closes the picker only, next closes detail, with correct focus and no ghost options |

For **each surface**, record initial opening, opening during entry, opening after
nonzero lane scroll, scrolling while open, resize across a density boundary,
section close/switch and reopen. Measure visual-viewport containment, ancestor
clippers, trigger anchoring and `elementFromPoint` hit testing; test pointer and
keyboard selection, not just an existing DOM node or `display:block`.

Layers currently has `left:anchor(left)` and no explicit horizontal flip/clamp
in its local rule; the reported right-edge defect is a **baseline candidate**,
not a newly measured Step 3 regression. Reproduce at `c418bb966` with a separate
baseline checkout if needed, then compare identical steps on the candidate.
Record pre-existing defects separately from CQ/transform regressions. An existing
defect that violates a mandatory popup/access criterion still blocks acceptance
or needs a user-approved separate disposition; it cannot be grandfathered into
GREEN. No unrelated popup/primitive rewrite is authorized by this design.

#### Implementation slices in order

Each slice is independently reversible; stop and report at its exit and obtain
explicit permission for the next phase. Do not delete the old path before the
replacement proves its responsibilities. Two unchanged-result attempts trigger
the issue-tracker stop rule, not an expanding test/refactor programme.

1. **Baseline and oracle first.** Entry: approved revised contract, current diff
   inventory, source-matched package. Extend existing observations, not a new
   testing framework. Record old density, real caption strings, focus timing and
   overlay defects; replace the attribute-gated expectation without removing its
   checks. Exit: observed RED for unmet new requirements, or baseline GREEN
   marked proof gap with the exact mutation below; no invented failures.
2. **Separate focus ownership.** Entry: failing/proof-gap production close-focus
   case. Add private Layout rescue and parent/detail close handling; remove only
   fitter focus rescue once production tests carry it. Exit: close/displacement,
   outside-destination and rapid reopen cases pass; shared roving observer intact.
   Density still uses the old fitter, so this slice can be reverted alone.
3. **Density pilot and Select sizing.** Entry: content-box baseline and independent
   oracle. Replace fitting-owned attributes with named CQ descendants and Select
   width rules as one coherent change; disable the old writer in the candidate
   rather than run two authorities. Keep the old implementation recoverable in
   Git until the pilot passes. Exit: accepted boundary ledger, all five sections
   including enabled Transform, closed states and 3D, constrained-parent/zoom/
   coarse-pointer geometry, and overlay container on/off results. Failure selects
   the fallback below or stops; it does not justify larger breakpoints by guess.
4. **Motion and overlay integration.** Entry: density settled, entry-only approval.
   Remove measured distance/exit animation, implement entry and overlay-open
   cancellation. Exit: sampled solid motion, stationary clip, nonzero scroll,
   rapid switching, fixed overlays and reduced-motion zero-animation gates pass.
5. **Retire fitter and integrate.** Entry: preceding evidence accepted. Delete
   clone/probe helpers, `ToolRowObservers`, font readiness callback, measuring
   CSS and `fit_key` producer/consumers (including 3D); retain helpers/imports
   used elsewhere. Inspect compiler inventory of removed field/type uses.
   Migrate the existing fitting test and named mutants, read the full diff, run
   affected gates and one fresh combined Spec+Standards review, then coordinator's
   integrated gate. Exit: no new mandatory proof gaps, explicit remaining
   follow-ups and approval for closure; no automatic next-phase implementation.

#### Test and proof-map migration

Expected presentation comes from this ticket's **accepted** W/state table and
literal action inventory, not from production density attributes or computed
display. In `select_label_report`, remove the
`data-action-labels=="full"` branch only in the same change that independently
computes expected label presence for **each** action from W and semantic state.
Use independently specified fixture identity for icon vs text-only actions;
a missing SVG must fail the icon oracle, not change the expectation to text-only.

Retain exact count five and literal Matrix/Row/Column/Key/Part names/accessible
labels. Check the **actual** DOM text independently of aria-label; expected shown
labels need positive text-range bounds, visible computed visibility/opacity,
icon/text containment in the button and no overlap. Expected hidden labels must
not paint or consume layout. Also check button target dimensions, all clipping
ancestors, row and visual viewport, effective disabled/inert state and center/
edge hit tests after scrolling into reach. Record separately: requested display,
intrinsic button contents fitting, and whole-control effective visibility.
`display != none`, `innerText`, or a density flag alone proves none of these.

The existing shell test
`tool_row_fits_final_labels_before_roving_past_closed_actions` remains the cheap
generic density/clip/roving story (rename only with an assertion-retention map).
Add SVGs to its icon+label actions so adaptive behavior is honest, **and keep a
distinct unpressed text-only action** that cannot go blank. It proves neither
Layout's five real sections/seven entry points nor production close-focus,
domain guards, Snap results or nested popups. Those stay in production Layout
mounted fixtures and `scripts/test-workbench-viewport.py` on the real packaged
page; preserve the per-size Snap parity correction in `c418bb966`.

| Retired assertion / mechanism | Surviving replacement (not an assertion deletion) |
|---|---|
| Wait for `data-action-labels=adaptive`; greedy allocation/pressed flag checks | Independent density expectations at both boundary sides; pressed **and checked**, unpressed SVG and text-only labels; actual text/bounds and Select width |
| Probe attach, aria-hidden/inert, cannot focus, removed after fit | Delete probe-only internals after no clone path remains; retain live closed-lane inert/aria-hidden, no paint/hit, no hidden Tab/arrow stops and focus rescue |
| Pre-toggle measurement / final-fit-before-motion mutation | First moving frame has final density and clip; boundary resize/content/font changes cannot strand entry points; no clone/measurement requirement |
| Measured `--workbench-tool-distance` / scrollWidth equality | Entry is one allocated track box, sampled solid motion and nonzero-scroll clipping; no claimed equal travel distance |
| Fit-key, MutationObserver, ResizeObserver and fonts-ready refit | Same W responds to semantic state/content and font swap; constrained parent resize responds without viewport resize; stale callbacks cannot act after unmount |
| Row 42px/no overflow, narrow native lane overflow, End→Inspect | Retain all; add complete-hit geometry and real offscreen keyboard scroll, not merely `scrollWidth > clientWidth` |
| Full-label mode viewport checks | Every expected label checked regardless of removed attribute; wrong text, hidden expected label and fixed-width labelled action each fail |

Step 3 proof rows below refine the original bounded-fitting/motion rows. All are
**planned**; earlier receipts are historical candidate evidence, not CQ GREEN.

| ID / requirement — precondition → action → independent oracle | Tier / status |
|---|---|
| D density — real CSS/fonts, worst captions, both sides of every band → close/open each section and resize parent at fixed viewport → exact heading/caption/chevron/utility policy, 42px, seven separated hit-testable entries, no row/page overflow, ≥1 whole enabled action in a valid fixture | mounted + packaged / planned |
| L label sizing — Select pressed and checked forms plus unpressed SVG/text-only actions → change state/width → literal text/presence, icon/label bounds, ≥32px target, no overlap; narrow oversized actions not counted as whole | mounted; real Snap/Select packaged / planned |
| F close focus — action focused before inert, outside destination or rapid reopen → close/switch/unmount → correct connected focus target, no stealing/no stale rescue, closed retained controls absent from Tab/arrows | production Layout mounted / planned |
| K scroll reach — initially clipped enabled action → trusted keyboard traversal then native horizontal touch/trackpad scroll → focus/action enters visible lane and activates; trigger positions and page scroll stable | packaged / planned |
| M motion — labelled enabled/disabled units, scrollLeft=0 then nonzero → open/rapid switch/resize → constant relative offset/opacity, intermediate translation, stationary clip per stable segment, no leaked closed content/stale animation | packaged / planned |
| R reduce — preference set before opening, observer installed before click → section and panel toggles → final state immediately, zero running row/panel animations/transitions | mounted + packaged / planned |
| O overlays — each named surface and nested picker, including during entry → scroll/resize/close/reopen/Escape → anchor/viewport/clip/hit and accepted reference, no invisible focus or stale open popup | production mounted + packaged / planned |
| B branch/content — 3D without sections, font swap, selection/caption change → resize/switch 2D↔3D → applicable controls reachable, one bounded row, no stale callbacks or new state-persistence behavior | page mounted + packaged / planned |

Original **Availability** and **Action parity** rows remain required, including
first-column/Follow-off/locked/empty guards, >12 reference choice and Snap/View
results. Existing no-selection guidance does not substitute for enabled Transform
or Align; exercise both valid and unavailable states without enabling a guard to
obtain a “usable action”.

Mutation migration is limited to these observable failure stories. Record real
RED before fixes; if baseline passes, mark GREEN + proof gap and kill the named
mutant through `scripts/mutation-check.py SPEC --locked --command "RUNNER"`.
Use one exact-source spec per mutation; restoration success is not a kill.
Do not remutate a row with sufficient RED evidence. The parent verification
document's 009 generic-fitting and 002 pre-toggle/probe mutations are retired
**only with** this replacement evidence and coordinator reconciliation.

| Row / precise proof-gap mutant | RUNNER / named failing oracle |
|---|---|
| D: show all closed headings/captions below their accepted band | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` for policy; packaged viewport for actual seven-entry geometry / expected density or entry containment |
| L: hide expected Matrix label; separately change its text or force its labelled width to 32px | `python3 scripts/test-workbench-viewport.py --page-url URL --width 900 --height 900` (fresh package, accepted full-label W) / SELECT-LABELS exact presence/text/containment, not an unrelated error |
| L: hide unpressed text-only Snap action | same viewport runner at `--width 320 --height 480` / exact Snap text and nonblank usable control |
| F: omit close rescue; separately force rescue over a valid outside destination | `python3 scripts/check.py browser --only layout` / displaced focus restored; outside focus unchanged |
| F/K: remove inert from a retained closed reveal | same Layout runner / closed action cannot take focus or enter real navigation; retain hidden-paint check in viewport runner |
| M: fade label; separately animate reveal clip instead of track | packaged viewport runner at a width with labelled units / constant opacity/unit offset; stationary clip |
| R: ignore reduced-motion rule | shell page runner + packaged viewport / zero animations observed from first update |
| O: remove parent-section detail dismissal | Layout runner / detail and nested choices closed, no stale focus/reopen |
| B/D: freeze width bucket (fallback only) or disable the candidate density query | shell page runner / parent-only resize crosses expected band |
| Existing guards: enable first-column Spread or Follow-off Rotate | `python3 scripts/check.py browser --only layout` / exact unavailable action/state assertion from original guard proof |

Use current runner arguments, not invented filters. The viewport runner currently
has page-url/width/height, not a row-width switch: extend its existing scenario
to constrain the row's parent at constant viewport and log achieved W. Set the
full-label mutant's width to an accepted full-label case if calibration changes
850; never choose a case where that mutant cannot fail. For packaged mutations
the command must rebuild fresh assets under the mutation runner's lock before
running the viewport probe; testing stale packaged assets is not a kill.

Required implementation gates remain `python3 scripts/check.py quick`,
`python3 scripts/check.py lint`, `python3 scripts/check.py browser --only layout`,
and `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs`;
include `web/src/presentation/layout_layers_tests.rs` when page/3D wiring changes.
Run the real viewport probe against fresh packaged assets for CSS, motion and
overlay claims. The coordinator alone runs the integrated `python3 scripts/check.py`
gate specified in verification.md, with shared-target build locking. Neither
focused tests nor this documentation task waive it. One fresh reviewer covers
Spec + Standards; replaced/retired oracles require delta review. Required
mutations are run only within the issue-tracker boundary.

#### Measurement / acceptance gates and fallback

The existing `.evidence/toolbar-rework/fit-matrix.jsonl` contains 35 rows at
reported row widths 304/374/544/712/884/1084/1424: closed, Select, Align, Snap,
View. **Transform-open is missing**, and its caption is null throughout. Those
fields do not establish content-box widths, boundary safety, hit testing or
effective caption visibility: a caption's own display can look visible under a
hidden heading. In particular 544 and 712 do not validate thresholds 500/700.
Do not promote this historical fitter matrix into new CSS acceptance.

Calibration procedure, before deleting the fitter:

1. Use actual production Layout markup and built styles with font readiness
   recorded, both Theme font stacks and supported browser versions. Log viewport,
   visualViewport, zoom/device scale, pointer mode, row border/client/content
   widths, padding/borders, row and inner-bank gaps, trigger/utility/lane rects,
   scroll extents and full captions. Compute W from untransformed content geometry
   (clientWidth minus inline padding for diagnostic comparison; preserve fractional
   precision from rect/computed box dimensions). No viewport-minus-16 assumption.
2. Parameterize closed plus **Select/Transform/Align/Snap/View** open. Closed
   worst cases include Matrix and Column, longest measured Transform tool name
   (include Stagger, not a guessed “longest” by character count), and Snap Custom,
   0.5 mm, 0.1 mm. Exercise their simultaneous wide combination, with real selected
   tool/reference state and the longest supported caption values. Include active
   scope/action changes at constant W and font fallback→loaded without a fit key.
3. At each candidate T, set parent width until measured W is T−1, T, T+1 CSS px;
   include fractional T−0.5/T+0.5 where zoom permits. Sweep the intervals and narrow
   supported endpoints, resizing in both directions. Measure required intrinsic
   trigger/utility sum + actual gaps + lane padding + a complete enabled action.
   For open wide-label bands also record horizontal action overflow; scrolling is
   allowed, clipping entry points is not. If a row does not fit, remove captions
   first or raise its density threshold, then repeat both sides and all affected
   states. Document chosen safety headroom for fonts/rounding, not an assumed one.
4. Repeat actual 320×480, 390×844, 728×1024 and desktop 1440×900, plus constrained
   parent at constant desktop viewport, coarse pointer, browser 200% zoom and
   short visual viewport. Browser zoom is not CSS transform scaling. Log achieved
   CSS row width; if 200% produces a width below the supported physical budget,
   report the conflict rather than claim the 320px check covers it.
5. Accept each numeric threshold only with a ledger of input/state, actual W,
   expected density, entry/label/action geometry and receipts on both sides.
   Every required surface must pass O; every enabled section must offer a complete
   usable action. Seven entry points must be effectively visible, not simply
   non-shrinking beyond an overflow-hidden row. Reuse parameterized cases rather
   than multiply every timing story by the entire width/theme matrix.

**Go:** named CQ passes content-box density, selector, focus, overlay and motion
gates on the supported targets. **No-go:** unsupported query/anchor behavior,
CQ-induced fixed-overlay clipping/anchoring regression, unstable intrinsic sizing,
or no bands satisfying entry/action geometry. After two unchanged-result attempts
stop with receipts and one different experiment; do not restore greedy probing.

If measurements reject CQ specifically, the bounded fallback is a **tiny
ResizeObserver in ToolRow** reading the row's content-box inline size and writing
one `data-tool-density` bucket only when that bucket changes. ToolRow is its
sole writer; CSS derives open/closed/pressed/checked presentation from existing
semantic state. No per-action probing, cloned DOM, font observer, mutation-driven
fitting, duplicated attributes or `fit_key`. Initialize conservatively, reconcile
on mount, disconnect on drop; prove transition/no-op writes and parent-only
resize. Remove CQ containment in this variant and rerun overlays. Width buckets
do not cure an impossible 32px/42px geometry budget or a transform bug; those
require a decision, not another observer. Media queries are permitted only if
viewport-to-row sizing becomes an explicit tested invariant; it is not one now.

#### Rejected alternatives

- Treating 500/700 as established safe limits, or showing every closed caption
  whenever headings fit: current evidence lacks worst captions and both sides.
- A shell-fixture-only focus effect, or unconditionally focusing the old opener:
  wrong production owner, inert timing loss, and outside-focus theft.
- Deleting density attributes while trusting computed display as the oracle:
  silently skips missing/wrong labels and clipped targets.
- Retaining per-action allocation, a second measurement engine, or two writers
  during rollout: defeats Step 3 and makes density authority ambiguous.
- Translating by 100% and calling it equal to scrollWidth; retaining an invisible
  exit animation; broad overflow-visible fixes; unproved global portal changes.
- Using only viewport breakpoints, iconifying text-only Snap, squeezing labelled
  Select to 32px, or testing only the two-section synthetic shell fixture.

#### Unresolved evidence / transfer

Implementation is **BLOCKED pending design approval and empirical calibration**,
not rejected in principle. Open gates: accepted numerical thresholds/caption
headroom; supported browser/version containment and anchor results; private focus
tracking timing; baseline Layers right-edge disposition; nested popup close and
motion cancellation; genuine native touch/trackpad and offscreen keyboard scroll;
200% zoom and constrained-parent budget. Entry-only close must be reconciled with
the parent motion contract before changing its oracle. No separate design document
is needed: this ticket holds the design, proof migration and gates within ownership.

### 2026-11-13 — Step 3 contract approved; Slice 1 baseline and oracle

**User approvals, 2026-11-13:** (1) the revised width-band contract, thresholds still
provisional; (2) entry-only 280ms motion with immediate close, reconciled in
[architecture](../architecture.md#motion-and-layering); (3) a control wider than its lane
is partially exposed, and the Handles check asks for reachable and hit instead of whole;
(4) the caption threshold is set by calibration, not fixed at 850.

Baseline taken at `e360ddc33` before approval; no production change, no gate waived. Fresh `build-web.py --fast` under the build lock
(`step3-baseline-build.log`, EXIT 0), HeadlessChrome 155, all logs in
`.evidence/toolbar-rework/`.

- **Row L, text-only Snap — RED.** New `text_only_action_report` in
  `scripts/test-workbench-viewport.py` fails `blank-Snap` at 320, 390 and 728: all
  eight per-size buttons have `display:none` labels, including the pressed `¼u`.
  1440 passes. Logs `step3-baseline-viewport-{320,390,728,1440}.log`. Observed RED,
  so no mutation is owed for this assertion.
- **Existing oracle RED at 320: `handles-View`.** Handles is 55.6px in a 42px lane
  and cannot be whole (two runs, `…-320.log`, `…-320-rerun.log`). `656ab6011`
  removed the `max-width:100%` ellipsis cap that made it fit. Resolved by approval 3:
  the check now accepts an oversized control that is enabled, 32px high, in the row
  and hit at its centre (`HANDLES-PARTIAL` in `step3-slice1-viewport-320.log`).
- **728 browser error was the probe's own.** The reduced-motion `MutationObserver`
  called `getComputedStyle(null)` on a mutation that lands before any section is
  open. It now records that mutation with no track and asserts zero animations on
  it too; `step3-slice1-viewport-728.log` has no browser error.
- **Content-box W** (`step3-baseline-density-*.json`): W = viewport − 20 in the
  standard layout (300 / 370 / 708 / 1420). Closed at 708 today shows all five
  headings plus captions `Key` and `¼u`; the candidate table removes captions there
  (<850). Lane width with a section open is 42px at W=300 and 112px at W=370.
- **Exit motion is already invisible.** 728 motion frames show every closed reveal
  clip at width 0 while its track keeps a translate; the 170ms exit never paints.
- **Layers.** At 728 and 1440 the list opens left-anchored, inside the viewport and
  hit-testable; no right-edge defect there. At 320/390 the trigger sits outside the
  scrolled lane and the measurement did not open it: still unmeasured.
- Two agent-browser daemon EOFs at `set viewport` (390, 1440) cleared after
  `close --all`; infrastructure, not product.

After approval (`step3-slice1-*` logs, same build):

- **Row L, Select labels — RED.** `select_label_report` now derives expected labels
  from W and pressed/checked state (`FULL_LABEL_W = 850`, provisional) and no longer
  reads `data-action-labels`; every earlier check is kept. 320 and 390: pressed `Key`
  label hidden. 728 (W=708): four unpressed labels shown below the band. 1440 passes.
- **Row F baseline GREEN, ad hoc.** With focus on a Select lane action, closing the
  section without moving focus lands on the Select trigger; switching to Snap lands
  on the Snap trigger; Escape in the lane does nothing. Same at 390 and 1440
  (`step3-slice1-measure-*.json`). This is a proof gap: the production mounted test
  and its omit-rescue mutation open Slice 2.
- **Layers right-edge defect confirmed at 390, pre-existing.** With the trigger
  scrolled into the lane, the list spans x 204–444 in a 390px viewport and its first
  choice is not hit-testable. Not caused by container queries or transforms; it
  violates a mandatory overlay criterion and needs a disposition before row O.
- **Intrinsic widths** (Source Sans 3 loaded, measured at W=1420 by forcing each
  form on the real triggers). Trigger: icon 34; heading 95.2 / 116.3 / 89.6 / 89.9 /
  88.2 (Select, Transform, Align, Snap, View); worst captions `Column` 140.5,
  `Stagger` 160.6, `Custom` 134.2. Utilities 74.4 + 73.3 labelled, 32 + 32 compact.
  Estimated minimum closed W (five triggers, four 4px bank gaps, two 3px row gaps,
  utilities): icons only 256; icons + utility labels 340; two headings 483; all
  headings 649; all headings + worst captions 783, or 699 with compact utilities.
  Candidates 360 / 500 / 700 hold with 20 / 17 / 51px to spare. Captions cannot
  show at 728 (W=708) while utility labels outrank them; 850 could drop to about
  800. These are sums, not both-sides measurements: Slice 3 verifies T−1/T/T+1.

Not done: both-sides threshold checks, Transform with a selection, overlay matrix
beyond Layers, zoom/coarse pointer, rows D/K/M/O/B oracles (written with their slices).

Slice 1 exit decisions (user, 2026-11-13):

- Slice 1 is committed as one change; Slice 2 (focus ownership in
  `LayoutAccordionSection`) may start.
- The Layers right-edge defect at 390 is fixed as its own small change before
  Slice 4, not accepted as a known defect. Row O stays blocked until it lands.
- Utility labels keep priority over captions. Captions therefore do not show at 728,
  and the caption threshold is calibrated near 800 in Slice 3 instead of 850.

### 2026-11-13 — Slice 2: close focus owned by the Layout section

`LayoutAccordionSection` now returns focus to its own section trigger when a close
strands it. The lane claims focus on focus-in and drops the claim on a focus-out
that happens while the lane is not yet inert (a voluntary departure). On open→closed
with a claim, a deferred check focuses the trigger only if the section is still
closed and focus sits in the hidden lane or on the body. `LayoutAlignToolbar` clears
its Reference detail when the Align section closes, by setting the signal directly so
no focus returns to the hidden Reference trigger. The fitter's own rescue is removed
from `fit_tool_row`; density still uses the old fitter. `ToolbarObserver` is untouched.

Deviation from the focus model: no close-generation token. The deferred check
re-reads section state and the active element when it fires, and the task is dropped
with the component; the reopen mutation below shows that guard is live.

Row F evidence (`.evidence/toolbar-rework/step3-slice2-*`), runner
`python3 scripts/check.py browser --only layout`:

- `closing_the_align_section_dismisses_its_reference_detail`: observed RED before the
  change ("the detail closes with its section"), GREEN after. No mutation owed.
- `closing_a_section_returns_only_stranded_lane_focus_to_its_trigger`: its pre-change
  run was masked by the Align panic poisoning shared WASM state, so it counts as a
  proof gap closed by mutation. Four mutants, each killed on its own assertion:
  omit the rescue; rescue over a destination chosen after the close; rescue after a
  reopen; rescue focus the lane never held.
- GREEN: Layout browser 277 + 2 passed; shell `workbench_shell.rs` 6 passed with the
  fitter rescue removed; `check.py quick` and `check.py lint` passed.
- Packaged page at 1440 and 390, built without the fitter rescue: closing Select with
  focus in the lane lands on the Select trigger; switching to Snap lands on the Snap
  trigger.

Not covered here: the unmount/3D-route case and trusted-keyboard traversal (rows B
and K), and the "remove `inert` from a closed reveal" mutant, which the existing
`accordion_sections_keep_existing_tools_inline` selector assertion guards unmutated.

### 2026-11-13 — Slice 2 fix: stale rescue cancellation + unmount proof

Corrects `step3-slice2-review.md` blocks. `LayoutAccordionSection` now carries a
close-generation token (third `Cell` in the focus hook): any open/close transition
bumps it, the deferred task captures it and returns early when stale. A document
`focusin` listener bumps it on any valid outside focus (connected, non-body, outside
the lane), so a voluntary visit cancels pending rescue even after later blur to body.
Listener added once on mount, removed in `use_drop`; `spawn` (not `spawn_forever`)
keeps scope-drop cancellation for unmount/route. No public API widened; existing
inert-displaced/body rescue, outside-destination preservation and Align detail
clearing intact; `ToolbarObserver` untouched.

Row F evidence (`.evidence/toolbar-rework/step3-slice2-fix-*`), focused runner
`bash -c 'exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9; export CARGO_PROFILE_DEV_DEBUG=false; wasm-pack test --headless --chrome web/crates/layout --locked --lib -- FILTER'`:

- RED pre-fix: `closing_a_section_returns_only_stranded_lane_focus_to_its_trigger`
  fails at "a voluntary outside focus before the timer cancels the pending rescue
even after blur" (`step3-slice2-fix-red-closing.log` EXIT 1, left Button right Body);
  with that block temporarily removed, fails at "the first close's rescue does not
  act on a second close without ownership" (`step3-slice2-fix-red-closing-reopen.log`
  EXIT 1, left Button right Body). Existing four assertions preserved.
- GREEN post-fix: same test passes with both new blocks
  (`step3-slice2-fix-green-closing.log` EXIT 0, 1 passed); Align
  `closing_the_align_section_dismisses_its_reference_detail` passes pre/post
  (`step3-slice2-fix-red-align.log`, `step3-slice2-fix-green-align.log` EXIT 0);
  `accordion_sections_keep_existing_tools_inline` passes post
  (`step3-slice2-fix-green-accordion.log` EXIT 0).
- Unmount `unmounting_a_section_cancels_pending_rescue` (new conditional-pill fixture,
  unmounts via `when_inert` click before timer): GREEN pre/post
  (`step3-slice2-fix-red-unmount.log`, `step3-slice2-fix-green-unmount.log` EXIT 0,
  pill gone, body, no panic). Proof gap closed by owed mutation: replace `spawn` with
  `spawn_forever` in `LayoutAccordionSection` close effect, runner above with FILTER
  `unmounting_a_section_cancels_pending_rescue`, assertion "unmount cancels the
  pending rescue without stealing focus" must fail (panic or steal); checker runs.
- Gortex exact post-fix: `detect` HIGH 8 direct/25 transitive, 1 file
  (`layout_toolbar.rs`); `tests` 0/4 graph links (mounted tests exist, same as review);
  `guards` none; `contract` behavioral/warn/high (generic param-object advice, not a
  finding); `impact` HIGH 8 direct/28 transitive. Pre-fix `impact` twice `view_building`,
  later exact; all writes via Gortex, no native fallback.

Not covered here: trusted-keyboard traversal (row K), 3D density (row B/D), motion/
reduce/overlays (rows M/R/O), and the "remove `inert`" mutant still guarded unmutated
by `accordion_sections_keep_existing_tools_inline`. No full suites/quick/lint/mutations
run (coordinator checker follows).

### 2026-11-13 — Slice 2 unmount proof correction (BLOCKED)

The prior “proof gap closed by owed mutation” wording was premature. Checker evidence
`step3-slice2-check-mutations/results.json` records the qualified `spawn_forever`
mutation **survived** (the earlier bare-name compile failure does not count).
The stale-focus/reopen RED/GREEN evidence above remains valid and unchanged.

A materially different diagnostic setup now observes browser zero-delay timer registration
while the closed reveal remains connected, then requests production pill unmount in a
microtask before timer delivery. Baseline and the qualified root-scope mutant both pass
`unmounting_a_section_cancels_pending_rescue` (1 each); `step3-slice2-diagnose-mutations/results.json`
records **survived**, `restored_exact: true`. This establishes a stronger pending-timer
precondition, not task cancellation or a killed disposal oracle. Production retains
scope-owned `spawn`; no new test-only production API or visibility change.

Stop after two meaningful unchanged attempts. Unmount proof remains BLOCKED: no named
behavioral mutation has failed the required no-stale-focus/no-panic oracle. A detached
trigger cannot alone demonstrate cancellation; callback delivery/resumption and generation
invalidation are still unresolved. Next experiment: observe timer delivery and cancellation
alongside unmount completion, then distinguish early generation return from dropped-signal
access before choosing a disposal/route mutant with an external failure. Do not infer
behavioral equivalence from these two passes. Formatting corrected only in the owned file.
Focused final results and exact commands: `.evidence/toolbar-rework/step3-slice2-diagnose-checkpoint.md`.
No full gate, commits, stash, review delegation or slice closure in this diagnosis.

#### Approved trace experiment — pending-rescue proof recovered

The explicitly approved follow-on experiment resolves the unknown above. Temporary traces
show baseline `spawn` clears the timer before scope teardown/DOM removal. The
`spawn_forever` mutant instead delivers after completed production unmount, resumes with
generation unchanged (2 → 2), and panics when reading the dropped reveal signal. The old
test nevertheless reports PASS: a detached asynchronous browser failure did not fail its
test future. This is an oracle gap, not evidence of behavioral equivalence or a product fix.

Cheapest faithful proof stays in the existing mounted production-pill unmount scenario:
retain timer-registration, disappeared-trigger and body-focus assertions; observe browser
`error`/`unhandledrejection` events through the existing awaited close and require none.
Listeners are removed before assertions; errors are not suppressed. No internal task-policy
assertion, production API, visibility widening, or permanent production instrumentation.

Proof row F/unmount — focused lane + registered close timer → conditional production
unmount → no stale focus **and no asynchronous browser failure** · mounted · GREEN + proof
gap mutation killed. Same qualified `spawn_forever` mutation now fails
“unmount with pending rescue produces no asynchronous browser failure” with
`["RuntimeError: unreachable"]` versus `[]`. The harness's first-failure summary identifies
the antecedent dropped-signal panic; the raw log also records the named assertion.
`step3-slice2-trace-attempt2/results.json`: killed, exit 1, exact restoration true.
The instrumented first attempt remains recorded as survived, not retroactively a kill.

All temporary traces removed before the successful mutation. Existing stale-focus/reopen
RED/GREEN retained; focused toolbar tests 19 passed. No new production behavior change.
Full commands, ordering traces, final formatting result and remaining gates:
`.evidence/toolbar-rework/step3-slice2-trace-checkpoint.md`.
This resolves the focused mutation gap, not whole-slice/ticket acceptance: coordinator
review/integrated gates and the remaining packaged/route acceptance stay outstanding.

### 2026-11-14 — Slice 3 density pilot (Attempt 1 GREEN, exit incomplete)

Base `4825f44b0` clean. Replaces clone/probe greedy fitting authority and fitting-owned attributes with named inline-size container queries on `.workbench-tool-row` (`container-name: workbench-tools`), per transfer density table. `ToolRow` drops `fit_key` (callers: `layout_workspace.rs::toolbar` 2D/3D + shell fixture, via text/Gortex inventory; native `cargo check` blocked by pre-existing ui-shared import error, wasm compilation served as compiler inventory); `fit_tool_row` keeps only measured `--workbench-tool-distance` from live tracks (no probe, no attributes), preserving 280/170ms motion mechanism for Slice 4 (no entry-only, no 100% fallback). `ToolbarObserver` (via `Toolbar`), tools/guards, inert/roving untouched; `layout_toolbar.rs` untouched; no Layers fix. Dead fitting helpers (`fit_open/closed_labels`, `track_fits`, `copy_attribute`, `RemoveAttribute`) deleted with the pilot (coordinator small fix; Gortex refactor delete rejected read-only twice after the 30s retry, native-edit fallback per AGENTS). Generic CQ rules live in owned `layout-toolbar.css` (shell.css not owned; higher specificity beats shell base/open-hide; note as follow-up to relocate). Select width via pressed/checked + CQ 850 (replaces `data-action-labels`/`data-show-label`).

Shell test migrated with map (same name): probe/adaptive waits removed; SVGs added to 5 icon+label actions (pressed First, checked Second via `aria-checked`, 3 unpressed), distinct unpressed text-only Sixth (never blank, text+bounds checked); narrow (520px/W516) pressed/checked/text-only shown + unpressed/hidden + headings/utilities hidden + beta inert/hidden; wide (900px/W896) all shown + open heading/utilities shown + closed heading hidden; 320px lane overflow; 42px/row-overflow/End→Inspect retained. Fixture adds `layout-toolbar.css` source include (bundle stale until rebuild).

Focused GREEN under lock: shell `run-wasm-tests.py --files workbench_shell.rs` 6/0 (`step3-slice3-shell-attempt1.log`); layout page `layout_layers_tests.rs` 19/0 (`step3-slice3-layout-page.log`); fresh `build-web.py --fast` EXIT 0 (`step3-slice3-build.log`, CQ verified site-subpath); packaged viewport 390/1440/819/820 EXIT 0, 0 failures each (`step3-slice3-viewport-{390,1440,819,820}.log`): 390 SELECT-LABELS W370 pressed Key auto56.7 shown + 4 unpressed 32px hidden + Snap text-only 8/8 shown (resolves Slice1 blank-Snap RED); 1440 W1420 all 5 auto shown; 820 W800 full acceptance with captions shown. Caption threshold 800 calibrated both sides: 819/W799 captions `Key`/`¼u` display none (tw 95.3/89.9), 820/W800 display block (tw 119.3/111.2), 821/W801 same shown; full viewport PASS at 819+820 (seven entries, 42px, no overflow). Gortex: impact ToolRow/fit MEDIUM lower-bound; detect 3 files/16 symbols MEDIUM; tests 0/3 graph links (wasm/viewport exist); guards none; contract behavioral/warn/high (generic).

Exit incomplete (no closure): thresholds 360/500/700/850/1200 remain provisional (Slice1 sums only); missing enabled-Transform selection, 3D branch, constrained-parent, 200% zoom, coarse pointer, overlay container on/off matrix, motion frames/reduced-motion gates; proof rows D/L/F/K/M/R/O/B stay planned; mutations owed to checker (D show-all-below-band, L hide-Matrix/32px + hide-text-only-Snap at 320, B/D disable-query via shell runner; F/K/M/R/O per Slice 4). Follow-ups: relocate generic CQ to shell.css when owned (or request ownership); Slice 5 deletes `ToolRowObservers`/fonts + measuring CSS + renames `fit_tool_row`.

Review disposition (fresh Astra high, `.evidence/toolbar-rework/step3-slice3-review.md`): Standards + Spec CHANGES REQUIRED, 2 blockers. B1 fractional-width endpoint gaps FIXED by coordinator: integer `max-width` endpoints replaced with half-open ranges (`width < 850px`, `850px <= width < 1200px`, `width < 500px`, `500px <= width < 700px`); shell story gains a fractional-W849.5 step — GREEN `step3-slice3-b1-green.log` (6/0), RED at that exact assertion with the old endpoint temporarily restored (`step3-slice3-b1-red.log`, exact restore), Clippy + `browser --only layout` green (`step3-slice3-b1-*.log`). B2 (exit ledger + D/L/B replacement-proof mutations, incl. closed-state shell oracle) remains open: it blocks Slice 3 acceptance, not this pilot commit; the density-evidence completion follows as its own delegated step. CSS relocation, dormant shell selectors and future popup-marker scoping stay non-blocking follow-ups; no observer-loop or text-only/popup regression established.

### 2026-11-14 — Slice 3 B2 density exit evidence (Checker/Builder)

Base `bbd5d0794` clean. Tests/evidence/ticket only: `web/src/presentation/workbench_shell.rs`
(+244, tests only — closed Alpha, open 849/850/1199/1199.5/1200 sides, 13-width closed table with
asserted content-box W), `scripts/test-workbench-viewport.py` (+248/−2, scenario hooks only:
`--setup-matrix`, `--scenario constrained|assembly3d`, closed-density/entry/overflow/error
helpers; default page path behavior unchanged), this entry,
`.evidence/toolbar-rework/step3-slice3-b2-*`. No production CSS/Rust changes, no Slice 4,
no Layers fix, no commits/stash. Packaged site rebuilt fresh (the pre-existing site carried
stale pre-B1 849px CSS); mutant builds restored exactly after each kill.

This supersedes the pilot entry's "thresholds remain provisional": 360/500/700/850/1200 are
ACCEPTED with the ledger below (each threshold: evidence log + both-side result with actual
content-box W). Caption 800 stays accepted on the reused 819/820/821 receipts (not rerun);
the new shell closed table corroborates caption hidden below 800 / shown with exact
"Current" text at 849+.

Accepted ledger (every run FAILURES 0; W = content-box inline size):
| Threshold | Shell parent-resize (W asserted ±0.05) | Packaged real five-section geometry |
|---|---|---|
| 360 utilities | 359 hidden / 360 shown | 379→W359 hidden / 380→W360 shown (full page runs) |
| 500 closed headings | 499 hidden / 500 shown* / 499.5 hidden | 519→W499 hidden / 520→W500 Select+Transform-only / constrained W499.5 hidden |
| 700 closed headings | 699 shown* / 700 shown / 699.5 shown* | 719→W699 partial / 720→W700 all-five / constrained W699.5 partial |
| 850 labels + open headings | open 849 folded+hidden / 850 shown (plus B1 849.5); closed 849/850 shown | constrained W849 folded / W850 shown, SELECT-LABELS both sides |
| 1200 open-wide | open 1199 open-only / 1199.5 open-only / 1200 all; closed shown all three | 1219→W1199 open-only / 1220→W1200 all-five / constrained W1199.5 (closed) |
| 800 captions | hidden below / shown 849+ | reused 819/820/821; real Key/¼u captions in constrained logs |

\*Two-section fixture cannot show the 500–700 partial band (alpha/beta are not
align/snap/view); the partial-vs-all contrast is proven packaged.

Packaged receipts: shell `step3-slice3-b2-shell-green.log` (6/0); page
`step3-slice3-b2-viewport-{379,380,519,520,719,720x844,1219x900,1220x900}.log`
(W359/360/499/500/699/700/1199/1200); constrained
`step3-slice3-b2-viewport-constrained.log` (viewport fixed 1440,
W499.5/699.5/849/850/1199.5 exact); enabled Transform
`step3-slice3-b2-viewport-matrix-1440.log` (hasGuidance false, usable 77.7×32 hit;
Align truthfully guard-disabled with this selection); 3D
`step3-slice3-b2-viewport-3d-{1440,320}.log` (W1420 labels shown / W300 hidden, 42px,
reachable controls); zoom `step3-slice3-b2-viewport-720x450.log` (W700 — the exact CSS
geometry of 200% browser zoom on 1440×900; chrome zoom itself is not automatable here:
Ctrl+Plus keypress unchanged, CSS-zoom probe scales rects, both documented in the B2
checkpoint). Final `check.py browser --only layout` EXIT 0, 278 passed
(`step3-slice3-b2-layout.log`); scoped rustfmt clean. Daemon EOFs between runs recovered
via `close --all` preamble; infra only, not product RED.

Mutations (all through `scripts/mutation-check.py --locked`, all `restored_exact: true`):
| Mutant | Oracle | Result |
|---|---|---|
| D show-all-closed-below-band (closed headings flex + captions inline) | shell closed table | KILLED at "closed density settles at W359" (`step3-slice3-b2-mut-D/results.json`) |
| L hide-text-only (drop `:has(> svg)`) | packaged TEXT-ONLY narrow | KILLED at blank-Snap, 7 actions, at 380 (`step3-slice3-b2-mut-L1-380/results.json`; 320 blocked, see below) |
| L labelled-Select-at-32px (full-band auto→32px) | packaged SELECT-LABELS at 900/W880 | KILLED at select-labels-Select overflow; pressed Key kept auto width (`step3-slice3-b2-mut-L2/results.json`) |
| B/D disable container query (`inline-size`→`normal`) | shell density story | KILLED at the initial narrow density oracle, not the resize step (`step3-slice3-b2-mut-BD/results.json`) |
| B/D neuter open-medium band | shell parent-resize | KILLED at post-resize "wide open shows its heading" |

Slice 3 exit rows — met vs still open:
- MET: closed/open density ledger all bands both sides + fractionals; enabled Transform
  with a selection; 3D closed-utility both sides; constrained-parent response including
  fractionals; SELECT-LABELS exact text/containment both sides of 850; text-only nonblank
  narrow; the D/L/B replacement-proof kills above.
- STILL OPEN: (1) 320 full-flow baseline — the pre-existing keyboard-scroll oracle demands
  full visibility of pressed-wide Geometry snap (113.5px label in a 42px lane),
  contradicting the approved partially-exposed policy for pressed labelled buttons; 2
  consistent REDs (`step3-slice3-b2-viewport-320-blocker.log`, `step3-slice3-b2-mut-L1/_baseline.log`)
  need coordinator disposition (blocks only L-1-at-320; the text-only oracle is proven
  killing at 380). (2) Coarse-pointer live receipt — no harness emulation after two
  attempts (`set device`, external Chrome `--touch-events`); the complete coarse-rule
  inventory proves no coarse rule reaches the ToolRow (static composition places the
  ToolRow outside the legacy `.layout-toolbar-strips`); needs disposition or the named CDP-mobile /
  real-device experiment. (3) B font-swap and selection/caption-change sub-proofs; F/K/M/R/O
  (Slice 4); Availability/Action parity; overlay container on/off matrix — untouched by B2,
  unchanged from the pilot.

Coordinator dispositions (2026-11-14): (1) 320 keyboard-scroll oracle — the full-visibility
 demand is superseded by the approved oversized-control rule (pressed label shown, control
 partially exposed, enabled/32px/in-row/reachable/hit-at-centre); the oracle rewrite lands
 with the Slice 4 keyboard-focus-scrolling proof, and no Slice 3 claim rests on the 320
 full-flow run. L-1's text-only kill at 380 stands as the Slice 3 proof. (2) Coarse pointer —
 accepted as static-bound for Slice 3 density (density does not vary by pointer and no coarse
 rule reaches the ToolRow); the live coarse geometry receipt moves to Slice 4 via CDP mobile
 emulation or a real touch device. Coordinator independently reran the D mutation:
 `step3-slice3-b2-coordinator-mutD/results.json` — killed at the same closed oracle,
 restored_exact true. Delta review (`.evidence/toolbar-rework/step3-slice3-b2-review.md`)
 found one blocker — open W1199.5 was claimed but only closed W1199.5 tested — fixed by
 coordinator: shell story gains the open W1199.5 step (GREEN
 `step3-slice3-b2-review-fix-green.log`); the fractional-endpoint regression mutant
 (medium band back to `max-width: 1199px` in both fixture stylesheet copies —
 layout-toolbar.css source and the generated tailwind.css bundle) is KILLED at that step
 (`step3-slice3-b2-review-fix-mut2/results.json`, restored_exact true; the single-copy
 mutant survived because the unmutated bundle copy still matched,
 `step3-slice3-b2-review-fix-mut/`). Ledger row 1200 attribution corrected accordingly.

### 2026-11-14 — Layers 390 right-edge fix (Slice 1 exit decision)

Builder (002-layers390): the Slice 1 exit defect — `#m1-layers-list` spans past the right edge at 390 with its first choice outside the viewport. New packaged oracle `layers_list_report` + shared `check_layers_open` in `scripts/test-workbench-viewport.py` (View open, trigger scrolled into lane, programmatic open, settled + tooltips closed): list fully inside with >=8px side margins, below trigger, first choice hit-testable at centre, no page horizontal overflow. Same oracle runs in the page flow and via `--scenario layers` (scoped fresh-page run following the constrained/assembly3d pattern); the scenario exists because the 320 full flow aborts earlier at the dispositioned Snap keyboard oracle (rewrite owed to Slice 4), so no 320 Layers claim could rest on it.

RED pre-fix (`step3-layers390-red-390.log` page flow EXIT 1; `step3-layers390-red-layers-{390,320}.log` EXIT 1): 390 list x212-452, first Hide Switches centre (413,147.5) outside, hit null; 320 list x199-439, first centre (400,147.5), hit null. 728/1440 pre-fix GREEN (settled): 728 list x298-538 hit true; 1440 x607.28-847.28 hit true. (Unsettled-fonts probes read shifted trigger positions — 728 x550, 320 open-shift; the harness settles fonts first, so oracle numbers are stable.)

Fix, owned-only `web/assets/layout-toolbar.css` Layers rule: `left: anchor(left)` becomes `left: clamp(8px, anchor(left), calc(100vw - 248px))` (248 = 240px list width + 8px margin; safe at every width — below 256px max-width caps the list and the 8px floor still holds). `position-try-fallbacks: flip-inline` (Context7 MDN anchor-positioning) was rejected: at 320 the trigger sits mid-lane (x199) where left-anchored overflows right and right-anchored underflows left, so no flip satisfies 8px margins. Gortex: impact LOW 0; detect MEDIUM (2 files, new oracle symbols + `#m1-layers-list`); tests 0/2 uncovered (packaged browser runs are the oracle); guards none; contract warn behavioral (reviewed, both callers mine).

GREEN post-fix: layers scenario `step3-layers390-green-layers-{320,390,728,1440}.log` EXIT 0 (320 list x72-312 first centre 273 hit true; 390 x142-382 centre 343 hit true; 728 x298-538 and 1440 x607.28-847.28 byte-identical to pre-fix, left-anchored preserved); page flow `step3-layers390-green-page-{390,728,1440}.log` EXIT 0 (0 failures, all existing oracles pass); 320 page flow still aborts at the dispositioned Snap assertion with byte-identical values, unchanged by this slice. `check.py browser --only layout` -> `step3-layers390-check-layout.log` EXIT 0 (278 passed). Mutation `layers390-bare-anchor-left` (revert to bare `left: anchor(left)`, rebuild + layers-390): KILLED at both named assertions (right edge 452, first-choice hit), `restored_exact: true` (`step3-layers390-mut/results.json`). No production Rust, no tailwind source edit (build regenerates the bundle copy), no commits. Row O unblocked; Slice 4 owns the overlay matrix. Coordinator independently reran the mutation (`step3-layers390-coordinator-mut/results.json`: killed, restored_exact) and the restored 390 scenario (`step3-layers390-coordinator-green390.log`: PASS).

### 2026-11-14 — Slice 4 motion + overlay integration (Builder)

Base `405e4fbe8` clean. Implements the approved entry-only 280ms motion, the K scroll-reach
proof with the owed 320 oracle rewrite, R section+panel proof, the O overlay matrix on
existing surfaces, the coarse-pointer CDP receipt and the B content sub-proofs. All Gortex
writes applied (no native fallback). Evidence `.evidence/toolbar-rework/step3-slice4-*`.

Motion mechanism (`web/assets/shell.css`, `web/src/presentation/workbench_shell.rs`):
measured `--workbench-tool-distance`/scrollWidth writing removed (`set_tool_distance` +
`query_all` deleted as dead; `fit_tool_row` is a stub kept for the Slice 5 observer
retirement); entry keyframes translate one allocated track box (`translateX(100%)` with
`transform-box: border-box`); the 170ms exit animation is gone (close is immediate).
DEVIATION from the design's preferred CSS overlay-cancel: a `:has()`-keyed
`animation: none` rule was implemented first and proven to RESTART the entry (translate
0→112px, running 1) on every overlay close — pinned RED by the new no-restart oracle
(`step3-slice4-overlays-390.log` first runs). Replaced with a `ToolRowOverlayCancel`
MutationObserver (own hook/effect, `data-state`/`aria-expanded` + childList): when a lane
overlay opens it `finish()`es that track's running `workbench-tool-enter` via reflected
WAAPI calls (ui-shared precedent, no new web-sys surface). Finish settles to the end
keyframe without re-arming: during-entry cancellation and no-restart both GREEN
under the sampled oracle (first-frame proof hardened in correction row B).
Shell test pins (same story): no distance var on either group, open track runs
`workbench-tool-enter`, closed track animation `none`. `ToolbarObserver`, density CQ,
focus rescue, measuring CSS untouched (Slice 5).

Rows (proof rows K/M/R/O/B per the Revised Step 3 design):
- M motion — packaged 728 (observed coverage; timing assertions hardened in correction
  row B): entry maxTx observed 450.0 against a 450 track box on the sampled run
  (upper-bound assertion only, not an exact one-box proof), intermediate translation,
  constant icon/text offset + opacity 1, stationary clip per segment, rapid
  Select→Snap→Select (<280ms, only Select open, settled `none`), nonzero-scroll entry
  (offset 200 retained, scrolled-out content never hit, settled clip — motion state
  during the scrolled entry unproven), resize endpoint settles clean (mid-entry timing
  unproven), close immediate (first closed frames w0/running0/none). RED on baseline:
  maxTx 747.0 (scrollWidth) and exit running 5.
- R reduce — packaged 728 (opening direction only; close direction added in correction
  row B): section + Objects-panel opening toggles under pre-set reduce show zero
  running row/panel animations from the first mutation (`REDUCED-MOTION`/
  `REDUCED-PANEL`). No mounted R test: the wasm harness cannot set the media.
- K scroll reach + 320 rewrite — the dispositioned full-visibility demand is replaced by
  the oversized-control rule (pressed label shown; enabled, 32px high, in-row,
  reachable, hit at centre; scroll into the lane as far as width allows). 320: arrows
  reach Geometry (scroll 0→436, labelW 77.5, center hit, bank + page stable), Enter
  toggles pressed true→false→restored with focus retained; wheel honestly UNPROVED.
  320 full page flow passes end-to-end (incl. Layers/Handles/guidance oracles).
- O overlays — new `--scenario overlays` (390 + 1440): Layers settled/during-entry/
  after-scroll/resize/reopen GREEN; during-entry cancellation GREEN on
  Layers/tooltips/guidance (running 1→0, none) with immediate placement+hit; no-restart
  GREEN; tooltips hover/keyboard(Tab)/scroll/resize/close GREEN; Reference mechanics
  GREEN (Close hit, Close/Escape→Reference, parent close→Snap, no reopen). O stays
  BLOCKED on the sub-proofs below (each reproduced on two widths where applicable;
  recorded REDs in `step3-slice4-overlays-{390,1440}.log`, reused without
  re-demonstration). Attribution is qualified per the Slice 4 review, not "all
  pre-existing": (1) Reference dialog clipped by its lane (390 x-44; nested trigger
  unhit; 1440 vertically clipped, canvas occludes) — O blocker; (2) nested choice +
  Escape ladder unproven packaged (picker disabled without references; Combobox choice
  stays mounted) — O proof blocker; (3) Layers list survives section switch (painted,
  inert) — O/M blocker with a concrete Slice 4 mechanism (the removed `forwards` exit
  changed fixed-descendant containment), regression attribution unresolved; (4) Layers
  scroll-follow escapes the clamp (computed left stays 142px, used x tracks scroll:
  x242 at scroll 0) — O blocker; (5) tooltips stick open (Layers-choice tip through
  2×Escape occluding the choice at 390; Handles focus tip stuck through real Tab at
  1440) — attribution unresolved, O blocker where a real close leaves a ghost;
  (6) View lane outside roving (arrows View→Objects→Inspect, 4 visible tab stops) —
  follow-up, uncoupled from scenario pass/fail unless it prevents a required O/K path.
  Escape leaves Layers expanded in both overlay logs, so no dismissal claim rests on it.
- Coarse pointer — live receipt via held CDP session (stdlib WS):
  `matchMedia('(pointer: coarse)')` true, footer coarse rule live (44px touch height),
  maxTouchPoints 1; row 42px + seven entries hit closed and Select/Snap/View open, no
overflow, no errors at 390 (`step3-slice4-coarse.log`). Proves live coarse activation
(media, rule, touch); the bounded-row geometry reconciliation lands in correction row F.
- B content — new `--scenario content` at fixed W800: monospace fallback swap and
  Key→Row scope change keep W800/42px/exact closed density/seven entries, caption
  becomes shown `Row`, no errors (`step3-slice4-content.log`).

Mutations (all `scripts/mutation-check.py --locked`, all `restored_exact: true`):
| Mutant | Oracle | Result |
|---|---|---|
| M fade-label (opacity in enter keyframes) | packaged 728 | KILLED at `opacity motion` |
| M animate-clip-instead-of-track (2% amplitude: full amplitude breaks clicks first) | packaged 728 | KILLED at `moving clip select x` |
| R ignore reduced motion (impossible media clause) | packaged 728 | KILLED at zero-animation assert (running 1, tx 450) |
| O remove parent-section detail dismissal | layout `closing_the_align_section_dismisses_its_reference_detail` | SURVIVED, unresolved named mutation (2 attempts): bounded redundancy hypothesis only — a Tab-out probe was reported to close the detail without a section change, but no retrievable evidence path was recorded, so the claim is dropped pending the row-E mounted experiment (signal/programmatic parent close with nested picker open); dismissal behavior stays GREEN mounted + packaged |
| F/K remove inert from retained closed reveal | layout `accordion_sections_keep_existing_tools_inline` | KILLED at the line-686 inert selector assert |

Gates: shell wasm 6/0; `check.py browser --only layout` 278+2 EXIT 0; scoped rustfmt
clean; wasm clippy `-D warnings` clean; packaged page flows 320/390/728/1440 EXIT 0
(728 via the mutation baseline on the final tree); fresh final build, site verified
clean of mutant residue. No drag-smoke/full gates, no commits, no stash (transfer).

Owned diff: 4 files including this ticket (script oracles/scenarios, shell.css motion
rules, workbench_shell.rs mechanism + test pins); no layout_toolbar.rs change (FK/O
mutants restored exactly); no tailwind hand-edit. O stays BLOCKED in this slice until
its required surfaces pass or the user changes the boundary — not rolled into Slice 5
silently. Remains for Slice 5/closure: retire observers/fonts/measuring CSS +
`fit_tool_row` stub; View-lane roving follow-up; Availability/Action parity; final
review + integrated gate.

### 2026-11-14 — Slice 4 correction pass, rows A–F (Builder)

Addresses the Slice 4 combined review (S4-1–S4-7, `.evidence/toolbar-rework/step3-slice4-review.md`,
CHANGES REQUIRED/BLOCKED) on the uncommitted candidate. Transfer:
`.evidence/toolbar-rework/step3-slice4-correction-transfer.md`; checkpoint
`.evidence/toolbar-rework/step3-slice4-fix-checkpoint.md`. No Slice 5, no primitive/popup
rewrite, no drag-smoke/full gates, no commits, no stash, no hand-edits to generated
tailwind.css (bundle regenerated by builds). Gortex impact before production changes;
detect/tests/guards/contract after (tests 0/3 graph-linked — packaged oracles live
outside the graph; guards none; contract behavioral-warn reviewed: additive changes,
existing callers unaffected).

- A ticket claims: qualified O attribution (6 items, O BLOCKED, no "all pre-existing",
  no Escape-sanity claim), M/R narrowed to observed coverage, O survivor restated as
  unresolved named mutation (Tab-out claim dropped: no evidence path), 4 files incl.
  ticket, 320-abort prose + Key→Row comment fixed.
- B timing honesty: exact-`none` settled assertions everywhere; R observers kept
  through close with phase-labeled transitions both directions (section + Objects
  panel); M one-box band against the untransformed track box + first-update capture +
  intermediate samples, per-frame scroll/tx/running in the scrolled segment,
  resize-mid-entry proof (entry running at the step, immediate realloc, per-segment
  clip), Align disabled-unit opacity segment (6 units @0.5 baseline); first-frame
  cancel recorder (exact `none` + B1 kills) on Layers/tips/guidance/Reference at
  390 + 1440. CORRECTION-2 AMENDS the timing claims: the recorder sampled action
  time after click/focus with a 60ms look-back (not same-task at-action), recorded
  rect/hit without asserting them, and the resize guard accepted any pre-step
  running sample (entry could finish before the step). Strengthened in
  correction-2 with same-task pre capture, first-frame geometry+hit+anchor asserts
  and resize bracketing. Guards without
  natural faults (one-box first-update promptness, resize immediate-reallocation,
  per-segment sampler copies) are documented as structural: existence assertions
  self-prove on GREEN (they fail on empty/settled data), the shared predicates
  carry kills, samplers got a diff read.
- C scenario honesty: roving diagnostic moved to a follow-ups list (uncoupled from
  pass/fail); Layers-Escape rewritten to the real path LayerPanel implements
  (real Tab into the list → Escape closes → focus visibly to trigger); tooltip
  close-with-tip-open without pre-dismiss (keyboard close; trusted clicks move the
  pointer — proven by probe); active_focus_state gains
  connected/hidden/inert/viewport/center-hit/occluder predicates.
- D production (all narrow, owner-contained): D1 Reference dialog fixed +
  clamped CSS plus the shared clamped-placement hook — the hook bypasses
  `position-anchor` while open (Chrome keeps scroll-gluing anchored dialogs past
  inline offsets and stale-maps their hit-testing; both proven by probe: +100px
  drift, inline-top pin restoring hit). CORRECTION-2 AMENDS the "anchored" claim:
  the GREEN `step3-slice4-fix-overlays-1440g.log` itself shows Layers detached —
  during entry list x1192 vs settled trigger x607.3, after parent-only resize
  trigger x352.2 vs list x607.3 (preserved RED; containment+hit passed, anchoring
  unasserted). Fixed in correction-2 via untransformed anchoring + row-resize +
  settle tracking with first/settle/post-resize anchor oracles. D2 nested
  admission (setup page + key
  selection → Combobox drive packaged: literal options/filter/choose/accepted/
  Escape-ladder; Select drive + two-stage ladder mounted) plus the staged-Escape
  flag+refusal fix (the primitive global Escape listener is immune to
  stopPropagation — first Escape closed both levels; RED then GREEN); D3 Layers
  section-close observer; D4 hook pinning (scroll-0 list x142, was x242/right-482);
  D5 ghost root-caused to harness-pointer hover (blur/mouseleave/close-clears all
  proven working; no product lifecycle change) plus Escape scrollIntoView and a
  trigger-associated recorder. Overlays 390 + 1440 EXIT 0 (FAILURES 0).
- E survivor experiment: mounted Reference-open + nested-open programmatic-close
  (E1) GREEN with rescue to the section trigger; focus-parked-outside variant (E2)
  GREEN; named O mutant rerun KILLED at the E2 dismissal assertion with exact
  restoration. CORRECTION-2 AMENDS the redundancy wording: E1 witnesses
  rescue→section-trigger (supporting, not proving, rescue→focusin→outside-dismiss
  as a bounded mechanism); the E2 kill is the separately focus-parked-outside
  experiment. The conjunction (nested-open AND focus-outside simultaneously) was
  not tested here; correction-2 attempted it and stopped per the stop rule (Select
  closes on blur + auto-focuses on open; see fix2 checkpoint row 3). S4-5 named
  survivor discharged; conjunction remains a proof gap listed as follow-up.
- F coarse geometry: live media activation unreproducible by this builder (CDP
  page/browser scope, metrics override, set-device — 4 attempts; stop rule), so
  no new activation experiment per the transfer. Fallback: static rule+selector
  proof (coarse strips rule exists; all five triggers match; lane/utility buttons
  keep 32px via higher-specificity shell rules) plus equivalent-sizing geometry
  (forced 44px triggers): 44-in-42 CONFIRMED REAL — 1.5px vertical spill both
  sides and Inspect pushed partly off-viewport at 390 (edge hit lost). Fix: the
  coarse strips rule scoped away from the accordion pill (`.layout-toolbar-
  strips:not(.layout-toolbar-accordion)`), verified live (triggers no-match,
  legacy strips still match; bundle regenerated). Fine-pointer gates cover the
  fixed geometry. CORRECTION-2 STATUS (S4-7, no CSS change here): awaiting user
  decision (44px coarse targets vs contracted 32px in the 42px row) + live
  post-fix coarse receipt owed.

Mutations (all `scripts/mutation-check.py --locked`, all `restored_exact: true`):
| Mutant | Oracle | Result |
|---|---|---|
| B1 neuter cancel_entries_for_open_overlays | overlays 390 | KILLED at all four first-frame assertions (Layers/tip/guidance/Reference still carry matrix/running) |
| B one-box half (100%→50%) | 728 page flow | KILLED at the one-box band |
| B disabled opacity when open (v2) | 728 | KILLED at disabled constancy (v1 survived: lost to shell (0,5,0); redesigned v2 (0,5,1)) |
| B clip transition (v2 grid) | 728 | KILLED at clip-stationary (v1 survived: width-transition vacuous on grid sizing; redesigned) |
| B 1ms entry | 728 | KILLED at intermediate-motion (endpoints-only sampling) |
| R ignore reduced motion (rerun) | 728 | KILLED at running-zero (open + close phase samples) |
| O remove parent-section detail dismissal (rerun) | layout E experiments | KILLED at the focus-parked dismissal assertion; nested path maps the redundancy |
| D2 drop nested on_value_change (both forms) | layout D2-Select + overlays 390 | KILLED at accepted-reference (both tiers; packaged kill is single-failure clean) |
| D2 ignore picker-Escape flag | overlays 390 | KILLED at ladder-1 (first Escape must close picker only) |
| D2-Select ladder pre-fix | layout D2-Select | RED (`step3-slice4-fix-mount-d2c.log`: detail closed by first Escape) → GREEN after the flag+refusal fix |

Gates (this pass, current tree): shell wasm 6/0; `check.py browser --only layout`
281 passed + 2 setup (278 + 3 new tests), 0 failed; scoped rustfmt clean; wasm
clippy `-D warnings` clean for boardstudio-web-layout + boardstudio-web-ui-shared
(native clippy checks stubs by design); packaged page flows 320/390/728/1440 EXIT 0;
layers 320/390/728/1440 EXIT 0; content EXIT 0; overlays 390/1440 EXIT 0. Fresh
final build; site verified mutant-free by restoration receipts. New seam:
`use_trigger_clamped_placement` (ui-shared popover.rs), shared by LayerPanel and the
Align popup — justified against duplication drift, flagged for review.

Remains for coordinator/closure: View-lane roving follow-up (item 6, recorded not
waived); native touch/trackpad scroll (still UNPROVED); live-coarse post-fix receipt
(no harness); Availability/Action parity rows; Slice 5 observer/font/measuring
retirement; delta re-review of this correction (production + oracle changes) per the
re-review rule; integrated gate. Touch-target tradeoff note: the row keeps its
contracted 32px entries (44px cannot fit 42px); changing that needs a user decision.

### 2026-11-15 — Slice 4 correction-2, Terra residuals rows 1–4 (Builder)

Addresses the correction delta re-review (S4-1–S4-7 residuals,
`.evidence/toolbar-rework/step3-slice4-fix-review.md`, CHANGES REQUIRED/BLOCKED) on
the uncommitted candidate. Transfer:
`.evidence/toolbar-rework/step3-slice4-fix2-transfer.md`; checkpoint
`.evidence/toolbar-rework/step3-slice4-fix2-checkpoint.md`. No Slice 5, no coarse CSS
or sizing change, no drag-smoke/full gates, no commits, no stash, no hand-edits to
generated tailwind.css. Gortex impact before production changes (popover CRITICAL
shared-params, canvas_layers HIGH); detect/tests/guards/contract after (detect MEDIUM
8 files; tests 0/3 graph-linked — packaged oracles live outside the graph; guards
none; contract behavioral-warn reviewed: additive hook fields/helpers, signature
stable, LayerPanel + Align callers unaffected).

- Row 1 anchoring (production + oracles): the hook now pins from the untransformed
  trigger (track tx/ty subtracted, so the first frame anchors before cancel
  finishes the entry), observes the row for parent-only reflow, re-pins every frame
  until the track settles (CORRECTION-3 AMENDS the bound claim: fix2's loop had
  no coded deadline or frame count — stopped only on settled/disappear/cleanup,
  so a persistent non-`none`/identity transform could persist; fix3 enforces
  `SETTLE_MAX_MS` 280ms deadline in `popover.rs`, see fix3 entry), and scopes to
  lane triggers
  (dock/floating LayerPanel content is not fixed, hook is a no-op there). Anchor
  bypass (`position-anchor: none` + inline left/top) retained — no return to the
  probe-proven scroll-gluing/hit-staleness. Oracles assert clamped horizontal
  anchoring (list/dialog x == clamped trigger x ±2px) at first frame, after settle
  and after parent-only resize for Layers and the shared-hook Reference dialog;
  containment+hit checks retained. 1440 GREEN shows the preserved REDs fixed:
  during-entry list x607.3 vs trigger x607.3 (was 1192), post-resize list x352.2 vs
  trigger x352.2 (was 607.3); first-visible x607.27 anchored with hit true.
- Row 2 timing: the recorder samples entry state in the same JS task immediately
  before the opening action (no 60ms look-back) and asserts first-visible exact
  `none`/zero plus geometry, viewport, hit and (Layers/Reference) untransformed
  anchoring (CORRECTION-3 AMENDS the first-frame claim: fix2 checked side margins
  + y>=triggerBottom but not y+h<=vh for Layers/Reference, and the RAF recorder's
  first-paint bound was unproven; fix3 adds vertical containment — killed by the
  +650 mutant with hit true — and records the MutationObserver-microtask-before-
  paint ordering proof, see fix3 entry). The resize segment captures pre/post entry
  state in the same task as
  the width step and requires bracketing running samples plus an intermediate
  moving sample around the step, with immediate final clip and stationary
  per-segment clip retained. 728 GREEN: pre/post running 1 (tx 450→410),
  stepped_at 1 bracketed.
- Row 3 conjunction: attempted twice, stopped per the stop rule (see checkpoint).
  Blur-after-open closes the Select listbox; park-then-click auto-focuses into the
  picker (DIV). The primitive makes nested-open + focus-body unstable, so no
  conjunction test joins the tree (CORRECTION-3 AMENDS the removal record: an
  `#[ignore]` conjunction test remained at `layout_toolbar.rs:1164` — 281+1, and
  its "JS click does not move focus" comment is contradicted by attempt 2's DIV
  focus; fix3 ran the materially different real-control reachability experiment
  for Select+Combobox, found nested-open+focus-outside unreachable in both, and
  removed the ignored test — 281+0, no coverage lost; E1+E2 stay GREEN). The
  named O mutant is not rerun: the existing kill at the E2 focus-parked dismissal
  assertion covers the parent-close signal path the conjunction would exercise
  (reachability, not dismissal, is the remaining gap; see fix3 entry).
- Row 4 ticket: anchoring/timing/conjunction claims corrected above with preserved
  REDs; S4-7 status: awaiting user decision (44px coarse targets vs contracted 32px
  in the 42px row) + live post-fix coarse receipt owed (CORRECTION-3 notes S4-7
  supersession: this "awaiting" status is historical; the coarse decision receipt
  entry below — user decision (a) + live held-CDP receipt — discharges S4-7 live
  geometry, with native touch/trackpad remaining UNPROVED separately; fix3 does
  not reopen S4-7).

Mutations (all `scripts/mutation-check.py --locked`, all `restored_exact: true`):
| Mutant | Oracle | Result |
|---|---|---|
| Fix2 anchor +50px offset | overlays 1440 | KILLED at anchor asserts (Layers first-frame/settled/post-resize + Reference first-frame/settled; list 657 vs 607, 402 vs 352, dialog 785 vs 735) |
| Fix2 hit +1000px drop | overlays 1440 | KILLED at first-frame hit asserts (Layers + Reference y1112/1116, hit false) |
| Fix2 no-entry (`animation: none`) | overlays 1440 | KILLED at same-task pre timing asserts (running 0/none at all four sites) |
| Fix2 resize-after-settle (test-side delay) | 728 page | KILLED at bracketing (`entry not running before the resize step`, pre/post 0/none) |
| B 1ms entry (reused spec, overlays 1440) | overlays 1440 | SURVIVED honestly: the same-task pre still catches the 1ms entry (running 1); 0ms (`none`) is the breaking case above |

Gates (this pass, current tree): shell wasm 6/0; `check.py browser --only layout`
281 passed + 1 ignored (CORRECTION-3 corrects the parenthetical: the ignored
conjunction test was present, not removed — fix3 removes it, 281+0; see row 3)
+ 2 setup, 0 failed;
scoped rustfmt clean; wasm clippy `-D warnings` clean for boardstudio-web-layout +
boardstudio-web-ui-shared (native clippy hits the pre-existing ui-shared import
error; wasm is the compiler inventory per Slice 2); packaged overlays 390/1440 EXIT
0; layers 390/1440 EXIT 0; page flows 390/728/1440 EXIT 0 (728 carries the resize
bracketing). Fresh final build; site verified mutant-free by restoration receipts.
No coarse CSS change; no Slice 5.

Remains for coordinator/closure: S4-7 user decision + live post-fix coarse receipt
(CORRECTION-3 updates: S4-7 discharged by the coarse receipt below); S4-5 conjunction
proof gap (CORRECTION-3 updates: fix3 reachability evidence below — needs explicit
user disposition, not a follow-up); View-lane roving, native touch/trackpad, Availability/
Action parity, Slice 5 retirement, delta re-review per the re-review rule, integrated
gate. No Slice 5 or integrated closure is authorized by this builder pass.

### 2026-11-14 — Slice 4 coarse decision receipt (Checker)

S4-7 user decision (2026-11-14): coarse option (a) — contracted 32px entries stay
in the 42px row under coarse pointers; the 44px coarse rule stays scoped away
from the toolbar pill (44-in-42 spill proven by the correction equivalent-sizing
geometry). Live post-fix coarse receipt GREEN on the freshly built packaged page
(no rebuild needed: served bundle sha256-matched to the worktree and carries the
scoped rule): `step3-slice4-coarse-decision-attempt1.log` — held CDP session
(stdlib WS: Target.getTargets → attachToTarget → session
Emulation.setTouchEmulationEnabled + setEmulatedMedia pointer:coarse/hover:none,
WS held for all measurements; no JS media override); matchMedia('(pointer:
coarse)') true, fine false, hover:none true, maxTouchPoints 1; live-rule witness
header Project button 40px/minH 0px → 44px/minH 44px; five section triggers 32px
high with no match on the scoped-away rule; row 42px with no row/page horizontal
overflow; seven entries center+edge hit closed and with Select/Snap/View open;
gaps ≥4px; no console/page errors. Checkpoint
`step3-slice4-coarse-decision-checkpoint.md` (incl. an honest serve-provenance
correction: first runs hit an unknown port occupant, preserved separately; the
receipt run used an owned fingerprinted server). S4-7 live geometry discharged
under decision (a); native touch/trackpad scroll remains separately UNPROVED.

### 2026-11-16 — Slice 4 correction-3, F1-F3 (Builder)

Addresses the fix2 delta re-review F1-F3 + ticket accuracy
(`.evidence/toolbar-rework/step3-slice4-fix2-review.md`, CHANGES REQUIRED/BLOCKED)
on the uncommitted candidate. Transfer:
`.evidence/toolbar-rework/step3-slice4-fix3-transfer.md`; checkpoint
`.evidence/toolbar-rework/step3-slice4-fix3-checkpoint.md`. Discharged items
(S4-1, S4-3, S4-4, S4-6, S4-7, S4-5 named survivor) not reopened. No Slice 5, no
coarse CSS/sizing, no commits/stash/full gates, no hand-edits to generated
tailwind.css. Gortex impact before the popover change (CRITICAL shared seam);
detect/tests/guards/contract after (detect MEDIUM 8 files; tests 0/3 — packaged
oracles live outside the graph; guards none; contract behavioral-warn reviewed:
additive hook fields, signature stable, LayerPanel + Align callers unaffected).
Gortex refactor delete is read-only on this view (probe/ignored removals via
Gortex file edits; one codemode-assisted exact deletion for the ignored test —
no native fallback, no stale writes).

- F1 placement RAF lifetime + re-pin bound (production `popover.rs` + packaged
  race oracle): the hook retains the outstanding RAF ID in a shared
  `settle_raf` cell and `cancel_animation_frame`s it before dropping the closure
  on every cleanup/replacement (close, reopen, unmount via the existing
  `remove_clamped_placement_listeners` paths); the re-pin loop enforces
  `SETTLE_MAX_MS` 280ms deadline (`js_sys::Date::now()` start, checked every
  frame) so it cannot persist on a non-`none`/identity transform. Race oracle
  `check_placement_raf_race` (Layers, overlays scenario): a style-mutation
  observer auto-closes in the same microtask checkpoint as the hook's effect
  (inline left write → RAF scheduled synchronously → observer microtask before
  any RAF), provably closing before the next frame; asserts cancel called, no
  in-page/page/browser errors, list closed, no stale inline styles, reopen
  geometry. RED pre-fix at 1440 (`step3-slice4-fix3-red-f1-1440.log` EXIT 1):
  pending [14] fired after drop → `closure invoked recursively or after being
  dropped`, cancel []. GREEN post-fix at 1440+390 (`step3-slice4-fix3-green-f1-
  {1440,390}.log` EXIT 0): cancel [14], RAF 14 never fired, no errors, reopen
  placed+hit. Corrects the "bounded 280ms fallback" wording to the implemented
  deadline (see Row 1 amendment above).
- F2 first-visible vertical containment + first-paint bound (oracle only): the
  first-visible branch for Layers+Reference now asserts `rect.y + rect.h <= vh`
  alongside existing y>=triggerBottom, x margins, anchor and hit. Killed by the
  `Fix3-vertical-overflow` mutant (+650px hook shift):
  `step3-slice4-fix3-mut-vertical/results.json` killed, `restored_exact: true`;
  first-frame bottom 922.5 (Layers) + 920.5 (Reference) with hit True both —
  the hit target stays in viewport, so the vertical predicate kills
  independently (Layers settled has no bottom check; Reference settled also
  fails as expected for the persistent mutant, with first-visible listed first).
  F1 race passes under the mutant (cancel [14], no errors). First-paint bound:
  CORRECTION-4 AMENDS the "no negative-control mutant" record below: fix4 ran
  the one-paint deferred-cancellation control (`Fix4-firstpaint-defer1raf` —
  `finish()` deferred by exactly one rAF in `cancel_track_entry`) and it is
  KILLED at the first-visible assertions (Layers/Tip/Reference all carry
  matrix/running + miss hit/anchor at first overlay-visible frame;
  `step3-slice4-fix4-mut-firstpaint/results.json`, `restored_exact: true`).
  The proven bound is therefore first-*sampled*-frame sensitivity: a
  one-paint-late cancellation is rejected by the oracle in the tested open
  paths. The ordering argument below is retained as supporting (observer
  microtask before RAF sample), not as a paint-instrumented proof; no claim
  is made beyond the sampled-frame control. Original fix3 ordering (retained
  as supporting): Overlay open always mutates an attribute the `ToolRowOverlayCancel` MutationObserver watches
  (`workbench_shell.rs`: `["data-state", "aria-expanded"]` + childList + subtree;
  Layers sets `aria-expanded`, Reference/tooltips set `data-state`); its callback
  synchronously calls `cancel_entries_for_open_overlays`, which WAAPI-`finish()`es
  the running `workbench-tool-enter` with no RAF deferral. MutationObserver
  callbacks are microtasks, which run after the opening mutation and before the
  next RAF sample and before paint — so the first RAF sample seeing overlay
  visible necessarily sees entry already finished (`none`/0), and no paint can
  show overlay-visible-with-entry-running. Combined with the B1 never-cancelled
  kill at all four first-frame assertions, plus the fix4 one-paint deferral
  kill at all three first-visible sites, the recorder is proven sensitive to a
  one-frame-late cancellation in the tested open paths. CORRECTION-4 replaces
  fix3's "one-paint miss is impossible" with this sampled-frame control; no
  paint-instrumented proof is claimed. No paint-level
  overclaim beyond this ordering.
- F3 conjunction reachability (materially different experiment; the two failed
  synthetic orderings not repeated): mounted fixture, real in-detail Close focus
  then real outside focus (`.focus()` on `#scope-probe-select-column`, which fires
  outside `focusin` without touching `open_menu`) plus outside `.click()` pointer
  variant, for Select (3 refs) and Combobox (13 refs), observing nested open state
  + `activeElement` at each step (`step3-slice4-fix3-probe-f3.log`, temporary probe
  since removed). SELECT: picker opens (active DIV); Close focus → picker CLOSED,
  focus restored to nested trigger; outside focus → detail CLOSED (active outside);
  reopen + outside click → picker/detail unchanged (`.click()` moves no focus,
  fires no outside-dismiss). COMBOBOX: picker opens (active INPUT, stays); Close
  focus → picker CLOSED, focus stays on Close; outside focus → detail CLOSED;
  reopen + outside click → unchanged. Verdict (CORRECTION-4 AMENDS the
  categorical "UNREACHABLE" below): nested-open + focus-outside was unreached
  in the two tested synthetic orderings only — any `.focus()` leaving the
  picker closed it (even in-detail), any `.focus()` leaving the detail closed
  it, and synthetic `.click()` moved no focus. Trusted pointerdown, touch and
  BODY-parking with picker open were untested here; fix4 completes those three
  paths for both pickers (see fix4 entry) and still reaches no conjunction —
  the proof gap remains for explicit user disposition, not a discharge. Original
  fix3 categorical text (superseded): nested-open + focus-outside is UNREACHABLE in both picker paths — any focus leaving the picker closes it (even
  in-detail), and any focus leaving the detail closes it; synthetic `.click()`
  cannot move focus outside at all. Per the transfer, STOP as proof gap needing
  explicit user disposition — no discharge claim, no combined test, no O rerun
  (existing E2 kill stands for the dismissal path; reachability is the gap). The
  leftover `#[ignore]` conjunction test removed (its "JS click does not move focus"
  comment contradicted by attempt 2's DIV auto-focus and by this probe's
  inside/outside click asymmetry); see Row 3 amendment above. Test-count impact:
  layout `281 passed + 1 ignored` → `281 passed + 0 ignored` — passed unchanged,
  ignored removed with no coverage lost (it never ran; E1+E2 stay GREEN).

Mutations (all `scripts/mutation-check.py --locked`, all `restored_exact: true`):
| Mutant | Oracle | Result |
|---|---|---|
| Fix3 vertical +650px overflow | overlays 1440 | KILLED at first-frame bottom (Layers 922.5 + Reference 920.5, hit True both) |

Gates (this pass, current tree): shell wasm 6/0; `check.py browser --only layout`
281 passed + 0 ignored + 2 setup, 0 failed; scoped rustfmt clean; wasm clippy
`-D warnings` clean for boardstudio-web-layout + boardstudio-web-ui-shared;
packaged overlays 390/1440 EXIT 0 (carry F1 race GREEN + F2 vertical); page flows
390/1440 EXIT 0; layers/content/728 not rerun (unaffected oracles; 728 resize
bracketing reused from fix2). Fresh builds under lock; site verified mutant-free
by restoration receipts + post-mutant rebuild. No Slice 5.

Remains for coordinator/closure: S4-5 conjunction proof gap — explicit user
disposition required (fix3 reachability evidence above; do not waive by review
wording); View-lane roving, native touch/trackpad, Availability/Action parity,
Slice 5 retirement, delta re-review per the re-review rule, integrated gate. No
Slice 5 or integrated closure is authorized by this builder pass.

### 2026-11-16 — Slice 4 correction-4, F2 first-paint + F3 bound (Builder)

Addresses the fix3 delta re-review F2 first-paint + F3 sections
(`.evidence/toolbar-rework/step3-slice4-fix3-review.md`, CHANGES REQUIRED/BLOCKED)
on the uncommitted candidate. Two bounded rows only; no Slice 5, no coarse
CSS/sizing, no commits/stash/full gates, no hand-edits to generated
tailwind.css. Owned: ticket 002 corrections,
`.evidence/toolbar-rework/step3-slice4-fix4-*`; `scripts/test-workbench-viewport.py`
recorder untouched (Row 1 killed without strengthening);
`web/src/presentation/workbench_shell.rs` untouched (mutant restored exact, no
test seam); `web/crates/layout/src/objects/layout_toolbar.rs` untouched (Row 2
reached no conjunction, so no combined test). Gortex `explore/task` for
localization; native reads for evidence/ticket (markdown, no localization
contract); native edits for ticket/probe (evidence/scratch + ticket-owned
markdown). Build/test under shared build lock; owned `serve-web.py` on 4173
(PID 1391231, killed unknown `http.server` occupant serving the same worktree
site, since stopped).

- Row 1 F2 first-paint negative control (packaged overlays 1440): the
  `Fix4-firstpaint-defer1raf` mutant defers the entry `finish()` in
  `cancel_track_entry` by exactly one `requestAnimationFrame` (per-animation
  `Closure::<dyn FnMut(f64)>::new` + `window.request_animation_frame` +
  `forget`; spec
  `.evidence/toolbar-rework/step3-slice4-fix4-mut-firstpaint-spec.json`).
  `scripts/mutation-check.py --locked` with rebuild +
  `test-workbench-viewport.py --scenario overlays` at 1440:
  `step3-slice4-fix4-mut-firstpaint/results.json` KILLED, `restored_exact:
  true`, exit 1. First failure is the intended first-visible assertion:
  `LAYERS-DURING-ENTRY` "first overlay-visible frame still carries the
  entry" with `transform: matrix(1, 0, 0, 1, 643.15, 0)`, `running: 1`,
  `hit: false` (list x1755.7, unanchored, outside margins); the same
  transform/hit failures repeat at `TIP-DURING-ENTRY` (matrix 643.18) and
  `REFERENCE-DURING-ENTRY` (matrix 643.17, y-28, unanchored) — 10 failures,
  all first-visible/first-frame (transform/geometry/hit), none settled-only.
  Baseline `_baseline.log` is GREEN (FAILURES 0). Attempt 1 was a
  compile-failure (missing `FnMut(f64)` annotation, does not count;
  preserved at `step3-slice4-fix4-mut-firstpaint-attempt1-compilefail/`);
  attempt 2 killed first try, so no recorder strengthening was needed and
  none was made. Proven bound (replaces fix3's "one-paint miss is
  impossible"): the oracle rejects a one-paint-late cancellation at the
  first sampled overlay-visible frame in the tested Layers/tip/Reference open
  paths. No paint-instrumented claim beyond this sampled-frame control; the
  MutationObserver-microtask ordering is retained as supporting only (see Row
  1 amendment in the fix3 entry above).
- Row 2 F3 bound completion (packaged, trusted input): extends the fix3 probe
  method with Terra's three paths, for Select (1×3 matrix, BUTTON form, 5
  options) and Combobox (5×6 matrix, INPUT form, 59 options) while
  nested-open, observing detail open + picker open + `activeElement` at each
  step (probe `.evidence/toolbar-rework/step3-slice4-fix4-probe-f3.py`, log
  `.evidence/toolbar-rework/step3-slice4-fix4-probe-f3.log`, EXIT 0): (a)
  trusted pointerdown on a real non-focusable outside element (agent-browser
  mouse down/up at canvas body-point 720,800, verified outside detail); (b)
  touch tap outside (held-CDP `setTouchEmulationEnabled` +
  `Input.dispatchTouchEvent` touchStart/touchEnd at the same point, WS held
  per the coarse recipe); (c) parking focus on BODY via
  `activeElement.blur()` while picker open. Per-path outcomes (all
  CONJUNCTION false):
  | Picker | Path | After | detail | picker | active | InDetail |
  |---|---|---|---|---|---|---|
  | Select | C blur | blur | true | false | BUTTON Alignment reference (nested trigger) | true |
  | Select | A pointerdown | down | false | false | svg canvas group | false |
  | Select | A click | up | false | false | svg canvas group | false |
  | Select | B touchStart | start | false | true (lingering, 5 opts) | BUTTON Align section trigger | false |
  | Select | B touchEnd | end | false | false | svg canvas group | false |
  | Combobox | C blur | blur | true | false | BODY | false |
  | Combobox | A pointerdown | down | false | true (lingering, 59 opts) | svg canvas group | false |
  | Combobox | A click | up | false | false | svg canvas group | false |
  | Combobox | B touchStart | start | false | true (lingering, 59 opts) | BUTTON Align section trigger | false |
  | Combobox | B touchEnd | end | false | false | svg canvas group | false |
  Blur closes the picker in both forms (Select refocuses its nested trigger
  inside; Combobox parks at BODY with detail still open — E2-like, picker
  closed). Trusted pointerdown and touch tap close detail+picker with focus
  outside (canvas/section-trigger); the touchStart/pointerdown transient
  `detail:false + picker:true` is picker linger after detail close, not the
  `detail:true + picker:true + focus-outside` conjunction. NO path reached
  nested-open + focus-outside-detail. Per the transfer, no combined test and
  no O rerun (existing E2 kill stands for the dismissal path); the conjunction
  remains a proof gap for explicit user disposition — do not waive by review
  wording. See Row 2 amendment in the fix3 entry above; fix3's categorical
  UNREACHABLE is superseded by this bounded table.

Mutations (all `scripts/mutation-check.py --locked`, all `restored_exact: true`):
| Mutant | Oracle | Result |
|---|---|---|
| Fix4 one-paint defer 1rAF | overlays 1440 | KILLED at first-visible transform/hit/anchor (Layers matrix 643.15 + Tip 643.18 + Reference 643.17, 10 first-frame failures) |

Gates (this pass, current tree): packaged overlays 1440 EXIT 0
(`step3-slice4-fix4-overlays-1440.log`, FAILURES 0) + 390 EXIT 0
(`step3-slice4-fix4-overlays-390.log`, FAILURES 0, incl. F1 race GREEN);
scoped rustfmt clean (`rustfmt --edition 2024 --check` on
`workbench_shell.rs` + `layout_toolbar.rs`, EXIT 0); clippy unchanged (no Rust
files changed — mutant restored exact, no recorder/test/seam edits; fix3 wasm
clippy receipts reused). Layout browser not rerun (no test added); shell wasm
not rerun (shell untouched); page/layers/content/728 not rerun (unaffected
oracles). Fresh post-mutant rebuild under lock
(`step3-slice4-fix4-build-postmut1.log`, EXIT 0); site verified mutant-free by
restoration receipt. Serve stopped + browsers closed at exit. No commits/stash.
Test-count impact: none (no tests added/removed; layout 281+0 unchanged).

Remains for coordinator/closure: S4-5 conjunction proof gap — explicit user
disposition required on the bounded fix3+fix4 reachability evidence above (do
not waive by review wording); View-lane roving, native touch/trackpad scroll,
Availability/Action parity, Slice 5 retirement, delta re-review per the
re-review rule, integrated gate. No Slice 5 or integrated closure is authorized
by this builder pass.

User disposition (2026-11-16): the S4-5 combined conjunction test is WAIVED on
the bounded fix3+fix4 reachability evidence — across Select and Combobox,
focus-inside, blur, trusted pointerdown and touch paths, the conjunction was
not reached in the observed snapshots of the tested paths (focus movement out
closes the picker; pointer/touch close detail+picker, mid-gesture linger only;
visibility during a held or cancelled gesture remains unproved). This is a
bounded observation, not a proof that no supported path can reach it. E1
(nested-open programmatic close), E2 (focus-parked programmatic close) and the
named O dismissal mutant killed at E2 stand as the dismissal proof. This
waives the combined test only; it does not discharge View roving, native
touch/trackpad scroll, parity, Slice 5 or the integrated closure gate.

### Earlier implementation receipts

- 2026-11-11 Availability/action-parity builder: both mounted tests GREEN in `.evidence/toolbar-m1/002-availability-parity-attempt4.log` (11 executed, 0 failed). First-column scope is chosen through Select before Transform, matching the existing scope controls and column-specific action labels. Attempt 2 reached a Follow-rotation fixture with no accepted scope; attempt 3 added an anchor source key, board membership and shared layout following `matrix_edit_test_support`, and availability passed. Attempt 3 parity failed because clicking SW14 did not populate the combobox input; the existing `combobox_floor_filter_active_descendant_selection_and_escape` test establishes filtering, ArrowDown and Enter as a selection path. Attempt 4 uses that path and retains the SW14 selection and resulting alignment assertions. The later host failures in attempts 1–3 followed the shared-WASM panic-hook cascade. Receipts: `.evidence/toolbar-m1/002-availability-parity-attempt{1,2,3,4}.log`. Gortex writes for the two initial test corrections returned `view_building` twice each, so those used the user-authorized native fallback; the parity correction and this ticket entry used Gortex. No production change, full gate, mutation, review or closure claim.

- 2026-11-11: Added an initial real-keyboard Snap lane probe; packaged fast build passed, but the probe failed after observing lane scroll without reaching its chosen `Snap increment` target. This is not yet evidence of a production defect: inspect the actual focus sequence and choose a reachable initially clipped action before retrying. Receipt: `.evidence/toolbar-m1/builder-keyboard-red.log`; build: `.evidence/toolbar-m1/builder-build.log`. Touch scrolling, motion/reduced motion, availability, action parity and 2D/3D persistence remain unproved in this attempt.

- 2026-11-11 Implementation transfer: Gortex available and ticket 002, 009 progress, verification and architecture read. No new tests, build, browser runs, or production changes were completed in this transfer; no new RED/GREEN receipts or logs. Solid motion, reduced motion, keyboard focus scrolling, touch scrolling, availability, action parity and section persistence remain open. Architecture specifies session preservation across workspace changes but does not explicitly resolve section state across a 2D/3D toggle; the earlier toggle receipt observes the section closes on remount. Next: add a focused real-keyboard offscreen-action oracle to the viewport probe, then investigate any observed failure; independently establish the intended 2D/3D section behavior before asserting it.

- 2026-11-11 Snap builder (002-snap): RED was `accordion_sections_keep_existing_tools_inline` at `by_role_name(&root, "combobox", "Snap increment")` (`.evidence/toolbar-m1/002-accordion-green.log`). Diagnosed as genuinely wrong role expectation: `LayoutSnapIncrementSelect` renders shared `Select` with `aria_label "Snap increment"`, whose trigger semantics are role `button` (listbox + options), not `combobox`. Independent evidence: `primitives_mount_tests::select_floor_label_navigation_selection_and_focus_return` (`button` "Fruit"), `select_close_requests_are_not_duplicated_while_popup_is_retained` (`button` "Select an option"), `scripts/test-command-palette.py` `action("button", "Snap increment")`, and `combobox_floor_*` (`combobox` "Search fruit" for `Combobox` input); closed detail `m1-layout-snap-menu` is removed from DOM so inline Select is unique. Changed only that one role string `combobox`->`button` in `web/crates/layout/src/objects/layout_toolbar.rs:670`; no production/CSS change. GREEN: `python3 scripts/check.py browser --only layout` -> `.evidence/toolbar-m1/002-snap-button.log` (269 passed + 2 setup_guide passed, `accordion_sections_keep_existing_tools_inline ... ok`). Five sections, Transform-once-inline, Select/Snap/View switching, closed-Transform inertness still asserted. No closure claim.
- 2026-11-11 Page builder (002-page-consumer): RED was `actual_layout_tool_row_has_seven_entry_points_and_stable_canvas` at `assert_eq!(count(&root, ".workbench-tool-row"), 1)` (`.evidence/toolbar-m1/002-page-red.log`, executed 7 failed 1, left 0 right 1, existing 6 pass). Implemented actual Layout ToolRow consumer (owned files only, no crate edits): `layout_workspace::ToolbarInput` + `handles_visible`/`on_toggle_handles`/`canvas_layers`; workspace-leave clears `command_menu` section; `layout_workspace::toolbar` rebuilt as normal-flow `ToolRow` (fit_key from section/selection/tool/snap/assembly/footprints/handles/context/transform/align; 2D pill `accordion:true` `snap_in_dock:false` with View (2D/3D idx41-42, Footprints idx43 if 2D, CanvasLayers Footprints-filtered, handles idx44) + `PanelUtilities`; 3D ToolRow 2D/3D+utilities only, pill hidden, no second row; save/recovery below row in flex-none div; wrapper div flex-none `data-workspace-toolbar=layout` with Space guard); `presentation.rs` View data construction (Footprints filtered from CanvasLayers groups, ToggleGroup sole owner), footer removes Layout CanvasLayers+SnapDock (keeps LayerDockOutlet/PCB), SVG overlay offsets → `flex-1`; `layout_layers_tests` host +`layout-toolbar.css`, new test (42px, 7 entry-points visible/pointer-events/no-cover, sections switch singly, canvas stable), `switch_visibility` steps moved Objects footer→View (oracles unchanged); `layout-toolbar.css` View ToggleGroups 32px, CanvasLayers trigger 32px list below clamped/scrollable with `:has` overflow escape, `.layout-toolbar-accordion` `pointer-events:auto` (fixes legacy overlay `none` making triggers non-hit-testable). No `workbench_shell.rs`/`shell.css` change (existing host sufficient). GREEN: `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs web/src/presentation/workbench_shell.rs` → `.evidence/toolbar-m1/002-page-green.log` (executed 7 failed 0); `python3 scripts/check.py browser --only layout` → `.evidence/toolbar-m1/002-page-browser.log` (269 passed + 2 setup_guide passed, test-css ok). 3D preserved: pill hidden, utilities+2D/3D reachable in single ToolRow, Footprints/handles/CanvasLayers hidden (matches `!assembly_3d`), section persists across 3D toggle. Remains planned (not claimed): measured widths 320/390/728 + motion frames + reduced-motion zero-animation + guard parity (first-column/Follow-off/locked/empty) + >12 reference action parity; proof-map rows stay planned; no closure claim.
- 2026-11-11 Toggle builder (002-toggle-fix): RED was `view_mode_and_visibility_toggles_keep_pressed_state_and_revision` at `2D starts pressed (left: "false", right: "true")` (`.evidence/toolbar-m1/002-toggle-red.log`, executed 8 failed 5 with 4 cascading panic-hook follow-ons). Root: `layout_workspace::toolbar` View ToggleItems used indices 41/42 (2D/3D), 43 (Footprints), 44 (handles) while controlled `pressed` and callbacks used 0/1 and 0, so pressed never matched and mode callbacks never fired; both 2D and 3D branches affected. Fix (owned files only): indices 41->0, 42->1 (both branches), 43->0, 44->0 in `web/src/presentation/layout_workspace.rs`, matching `DesignViewToolbar` convention (0/1, 0); callbacks, pressed logic, roving, PCB, disabled guards unchanged. Test: new actual-Editor `view_mode_and_visibility_toggles_keep_pressed_state_and_revision` in `layout_layers_tests.rs` (open View, assert initial pressed 2D on/3D off/Footprints off/handles on + revision, click 3D -> 3D on/2D off/View-only hidden/one row/revision, click 2D -> restore + reopen View if remount closed section, toggle Footprints on -> independence, toggle handles off -> independence, revision unchanged throughout). Corrects overstated hit-test claim: seven-entry test proves bounds/pointer-events/no-hidden-inert/enabled + synthetic click switches sections + canvas stable, not unobscured/topmost; `elementFromPoint` topmost check attempted but reverted after three viewport attempts (button center persistently outside viewport, e.g. (500,-149), scroll_into_view + wait did not bring into viewport) - remains planned follow-up, do not imply synthetic click proves unobscured. Also corrects section-persists claim: View section does not persist across 3D remount (pill remount closes section), test reopens View; follow-up to verify intended behavior. GREEN: `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs web/src/presentation/workbench_shell.rs` -> `.evidence/toolbar-m1/002-toggle-green5.log` (executed 8 failed 0). No `workbench_shell.rs`/CSS change. Remains planned: measured widths 320/390/728 + motion + reduced-motion + guard parity + >12 reference parity + true elementFromPoint hit-test + section-persistence intent; proof-map rows stay planned; no closure claim.
- 2026-11-11 Viewport builder (002/009-viewport-proof): RED is real-page `scripts/test-workbench-viewport.py --page-url http://127.0.0.1:4173/boardstudio/` at 320x480 (`.evidence/toolbar-m1/viewport-320-red2.log`, EXIT 1). Packaged build `python3 scripts/build-web.py --fast` -> `.evidence/toolbar-m1/viewport-build.log` (EXIT 0); served via `python3 scripts/serve-web.py web/target/site 4173`. Browser viewport exactly 320x480, visualViewport 320x480 offset 0,0 scale 1, fonts loaded, 0 running anims, no page overflow. Row: count 1, 42.0px high, x8 y72 w304 (passes). Entries: only Align passes true hit test (rect x115 y92 w89 h32, center 160,108, hit SPAN workbench-tool-name); Select offscreen center (160,-65) elementFromPoint null; Transform center (160,24) occluded by Project button; Snap (160,145)/View (160,184)/Objects (261,222)/Inspect (160,257) occluded by empty-state content. Cause: `#m1-layout-command-pill` carries both `layout-toolbar-strips` (flex-direction column) and `layout-toolbar-accordion` (no direction, so column wins), stacking 5 sections + utilities vertically (pill h361 y-88..273) inside 42px overflow-hidden row; same at 1440x900. Panels themselves pass: Objects/Inspect open via accessibility click, bounds x8 y114 w304 h270 inside 320x480, Close 64x40 at (238,119) true hit, no overflow, no browser errors. No production/CSS edit per transfer (defect stop). Proposed narrow fix for coordinator: add `flex-direction: row;` to `.layout-toolbar-accordion` in `web/assets/layout-toolbar.css` (accordion-only, preserves PCB legacy strips). Next experiment after fix: re-run same script (expect GREEN entries), then 390/728/desktop + each section open. Remains planned: measured widths, motion, reduced-motion, guard parity, >12 reference parity, section-persistence intent; proof-map rows stay planned; no closure claim.
- 2026-11-11 Row-fix builder (002-row-fix): RED was viewport-320-red2.log (6/7 entries fail, pill vertical stack). Fix owned-only: `web/assets/layout-toolbar.css` `.layout-toolbar-accordion` add `flex-direction: row;` (accordion-only, PCB legacy strips preserved). Gortex exact worktree: impact LOW 0 dependents, detect 3 files LOW, tests 0/1 uncovered (CSS, real-browser proof is oracle), guards no_rules_configured, contract allow LOW behavioral. Build `bash -c 'exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9; python3 scripts/build-web.py --fast'` -> `.evidence/toolbar-m1/row-fix-build.log` EXIT 0 (dx 20s, CSS verified in site-subpath). Serve `python3 scripts/serve-web.py web/target/site 4173` (PID cleanup, port free, browser close --all). Reran unchanged `scripts/test-workbench-viewport.py` (no oracle weakening): 320x480 `.evidence/toolbar-m1/row-fix-viewport-320.log` EXIT 0 (row 42px, 7/7 true hit, centers y92.5 x34-288, no overflow, panels/Close pass, 0 errors) + confirm log; 390x844 `.evidence/toolbar-m1/row-fix-viewport-390.log` EXIT 0; 728x1024 `.evidence/toolbar-m1/row-fix-viewport-728.log` EXIT 0; 1440x900 `.evidence/toolbar-m1/row-fix-viewport-1440.log` EXIT 0 (two transient agent-browser daemon EOFs on first desktop attempts, recovered after close --all, not product RED). No test script edit. Remains planned: each-section-open 7-entry hit tests, measured action widths, motion, reduced-motion, guard parity, >12 reference parity, section-persistence intent; proof-map rows stay planned; no closure claim.
- 2026-11-11 Spacing builder (002-spacing-fix): precommit blocker was architecture `Icon hit regions ... separated by at least 4px` vs `.layout-toolbar-accordion {gap: 3px}` (adjacent triggers 3px at 320px). Added independent explicit measured spacing oracle to `scripts/test-workbench-viewport.py` (SECTIONS + spacing_report() bounding-rect gaps sorted L-to-R, fail if gap+1e-6<4.0, separate "spacing" key, entry-hit/panel/overflow oracles untouched). RED on current build: `.evidence/toolbar-m1/spacing-320-red.log` EXIT 1 (row 42px, 7/7 hit true, SPACING 4x3.000px fail, no overflow, panels/Close pass, 0 errors). Minimal accordion-only fix: `web/assets/layout-toolbar.css` `.layout-toolbar-accordion` gap 3px->4px (flex-direction:row preserved). Gortex exact worktree: impact script HIGH 6 param nodes test-only / CSS LOW 0; detect 3 files LOW (.layout-toolbar-accordion L34; untracked script not listed); tests 0/1 uncovered (CSS, real-browser oracle); guards no_rules_configured; contract allow LOW behavioral. Build under lock -> `.evidence/toolbar-m1/spacing-build.log` EXIT 0 (CSS gap:4px verified in site-subpath). Reran unchanged oracles: 320x480 `.evidence/toolbar-m1/spacing-320-green.log` EXIT 0 (SPACING 4x4.0, 7/7 hit centers y92.5 x34-288, 42px, no overflow, panels pass); 390x844/728x1024/1440x900 logs EXIT 0 (SPACING 4x4.0 each, entries/panels/overflow pass). No occlusion from +1px/gap, no further CSS issues, stayed in owned files. Remains planned: each-section-open 7-entry hit tests, measured action widths, motion, reduced-motion, guard parity, >12 reference parity, section-persistence intent; proof-map rows stay planned; no closure claim.
- 2026-11-11 Open-sections builder (002-open-sections): RED is extended real-page `scripts/test-workbench-viewport.py --page-url http://127.0.0.1:4173/boardstudio/ --width 320 --height 480` (`.evidence/toolbar-m1/open-section-320.log`, EXIT 1, FAILURES 25). Reused packaged build from HEAD 66766f5d8 (CSS gap:4px row verified in site-subpath, no rebuild needed); served via `python3 scripts/serve-web.py web/target/site 4173` (PID cleanup, port free, browser close). Extended owned probe only (SECTION_KEYS/MIN_HIT/MIN_GAP + seven_spacing_report/open_state_report/action_lane_report, 32px closed check, per-section trusted-click loop with one-open/42px/7-entry 32px-inside-hit/7-gap-4px/no-overflow/lane-room oracles, prior closed/panel/spacing oracles preserved). Closed row still GREEN (42px, 7/7 hit, 4x4.0 spacing, no overflow, panels/Close pass, 0 errors). Each open section fails 5 ways at 320x480 (visualViewport 320x480 offset 0,0 scale 1, fonts loaded, 0 anims, motion settled): open trigger hit false occluded by next trigger icon (Select x15 w32 center 31 occluded by Transform icon; Transform x81 occluded by Align; Align x143 occluded by Snap; Snap x205 occluded by View; View x267 w32 occluded by Objects button), Inspect center outside (322 for Select, 326 others, x305-309 w34, elementFromPoint null) + hit fail, spacing -28.000px overlap (Select>Transform etc, View>Objects), lane track clientWidth 3 revW 0 scrollW 196-273 overflowX auto (require >=32px for one action). One-open holds (single data-open true, correct key), row 42px, no page overflow. Gortex exact worktree: impact HIGH 6 param test-only, detect 8 symbols MEDIUM 4 dependents 0 test files, tests 0/4 uncovered (script is oracle), guards no_rules_configured, contract warn behavioral high (162-line body, no covering tests, test-only). No production/CSS edit per transfer (defect stop). No 390/728/desktop (320 RED). Proposed narrow next step for coordinator: diagnose why closed triggers stay 58-62px (headings visible) when open with data-fit-headers none (expect icon-only 32px via `.workbench-tool-row:has([data-open=true]) .workbench-tool-heading{display:none}`) and why open reveal grid stays 0fr/clientW 3 (expect 1fr/flex:1 with >=32px lane); likely fitting (`workbench_shell.rs` fit_open_labels/probe) vs Layout accordion CSS (`layout-toolbar.css`/pill flex) interaction; verify computed heading display + reveal grid + group flex at 320 open-Select, then scope minimal accordion-only or host fix. Unproved: zoom/visualViewport offsets/keyboard, motion frames, reduced-motion, guard parity, >12 parity, section-persistence; proof-map rows stay planned; no closure claim.
- 2026-11-11 Open-fix builder (002-open-fix): RED was open-section-320.log (25 failures: open trigger occluded, Inspect x322-326 offscreen, gap -28px, lane clientW3). Diagnosed live at 320 open-Select (row fit none scrollW331>304): closed triggers 58px (icon18+chev18+gap6+pad16, heading none but chevron block) vs icon-only 34px; open group flex 1 1 auto min-width0 collapsed to groupW0 (Transform x19 = Select x15+0+4), reveal grid 1fr computed 0px (no space), track cw3 = padding. Root: :has hides headings only, not chevrons; data-closed-hide (hides both) removed when open; fit_open_labels never folds chevrons; data-fit-headers all (0,3,0) loses to hide (0,4,0) (latent). Fix owned-only shell.css: :has hides heading+chevron (icon-only when open), active restores open heading+chevron (0,5,0), all restores all headings+chevrons via :has-boosted (0,5,0, fixes latent all); row padding 5px 7px->5px 2px (saves 10px, lane 24->34). Reuses measured data-fit-headers, no RS/new policy/remount, generic. Gortex exact: impact LOW 0, detect MEDIUM (4 files incl prior WIP), tests 0/1 (oracle), guards none, contract warn (test WIP). Build under lock -> open-fix-build3.log EXIT 0. Reran unchanged probe: 320x480 open-fix-320-green.log EXIT 0 (7/7 hit closed+each-open, gaps 38/4 (lane34+4), lane34 rev34 scroll196-273 auto, 42px, no overflow, panels/Close pass, 0 errors); 390x844/728x1024/1440x900 logs EXIT 0 (lanes 104/255-442/740, headings+chevrons+utility-labels restored when space, all oracles pass). Focused `run-wasm-tests.py --files workbench_shell.rs layout_layers_tests.rs` under lock -> open-fix-wasm.log EXIT 0 (executed 9 failed 0). No test weakening. Remains planned: motion frames, reduced-motion, guard parity, >12 parity, section-persistence; proof-map rows stay planned; no closure claim.
- 2026-11-11 Lane-fix builder (002-lane-fix): RED was strengthened oracle `lane-fix-320-red.log` EXIT 1 (3 failures, existing 7/7 hit/spacing/42px/overflow/panels pass): usable-Select none (5x38x40 enabled, track x48 w34, first x50 w38 clipped 6px withinTrack false, after scroll 158 none fit w38>34), guidance-Transform P x86 y29.9 w57 h125 vs row 72-114 withinRow false, view-icon Canvas w16 h32 fails 32x32; Snap usable after scroll proves scroll (Geometry x164 w32 within 162-196 hit true), View usable initial (2D x200 w34 fits), Align truthful no-selection (6 disabled guards, usable unproved). Fix owned-only layout-toolbar.css accordion-only (PCB legacy preserved): Select scope-trigger 40x38->32x32 (height/min/width 32, padding 5px, flex none, icon22+10), Transform guidance single-line 32px truncated readable (flex, nowrap, ellipsis, max 100%, height/min/max 32, padding 0 8px, min-width 32 to keep lane >=32, font 12px), View collapsible min-width 0->32 flex none + trigger min-width 32 flex none (parent w0->96, trigger w16->96). Gortex exact: impact test HIGH test-only / CSS LOW 0, detect MEDIUM 5 files, tests 0/4+0/2 uncovered (oracle), guards none, contract warn test-only + allow LOW CSS. Build under lock -> lane-fix-build3.log EXIT 0. GREEN: 320x480 lane-fix-320-green2.log EXIT 0 (usable Matrix x50 w32 fits 48-82 hit true, guidance x86 w32 h32 within row/track, Canvas w96 passes, Snap scroll still proves, Align still truthful, 7/7 hit, gaps 38/4, lanes 34, 42px, no overflow, panels/Close pass, 0 errors); 390x844/728x1024/1440x900 logs EXIT 0 (lanes 104/255-442/740, headings restored, guidance w101/252/737 readable, usable/icons pass). No RS change, no focused wasm (host unchanged per transfer). 390 needed min-width 32 (lane 22->104, trigger 116->34) as second fix within two-attempt rule; 728/1440 transient EOFs recovered after close --all, not product RED. Remains planned: motion frames, reduced-motion, guard parity, >12 parity, section-persistence, 3D, focused roving, nonzero visualViewport; proof-map rows stay planned; no closure claim.
- 2026-11-11 Lane-label builder (002-lane-label-fix): RED was strengthened oracles (prior 7/7 hit/spacing/42px/overflow/panels intact): 320 `lane-label-320-red2.log` EXIT 1 (guidance truncated scrollW193>clientW32 no tooltip, canvas 96px in 34px track withinTrack false after scroll), 390 `lane-label-390-red3.log` EXIT 1 (guidance 193>101; canvas passes via start scroll x200 w96 in 200-304), 728/1440 `lane-label-{728,1440}-red.log` EXIT 1 (select-labels mode full, 5 icons+labels overflow 32px buttons, flex centers overflow both sides). Corrects lane-fix overstatements: Transform 32px ellipsis was NOT readable (16px content, scrollW193, no alternative), Canvas w96 was NOT whole-usable in 34px lane, Select fixed 32px clipped wide labels, usable checked only track+row not all clippers/effective inert. Fix owned-only: Select CSS responsive (icon-only 32px adaptive without show-label, auto+8px pad when full/show-label, fitter remeasures – at 728 utility labels fold preserving action priority, lane 276->381, buttons 75/65/84/62/64px labelsFit); Canvas CSS max-width 100%+ellipsis (collapsible+trigger+toggle-items – at 320 trigger 96->32 usable via nearest x202, at 390+ full 96 fits and usable); Transform RS Tooltip wrap (existing Tooltip/Trigger via r#as+merge_attributes on p with tabindex 0, Content full text, same condition/text/reason, accordion only, PCB p unchanged) + CSS :has overflow visible when open (harmless, Transform lane has no overflow) + content below/left/clamped/wrapping + focus-visible/cursor. Test: usable now every clipper+viewport+effective dis/inert (withinAllClip), programmatic scroll only (keyboard focus scroll + native touch unproved, do not claim); new canvas/select-labels/tooltip oracles RED first; closes hover tooltips before LANE for steady-closed measure. Gortex exact: impact script HIGH test-only / CSS LOW 0 / RS CRITICAL minimal hunk no sig; detect 6 files 24 symbols CRITICAL (incl prior WIP); tests 0/6 uncovered (oracle + layout suite); guards none; contract warn behavioral high blast 29 (reviewed, no caller break). Build under lock -> lane-label-build-fix1.log EXIT 0. GREEN: 320/390/728/1440 `lane-label-{320-green2,390-green2,728-green,1440-green}.log` EXIT 0 (guidance tooltip focus+hover open matches in-viewport hit at 320/390, inline readable at 728/1440; canvas usable all widths; select labelsFit all modes; 7/7 hit, 42px, no overflow, panels pass). Layout `check.py browser --only layout` -> lane-label-layout-browser.log EXIT 0 (269+2 passed). No workbench_shell.rs change. Remains planned: motion, reduced-motion, guard parity, >12 parity, section-persistence, 3D, arrows/Home/End, keyboard focus scroll, touch, short viewport/keyboard/zoom; proof-map rows stay planned; no closure claim.
- 2026-11-11 Roving-guidance builder (002-roving-guidance): RED was strengthened real-keyboard oracles (prior 7/7 hit/spacing/42px/overflow/panels intact): 320 `roving-guidance-320-red.log` EXIT 1 (4 fails: guidance extra Tab stop zeroCount 2 Transform+P tooltip-trigger, ArrowRight Transform->Align skipping guidance isGuid false tip closed, tooltip from reached focus fails, Handles 64px in 34px track withinTrack false), 390 3 fails (truncated w101 scrollW193, same roving), 728/1440 2 fails each (readable w252/737, extra stop+arrow miss). Corrects lane-label overstatements: Transform p tabindex0 was independent extra stop (not single roving), tooltip proof used imperative p.focus() (not real Tab/arrow), select_label_report vacuous (labelsFit true without 5-count/exact Matrix/Row/Column/Key/Part/visibility/within/overlap), Handles w64 not whole-usable in 34px lane. Fix owned-only: RS guidance TooltipTrigger r#as ToolbarButton index11 (roving, data-slot filtered to preserve toolbar-button, aria full, span role status guidance-text truncation, same condition/text/reason, PCB p unchanged, no sig/API); CSS guidance button border0+inner span min0 flex ellipsis + Handles chain ToggleGroup+Tooltip+Button max100% min32 flex none ellipsis (narrow 64->32 whole, wide 64 full preserved); Test usable excludes guidance, LANE isGuidance+cls200, guidance inner truncation, select strict 5-count/exact aria+full-mode visible text==expected within no overlap texts, roving single-stop + keyboard traversal real ArrowRight/Left/Home/End no focus() + hover-only tooltip + Handles usable via scroll, cleanup includes guidance. Gortex exact: impact CRITICAL minimal hunk no sig (18 direct 29 transitive, guidance branch LOW); detect 6 files 28 symbols HIGH (7/7); tests 0/6 uncovered (oracle + layout suite); guards none; contract warn behavioral high blast 24 (reviewed, no caller break). Build under lock -> roving-guidance-build2.log EXIT 0 (CSS verified site-subpath). GREEN: 320/390/728/1440 `roving-guidance-{320,390,728,1440}-green.log` EXIT 0 (single stop zeroCount1, ArrowRight reaches guidance BUTTON visible 32px, tip open/match/viewport/hit at 320/390 truncated, roving still at 728/1440 readable, hover pass, select exact 5 full visible within no overlap at 728/1440 + adaptive icon-only at 320/390, Handles w32 whole usable via nearest x202 at 320 + full 64 usable at 728+, Canvas still usable, 7/7 hit, 42px, no overflow, panels pass). Layout `check.py browser --only layout` -> roving-guidance-layout-browser.log EXIT 0 (269 passed). No workbench_shell.rs change. Select wide GREEN with strong proof (not vacuous); named future mutation: hide one label in full mode or change Matrix label text, expect SELECT-LABELS labelsFit false (runner test-workbench-viewport.py 728). Remains planned: motion, reduced-motion, guard parity, >12 parity, section-persistence, 3D, keyboard focus scroll for offscreen actions (programmatic scroll only, guidance already visible no scroll needed), touch, short viewport/keyboard/zoom; proof-map rows stay planned; no closure claim.
- 2026-11-11 Handles-tooltip builder (002-handles-tooltip): Corrects implied hover: roving-guidance Handles w32 usable did NOT prove tooltip visible — checker genuine hover showed tooltip 98x32 y36.5-68.5 vs wrapper y76.5-108.5 overflow:hidden zero intersection, center hits Saved locally (handles-tooltip-investigate.log), introduced by WIP Handles chain overflow hidden. Fix owned-only layout-toolbar.css accordion+handles scoped: button anchor-name, tooltip-content fixed+anchor below (top anchor(bottom) left anchor(left) margin 8, nowrap 98x32, clamped), chain visible when open, row/track untouched (preserves scroll). Attempt1 absolute+row/track visible failed (scrollLeft 141→0, button x201→342 offscreen). Gortex exact: impact LOW 0/0/0 (both attempts), detect 6 files HIGH (prior WIP 7/7/0), tests 0/3 uncovered (CSS oracle), guards none, contract allow LOW behavioral. Build under lock handles-tooltip-fix-build2.log EXIT 0 (anchor verified site-subpath). Real packaged 320x480 genuine pointer hover + focus (handles-tooltip-fix-320b.log): closed button 32x32 x201 y76.5 center hits Handles, track x200 w34 scroll141, row 42px; hover open tooltip 98x32 x151.9 y116.5 inViewport+inSection true center hits tooltip itself, button still hits Handles, no page overflow; focus opens same tooltip (focused true). Closed after: state closed, button x201, scroll141. No test files, no RS/ui-shared change. Remains planned: motion, reduced-motion, guard parity, >12 parity, section-persistence, 3D, keyboard focus scroll offscreen, touch, short viewport/keyboard/zoom; proof-map rows stay planned; no closure claim.

- 2026-11-11 Coordinator closure evidence (pre-review): keyboard focus scroll, solid motion, reduced motion, availability, >12-reference parity and 2D/3D section behaviour have receipts (commits 14514ec4b, 4f4523a71). 2D/3D: architecture is silent; the test asserts current behaviour (View section closes after a 3D round trip) — open question for review, not an invented rule. Mutations killed: Follow-off Rotate, first-column Spread (layout crate oracle `column_zero_hides_spread_with_an_accessible_reason`), label fade (`opacity motion`), ignore reduced motion (zero-animations check). Widths 320/390/728/1440 pass. Touch/wheel lane scroll unproved (cheap wheel attempt did not scroll). Combined review pending.
