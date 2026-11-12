# Map: Core simplification

Label: wayfinder:map
Status: **decisions D1–D4 made by the user on 2026-11-12. Tickets come next; implementation follows the branch deletions and the toolbar work.**
Base: `cleanup/remove-dead-weight`.

## Destination

Make the mirrored half a one-way reflection with typed per-item states, run every Core edit
through one transaction, and remove the second ways of storing the same thing. The target for
`core/src/mirrored_half/` is roughly 12–15k lines including tests, down from 49k (estimate;
slice 1 alone removes about 8.2k measured lines).

The product intent, confirmed by the user on 2026-11-12:

- Edits to the source half flow to the mirrored half.
- The mirrored half may differ in small ways: an item can be left out, or changed locally.
- Edits do not flow from the mirrored half back to the source.

[ADR-0010](../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) and
[ADR-0008](../../adr/0008-linked-halves-stay-linked.md) describe the current design. Slice 1 writes the
ADR that supersedes ADR-0010's inverse-mapping and exception-overlay clauses.

## What exists today

Measured or read from code on 2026-11-12.

| Part | Source lines | Test lines | Job |
| --- | --- | --- | --- |
| `reconcile.rs` and `reconcile/outlines.rs` | 8,255 | 14,333 | Re-derive linked mirror items from the source after an edit |
| `lifecycle.rs` | 4,200 | 5,783 | Local, omitted, rejoin and unlink transitions |
| `inverse.rs` and `inverse/` | 3,604 | 4,579 | Map an edit made on the mirror back onto the source |
| `reconcile/prepare.rs` | 1,951 | 2,959 | Orchestrate inverse, apply, lifecycle, reconcile and validation |
| `matrix_addition.rs` | 1,076 | 1,329 | Create the mirror of a newly added matrix |

Facts the design rests on:

- **The mirror is stored in full.** ADR-0010 chose this because the UI read stored parts in about
  282 places. Consumers still read mirror items as ordinary document items.
- **"Linked" is not stored.** `MirroredHalves` holds a `correspondences` list and a separate
  `exceptions` list keyed by kind and mirror id (`core/src/model.rs:1549-1601`). An item is linked
  when it has no exception. Nothing in the types ties an exception to a correspondence, so
  `validate_halves` (`core/src/mechanical/halves.rs:551-705`) checks at runtime that every
  exception has one, that a local item exists and that an omitted item is absent. About 240 tests
  named "refuses" exist to cover those checks and the "nothing changed" half of each refusal.
- **Eleven item kinds participate**: matrix, layout, part, outline, net, assembly, module,
  electrical board, outline version, mechanical feature and embedded circuit.
- **Mirror-side edits are accepted today.** `prepare_transaction_with_ledger` calls
  `inverse::canonicalize` on every edit; an edit to a linked mirror part, constraint or outline is
  rewritten as an edit to the source and then reflected forward.
- **There are two mirror models.** Two-board halves use `hardware.pairs[].halves`. Same-board
  layout pairs use `Layout.mirror_link` with its own correspondence and exception lists, kept in
  step by `sync_components`, `ensure_part_mirror`, `adopt_half_frames` and `unadopt_pair` in
  `core/src/matrix/layout.rs`. Extra code exists only to refuse documents where the two overlap
  (`validate_no_native_legacy_overlap`, `overlaps_legacy_link`).
- **The acceptance suite is separate from the module.** `core/tests/mirrored_half.rs` and its
  sub-directory hold 110 tests in about 11k lines, and `core/tests/split_*.rs` hold 34 tests in
  2.4k lines. They drive `CoreEngine` only, so they can stay as the oracle while the module
  changes underneath.

## Design

### 1. A correspondence carries its own state

```rust
struct MirroredCorrespondence {
    kind: MirroredItemKind,
    source_id: String,
    mirror_id: String,
    state: MirrorState,
}

enum MirrorState {
    Linked,
    Local,
    Omitted { removed_copper: Option<OmittedCopper> },
}
```

The `exceptions` list goes away. An exception without a correspondence, a duplicate exception and
a local item recorded as omitted can no longer be written, so their validators and tests go with
them. This changes the saved shape, so the format version becomes 8 and format 7 is refused, as
older formats already are. Demo projects are generated in Rust and follow the model; the nine
format-7 JSON fixtures are rewritten.

### 2. Reflection runs one way

After an accepted edit, Core regenerates every `Linked` mirror item from its source item and
leaves `Local` and `Omitted` items alone. That is the existing forward `reconcile`, kept.

`inverse` is deleted. An edit that targets a `Linked` mirror item needs a new rule; see D1.

### 3. One transaction for every edit

```
accepted document → clone → apply operation → reflect → validate → commit, or discard the clone
```

