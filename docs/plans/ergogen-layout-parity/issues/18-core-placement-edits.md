# 18: Implement named placement, inverse moves and exact origin compensation

Status: resolved
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md), [17c](17c-kernel-production-routing.md) (parent: [17](17-cut-over-format-and-model.md))
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Extend existing typed placement/splay seams for Spread, Padding, row/key Shift/Bend,
Column rotation, independent offsets and origin hold-still/swing. Core owns inverses,
exactness and changed IDs; callers do not rebuild matrices with their own geometry.

## First RED / GREEN slice

Fail a public prepared edit that moves a key in a turned/curved column by its requested
world delta without moving its later keys. Implement the offset-frame inverse; then
add one carried Shift/Bend case and its preview/cancel/Undo behavior.

## Subsequent acceptance

- Field, gesture and nudge inputs resolve against accepted state. Preserve companion
  carry/constraint precedence and independent key identity, changed IDs and no-op rules.
- Column rotation affects only its column; switching Splay scope changes no value.
- Origin hold-still uses this/next step compensation, with matrix Origin for column 0;
  cover Column rotation, previous Splay, Bend, mirrors and nonzero custom origins.
- Inverse-solve a dragged/Picked final origin after compensation, rather than merely
  transforming its initial pointer delta. Fields/nudges hold; Ctrl swing writes origin
  only and is locked at pointer-down; Reset holds and Ctrl base snap swings.
- Implement reusable Core rigid-column translation for Align, with the correct next
  step compensation under Splay, not column/per-key offset scattering under Bend.
- Refuse nonrepresentable exact requests without changing state. Keep Local ownership
  promises; anchor/linked paths in 19–21 extend the same implementation, not another
  inverse. Preserve explicit Spread/pitch behavior and one operation/Undo per edit.
- Keep Y-only Convert exact eligibility: no Splay, Column rotation or carried Bend;
  transfer to this Stagger, subtract next, clear offset. Core owns the blocked reason.

## Checks

Extend `core/src/matrix/{placement,splay,layout}.rs` and existing placement, companion,
splay reconciliation and part-offset tests through public Core/Session routes. Run
focused `boardstudio_core` tests, then quick checks. Private numeric tests supplement,
never replace, prepared-route preview/commit/refusal/Undo evidence.

## Progress

### Parallel implementation claim — 2026-10-23

Claimed for an isolated implementation lane. Acceptance, TDD, review and gates are
unchanged; integration owns shared baselines, test aggregation, map and closure.

### Isolated implementation progress — 2026-10-23

Worktree `t3-d32e0528`, branch `feat/core-placement-edits`, claim base
`964762b519d9c57adf9cab3dda2683c2560fda08`. Status remains claimed; parent owns
independent review, shared baselines, integration and closure.

- Added named Spread/Padding and row/key Shift/Bend intents to the existing placement
  seam, with finite/bounds checks and no-op preservation. Independent key world moves
  already used the kernel inverse; the first regression found excessive changed IDs,
  not an incorrect inverse. Reports now retain only moved members and the matrix ID.
- Added relative-origin hold and world-origin swing intents. Hold/Pick solves through
  the published fully turned column frame, so a representable partial-Follow request
  is no longer refused by unrepresentable unit probes. Compensation retains the agreed
  this/next step arithmetic; swing writes only the relative origin. Column 0 retains
  its mirror-aware incoming X axis in the kernel, without adding a Spread handle.
  Unchanged placement and splay requests preserve authored records, revision and history.
- Exact rigid translation retains Local ownership guards and checks the resolved
  candidate against the requested positions/headings before publication. Ordinary
  internal constraints translate without retuning; nonrepresentable results are refused.
  Existing column translation passes curved,
  splayed, turned and X/Y-mirrored cases without scattering independent offsets.
- Real Core accepted scene/document, native prepared pair and Session
  preview/cancel/persist/commit/Undo/Redo coverage lives in existing test modules.
  Focused native checks passed (27 placement, 14 kernel, 11 splay, 9 companions, 20 part-offset,
  one native prepared pair, one Session test). No shared aggregator or fixture/golden
  changed. New native test names need parent baseline reconciliation; no new WASM tests.

Local plan, RED/GREEN logs, exact commands and final gate status are recorded under
`.evidence/ergogen-18/` in the assigned worktree. Follow-on UI/lifecycle tickets and
parent merged gates remain outside this implementation.

Changed-file quick stopped at the retained native boundary golden:
`transcript.core[4:preview].bodyDigest` is `6579:bd874ef70034195d`, expected
`6684:6191b7cfa510ee5e`. The 105-byte reduction matches removing five unchanged
member IDs from the key-move preview. The fixture is unchanged for parent review;
no blessing or baseline-update flags were used. Repo checks, native/WASM Clippy
and page typecheck passed; the receipt records 2,008 native passes, one golden
failure, zero missing retained names and 15 new names within the executed scope.
The CSS/browser phase did not run. See `.evidence/ergogen-18/quick.json` and its
summary. Production source stayed unchanged after that run; the native prepared
test then gained a mirror-side Swing assertion with separate focused verification.

### P2 and semantic golden follow-up — 2026-10-23

