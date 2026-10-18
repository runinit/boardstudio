# 012: Cut over and remove Cadrum, OCCT and C++

Status: resolved
Type: build
Blocked by: None

## What to build

Finish the production switch: exact layered cases/PCBs and their STEP export, mesh keycaps for appearance/optional printing, complete vendor STEP previews/alignment and mesh collision validation. Remove Cadrum/OCCT/C++ from CAD production, build, packaging and required-test paths. A general exact B-rep Boolean engine and exact keycap STEP are not cutover requirements.

## Acceptance criteria

- [x] Current production delivers exact cases/PCBs and STEP, mesh keycaps, source-retaining vendor previews and supported collision validation. Required native/WASM/browser checks and the Monstertruck-only performance baseline pass under the user's revised cutover scope; deep 007 quality work is separate.
- [x] Remove old CAD dependencies/locks, OCCT provisioning, C++ oracle/toolchain, stale package/worker paths and obsolete CI caches/image workflows; preserve unrelated native dependencies and source data.
- [x] Prove removal using actual dependency graphs, builds and packaged-artifact audits; no OCCT/C++ runtime fallback remains. Preview meshes cannot replace exact case/PCB geometry or STEP. Mesh keycaps and vendor previews are the approved design and are not a forbidden preview-only replacement.
- [ ] Pass required checks and independent Standards/Spec review on the final integrated source/artifacts before declaring the full goal complete.

## Comments

2026-10-17 removal audit: current root/CAD Cargo metadata and lockfiles contain no Cadrum/OCCT dependency; CAD resolves Monstertruck and the reviewed renderer cgmath fork. The freshly packaged site has the CAD provider and CAD-worker WASM assets, no native shared libraries and no Cadrum/OCCT JavaScript runtime references. OCCT authoring-system strings in source STEP assets are metadata, not links. Audit is retained in `.evidence/cad-dependency-audit.log`. Native test registration and strict workspace lint pass; browser acceptance remains red at Stagger durable saving. CAD cgmath security is repaired and independently reviewed; overall security still fails on root ZIP 8.6.0 advisories, without suppression. Performance and final feature acceptance remain open.

2026-10-17 continuation: `dev` through `16ac60269` is merged at `bdfafbdfa`. The dirty checkout removes the Cadrum dependency, OCCT image/provisioning scripts, C++ STEP oracle and obsolete packaged licences, and builds the provider for `wasm32-unknown-unknown`. These edits are not final acceptance: dependency/artifact audit, full required checks, integrated performance, collision delivery and final independent reviews remain open. Preserve the existing uncommitted work; do not mark resolved from removal alone.

2026-10-17 scope reconciliation:004 and general006 kernel hardening are off the critical path. Dependencies001+002 retain only the generated-exact and complete vendor-mesh obligations that remain in scope;010 carries013/006/008,009 and007/014 integration. Do not delete historical evidence or weaken the applicable case/PCB/source-completeness checks when replacing required tests.

2026-10-17: User decision: cutover no longer requires a general exact B-rep Boolean engine. Exact geometry is required for cases and PCBs via 013; keycaps and vendor STEP are mesh-based by design, and that is not a forbidden preview-only replacement.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.


2026-10-18 standalone CAD security correction: the earlier three-lock security green did not cover `cad/monstertruck/Cargo.lock`, which still selected registry `cgmath 0.18.0` and failed the focused unsoundness audit (`../../../../.evidence/generated-014-integration/parent-cad-lock-audit.log`). Its manifest now uses the existing `../../renderer/vendor/cgmath-next-0.18.1` patch. The package remains `cgmath 0.18.0`; verified upstream identity is `cgmath-next 0.18.1`, revision `ca1e234d333d184894d1838cd67451a871f33ba1`. A minimal offline `cargo update -p cgmath` removes only that lock entry's registry source/checksum; all other packages, locks, fork contents and advisory exceptions are unchanged. One shared manifest inventory now drives provenance verification and all four lock audits. New regressions preserve registry-resolution RED (`../../../../.evidence/standalone-cgmath-security/guard-registry-red.log`), omitted-audit RED (`../../../../.evidence/standalone-cgmath-security/guard-audit-red.log`), and actual pre-patch guard rejection (`../../../../.evidence/standalone-cgmath-security/actual-standalone-resolution-red.log`).

The expanded project security gate (`../../../../.evidence/standalone-cgmath-security/project-security.log`) passes five guard tests and all four audits, retaining existing unmaintained notices. Focused native qualification 16/16, profile 27/27, production-only strict Clippy, formatting, diff and repo/docs checks pass; unadjusted strict test lint is not claimed green and the dependency future-incompatibility warning remains. The source/resolution receipt (`../../../../.evidence/standalone-cgmath-security/source-resolution-receipt.log`) confirms the unchanged fork tree `8e63c484cd2412397c55fcd66274519de1327477`, so the existing pinned local Miri 36/36 (`../../../../.evidence/miri-cgmath/miri-soundness.log`) is reused. No Miri, broad geometry/WASM/browser build or hosted CI is rerun or claimed; root owns the subsequent two-axis feature review. This closes the standalone dependency-route omission, not migration acceptance.

2026-10-18 CURRENT bounded removal audit: root accepts the audit findings (`../../../../.evidence/final-removal-012/audit-summary.log`) at clean `1abc1b379`, following tested source `01680d84` and documentation-only notes. Current locked production/root graphs, provisioning, required oracles and packaged runtime routes contain no active Cadrum/OCCT/C++ CAD dependency or fallback. Root `.gitmodules` is absent; unrelated native TLS/coverage/platform/fuzz C dependencies and STEP author metadata remain preserved. Actual packaged import inspection (`../../../../.evidence/final-removal-012/packaged-import-inspection.log`) found CAD 75 and Host 103 function imports solely from their respective wasm-bindgen glue and successfully instantiated both bindings. CAD 10,314,033 and Host 14,741,424 bytes are release/no-opt artifacts with a debug page, not optimized-deployment or performance evidence.

Default/production graphs succeeded; optional standalone qualification all-features metadata remains limited by offline absence of `cadmpeg-codec-iges 0.4.0`. The unchanged-fork local Miri receipt above is reused within its original scope; no new Miri or hosted CI result is claimed. The combined qualification handoff (`../../../../.evidence/qualification-v2-integration/HANDOFF.md`) establishes supported generated/vendor/mixed delivery, with unsupported classes and insufficient accuracy Unknown. Remaining order is 007 complete repair movement and default/selected budget enforcement, root disposition of 007/014/010, matched 011 performance (preparation only, no timings), then 012 final checks/review. Isolated owner-witness tracing has not integrated new source. This audit does not close 012 or change ticket states/checkboxes.

## CURRENT — 2026-10-18 (rescope)

Resolved on actual removal/build/artifact evidence and standard final verification: `.evidence/final-cutover` records `check-full=0` across repo/tooling/lint/build/test/typecheck/browser/security/deny, with no active Cadrum/OCCT/C++ CAD dependency in production/root graphs or packaged imports. Historical source comments/test fixtures are not active dependencies. 001/002 quality follow-ons and deep 007 remain explicit non-blockers.
