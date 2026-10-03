# F7.4 closure, fit, and hardware form implementation

This source-only receipt covers the missing fields in the pinned React `Hardware & critical fits` form. It extends the existing `MechanicalSettings` editor and controller; it does not add a geometry or persistence authority.

- Closure hardware exposes the supported custom numeric fields, thread designation, head profile, length datum, positive available screw lengths, and the existing insert/drive/installation/fixed-length choices. Custom dimensions and text choices are patches to the current `InternalClosureHardware` leaf; selecting the insert preset retains the existing dependent-dimension controller behavior.
- Critical fits support add/remove, generated Case part selection, name and tolerance edits, all four document-XY endpoints, and calculated endpoint distance.
- Hardware specifications support add/remove, generated-mount links, designation, thread, length, quantity, tolerance, and notes. Add uses the reference defaults and a stable collision-free local ID. Each edit targets the current accepted settings through the existing scoped owner/controller and preserves unrelated configuration fields.
- Mount choices use current resolved generated body features when present and the accepted configured mount lists as fallback, matching the reference selection rule. Missing saved links remain visible as an unselected choice rather than being silently rewritten.
- Issue 05 remains open because no paired browser journey, build, or test was run for this source packet. Existing pointer-drag/handle picking qualification also remains open; this packet does not change it.

Source commit: `58e155388e51133bc1c6b0b1a4d19bf9c77f7db8` for closure and critical-fit controls; the linked-hardware source follow-on is recorded in the current packet commit.

Refactoring ledger delta: no new architecture authority or seam was introduced. The controls route through the accepted mechanical settings owner and existing operation/history path; RF-001 and RF-006 remain in force. No broader refactoring inference is claimed.
