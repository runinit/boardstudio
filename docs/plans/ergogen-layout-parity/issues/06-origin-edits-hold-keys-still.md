# 06: Decide how origin edits hold keys still

Status: resolved
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

Moving a column's splay origin keeps every key still by default, as it does today. A modifier
swings the keys about the new origin instead, as ergogen does. Today the compensation is written
as column offsets on this column and every later column (`splay.rs::preserve_columns`).

With the origin relative to the column base, decide:

- where the hold-still compensation is stored. For example, a translation folded into this
  column's spread and stagger moves later columns and their origins rigidly, so only one column's
  values change;
- which fields visibly change when the origin moves, and how that reads in the Inspector;
- the modifier, and the Undo granularity;
- the Origin tool's "Reset to column base" behaviour.

The same origin also turns the column's **Column rotation**
([ticket 05](05-this-column-splay-representation.md#answer)), so holding keys still must cover it
too.

## Answer

Decided with the user on 2026-10-22 in one grilling round; every recommendation was accepted.

Today every origin edit (drag, Pick origin, X/Y fields, the "Column base" choice) holds keys
still by writing compensating column offsets on this and every later column. There is no swing
modifier.

### Why step fields can hold keys still

In the [ticket 04](04-layout-kernel-order-of-operations.md#answer) kernel, take column c with
its own turn θ = Splay + Column rotation. Moving its relative splay origin by Δ, measured in the
column's incoming frame, moves:

- column c's keys rigidly by (I − R(θ))Δ;
- every later column rigidly by (I − R(Splay))Δ.

Both are translations, so changes to the step fields cancel them exactly, whatever the rows'
Shift, Bend and padding. A random check of 2,000 layouts held every key within `4e-14` (gitignored
`.evidence/ergogen-layout-parity-06/hold.py`). Column offsets cannot do this, because they act in
each key's carried frame and stop being rigid once rows have Bend.

### Decisions

1. **Compensation goes into step fields.** A hold-still edit adds −(I − R(θ))Δ to this
   column's Spread and Stagger. When this column has a nonzero Column rotation, it also adds
   (I − R(Column rotation))Δ to the next column's Spread and Stagger. Nothing else changes.
   Edits that would produce a non-finite value are refused.
2. **Column 0 writes the matrix Origin.** Column 0 has no Spread (ticket 04, decision 12), so
   the whole translation goes into the matrix's Origin, in its anchor or world frame. For
   linked halves, the follower must also hold still. That needs a mirrored Origin change on the
   follower, and the rule is handed to the map's mirrored-halves fog.
3. **Hold-still pins Spread.** Compensation stores an explicit Spread, so later pitch changes no
   longer move that column. The Inspector shows the Spread as explicit, and it can be cleared
   like any other value.
4. **Inspector.**
   - Origin X/Y show the stored relative value (from the column base, in the column's incoming
     frame), not world coordinates.
   - The note reads "Moving the origin keeps keys still by adjusting this column's spread and
     stagger."
   - After each edit, a status line names every field that changed, including the next
     column's.
5. **The swing modifier.** Ctrl held at pointer-down swings the keys about the new origin
   (ergogen's behaviour: only the origin is written). This matches Ctrl turning a Stagger drag
   into an Offset.
   - The choice is locked for the drag, and the drag readout shows "Swing keys".
   - Pick origin follows the same rule: Ctrl-click swings.
   - Alt still means "no snap".
   - Arrow-key nudges and the Inspector X/Y fields always hold keys still.
6. **Undo.** Each origin edit, swing or hold-still, is one Core operation and one Undo entry,
   covering the origin and every compensation field. A drag commits once, on release.
7. **Reset to column base.** The "Column base / Custom point" select is removed, since 0 now
   means the base. A **Reset to column base** button appears when the origin ≠ 0 and holds keys
   still. The column base becomes an origin snap target, so a Ctrl-drag onto it resets
   ergogen-style, with the keys swinging back.

### Follow qualification (2026-10-23)

[Ticket 11](11-mirrored-halves-rules.md#answer) settles the linked-frame rule and one
representability limit: if column-0 compensation requires translation in a direction
discarded by Follow, refuse the hold-still edit and explain which Follow component to
enable, or offer explicit Detach. Never silently enable Follow, detach or scatter
compensation across keys. Ctrl swing remains available. Otherwise the single Core edit
must keep the promised poses on both halves, using their actual anchor frames.
