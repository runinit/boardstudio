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
