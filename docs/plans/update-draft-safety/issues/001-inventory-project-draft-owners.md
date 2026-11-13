# 001: Inventory project draft owners

Status: resolved
Type: task
Label: planning:task
Mode: AFK
Assignee: Chris D
Blocked by: none
Parent: [Draft-safe update adoption](../map.md)
Related: [Draft-safety research and change map](../../../research/2026-11-07-019-draft-safety-change-map.md)

## Question

What is the source-grounded census of live project-editing draft owners, and which
values would be lost on reload without ever reaching pending work or persistence?
The facts unblock selection of the draft-safety interface; this is not an implementation
or a claim that every owner already has protection.

## Evidence to obtain

- Start from the two confirmed explicit-submit owners and the contrasting controls
  in the existing research. Cover Layout, PCB, Keymap, Keycaps, Case, Parts, setup,
  project/board metadata and dialogs; classify navigation/search/filter text separately.
- For each logical owner, record production mount and scope/lifetime, value and
  accepted baseline, input/change behavior, blur/focusout/key/Apply/Save admission,
  validation/invalid markers, pending-edit/save route, and unmount/hidden retention.
- Identify existing `PendingEditSignals`, `OwnedEdits` or other real seams and which
  local values they do not expose. Distinguish source-confirmed behavior, graph gaps
  and browser-dependent ordering; cite exact paths, spans and Gortex symbol IDs.
- Produce one linked inventory asset with a checked coverage denominator and explicit
  unknowns. Do not infer missing owners from no-match graph results. Recheck current
  dirty source; do not copy old counts as fresh evidence.

## Completion and boundaries

Resolve when the census and named remaining unknowns are sufficient to compare
interfaces. If materially blocked, record the exact evidence gap and leave this ticket
unresolved; a blocker transfer is not completed evidence. This is local fact collection:
no product changes, no global registry design and no acceptance pass.
Claim before collection. Keep the answer in this ticket, link the asset, and return
new concrete questions to the map rather than silently widening the task.

## Answer

Resolved 2026-11-07. The census is in the merged asset
[001 census: project draft owners](../inventory/001-draft-owner-census.md),
with per-owner detail in its four linked fragments (Layout+Geometry+metadata,
PCB+Parts, Keymap+Keycaps+Case, Setup+dialogs+seams). Denominator: every
workspace's production editor files enumerated per fragment (Layout 20
files/sections with a verdict, PCB+Parts 56 files, Keymap+Keycaps+Case 50
files, plus Setup/dialogs/seams surfaces), each owner classified A–E/U with
mount/lifetime, baseline, admission, invalid markers, pending route and
retention.

Headline facts for the interface question:

- **12 Class A owners** hold project values in local signals with explicit
  Apply/Save/Add admission only, no blur commit and no gate visibility —
  the two research gaps (Layout constraint composite, Geometry scripts)
  plus legacy position, companion placement, Parts generator parameters,
  module profile, mechanical fit profile, assembly draft, PCB
  mounted-module placement, module-attach options, new-net name, and the
  New matrix creation form (no accepted baseline; its protected-set
  classification is left to the interface ticket). `PendingEditSignals`
  binding does not change this: it observes only post-`begin_field` state.
- **22 Class B owners** commit on blur/focusout/Enter (ordering risk only);
  17 C groups commit immediately; 16 D rows are preview/post-submission
  only; search/filter/navigation/preference text is E and excluded.
- The gate's full current view is `Runtime::update_safety_inputs`
  (runtime.rs:930–946) plus the `aria-invalid` DOM query; neither
  `PendingEditSignals` (no baseline, no key enumeration, no registry) nor
  `OwnedEdits` (one controller only) exposes an unsubmitted draft to it.
- This is source-confirmed classification, not observed A→B loss; that
  probe is [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md).

Named remaining unknowns are sufficient to compare interfaces but not to
claim completeness; they are graduated, not left as fog, into
[Close the named census gaps](007-close-named-census-gaps.md), which now
also blocks the interface ticket. No product code changed; no tests,
builds or browser runs were performed in this ticket.
