# 12: Decide how Relations explains carried chains and anchors

Status: resolved
Type: grilling
Blocked by: [01](01-name-the-carried-parameters.md), [04](04-layout-kernel-order-of-operations.md), [05](05-this-column-splay-representation.md), [07](07-matrix-anchors.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Relations (`web/crates/layout/src/matrix_relations.rs`, `web/src/presentation/layout_matrix_relations.rs`)
and the canvas highlight currently explain cumulative stagger, splay, pitch, gap and companion
modes for one matrix. Decide how they explain:

- carried chains: which later rows and keys inherit a Shift or Bend, and which columns inherit
  spread, stagger and splay;
- Column rotation, which applies to one column only
  ([ticket 05](05-this-column-splay-representation.md#answer), decision 8);
- padding, whose step affects this and later rows, versus offsets, which do not carry
  within the matrix;
- the Shift and Bend that disabled and deleted keys still carry
  ([ticket 04](04-layout-kernel-order-of-operations.md#answer), decision 7);
- dependencies between matrices: what a matrix follows, which matrices follow a key, and the
  Follow subset ([ticket 07](07-matrix-anchors.md#answer));
- the entries and wording, what hover or focus highlights on the canvas, and which tool each
  entry opens.

Relations follows the stagger pattern and says "origin", not "pivot"
([ticket 01](01-name-the-carried-parameters.md#answer)).

## Answer

Decided with the user on 2026-10-23 in one round; all six recommendations were
accepted. This completes the Relations question left open by the accepted
[variant D prototype](09-inspector-and-toolbar-prototype.md#answer).

### 1. One structural explanation

Use the collapsible **How it is placed** chain as Relations' structural section,
not a second competing placement explanation. Keep the ordinary parameter editors
from the prototype. Rename **Local adjustments** to **Independent adjustments**
to distinguish offsets/independent rotations from linked-half Local exceptions.
Keep **Authored constraints** separate, including Core's companion precedence facts.

The chain explains the relevant matrix/anchor frame, column steps and turns, row
Padding, row/key Shift and Bend, and the selected item's independent adjustments in kernel
order. Row/key contributions retain their source identities even when their values
are added into one kernel step. Column rotation is explicitly **this column only**;
earlier independent column/row/key adjustments are not described as carried steps.

Selections spanning matrices have no invented common chain; ask the user to select
one matrix context for detail. Within a context, show common values or **Varies**,
not a representative key's value presented as universal.

### 2. Accurate values supplied by Core

- **Cumulative bend:** carried row/key Bend through the selected key. Do not include
  Splay, Column rotation, Key rotation or matrix/anchor rotation in that total.
- **Cumulative shift:** the accumulated translation expressed in matrix axes, with
  each Shift transported through the running carried heading. It is not a raw sum of
  differently turned vectors, a final key position, or a world-space displacement.
  For example, starting at zero heading, Shift (10, 0), Bend 90°, then Shift (10, 0)
  contributes (10, 10) in matrix axes, not (20, 0).
- Stagger step totals are labelled as **before turns**, not final positions.
- Default Spread and Padding show the effective pitch value and a **pitch** indicator.
  Respect the absence of Spread on column 0 and Padding on row 0.

The kernel/scene interface supplies the numeric contributions, effective frames,
source identities and dependency/exception facts. Relations formats them; it does not
add a second layout implementation or reconstruct companion/constraint precedence.
Grouping, wording, reference controls and editor navigation remain presentation work,
not a fully formatted UI tree in Core. Cumulative structural contributions do not
promise that a Local key's final world pose moves.

### 3. Carry and anchor dependencies are distinct

- **Inherited by…** identifies receiving rows/columns/keys within the matrix.
  Padding carries its step to this and later rows; offsets do not carry within the
  matrix. Use **later rows**, never **above**, and **origin**, never **pivot**.
- **Follows…** names the actual anchor key and Follow subset. **Followed by…** lists
  direct dependent matrices; navigate those references to explore longer chains
  rather than inventing another graph editor.
- **This key/row/column only** describes scope within its matrix. An independent
  offset or rotation can still affect another matrix anchored to the resulting pose;
  list that anchor relationship rather than promising that nothing else can move.
- Explain Local keys as holding their final pose while their carried fields remain
  shared. Follow references use the actual Local key, and linked partial Follow uses
  the transported source-frame directions settled in
  [ticket 11](11-mirrored-halves-rules.md#answer).

### 4. References and edit previews have different highlighting

Pointer hover and keyboard focus on a source-step reference highlight its contributors;
**Inherited by** references highlight the receivers they name. Reference highlighting
is **violet dashed** and remains distinct from selection. It changes neither selection,
camera, visibility, document revision nor Undo history. Release it on leave, blur,
unmount, selection/context changes or removal of the referenced item, and resolve
references against current accepted identities rather than stale coordinates.

**Amber** belongs to actual edit previews, using Core's effective pose changes, including
anchor dependants and Local/Omitted rules. A structural reference is not a simulation
of which visible keys will move. Navigation into an editor may start that editor's
normal preview; it is not itself a document edit.

### 5. Invisible contributors remain explainable

Keep relevant Disabled, Deleted and Omitted contributors in the chain, labelled with
their state, because their stored values can still carry or anchor. Hidden layers do
not erase the explanation. Do not resurrect parts, enable hidden layers or fabricate
visible key shapes merely to highlight them. Explain unavailable canvas highlighting
in text and permit inspection of the current logical matrix/key context.

Inspection does not bypass Omitted editing restrictions or silently Rejoin an item.
An actually removed logical key is not a Deleted cell: clear its stale reference and
never navigate to a replacement key merely because it reuses a coordinate.

### 6. Navigation does not edit values

A parameter entry selects its owner and opens the matching tool/editor. Source and
receiver references navigate to their named context; anchor editing opens the anchored
matrix's Anchor section. Keep Reset, Convert and other changes as separate explicit
actions. Show a visible, accessible reason when an action is unavailable, including
editing restrictions and Core's conversion/refusal reasons.

Required implementation acceptance covers key/row/column/matrix contexts, **Varies**
and cross-matrix selections; curved Shift/Bend totals versus raw sums; Column rotation
versus Splay; defaults; actual/partial/Local/Omitted anchors and independent adjustments
with followers; pointer and keyboard reference lifecycle; hidden/deleted contributors;
separate amber previews; navigation with unchanged document/history/camera; and explicit
actions with accessible blocked reasons. Exercise the mounted panel and real scene
facts, not only a second UI-side arithmetic oracle.

Verification for this decision was source inspection of Relations, scene consumers,
navigation and existing highlight tests. No production UI changes, builds or application
runtime tests were performed; the acceptance above belongs to the implementation tickets.
