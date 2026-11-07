# 014: Creation-form owners

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the creation forms that have
no accepted value to project — the new matrix form, the mirrored-pair form,
and existing-half setup. Their entered values are judged against the snapshot
of the form's starting values taken at mount, so a half-filled creation
defers Update Now like any other unresolved draft, including input the form
itself would silently reject on submit.

## Acceptance criteria

- [ ] Each form defers Update Now when its entered values differ from its mount-time starting values, and clears on revert to those values or on resolution through the form's own controls.
- [ ] Silently-rejected invalid input (values the submit path would return without effect) still defers; protection never depends on a submission attempt or an invalid-field marker.
- [ ] Each deferral message names its editing area and remedy.
- [ ] A form that unmounts without submitting retires its registration and never blocks a later update.
- [ ] Hidden or keep-mounted instances keep blocking.
- [ ] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map); the rows restate the
criteria above per form (new matrix, mirrored-pair, existing-half) and add none. The shared gate and
lifetime mechanics are proven at the 008 and 011 seams, so each row proves form wiring only. Tier
`mounted` (Update Now seam); every row `planned`.

- P1 Mount baseline — form freshly mounted and untouched → Update Now → no deferral; an entered value
  differs from the mount-time starting values → defers; revert to them or resolve through the form's
  own controls → clears. Oracle: Update Now defers or proceeds; the message names the editing area
  and remedy. One scenario per form.
- P2 Silently invalid input — a value the form's submit path returns on without effect, entered with
  no submit attempt and no invalid-field marker → Update Now → still defers (and still defers after a
  submit attempt that returns without effect). Oracle: deferral state with no invalid marker
  present, so the marker query cannot be the cause. The invalid value is chosen from each form's
  own submit path.
- P3 Unmount without submit — value entered → form unmounted unsubmitted → later Update Now → not
  deferred by the retired registration. Oracle: update proceeds.
- P4 Retention — form hidden but mounted → Update Now → still defers. Oracle: deferral present.
- P5 Adapter RED/GREEN — each adapter's rows run RED then GREEN through the mounted seam. A row the
  existing product already passes is a `proof gap` recorded GREEN, not a fabricated RED.
