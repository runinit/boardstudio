# 008: Export exact cases and PCBs and replace their oracle coverage

Status: resolved
Owner: /root/sol_frame_serde_spec
Type: build
Blocked by: None
Related: 013: Layered construction; 006: Production case and PCB adoption

## What to build

Export single and multiple exact case and PCB bodies as STEP and replace their required OCCT oracle with independent Rust/analytic/reference coverage. Continue the integrated writer and bounded oracle work; certify013's direct planes, cylinders and cones and their trim topology. No exact keycap STEP or general B-rep Boolean export is required. Existing005 solids permit independent progress; final combined acceptance uses013/006 output.

## Acceptance criteria

- [ ] Export actual exact case/PCB solids with correct units, body identity/counts, placements and complete trim geometry, including outline plus drilled holes and round/conical seats. Preserve exact rational representations where STEP uses them; do not replace cones/circles with facets.
- [ ] Verify bounds, volume, material, topology and source completeness against analytic or independently sourced expectations; same-kernel roundtrips alone are insufficient.
- [ ] Preserve applicable case/PCB acceptance fixtures, tolerances and negative controls while replacing their OCCT-dependent role. Explicitly map former keycap exact tests to009 mesh/print checks rather than retaining an obsolete exact gate or deleting intended feature coverage.
- [ ] Pass native/WASM checks and Standards/Spec review on the actual013/006 exports; independent reference checks establish interoperability beyond internal roundtrips. Keycap export uses009's mesh format, not this writer.

## Comments

2026-10-18 parent integration: the B5 fixture slice (`8c5086e72` → `7335da4e` → `09b8a2e68` → `a4940488e`) is integrated on the monstertruck step-support branch as `9dbe14abe` → `b61d4534d` → `f520e52d6` → `d5bd7b271`; the affected sources match the reviewed tip exactly. This parent re-ran the ten `fixture_exports` tests (all pass, debug profile), scoped test Clippy with the two inherited allowances (`chunks_exact_to_as_chunks`, `items_after_test_module`), scoped rustfmt, doc links, repository and diff checks, and reuses the untouched 42-test export receipt and the one-time FreeCAD/OCCT interoperability receipt. No WASM/browser/performance/external-FreeCAD rerun. 008 remains open; this is bounded fixture coverage, not general B-rep validity or final closure.

2026-10-18 round-one SPEC P2 correction: the new exact single-record LINE direction mutation
reproduced acceptance of malformed slot bytes (`Ok(())`); see RED (`../../../../.evidence/b5/line-direction-red.log`).
The test-only `circular/fixture.rs::edge` now requires a positive projection of normalized
LINE direction onto the normalized base edge chord, reusing finite/nonzero checks and
retaining true-only same_sense. No new tolerance, API, dependency or production change.
Standards scope concern assessed: `step_accuracy.rs::direct_products`/`solid_scales` bind
direct ownership and reject occurrence/mapping/relationship paths; `fixture.rs::compare`,
`surface`, `edge` and `trim_loop` consume the typed active face/curve/trim graph against the
complete references. No further active semantic bypass was identified. Unknown unreferenced
metadata is not a schema-validity claim. Calling `Document::check_scope` would reject the
intended cylinders/rational curves because `supported_record` is deliberately planar-only;
that reader contract is unchanged. See correction and scope disposition (`../../../../.evidence/b5/LINE-DIRECTION-CORRECTION.md`),
scoped GREEN (`../../../../.evidence/b5/line-direction-green.log`) and
scoped Clippy (`../../../../.evidence/b5/line-direction-clippy.log`). All 10 scoped tests,
formatting, scoped Clippy (unchanged inherited exceptions), 156-ticket and diff checks pass.
The original 42-test and
source-7335da4e one-time reader receipts are reused without regenerating/rebinding their
artifacts. Parent owns integration/final 008 closure; no new review loop was launched.

2026-10-18 B5 corrective fixture slice (`7335da4e`, descendant of recoverable WIP `8c5086e72`):
removed the unsound general prism certificate and replaced it with a test-only comparison
of actual emitted Part21 faces against authored reference trims. The unchanged Decimal VIK
oracle, analytic reflected slot/circle/half-disc, and the current placed gasket tray/lid
define the complete expected surface/trim sets; neither writer output snapshots nor kernel
meshes supply expectations. Directed chains, shared vertices, supports/sweeps, units and
surface associations are compared; harmless cyclic starts, numeric IDs and monotone
collinear subdivisions are irrelevant. Both WIP false-acceptance controls now reject;
gasket placement/depth, product ownership, lost trims, units and occurrence controls pass.
A surface-independent direct-product reader preserves the original planar reader's
Unsupported contract. Nine scoped tests, formatting, diff and scoped Clippy pass with the
same inherited lint exceptions; the untouched 42-test export receipt is reused.
The installed FreeCAD 1.1.4 / OCCT 7.9.3 probe was run **once** on six current emitted files:
all eight solids are valid/closed; VIK volume is 946.578323499303 mm³, reflected slot
638.683752280681 mm³, tracers 8π/4π, gasket bodies 9122 mm³ each. The
one-time receipt (`../../../../.evidence/b5/FREECAD-CURRENT-RECEIPT.md`) binds source commit,
exact command, input hashes and ordinary output-byte hashes. Its raw VIK bounding box
extends about 8.65e-6 mm beyond authored analytic bounds; no tolerance was widened or exact
external-bound agreement claimed. See scope, reference mapping and limits (`../../../../.evidence/b5/BOUND-REFERENCE.md`).
Production, dependencies and public API are unchanged; no required C++/OCCT path was added.
This is bounded fixture/reference coverage, not general B-rep validity or a new cone/stepped
checker. Parent Standards/Spec review and full 008/006/013 closure remain pending.

