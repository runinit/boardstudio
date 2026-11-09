# 001 census: project draft owners (merged asset)

Authoritative merged census for
[Inventory project draft owners](../issues/001-inventory-project-draft-owners.md),
2026-11-07, against the dirty `fix/ui-integration-recovery` worktree (Gortex
overlay `01a1b3cf`, disk-verified reads). Detail lives in the four fragments;
this asset is the checked summary, totals, denominator and unknowns. It is a
source census, **not** browser loss proof and not an interface decision.

Fragments:
[Layout + Geometry + metadata](001-layout-metadata.md) ·
[PCB + Parts](001-pcb-parts.md) ·
[Keymap + Keycaps + Case](001-keymap-keycaps-case.md) ·
[Setup + dialogs + seams](001-setup-dialogs-seams.md)

Class key: **A** explicit-submit draft, no blur admission, no gate visibility
(confirmed unprotected in source) · **B** blur/focusout/Enter-committing
(admission on focus move; ordering risk only) · **C** immediate commit on
change/select · **D** preview-only or post-submission pending only ·
**E** disposable navigation/search/filter/preference (charter-excluded) ·
**U** unknown/browser-dependent.

## Totals (logical owners, fragments as counted)

| Class | Layout/meta | PCB+Parts | Keymap/Keycaps/Case | Setup/seams | Total |
|---|---|---|---|---|---|
| A | 5 | 7 | 0 | 2 (creation forms) | **14** |
| B | 10 | 4 | 8 | (L16 counted in Layout) | **22** |
| C | 3 | 9 | 7 groups | 0 | **19** |
| D | 2 | 10 | 4 | 0 | **16** |
| U rows | 0 | 1 | 2 | 0 | **3** (all browser-dependent, below) |

007 update (2026-11-07): [Close the named census gaps](../issues/007-close-named-census-gaps.md)
reclassified every named unread region/lifetime below; totals above are
post-007. Two new Class A owners promoted (L20 existing-half, S2b
mirrored-pair creation form); no other A promotions.

E groups are listed per fragment and not totalled (they are excluded by the
approved charter, not a safety surface).

## Confirmed unprotected owners (Class A)

