# 01: Confirm the matrix transform contract and its seams

Status: resolved
Type: grilling
Blocked by: none
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Question

The [spec](../spec.md) proposes resolutions to the handoff's five decisions. Every
build ticket depends on them. Confirm or change each item below. Every recommended
default needs **no** new public Core interface and **no** document format change.
Items marked *approval* would add one.

| # | Decision | Recommended default | Alternatives |
| --- | --- | --- | --- |
| A | Toolbar tool set | **Stagger** edits the relative column stagger, as a 1-D drag along the stagger axis. A new **Offset** tool keeps today's 2-D drag for row and column-only offsets. A Row selection offers Offset, not Stagger. | Keep one Stagger tool and hold a modifier (for example Shift) for the old 2-D offset; or drop the toolbar offset gesture and keep offsets in the Inspector only. |
| B | Module seam | Deepen `web/crates/layout/src/matrix_transform_operation.rs` into the one semantic module: a typed `MatrixAdjustment` (`Set`/`By`) applied to a `Matrix`, plus gesture-frame conversion. The result stays `SetMatrix`. Native tests pin the web axis maths to Core geometry through `CoreEngine`. | *Approval:* a typed Core edit `EditOperation::AdjustMatrix { matrix_id, adjustment }`. Core would own the parameter rule and world→local conversion (ADR-0006 direction), at the cost of a public `EditOperation` variant and transcript fixture churn. Or, also *approval:* add `staggerAxis` to the serialized `MatrixColumnBasis` scene so web stops duplicating the axis rule. |
| C | Relations source | A pure web projection in a new `web/crates/layout/src/matrix_relations.rs`, read from the accepted document and scene, with native tests against Core outcomes. | *Approval:* a Core scene projection of relationships (public scene change). |
| D | Stagger keyboard | The arrow pair aligned with the on-screen stagger axis changes Stagger by ±0.1 mm (Shift ±1 mm). The perpendicular pair is not consumed. | Always Up/Down = ± of the parameter. That is simpler, but on a Y-mirrored matrix it moves against the arrow. |
| E | Snapping | Keep today's rule: the gesture delta snaps to the pitch fraction, and Alt disables snapping. | Snap the resulting absolute stagger value to the grid. |
| F | Column 0 stagger | Allowed. It is labelled "from the matrix origin" and moves every column. | Hide or disable it for column 0. Existing projects may already hold a value there. |
| G | Inspector wording | Column: "Stagger" with the hint "Moves this column and the columns after it". Then "Column-only offset X/Y" and "Reset column offset". Row: "Row offset X/Y". The Column label becomes 1-based. | Put the offsets in a collapsed "Independent adjustment" disclosure, opened when nonzero. |
| H | Referenced-object feedback | Navigation buttons only. Canvas highlight on hover goes to the backlog. | Include the canvas highlight in this effort (it touches the canvas render path). |
| I | Existing offsets | Never converted. They are shown as departures with Reset. | Offer a one-shot "Convert to stagger" action (backlog). |
| J | Record | Add the terms to `GLOSSARY.md` (column stagger, column offset, key local offset, structural relationship, explicit constraint). Write an ADR, "Matrix gestures edit named parameters", recording A, B and I. | Terms only; no ADR. |

## How to work it

Call the `grilling` and `domain-modeling` skills with the user. Record the answers
under `## Answer`. Update `GLOSSARY.md` (and the ADR, if J is accepted) in the same
commit. Where an answer differs from the spec, edit the spec and the affected tickets
before setting this ticket `resolved`. If B or C chooses a Core interface, add a Core
build ticket before ticket 02 and re-block 02 and 05 on it.

## Answer

Decided with the user on 2026-10-08. Recorded in
[ADR-0007](../../../adr/0007-matrix-gestures-edit-named-parameters.md) and
[GLOSSARY.md](../../../../GLOSSARY.md#matrix-layout). The [spec](../spec.md) is updated
to match.

- **A — Tools:** a separate **Offset** tool, plus **Shift held at pointer-down** on
  the Stagger handle for a quick column offset on that drag. Shift already means
  "large step" on the keyboard, so the modifier applies to drags only.
- **B — Seam:** the user asked which option follows the architecture and is more
  reliable. That is **a typed Core edit plus Core-published axes**:
  `EditOperation::SetMatrixPlacement` (shaped like `SetMatrixSplay`) and
  `MatrixColumnBasis.staggerAxis`. Core owns the placement rules and the
  docs/architecture split. The web-only option would keep a second copy of the
  splay, mirror and rotation maths that can drift from Core. The custom-origin case
  already shows how subtle that maths is. This chooses two public Core interface
  additions. The saved document format is unchanged.
- **C — Relations:** for best practice and fit with the design, choose a **hybrid**.
  Core publishes companion placement modes in the scene (the precedence rule has one
  source). The web projection does grouping, wording and navigation, because
  presentation displays accepted state. A full Core relations projection would put
  UI grouping into Core.
- **D — Keyboard:** the arrows follow the stagger's possible direction on screen.
- **E — Snapping:** "the change and the final value". Snap the resulting value to
  the grid. That keeps on-grid changes in whole steps and brings off-grid values
  onto the grid. Alt disables snapping. Keyboard steps are exact.
- **F — Column 1 stagger:** not answered. The default stands: editable, labelled
  "from the matrix origin".
- **G — Wording:** keep "Stagger" and "Offset". A later `impeccable` pass does the
  wording and visuals. This effort only fixes the 1-based Column label.
- **H — Highlight:** in scope. Hovering or focusing a Relations reference
  highlights it on the canvas.
- **I — Convert:** yes. It is a one-shot "Convert to stagger" that Core offers only
  when geometry is exactly preserved.
- **J — Record:** yes and yes. ADR-0007 and the GLOSSARY.md terms.

The tickets were re-sliced to match: 02–03 Core, 04–07 edits, 08–10 Relations and
11 verification.
