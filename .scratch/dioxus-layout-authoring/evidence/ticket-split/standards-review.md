# F3.2 draft children — Standards review

Reviewed integration `.scratch/dioxus-layout-authoring/` at exact SHA-256:

- 01: `f61cd1136b65e2e1d52932d791a5edbbde957cd875600e3ca817034a3408b871`
- 02: `f1ddbfb34d6086299fa107ec408d8bdef7b29fbd1c27776b4a1ceeb9550f47b0`
- 03: `76a9863a4127ebfe4266f1914a5316a931d452fd274b2ac6a347aa18f856aad4`
- 04: `f1fbe29290cac974fa31edb2d8c9219a954f737e481644732dc6a8678fc1384f`
- Source contract: `5c493acd6cff74918fe43b7b93c5158db5ea97596346d21dc3ec6c5c78ae7c36`

**P2 — Resolve attached-member control ownership and acceptance overlap.** Draft02’s linked inspector must “allow the existing explicit local component substitute behavior,” while draft04 expressly owns attached-member replacement and local override. Draft04 additionally requires “attached add/replace/remove” despite assigning insertion to03 and asserting no child dependency. This conflicts with CONSTRAINTS.md Architecture’s single state/lifecycle owner and separately reviewable bounded workflows, and the source contract’s disjoint feature ownership. Assign02 relationship display/unlink, with propagation/local-override verification through fixtures or a clearly identified parent integration check; assign replacement/local override controls solely to04. Verify04 independently using existing attached members; label03 insertion coverage as parent integration rather than04 completion criteria.

Otherwise the split is coherent: guided matrix creation versus ghost placement, linked-pair versus existing-half operations, standalone versus selected-key insertion, and structural configuration versus F3.3 transforms are explicitly distinguished. Targeted React placement/inspector and Core operation reads support those seams. Root retains shared mounting, IDs, Session/history translation and CSS; no public API widening or new canonical edge is proposed. Each child requires actual public UI evidence, history and reopen rather than helper-only acceptance.

Source/planning review only; no edits, Cargo or public verification. Publication awaits the bounded correction and independent Spec clearance. No new refactoring takeaway observed; this is a ticket ownership clarification, not an implementation architecture finding.

## Corrected exact-hash re-review

Final reviewed hashes:

- 01: `f61cd1136b65e2e1d52932d791a5edbbde957cd875600e3ca817034a3408b871`
- 02: `b0c309055cdc2e6a4ae7cb408c39dfcd60b9174260feee136405d31c4194e446`
- 03: `0f1af160108d21e1607e3b1ebbe3ba2843bddc82c5abe3bde7cf7c5dd7ea6be2`
- 04: `e741459f660e6de21c25c32c677edc91a3372d60b205779db29185e34b56458b`
- Source contract: `cb487052c516a2a8eb91aaabc10320719c659fd592834c464988fa6d5eb9bc55`

**Standards clear; the prior P2 is closed.** Draft02 owns partner display/unlink and explicitly excludes local member controls. Draft04 owns replacement/removal/local override using fixture-provided members; insertion via03 is a parent integration check rather than a child dependency. Source-contract ownership remains consistent.

Also inspected the new03 reversible-Ergogen normalization requirement and04 required variant handoff against `createWorkbenchPlacementActions.ts` and `createProjectActions.ts`. These preserve source-backed responsibilities: root owns definition normalization, project copying/open/edit/save and recovery; the private UI does not gain a second authority or public API. Variant creation is explicitly distinguished from Undo in the original project.

No remaining material Standards findings in these draft hashes. Publication still requires the independent Spec verdict and coordinator action; this is planning clearance, not implementation or F3.2 acceptance. No source changes, tests, Cargo, or publication performed. No new refactoring takeaway observed.
