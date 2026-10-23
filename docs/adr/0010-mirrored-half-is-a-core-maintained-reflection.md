---
status: accepted
---

# A mirrored half is a Core-maintained reflection

[ADR-0008](0008-linked-halves-stay-linked.md) decided that the second half of a pair is
the first, reflected, in every respect. On 2026-10-09 the user accepted the following
design for how Core stores and maintains it
([decision ticket](../plans/typed-core-edits/issues/16-native-mirrored-halves.md)).

- **Hybrid representation.** One Core operation, `CreateMirroredHalf`, copies and
  reflects the source half and stores the result in full, so every consumer that reads
  stored parts, matrices, outlines and wiring keeps working. The pair record keeps one
  map, a construction source, a correspondence table (source id to mirror id) and the
  per-item exceptions. After every accepted edit, and on Open, Core re-derives the mirror
  from the source; an edit made on the mirror is inverse-mapped to the source.
- **One pair record.** The mechanical pair is extended with `constructionSource` and
  `halves`. There is no second map that could disagree with the mechanical one.
- **What is reflected:** matrices, keys and layouts; components and parts; the board's
  outlines and the modules on it; wiring that is structurally the same; mechanical
  features. Closure mounts are regenerated from the pair, never copied.
- **What is never reflected, by nature:** keymap bindings, instance role, half and name,
  board references, and copper traces and vias.
- **Exceptions** are per item: `local` or `omitted`, with `linked` as the default.
  Rejoining reflects the source again.
- **Reflected heading.** One rule replaces the three current conventions. A footprint
  keeps its side and takes the reflected heading. (Amended below: parameters are never
  changed.)
- **Numbering.** The mirror keeps the source's matrix numbering (column 0 is the same
  physical column on both halves, with the same pins). The recipes' reversed numbering
  goes away, and right-half part and net ids change once.
- **Two modes.** `new-board` creates a second board (the shipped splits stay two boards
  for now); `reuse-board` creates a flipped instance of the same board and copies
  nothing.
- **Compatibility.** The new fields are optional and absent means unpaired, so existing
  projects are never linked automatically. The format moves to version 5 with no data
  rewrite, so an older build refuses the document instead of silently dropping the link
  ([ADR-0004](0004-document-format-version-and-id-rename.md)). Linking existing
  independent two-board projects is out of scope here.
- **Rejected:** a derived-only half (the page reads stored parts in about 282 places), a
  one-time copy with per-operation propagation (it drifts), recipe mirror arithmetic, and
  links kept in a part property bag.

## Amendment: mechanical features and reversible parts (2026-10-10)

- **Mechanical features are linked pair features, mirrored.** A mirrored half starts with
  the source's openings and mounts as linked features mapped through the pair, in both
  modes. They are never cleared and never cloned as independent copies
  ([ADR-0008](0008-linked-halves-stay-linked.md)).
- **Two supported designs, both first class.**
  - A *reversible* design is one board that is flipped: `reuse-board` creates a flipped
    instance of the same board with its pair record and linked features.
  - A *mirrored* design has single-sided parts placed as a mirror image: `new-board`.
  - A linked mirror may also use reversible footprints.
- **Parameters are never changed by mirroring.** A mirrored copy keeps every generator
  parameter, including `reversible` and `side`, exactly as the source has it. Mirroring
  changes pose only: position and heading. This replaces the earlier "use the reversible
  variant" wording.

## Amendment: details of creation (2026-10-10)

- **Keycap legends** are copied to the mirror as they are. Key bindings are never mirrored.
- **Net names** on the mirror are for the mirror's side; they are not carried over from the
  source (a mirror does not keep names such as `PERIPHERAL_LEFT_...`).
- **`reuse-board`** validates the map against the flip and refuses a non-zero map.
- **Topology, transport and relabelling** of the primary half are set by the web adapter
  (physical setup), not by `CreateMirroredHalf`.
- **Modules and embedded circuits** on the source board are mirrored by `new-board`:
  modules by pose and id ([ticket 27](../plans/typed-core-edits/issues/27-mirror-modules.md)),
  embedded circuits under their own `embedded/<mirror id>/` namespace with their component
  definitions copied ([ticket 28](../plans/typed-core-edits/issues/28-mirror-embedded-circuits.md)).
  Neither is refused. A mirror id or namespace that is already taken refuses the record by
  name, as does a module reference the mirror cannot follow. `reuse-board` copies neither (an
  embedded circuit has no instance pin).

