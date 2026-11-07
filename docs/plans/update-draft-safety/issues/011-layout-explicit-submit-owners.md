# 011: Layout explicit-submit owners

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the Layout cluster's
remaining explicit-submit owners — the Layout constraint composite, the legacy
position Inspector, and companion placement — so their unapplied Draft values
defer Update Now until resolved through each form's own submit or cancel
controls. (Geometry scripts, the fourth Layout owner, lands with the tracer.)
The Layout constraint adapter covers the confirmed hazard that an accepted
update currently clobbers an open constraint draft.

## Acceptance criteria

- [ ] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [ ] Each owner's deferral message names its editing area and per-key remedy; resolution happens only through that form's own controls.
- [ ] The Layout constraint draft is protected while open: an accepted update landing does not silently discard the designer's unapplied constraint draft, and the owner's unresolved judgement follows the settled baseline rules.
- [ ] Per-key behaviour follows the contract: landing updates the baseline, failure/refusal never clears, revert-to-baseline clears, newer typing re-raises, older settlement never clears newer input.
- [ ] Hidden or keep-mounted instances of these owners keep blocking; unmount or scope change retires them.
- [ ] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.