`mechanical/edit.rs`, `reconcile` and `lifecycle` each stage their own draft and prove their own
atomicity today. With one transaction in `CoreEngine`, a refusal can never leave a partial write,
and one property test ("a refused operation returns the accepted document unchanged") replaces
the per-case "without mutating" assertions.

### 4. Keep the stored mirror

Computing the mirror on read would remove more code inside Core, but every consumer would have to
read a projection, and the scouts could not bound that work: ADR-0010 counted about 282 reads in
the UI alone. Steps 1–3 get most of the reduction without touching consumers. Projection stays a
later option; this plan does not depend on it.

### 5. One mirror model

Same-board layout pairs and two-board halves reflect the same kinds of item. Subject to D2, both
use the correspondence type and the reflection of steps 1–2, and the overlap refusals go away
because a layout can no longer be claimed by two models.

## Decisions

All four were answered by the user on 2026-11-12; each answer follows its question.

**D1. What happens when someone edits a linked item on the mirrored half?**
Recommended: refuse the edit and say why, with one offered action, "Make local", which sets the
item to `Local` and then applies the edit. The alternative is to make the item local silently on
first edit; that is one step shorter for the user and easier to do by accident.
**Decided: refuse, and offer "Make local".**

**D2. Are both mirror models needed as products?**
Two-board halves are a split keyboard. Same-board layout pairs are a mirrored layout on one PCB.
Recommended: keep both as products and give them one implementation. If same-board pairs are not
needed, `Layout.mirror_link` and about 1.5–2k lines in `matrix/layout.rs` can be removed outright.
**Decided: keep both products on one implementation.**

**D3. Which second storage forms go?** None of these is old-project support; each is a live
alternative inside the current format. Removing one is a product change, and the shipped demo
projects and the catalogue must be checked for each shape before it is deleted.

| Form | Where | Recommendation |
| --- | --- | --- |
| Literal (non-framed) constraint records | `constraints/relation.rs` `from_legacy`, `endpoints.rs` `compile_legacy` | Remove; store framed records only |
| `Legacy` construction ownership and the `MaterializeLegacyConstruction` edit | `mechanical/edit.rs`, `inputs.rs`, `change.rs` | Remove |
| Text where a generator parameter expects a number | `generators.rs` `is_legacy_text` | Remove; reject on open |
| Missing `formatVersion` defaulting to 1 | `model.rs` `legacy_format_version` | Remove; make the field required |
| Per-axis matrix arrays in a test fixture | `matrix.rs` `grouped_from_legacy` | Remove after rewriting `matrix-projections.json` |
| KiCad's older arc form, MX stabilizer footprints, `&kp` bindings | `artifact/`, `mechanical.rs`, `keymap.rs` | Keep; these are external formats |

**Decided: the recommendations in the table stand.**

**D4. Which mirror features stay?** Each is beyond "source drives mirror, with local and omitted
items", and each can be dropped independently.

- Companion placement modes and per-item tweaks (`MirroredCompanionMode`, `MirroredCompanionTweak`).
- Saved copper that is restored when an omitted item rejoins (`OmittedCopper`).
- Constraint endpoint retargeting when an item changes state (`lifecycle.rs`).

Recommended: keep the first, since it positions real parts, and drop the other two unless you
rely on them.
**Decided: keep companion placement; drop saved copper restore and endpoint retargeting.**

## Slices

Each slice leaves `core/tests/mirrored_half*` and `core/tests/split_*` passing, apart from tests
of behaviour the slice removes on purpose, which it lists.

1. **Remove inverse mapping.** Delete `inverse.rs` and `inverse/`, add the D1 rule in
   the transaction path, and update the UI surfaces that edit mirror-side items
   (`existing_half.rs`, `parts.rs`, the part and outline inspectors).
2. **Typed correspondence state and format 8.** Change the model, rewrite fixtures, delete the
   exception validators and the tests that construct invalid exception lists.
3. **One transaction.** Move staging into `CoreEngine`; delete per-module staging and the
   per-case atomicity assertions; add the property test.
4. **Fold `lifecycle` into state transitions** on the typed correspondence, with the D4 outcome.
5. **One mirror model.**
6. **Remove second storage forms**, one per commit, each with its fixture rewrite.
7. **Split what is left by concern**: `mechanical/edit.rs` (5.9k lines, 88 functions) and
   `matrix/layout.rs` (4.9k lines, 97 functions).

Slices 1 and 2 are independent and give the largest reduction. Slice 3 should follow slice 1,
because inverse is the main reason `prepare.rs` orchestrates so many stages. Tickets with proof
maps are written per slice before it is built.

## Open fog

- No count yet of consumers that read mirror items directly, so the cost of step 4's alternative
  is unknown.
- The catalogue and the demo project constructors were not read in full, so D3 needs a per-shape
  check before each deletion.
- Whether any UI flow depends on mirror-side edits reaching the source beyond replacing a
  component on one side and adding a mirror-only part.