## Amendment: the heading rule in practice (2026-10-10)

The user's description of the single reflected-heading rule, which also answers the
question about handed modules:

- **Mirroring across the vertical axis maps a heading `r` to `180 - r`, over the full
  360 degrees, never modulo 180.** A part at the left or right edge that faces outward,
  such as a TRRS jack or a side reset button, turns through 180 degrees: it must face
  outward on the mirrored half. A part facing up or down needs no rotation, only the
  mirrored X coordinate. A part away from the edge, such as an RGB LED, takes the same
  rule: mirrored position, reflected heading.
- **The part is not flipped.** It stays on its authored side with unchanged parameters
  (the earlier amendment), so a mirrored board keeps nearly the same routing on both
  halves.
- **Modules take the same rule.** A handed module is placed at the reflected pose; Core adds
  a finding for a part or module whose result is uncertain: a non-symmetric footprint at a
  heading that is not a multiple of 90 degrees, or a module whose own geometry is handed.
  The finding asks a person to confirm it. Core does not refuse and does not guess silently.
- **The pair's angle function must use the full 360 degrees.** The earlier comparison of
  headings modulo 180 is not enough for this rule.

## Amendment: key switches and their companions (2026-10-10)

- **Key switches mirror on X only.** A switch is not given the 180-degree turn of the
  general rule: its body keeps its orientation and takes the mirrored tilt (a key rotated
  by `r` becomes `-r`, which is what the shipped right halves already do). Turning it
  would put its LED position on the wrong side.
- **Companions are chosen separately.** The diode and the LED that belong to a switch are
  mirrored in position and may be rotated, per variant. In particular an RGB LED variant
  may be turned so its pads match the switch's LED position. When Core is unsure which
  heading is right, it adds a finding and does not guess silently.
- **Everything else** follows the general rule above (`180 - r`, full 360 degrees).

