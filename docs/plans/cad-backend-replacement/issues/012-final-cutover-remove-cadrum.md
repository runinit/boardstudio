# 012: Cut over and remove Cadrum, OCCT and C++

Status: ready-for-agent
Type: build
Blocked by: 001: Closed STEP boundaries; 002: Shared-edge angular sampling; 010: Production cache/worker/browser integration; 011: Integrated CAD performance

## What to build

Finish the production switch: exact layered cases/PCBs and their STEP export, mesh keycaps for appearance/optional printing, complete vendor STEP previews/alignment and mesh collision validation. Remove Cadrum/OCCT/C++ from CAD production, build, packaging and required-test paths. A general exact B-rep Boolean engine and exact keycap STEP are not cutover requirements.

## Acceptance criteria

- [ ] Every capability under the dated scope decisions is satisfied by current production code:013/006 exact cases/PCBs,008 their exact STEP,009 mesh keycaps,007 complete source-retaining vendor assets and014 collision validation. Pass required native/WASM/browser checks and comparable performance measurements.
- [ ] Remove old CAD dependencies/locks, OCCT provisioning, C++ oracle/toolchain, stale package/worker paths and obsolete CI caches/image workflows; preserve unrelated native dependencies and source data.
- [ ] Prove removal using actual dependency graphs, builds and packaged-artifact audits; no OCCT/C++ runtime fallback remains. Preview meshes cannot replace exact case/PCB geometry or STEP. Mesh keycaps and vendor previews are the approved design and are not a forbidden preview-only replacement.
- [ ] Pass required checks and independent Standards/Spec review on the final integrated source/artifacts before declaring the full goal complete.

## Comments

2026-10-17 scope reconciliation:004 and general006 kernel hardening are off the critical path. Dependencies001+002 retain only the generated-exact and complete vendor-mesh obligations that remain in scope;010 carries013/006/008,009 and007/014 integration. Do not delete historical evidence or weaken the applicable case/PCB/source-completeness checks when replacing required tests.

2026-10-17: User decision: cutover no longer requires a general exact B-rep Boolean engine. Exact geometry is required for cases and PCBs via 013; keycaps and vendor STEP are mesh-based by design, and that is not a forbidden preview-only replacement.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.
