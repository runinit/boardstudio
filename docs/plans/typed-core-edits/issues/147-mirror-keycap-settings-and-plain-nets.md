# 147: Mirror keycap settings and plain nets after creation

Status: resolved
Type: build
Blocked by: [112](112-inverse-map-hosted-electrical-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [115](115-prepare-mirrored-edit-transaction.md), [111](111-integrate-mirrored-half-reconciliation.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Every item on a mirrored board must be mirrored. Today reconciliation does not project keycap
settings after creation, nor non-circuit (plain) nets, so an edit to either changes one half only
(found in 112's review). The two halves must stay in step.

## Rules

- Keycap settings (board, scalar and legend payloads, matrix keycap choices) and plain nets
  follow their source through linked correspondences, like every other item.
- **Omission is allowed.** An item the user omits from the mirrored pair is absent on the mirror, and
  the nets that exist only because of it are removed with it (ADR-0010's private-net rule: a mirror
  net whose source net has no non-controller pin outside an Omitted item is absent). Local items keep
  their own value on the mirror.
- Key bindings are still never mirrored; legends are copied as written.

## What this ticket owes

- Forward projection for keycap settings and plain nets in the reconciliation kernel, with
  idempotence, exceptions (Local/Omitted) and the private-net rule.
- Inverse mapping already exists in 112; add the round-trip cases it could not test without
  forward projection (mirror-side keycap/net edit reaches the source then the mirror exactly once).
- Net registration on the document's first board only (`SetNet` execution) is a pre-existing
  wrinkle; decide here whether a mirrored net must register on the mirror board.

## Acceptance

- [x] Red-first: keycap board/setting and plain-net pin edits from either half appear on the other.
- [x] Omitted keycap/part: no mirror item, and orphaned plain nets are removed, not left empty.
- [x] Local item keeps its own mirror value across unrelated source edits.
- [x] Exactly-once, idempotent reconcile; unrelated geometry unchanged; mutation check.

## File ownership

Private mirrored-half reconciliation modules and established tests; no production call site before 111.

## Outcome (2026-10-12)

Implemented as a private forward projection in `core/src/mirrored_half/reconcile/settings.rs`,
called by `reconcile` after the matrix, circuit and module stages and before outlines. Tests are in
`reconcile/settings_tests.rs` (22). No call site and no payload change; two fixture helpers of
ticket 145's tests (`source_half`, `create_from`) became visible to the sibling test module. Written
red first: 13 of the first 15 tests failed against a no-op projection before any code.

Rules as implemented:

- **Keycap settings.** The board entry follows the pair's boards (no exception kind of its own).
  A linked matrix or key shows exactly its source's entry, or none when the source has none, so a
  stray mirror entry is removed. Local keeps its own entry. An Omitted overlay, and a linked item
  whose mirror part or matrix is absent (the child of an Omitted matrix or circuit), has no entry.
  Bindings are not touched; legends are copied as written.
- **Plain nets** are corresponded nets that are neither a circuit's own nor a joined host net
  (those stay with the circuit projection, which already owns them). The mirror net takes the source
  pins on linked parts, mapped through the part correspondences, plus the pins the mirror owns
  (Local parts, parts with no source). Pins on Omitted or absent parts are dropped; a pin removed
  from the source leaves the mirror.
- **Private nets.** A net absent by ADR-0010's rule is removed from `nets` and the mirror board's
  `netIds` unless a mirror-owned pin still uses it; it is projected again as soon as it stops being
  private. Copper that referenced a removed net is left alone (Omit already saves what it owned;
  validation does not tie a trace to a net).
- **Local and Omitted nets** are never projected over, and an Omitted net is not resurrected.

Judgement calls:

- **Net names.** A net the mirror shows keeps its own name (ADR-0010: names are for the mirror's
  side; a mirror-side rename already reaches the source through `SetNet`). A net projected again
  takes the source name with the board token swapped, as at creation. Projecting the name every
  time was tried and rejected: it rewrote a fixture's deliberately different mirror name and broke
  the idempotence of the existing circuit-removal test. A source-only rename therefore does not
  travel; if a product decision wants it to, it is one line in `plain_nets`.
- **Registration wrinkle (decided).** A projected mirror net is registered on the mirror board's
  `netIds`; the projection does this itself instead of relying on `SetNet`, which registers a new
  net on the document's first board only. `SetNet` execution is unchanged (outside this ticket's
  files); a net authored on a non-first board is still registered on the first board, which is a
  general editor matter, not a mirroring one.
- **No transitions added.** A new source net, a source net deleted outright, and the pins of a new
  source part get their correspondences through the existing transitions (145 for matrices and
  their nets, 106 for independent parts); this slice only follows corresponded items.
- Pin order is kept when the set of pins is unchanged, so an unrelated edit never reports a net as
  changed.

Round trip (the cases 112 could not test): a mirror-side keycap key edit, a keycap board edit and a
net pin edit each canonicalize to the source, execute once, reconcile to the mirror, and a second
pass is a no-op.

Evidence (`.evidence/ticket147/`, gitignored): 16 mutations (board, Local key, Omitted, absent part,
matrix, key identity, Local net, private-net removal and its mirror-pin exception, stale and owned
pins, registration, name, circuit-net skip, Local pin mapping, call removed). Three survived the
first run (Omitted-with-present-part, absent-part-without-overlay, circuit-net skip) because
`reconcile` refuses those states; direct projection tests were added and all three were then
killed. `check.py repo test lint` passes; the native baseline is addition only.

