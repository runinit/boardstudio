# Map: Remaining ticket closure (simplified)

> **Executed 2026-10-18; superseded.** All five tickets (001, 002, 006, 008, 013)
> closed under [cad-backend-replacement/map.md](../cad-backend-replacement/map.md)
> ("Closure rescope") using the approach below: one native pass per track and one
> combined Standards+Spec review per track. This directory is the executed record;
> the authoritative status lives in the `cad-backend-replacement` tickets, which
> are `resolved`.

## Destination

Close the five remaining claimed tickets (001, 002, 006, 008, 013) using the
simplified approach from the Opus analysis: one combined closure review per
track, reuse existing evidence, no repeated 34-asset census or expensive
verification. Sign off each ticket on evidence, not on universal certification.

## Why

All five tickets have the same pattern: code works, tests pass, but formal
acceptance boxes are unchecked because they require expensive 34-asset census
runs and combined reviews. The 007 deep accounting was already simplified using
industry-standard approaches. The same simplification applies here: reuse
existing evidence, run one combined review per track, and accept named limitations.

## Decisions

1. **Two independent tracks.** Track A (generated construction/export):
   013 → 006 → 008. Track B (importer/mesher quality): 001 + 002 joint.
2. **Close 013 now.** Record adoption recommendation and named refusals. Do not
   wait for 006 to implement every limitation.
3. **Sign off 006 with named limitations.** Accept bounded production adoption
   with explicit named gaps (cone-wall crossing, sealed polygon openings refused).
   Do not build a general Boolean engine.
4. **Close 008.** Reconcile existing fixture checks. Reuse FreeCAD receipt.
   One live export-to-certificate check if missing.
5. **Joint 001+002.** One combined review. Reuse existing evidence. Do not
   rerun the 34-asset census. Explain recovered-face changes as diagnostics.
6. **One combined review per track.** Not per ticket.
7. **Reuse existing evidence.** Do not rerun 34-asset census, FreeCAD, or
   WASM/browser for unchanged code. Run once if a genuine coverage gap exists.

## Scope

In scope:
- Update 001, 002, 006, 008, 013 tickets to reflect simplified acceptance
- Close 013 (record adoption recommendation)
- Sign off 006 (explicit adoption decision with named limitations)
- Close 008 (reconcile fixture checks)
- Joint 001+002 reconciliation (one combined review)

Out of scope:
- New geometry implementations
- General Boolean engine
- Repeated 34-asset census
- Repeated FreeCAD/external interoperability runs
- Repeated WASM/browser gates for unchanged code

## Issues

1. [01 — Close 013 prototype](issues/01-close-013-prototype.md)
2. [02 — Sign off 006 with named limitations](issues/02-sign-off-006.md)
3. [03 — Close 008 export](issues/03-close-008-export.md)
4. [04 — Joint 001+002 reconciliation](issues/04-joint-001-002.md)
5. [05 — Combined review and close](issues/05-combined-review-close.md)

## Progress

- (none yet)