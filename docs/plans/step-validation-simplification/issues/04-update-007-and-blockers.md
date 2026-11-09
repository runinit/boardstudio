# 04: Update 007 ticket and blockers

Status: resolved
Type: build

## What to build

Update the 007 ticket and the map to reflect that packages 2–6 (per-route
accounting) are superseded for import admission by the simpler approach in
this plan. Per-route accounting that 014 needs for collision certificates
remains in 007's scope but is scoped to collision-relevant routes only.

## Acceptance criteria

- [ ] 007 CURRENT block updated: packages 2–6 no longer block import admission.
  Per-route accounting retained only for collision certificate routes (014).
- [ ] 010 blockers updated: no longer blocked by 007 packages 2–6. 010 can
  close once aggregate reports + GVP + invariants pass.
- [ ] 012 blockers updated: no longer blocked by 007. 012 can close once
  removal audit + checks pass.
- [ ] Map updated: 007 status changed to reflect new scope.
- [ ] 011 unblocked: performance comparison can proceed.

## Notes

- This is a documentation/ticket change, not a code change.
- The 007 deep accounting work is not deleted; it's re-scoped to 014's needs.
- 014 remains the owner of collision-relevant per-route accounting.

## CURRENT

Done. 007 ticket already re-scoped (packages 2-6 deferred, non-blocking).
Map already shows 010/011/012/014 as resolved. No further changes needed.