2026-10-18 B5 interrupted-candidate assessment: preserved the test-only raw-Part21 prism WIP;
it is **not ready for integration or acceptance**. Current emitted Core VIK, reflected-offset
slot, circle and half-disc positive checks pass against the unchanged source/analytic oracles.
Two new active negative tests in `export_tests::pcb_prism` prove false acceptance: a wall
EDGE_LOOP with disconnected traversal, and a slot translated wholly outside its outer cap,
both return `Ok(1)`. Wall connectivity and cap-loop containment/disjointness must be proved
or explicitly refused before this candidate can certify geometry; its numerical envelope
and SURFACE_CURVE associations also need assessment. The combined-body test reaches both
volume assertions but fails at the existing planar-only product reader's deliberate curved
scope refusal. Do not widen that reader to make the test pass. Current assembled gasket/
case composition remains uncovered; cones remain unsupported by the new prism path.
All 42 existing export tests pass; candidate tests are 4 passed/3 failed. Formatting,
diff checks and scoped test Clippy pass with existing lint exceptions; strict test Clippy
is blocked by untouched `mesh.rs`/`profile.rs` test-module ordering. No exporter/builder
defect, production edit, FreeCAD run, dependency, tolerance change, WASM/full-web rerun or
review is claimed. See assessment and required delta (`../../../../.evidence/b5/ASSESSMENT.md`)
and active failures (`../../../../.evidence/b5/prism-assessment.log`). Existing evidence is
retained; 008 stays open under the user's preserve-WIP instruction.

2026-10-17 root countersink integration: Exact reviewed v9 postimages (`../../../../.evidence/production-step-export-008/root-integration-v9/REPORT.md`) now pass47 actual shared native tests (45 unit+2 integration), retaining all44 prior root tests and all43 private v9 unit names; WASM library/tests, formatting and scoped test Clippy pass. Runtime/API/Cargo/vendor unchanged. The actual013 countersink export has independent analytic volume, complete cone/trim/source and material/topology coverage. Full008 case/PCB coverage and production adoption remain open.

2026-10-17 root integration: The reviewed circular/annular oracle successor through v8 is now in shared production-crate tests. Actual locked44 native tests pass (42 library+2 integration), retaining all33 prior root tests and all40 private-v8 tests; WASM library/tests, formatting and scoped test Clippy pass with the existing005 lint exception. Only seven owned test/fixture postimages and two direct development edges changed; runtime/API, dependency versions and local overrides are preserved. See integration (`../../../../.evidence/production-step-export-008/root-integration-v8/REPORT.md`). Full008 remains open; the same owner continues the actual013 countersink certificate through an additive successor.

2026-10-17 scope reconciliation: Keep the integrated v5 writer/checker and current v7/v8 case-cylinder/annulus accuracy handoffs. Stop expanding exact keycap/general-intersection coverage. Historical broader references below do not add a new delivery requirement.

2026-10-17: User decision: exact STEP export scope is cases and PCBs only. Keycaps export as mesh (STL/3MF) and need no exact writer coverage.

2026-10-17: Root integrated the reviewed bounded exact writer plus independent raw-Part21 checker through oracle-v5. Fresh actual shared33 native tests, WASM library/test compilation, strict library Clippy and formatting pass. See integration (`../../../../.evidence/production-step-export-008/root-integration-v1/REPORT.md`). This preserves the v5 unsupported-assembly refusal and independent unit/placement/Plane checks; curved-family and full reference/placement coverage remain open. The separate v6 curved certificate is private and under review; no OCCT required test is removed.

2026-10-17: Prerequisite005 is integrated and resolved. The private exact writer/oracle continues; full independent accuracy, curved-family and reference coverage remain open. Rejecting unsupported external assembly graphs prevents oracle source loss but does not qualify positive imported-placement coverage.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.

2026-10-17: Private implementation assigned against the agreed opaque exact-model contract. Integration remains blocked by reviewed005 acceptance; existing application and unrelated changes are preserved.

2026-10-17 root integration: The independently reviewed rational-circle checker, retracing/unique-vertex fix and helper-reuse delta are applied to six exact test/fixture postimages. Fresh52 affected native tests, all168 registrations, full library/tests WASM compilation, scoped Clippy and formatting pass with509 unchanged inputs. See root report (`../../../../.evidence/production-step-export-008/root-integration-v10/REPORT.md`). Runtime exporter/API/vendor/dependencies remain unchanged; broader PCB/case reference coverage remains open.