Independent Spec review of `9d6cc4c6` found that its blanket constraint-ownership
guard refused representable internal Offset constraints. Public Matrix/Column
translation previews failed RED on that refusal. The shared final-candidate check
now runs after the existing solver in commit and both preview routes. Tests prove
requested rigid movement, unchanged constraint records and independent parameters,
neighbour/member identity, preview nonmutation and exact Undo/Redo. The external
constraint test is clarified and strengthened to check both refusal phases/history.

The additional native linked case exposed the pre-existing membership refusal in
`mirrored_half/reconcile.rs`. Parent explicitly retained that contract for ticket 20.
Its identical inputs now characterize the exact refusal and complete state/history
preservation. This does not implement native constrained Align or waive acceptance;
the next independent review owns the scope/dependency boundary. No native membership,
companion ownership or Open admission code changed in this follow-up.

Fresh controlled replay compared all 130 raw Core replies with the claimed-base
provider; that provider reproduced the complete original golden through the existing
native replay/compaction path. Kinds, revisions and refusal reasons are unchanged.
The only semantic differences are precise changed IDs in preview/commit and origin
inverse rounding (maximum `3.02e-14`) inherited by 12 subsequent replies. All 67 primary
IDs/order, 11 archive cases and four compound summaries are preserved. Shared semantic
assertions now run before compaction/blessing. Supported `BOUNDARY_BLESS` regeneration
and unblessed replay passed, with seven boundary, 29 placement, 14 kernel, 11 splay,
nine companion, 20 part-offset, one native scope-characterization and one Session test.

Detailed field-by-field rationale, source/runtime evidence, exact commands and final
fresh quick result are under `.evidence/ergogen-18/golden-*`, particularly
`golden-notes.md` and `golden-quick.json`. Earlier failed quick remains historical
failure evidence, not acceptance. Status remains claimed; source changes after the
pinned review require parent follow-up review, integration and merged gates.

### Independent review response and parallel integration preflight — 2026-10-23

The independent combined Spec and Standards review passed pinned candidate
`1a5385b0ef709468b02fb174f6607118960dfce4` with no blockers, including the P2 ordinary
representability repair and 130-reply semantic golden audit. This approval is for that
pinned source; the isolated integration preflight adds comments and this progress note
only. Status remains claimed pending parent-owned fresh baseline and merged gates.

The linked-pair constraint-driven rigid Align refusal characterized by
`production_linked_rigid_translation_retains_membership_refusal` is pre-existing policy,
and ticket [20](20-reflect-linked-layouts.md) is not scheduled to lift it: its acceptance
explicitly preserves constraint refusals. Ordinary prepared native Align works. The
linked case verifies the exact refusal and state/history preservation; it is not evidence
of native constrained Align support. A dedicated unlinked prepared-half/internal-constraint
case was not added; actual public Core Matrix/Column preview, commit and Undo coverage
remains the evidence for ordinary representable constraints. Changing linked refusal
policy needs an explicit future scope decision. Golden deltas are justified by the inverse
helper's Follow behavior, not by the `3.02e-14` rounding magnitude alone.

## Answer

Integrated the independently reviewed source `1a5385b0ef709468b02fb174f6607118960dfce4` with ticket 25 at merge `f2e53b7b04867e3e413da47757f4ebc602df6350`. Documentation/comment polish is `da4412f4eaa876e6ba270398d4c63de490bb0d0a` and `e50bcc8167bcab1c16d61bddfcb1898a49575500`; the supported native-baseline update is `04cbd057a003`. Fresh merged verification ran at tested HEAD `04cbd057a00367036222c6703a61030157b5abbb` (full SHA recorded by Git).

- The independent combined Spec and Standards review passed the source pin with no blockers. The minor tolerance suggestion was not adopted; the reviewed exactness assertions and 130-reply semantic golden audit remain intact. The final origin inverse delta is explained by Follow/frame reuse, not by its magnitude alone.
- The supported updater passed with 2,172 native tests. The baseline grew from 2,154 to 2,172: exactly the 18 names in the author inventory, with zero removals or renames. Receipt: `.evidence/ergogen-18-integration/native-baseline.json`.
- Fresh quick passed 48/48 commands: 2,016 selected native tests passed, zero retained names missing/new, and all 14 WASM launches passed (598 tests). Receipt: `.evidence/ergogen-18-integration/quick.json`.
- Fresh full passed 55/55 commands: 2,172 native tests passed with 2,172 retained names matched and zero missing/new; all 14 WASM launches passed (598 tests); the page suite passed 77/77. Packaged trusted drag/Undo passed for Part, Column, Stagger and Ctrl+Offset; collision smoke observed four PCB pairs; CAD-provider and CAD-browser checks passed. Receipt: `.evidence/ergogen-18-integration/full.json`.

Known limit: the linked-pair constraint-driven rigid Align route retains its pre-existing membership refusal for ticket 20; this is not native constrained Align support. Ordinary public Core Matrix/Column preview, commit and Undo cover representable internal constraints. A dedicated unlinked prepared-half/internal-constraint case was not added. No ticket 20 policy or release approval is implied.
