# 013: PCB placement, attach, assembly and new-net owners

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the PCB cluster: mounted-module
placement, module-attach options, the assembly draft form, and the new-net
name form. The assembly form is the multi-admission exemplar — Save assembly,
Place on board and Apply to matrix are separate admission keys over one draft.
Attach options and new-net are judged against their mount-time starting
values, including the new-net draft's survival of part switching and collapse.

## Acceptance criteria

- [ ] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [ ] Assembly drafts are judged per admission key: Save, Place and Matrix each clear only their own key; submitting one never clears a draft belonging to another, and an older settlement never clears newer input.
- [ ] Mount-snapshot owners (attach options, new-net name) defer when their entered values differ from their mount-time starting values and clear on revert or resolution.
- [ ] The new-net draft keeps blocking while its owner stays mounted across part switches and collapse, and retires on workspace switch or unmount.
- [ ] Each deferral message names its editing area and per-key remedy; resolution happens only through that form's own controls.
- [ ] Hidden or keep-mounted instances keep blocking; unmount or scope change retires them.
- [ ] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map); the rows restate the
criteria above per owner and add none. The shared gate, epoch and lifetime mechanics are proven at
the 008 and 011 seams, so each row proves owner wiring only. Tier `mounted` (Update Now seam);
every row `planned`.

- P1 Deferral — per owner (mounted-module placement, attach options, assembly draft, new-net name):
  a real valid unapplied draft, or a realistic invalid unsubmitted draft → Update Now from the toast
  and from the header → the update defers and the draft is untouched; the message names the editing
  area and per-key remedy. Oracle: deferral state, message against the settled per-(owner, key)
  copy, unchanged draft. Draft kind and entry point are table cases.
- P2 Assembly admission keys — one draft with tree and origin values unresolved; Save, Place on
  board and Apply to matrix are separate keys → submit one key and land it → only that key's part
  clears and the other two still defer with their own remedy (origin values are consumed by Place
  and Matrix only, so Save leaves them deferring with the Place/Matrix remedy). Oracle: the
  remaining deferral names exactly the unsubmitted keys' remedies. Table over the three keys.
- P3 Assembly older settlement — Save in flight and Place in flight with different submitted
  values, newer typing, then either older settlement → the newer input still defers. Oracle: deferral
  state after each settlement.
- P4 Mount-snapshot owners (attach options, new-net name) — entered values differ from the
  mount-time starting values → defers; revert to them or resolve through the form's own controls →
  clears. Attach: the Attach click while the connector is still loading (before the submission
  begins) still defers. Oracle: deferral present, then absent.
- P5 New-net lifetime — name typed → switch part, collapse → still defers; workspace switch or
  unmount → no deferral. Oracle: deferral state at each step in one scenario.
- P6 Retention — an owner hidden but mounted → Update Now → still defers; unmount or scope change →
  no deferral from the retired entry. Oracle: deferral present, then absent.
- P7 Adapter RED/GREEN — each adapter's rows run RED then GREEN through the mounted seam. A row the
  existing product already passes is a `proof gap` recorded GREEN, not a fabricated RED.