Editing an accepted value, lost on reload if the gate passes (source-confirmed;
actual A→B loss is ticket 002's probe, not claimed here):

1. **Layout constraint composite** (L1) — Save/Add constraint only.
2. **Geometry scripts name/source/enabled** (L10) — Apply script only; script
   switch silently replaces an unsubmitted draft.
3. **Legacy position Inspector X/Y** (L8) — Enter/Apply only; live preview is
   not admission (preview-settle window remains U).
4. **Companion placement mode/tweaks** (L15) — Apply placement only.
5. **Parts generator parameters** (A1) — Apply generator settings only;
   changes fire a preview, never an edit.
6. **Module profile + rotary + volume** (A2) — Save project module profile only.
7. **Mechanical fit profile + contour** (A3) — Save fit profile only;
   Cancel/close discards silently.
8. **Assembly draft (name/members/origin/models)** (A4) — Save assembly /
   Place / Apply-to-matrix only; Close drops the draft.
9. **PCB mounted-module placement + connections + support** (A5) — Save
   placement only; `PcbInput` cannot express `aria-invalid` at all.
10. **Module-attach options** (A6) — Attach module only.
11. **New-net name** (A7) — form submit only; signal lifetime is U1 below.

Creation-form sub-case (no accepted baseline; baseline is form defaults):
12. **New matrix form** (S2) — Create matrix only; whether creation forms are
    in the protected set is a classification for
    [Choose the draft-safety interface and lifetime contract](../issues/004-choose-draft-safety-interface.md),
    recorded here, not decided.

Promoted by 007 (explicit-submit, no blur admission, no gate visibility):
13. **Existing-half setup** (L20) — mirror-axis text + source-layout choice
    are local signals; submit only via **Create linked half** form submit.
14. **Mirrored-pair Add-matrix form** (S2b) — left/right names + gap are
    local signals; submit only via **Preview placement** form submit
    (creation-form sub-case, same ticket-004 classification question as S2).

Pattern across all 14: local signals hold the value; `PendingEditSignals`
(where bound at all) only observes state **after** `begin_field`/`begin_one_shot`;
nothing compares the live draft with the accepted baseline for an outside
reader; no `aria-invalid` marks the unsubmitted value. This confirms and
widens the research note's two gaps
([Draft-safety research and change map](../../../research/2026-11-07-019-draft-safety-change-map.md)).

## What the gate sees today

`Runtime::update_safety_inputs` (`web/crates/runtime/src/runtime.rs:930–946`)
assembles lifecycle/recovery, storage, Session pending work, durability vs
accepted revision, save buffer/conflict, and `visible_invalid_draft()`
(281–293, a DOM `aria-invalid` query). `update_safety`
(`web/crates/runtime/src/update_safety.rs:139–174`; gap recorded at 25–27)
has no dirty-draft input. Seams available to a future interface, and their
limits, are tabulated in the Setup/seams fragment: `PendingEditSignals`
(no baseline, no key enumeration, no registry), `OwnedEdits` (one
controller/owner only), Runtime `PendingEdits` (post-submission only).

## Coverage denominator

- Layout/metadata: 20 files/sections searched with a verdict (3 negative),
  plus a named not-fully-covered list (fragment §Coverage).
- PCB+Parts: 56 files enumerated (29 PCB + 27 Parts): 14 files own draft
  inputs, 21 production files have none (reason per file), 9 test/example/
  wiring, remainder logic cited as evidence.
- Keymap+Keycaps+Case: 50 files enumerated (18+8+24); 20 files/regions
  source-confirmed with or without owners; a repo-wide `use_signal` sweep
  found no other draft-shaped signals in scope.
- Setup/dialogs/seams: setup workspace/guide, matrix/board setup, catalogue
  setup logic, Export, command palette, Library/Parts search, dialog
  primitives and the update warning searched (fragment §S1–S8).
- Mount reachability is source-read, not graph-proven: Gortex `callers`
  returned no/inferred edges for several editor components (e.g.
  `LayoutComponentInspector`); production mounts are cited per fragment.

## Named remaining unknowns (not counted as safe)

Status after 007 ([Close the named census gaps](../issues/007-close-named-census-gaps.md),
2026-11-07): every source-settleable item is resolved in the fragments.

1. ~~Unread owner-shaped regions~~ — resolved: L7 controls are immediate
   (C); `keycap_size` immediate (C), `layout_align` view/one-shot (D/E),
   `existing_half` is new Class A L20; outline editors beyond L13/L14 add
   no owner (B/C/D sub-cases); both matrix controllers' owner/currency
   guards confirmed; mirrored-pair form is Class A creation form S2b;
   Case S5 pickers are immediate (C); Keycaps `lib/scene/fit/finding_marker`
   hold no draft owner (E/no-owner); macro-name `TextDraft` has
   `trim_on_commit` unset (false) and no Escape handler (B stands).
2. ~~Untraced lifetimes~~ — resolved: U1 new-net name is owned by
   `PcbWiringInspector`, survives part switch and panel collapse, drops on
   workspace switch/unmount (A7 retention clarified, no new owner); U2
   constituent rows are placement initiators (D), not part of A5; U4
   `settle(true)` is intentional (child remounts per model owner, so
   mounted ⇒ live; helper Scope lineage still guards landings); L1
   accepted updates **clobber** an open constraint draft (no dirty/pending
   guard — retention gap, stays A); L10 has **no guard** on script switch
   (draft silently replaced, stays A).
3. Browser-dependent, still U (ticket 002's territory): native `type=number`
   sanitization making unparseable drafts untypable (macro NumberDraft,
   PCB/Parts numerics), combobox-portal focusout (KeycodeField), Enter→blur
   ordering, and programmatic unmount-without-blur dropping B-class drafts.

## Validation

- Each fragment: Gortex MCP `search`/`read`, exact dirty-overlay views,
  symbol IDs + line spans; orchestrator spot-checked A1/A2/A5 admission
  buttons at their cited lines (`Apply generator settings`
  generator_settings.rs:1359, `Save project module profile`
  module_profile_editor.rs:807, `Save placement` pcb_module_inspector.rs:921).
- 007 additions: three Spark delegates (Pi `meta/muse-spark-1.3-contributor`)
  classified the named gaps per fragment; orchestrator re-verified via
  Gortex the L20 `ExistingHalfSetup` admission (local axis/source signals,
  form `onsubmit` only), the L1 `accepted_constraint` clobber effect
  (layout_component_inspector.rs:974–1024, no dirty/pending guard), and
  `MirroredPairForm` (mirrored_pair.rs:126–286, form submit only).
- No product code changed; no tests, builds or browser runs in this ticket.
