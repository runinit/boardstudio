# Layout Select + Snap contract Spec review

Reviewed contract SHA-256 `6a4b6d53ba3114250fe5cbf9631b2e28be1fb148baf643ced976346d3fce7a97`; ticket SHA-256 `98ab6f1802b6005c880ec085ac4b46c23b0c7b76a25e7493d12ac18355624116`.

**Hold for one bounded scope correction (P2).** The packet promises matrix/row/column/key drag parity through the existing Session gesture adapter. React `app/src/ui/createCanvasInteractions.ts:219–258` changes matrix origin, row/column offsets, or cell/assembly offsets through `SetMatrix`; row/column/key displacement is transformed into local matrix coordinates before snapping. Integration `application/src/interactions.rs:326` instead snaps world position deltas, and `application/src/session.rs:1599` emits only `MoveParts`. Core `matrix/layout.rs:900` converts linked primary moves into individual cell offsets; ordinary matrix members otherwise become absolute part overrides (`core/src/lib.rs`). Selecting the correct IDs does not recover the reference metadata or rotated/splayed local snapping behavior.

The Select menu and existing standalone drag/snap capability may start immediately. Correct the child’s promised/verified drag scope to existing operational MoveParts behavior and explicitly retain matrix/local-coordinate gesture parity under F3.3, or separately review a concrete matrix gesture seam. Do not imply Begin’s existing fields already supply those semantics. This does not require waiting for whole F3.1/F3.2 acceptance or widening APIs.

Other reviewed amendments are sound: root selection preference distinct from modifier mode; semantic empty cell without fake IDs; Begin-only capture and live Alt; editable gap draft; selected-part footer coordinates; shared preference with placement/outline joins retained; mandatory RF handoff. Parent graph unchanged. No code/build/browser execution performed; public evidence remains an implementation gate.

## Corrected exact packet — clear

Contract `5f656f9aa3730f74e0420bce7ac7a195af375408998c56f6a16d882699d64661`; ticket `3ce8699cc6eb6e2cd551489a78a42a9abbb8e1af7d70733effa5ba1ff45565ce`. The finding is closed: selection context/target resolution is distinguished from matrix gesture semantics, and mutation acceptance now uses a real non-matrix standalone part through the existing MoveParts/Session seam. Matrix/row/column/cell local-transform and snapping parity remains explicit open F3.3 work, recorded as source evidence under RF-005 rather than a new architecture project or claimed public reproduction.

No remaining material Spec findings. Ready for bounded publication/implementation subject to independent Standards amendment clearance. Parent criteria/joins remain intact; compiler and paired public acceptance remain future gates.