Note (ticket 27 verification, 2026-10-10; the rule is unchanged, the user decides): heading `r_map + 180 - r`
without a flip turns a part rather than mirroring it. A pad at part-frame `(x, y)` lands at the mirror of the
source pad with its local `y` reversed, off by `2 |y|`, at every heading including the 90-degree grid; it is exact
only for pads on the heading axis (`y == 0`). Footprints with pads off that axis (the MCU, TRRS jack, display and
encoder) are therefore not an exact mirror even on the grid. Proposed for the user: a Core finding for any
independent part whose pad layout is not symmetric about its heading axis, computed from the footprint, in place of
the off-grid-only trigger. Details and the shipped footprints:
[ticket 27, Verification](../plans/typed-core-edits/issues/27-mirror-modules.md#verification). Mounted modules are
reflected (ticket 27) and embedded circuits are reflected too (ticket 28), so no hosted item is refused
for lack of a reflection rule any more.

## Amendment: companion modes, pads, labels (2026-10-10)

- **Companion placement has three modes** for a switch's diode and LED on the mirror:
  1. **Literal mirror** (the default): the companion takes the mirrored side of its switch.
  2. **Duplicate position**: the companion keeps the source's side and offset relative to
     its switch (how the shipped right halves place their diodes).
  3. **Literal mirror with an offset or tweak**: the mirror plus a stored adjustment.
  The mode is a per-pair default with a per-item override; a tweak is stored with the item
  and survives reconciliation.
- **A part's pads are not a Core concern.** A port such as a USB-C socket moves to the
  mirrored X coordinate and takes the heading rule; its pad layout is not altered or
  checked, so Core adds no pad-symmetry finding. The off-grid heading finding stays.
- **Labels.** Core does not refuse a source that is neither left nor right. The
  `CreateMirroredHalf` request carries optional labels for the source and the mirror
  (name, half, role) so the caller can rename them; otherwise the labels are kept.

## Amendment: stabilisers, fallbacks and editing the mode (2026-10-10)

- **Stabilisers are never rotated.** A stabiliser keeps its position mirrored on X and its heading unchanged, in
  every companion mode. The relative-heading difference between duplicate and literal mirror does not apply to it.
- **No unmaterialised parts, and a safe default.** Every matrix cell on a mirrored half has its parts. If a
  companion's source part is missing anyway, the mirror defaults to the X-coordinate mirror of the switch's
  companion position instead of erroring.
- **The mode is editable after creation, in the Inspector.** The Inspector panel changes the companion placement
  mode (and tweak) for one companion, a row or a column, a whole matrix, one part, or several selected parts
  at once. Core exposes one typed operation for it; the Inspector builds the target list from the selection.

## Amendment: selective exceptions and framed constraints (2026-10-11)

The user approved selective Make Local, Omit, Rejoin and Unlink for independent parts as well as switch companions. Screens, encoders, controllers and connectors can be exceptions while the rest of the pair remains linked. Companion placement modes remain specific to switch companions. Omit is durable and reconciliation must not resurrect an omitted item. Rejoin uses the current source; Unlink retains visible geometry.

The user also chose support for slanted mirror axes and relationships between linked and Local parts through a separately scoped model extension ([ticket139](../plans/typed-core-edits/issues/139-generalized-mirrored-constraint-model.md)). A relationship across different ownership frames cannot generally be expressed as one fixed canonical Offset: moving a Local endpoint or changing the pair map changes the inverse-projected source geometry. Permanent refusal of these cases would not satisfy the accepted scope.

- **One constraint, live endpoint views.** Add a framed constraint form whose endpoints name either a literal Part or a linked mirror binding (pair, source part and visible mirror part). Resolve the current map and effective ownership, rather than storing a frozen transform. The authored source drives the authored target. Editing through either half uses the same durable constraint ID; it does not create a reciprocal relation or duplicate record. Existing visible source/target getters keep their meaning; authoritative driving lookup is separate.
- **General reflection line.** A framed relation supports the existing world-plane Offset semantics or reflection across a finite anchor and a tangent angle in document-plane degrees. Keep existing horizontal/vertical Mirror and Offset wire shapes and behavior. Endpoint projection follows the existing independent-part, key, companion-mode and stabilizer rules; companion owner poses are live dependencies.
- **One prepared edit and dependency graph.** Fold linked aliases to their authoritative parts, include endpoint-owner dependencies, and reject duplicate authoritative targets or cycles before publishing geometry. Apply and both resolution stages consume the same prepared operation. Refusal preserves document, revision and history; preview restores exactly. This extends the shared constraint engine, without board-specific builders or a second solver.
- **Exceptions preserve deliberate ownership.** Make Local materializes geometry and changes affected endpoint bindings to literal visible Part IDs. Omit removes only relationships referencing actually removed visible endpoints. Rejoin restores source-derived geometry and retires an incoming constraint authored to that Local target, which would otherwise override source-wins behavior; outgoing and unrelated relationships survive. Undo restores the retired record and geometry. Unlink materializes linked views and converts their endpoints to literal visible IDs while retaining constraint IDs, values and direction.
- **Compatibility and activation.** Introduce format6 with the persisted framed arm and production activation in111. The v5-to-v6 migration only stamps the format; it does not invent links, regenerate IDs, rewrite existing constraints or alter geometry. Reuse the common archive/storage migration path. Older builds refuse the newer format. Private preparation may land earlier, but it does not establish delivered support.

The reviewed implementation design is `/home/chris/.cache/boardstudio-baselines/ticket139-planning/design.md`, SHA256 `ca7cbd334e6b41c9fb9fbce1e87b7544c2c2b904030604b76fa3719785884274`. Ticket139 records exact ownership, synthetic literal oracles and required acceptance. Full mirrored-edit and exception lifecycle acceptance remains required before111 activation.

## Amendment: a correspondence is the identity, an exception is a state overlay (2026-10-12)

Making an item Local or Omitted used to cut its correspondence and keep its identity somewhere else. Reconciliation only follows correspondences, so everything that points *through* an excepted item (a module's connector, the controller and pin assignments of the electrical board, circuit net pins, layouts) broke, and each case needed its own patch. This amendment replaces that with one rule. Mechanical features (`omit_member`) and inherited matrix and circuit children already worked this way.

- **A correspondence records source ↔ mirror identity in every state.** Local and Omitted add an exception that overlays it, matched by `(kind, mirrorId)`. Rejoin removes only the overlay. `MirroredException.sourceId` does not exist; the source of an overlay is its correspondence. Companion mode and tweak stay on the correspondence, and an overlay on a correspondence must not carry them.
- **Legacy meaning is kept.** An exception with no matching correspondence is the older v5 record: its mirror item is present and no source is inferred. It stays valid and is never linked. There is no format bump. A document written with overlays is refused by an older validator; no user project exists to migrate.
- **Inherited state stores nothing.** Children of an Omitted matrix or circuit, and a mirror net whose source net has no non-controller pin outside an Omitted item (a *private net*), are absent by rule. The validator and reconciliation compute the rule the same way.
- **Omit cleans the mirror half and only the mirror half.** Inside one atomic transaction it removes the item's footprint (the generated companion copper that is part of the footprint goes with it), its private nets, and the board copper it owned: traces and vias on those nets, and trace ends and vias lying exactly on an omitted pad. That copper is saved in the overlay as `removedCopper`, with the pad positions it was routed to. The item's MCU pins are freed. Every item still mirrored is locked to the source's pins, so a mirror-authored entry can only use a freed or unused pin. A linked module plugged into an Omitted connector keeps everything but its connection, as deleting the connector clears it. The source half never changes.
- **Rejoin restores from the current source.** The footprint, nets and pins are re-derived. Saved copper returns only if every saved pad stands where it did, compared exactly (to 1e-6 mm); otherwise no copper is restored and the transaction reports a notice, and the saved copper is dropped with the overlay. A saved trace or via id that is already in use refuses the transaction. Rejoin reclaims its source pins and releases a mirror-authored assignment that took one.
- **Not decided here.** Mirror-only replacement parts that receive pins automatically when placed, and that are re-pinned after a Rejoin releases them, are specified separately in [ticket 109b](../plans/typed-core-edits/issues/109b-mirror-only-replacement-parts-auto-pins.md).

## Amendment: physical setup creation is atomic in Core (2026-10-14)

The 2026-10-10 creation amendment assigned topology, transport and relabelling to the web adapter.
That ownership is superseded for mirrored-half creation. Physical setup still chooses the requested
values, but `CreateMirroredHalf` accepts them and Core applies them in the same atomic operation
that creates the mirror instance and pair. This keeps topology, transport, physical instance labels,
and the native mirrored-half record in one ResolveEdit commit and one Undo step; the adapter must
not submit a whole-document replacement or split these changes across edits.

- Core validates the requested topology/transport combination and the resulting physical instance
  and pair invariants before accepting the operation. It also validates that the supplied
  `sharedConstruction` belongs to the configured physical board, then preserves that value exactly.
  The adapter resolves it using physical setup's established precedence (existing hardware value,
  otherwise the selected primary's mechanical settings with the existing board-settings and
  initial-settings fallbacks) and passes it explicitly. An invalid combination or any other
  creation error leaves the accepted document unchanged.
- The adapter supplies physical setup's source and mirror label inputs (name, half and role) through
  the operation's optional `labels`, and supplies topology, transport and `sharedConstruction` as
  explicit request inputs. Core applies these values atomically with mirror creation; it does not
  infer labels from source names or half labels.
- The operation's changed-id report includes the created pair and mirror instance and every existing
  instance or hardware record changed by topology, transport, or relabelling.
- This amendment concerns physical setup's mirrored-half creation path. Other adapter-owned
  topology and transport edits retain their existing ownership unless separately decided.

## Amendment: layout parity, anchors and matrix exceptions (2026-10-23)

The user accepted [the matrix/anchor exception rules](../plans/ergogen-layout-parity/issues/11-mirrored-halves-rules.md#answer)
in four decision rounds. Same-board Layout links gain the same per-matrix/key Local,
Omitted and Rejoin meanings as native mirrored halves. Local keys retain their world
poses through shared carried-layout edits. Local matrices retain independent dimensions
and layout, and making an anchored matrix Local detaches it while preserving pose.

Anchors follow actual Local keys; Omitted matrices/keys retain logical anchor poses.
Partial Follow uses transported source-world origin/axes, rather than independently
zeroing partner-world components. Shared parameter ownership does not erase an explicit
anchor exception's effect on a dependent matrix's placement.

Keep-pose detachment that requires another Local exception asks for confirmation before
any change, including when triggered by actual anchor removal. Source deletion preserves
a complete Local matrix and its surviving Layout ownership, retiring only the defunct
correspondence. Rejoin cannot target a deleted source or silently bind a replacement.
The linked-half decision names the exact inverse/refusal rules and required prepared-route
acceptance; it does not claim production implementation.
