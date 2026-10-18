# 011: Measure integrated CAD performance

Status: resolved
Type: build
Blocked by: None

## What to build

Record a source-bound Monstertruck-only production baseline for build, size, startup, representative CAD operations, STEP export and sampled memory. The user removed the Cadrum comparison requirement; this baseline makes no comparative regression or speedup claim.

## Acceptance criteria

- [x] Bind current Monstertruck observations to source, artifacts, workloads, options, environment and cache state.
- [x] Measure one clean release build and artifact sizes, startup, representative cold/warm CAD operations, STEP export and sampled process-tree memory.
- [x] Publish descriptive measurements, sample counts and limitations; make no invented comparison, numerical acceptance threshold or unsupported memory-bound claim.
- [x] Check the measurement harness and preserve the existing native/WASM/browser acceptance. Final migration review covers the resulting evidence and scope.

## CURRENT — 2026-10-18

Resolved under the user's single-backend scope. The [production baseline](../../../research/cad/monstertruck-performance-baseline-2026-10-18.md) records one clean release build, 15 CAD workloads with one cold and five warm repetitions each, two startup modes and sampled process-tree PSS. All 37 selected release-baseline coverage cells are complete. U1 render/download checks are validation only; checked-profile, growth, no-op and incremental modes were not selected. Deep 007 work and a Cadrum comparison are not prerequisites.

## Comments

2026-10-17 continuation: The retained layered-case performance report measures two frozen native construction fixtures and explicitly excludes production/browser/startup/export/memory acceptance. It is insufficient to close this ticket. Comparable integrated cold/warm, build-size, startup and memory measurements remain required after 010 acceptance.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.

2026-10-18 untimed harness preparation: the [executable runbook](../../../research/cad/integrated-cad-performance-runbook-2026-10-18.md) binds retained C1/C2/C3/P1/V2/K1 requests and current complete155-face Panasonic admission without reopening delivered CAD scope. A small driver captures normal builds/sizes, packaged production-worker cold/warm/STEP/print/refinement/owned-query observations, process-memory samples and descriptive CSVs after root clearance. Three Python and four controlled Node protocol checks pass; no browser/provider, Cargo/wasm-pack/Dioxus/container build or acceptance benchmark ran. The historical detached no-opt provider remains read-only. Fresh matched full/optimized artifacts, the quiet window, frozen-project U1 rendered/download endpoints and full printable STL/ZIP admission remain explicit prerequisites. Status and acceptance boxes are unchanged; root owns final measurements/reviews/closure.

2026-10-18 untimed validation successor: the explicit `validate` route binds the retained official parent artifact receipt (tested01680d84, docs-only d9b58b6d) without fabricating build-timing data. A source-frozen nine-invocation replay passes actual packaged worker C1 preview/STEP, complete Panasonic155/V2-529 import, V2 refinement, qualified default/tight0805 queries and25-cap print transport, plus current-page import/render/export of the frozen ordinary U1 project. U1's real Runtime mechanical ZIP has five-solid Case/foam/physical-PCB assembly STEP and four part STEPs; its full four-cap STL/identity/mm-manifest/ZIP/download passes independent decoding/material checks and optional installed STEP-reader QA. Guide-hidden-inspector failures and a rejected concurrent-driver-commit capture remain recorded. Five Python and five controlled Node protocol checks plus STL controls pass. Diagnostic captures are rejected by benchmark reduction. The earlier36-build paragraph is retained with an explicit root override:12 single initial observations across checked/release, extra cycles only for unresolved variability/regression; runtime session sampling is unchanged. Historical loader/page/worker/exports, optimized products, post-007 admission and quiet-window acceptance timings remain blocked. No source geometry, contract, dependency, parent/protected baseline, status or acceptance checkbox changed; no build/install/review/full benchmark ran. Exact commands, artifacts and limits are in the linked runbook and lane `.evidence/011-validation/HANDOFF.md`.

2026-10-18 parent tooling integration: the complete reviewed source chain from `t3-1d37b199` landed without conflicts: `d5ad6b523 → ba0af915c`, `2c6804532 → eb4b242c7`, `f51018ca1 → c6e588ce3`, `2bd9237e4 → e98438bf9`, `2ea2ca48d → 8ccc61f8b`, `6de0b96ae → 3b88bdfc7`, `a8297a533 → a2ffbeb3d`. Full mappings (`../../../../.evidence/011-tooling-integration/source-mapping.log`) retain exact hashes. All19 source paths matched accepted `a8297a533` byte-for-byte before this append-only note; scripts, frozen fixtures and both research documents remain identical afterward. Standards had no actionable findings. Root accepted the Spec P2 correction after inspecting the shared capture/reduction binding validator and genuine frozen-driver RED (24 failed assertions)/corrected GREEN evidence: missing/mixed/drifted source or harness bindings fail closed, only the exact authorized incremental comment is allowed, and generated outputs/stamps may change.

Fresh parent Python12 (`../../../../.evidence/011-tooling-integration/python-tests.log`), controlled Node5 plus STL controls (`../../../../.evidence/011-tooling-integration/node-tests.log`), syntax, repo/tooling (`../../../../.evidence/011-tooling-integration/repo-tooling.log`) and diff checks pass. The source lane's nine actual-worker/page diagnostic runs and original RED/GREEN receipts are reused in place via its `.evidence/011-validation/HANDOFF.md` and `.evidence/011-build-binding-fix/HANDOFF.md`; no ignored historical evidence was copied or reconstructed. No production CAD/web, dependencies or analysis contract changed. No Cargo/wasm-pack/browser/build/container/install/benchmark or further review ran.011 remains ready/blocked with acceptance timings uncleared (see CURRENT for the live prerequisites);012 remains blocked by007 and011. No ticket state or acceptance checkbox changes.

## Answer

The retained release build takes 606.90 s; CAD WASM is 7,317,515 bytes (2,624,504 gzip-9). Representative C1 preview latency is 45.03 ms cold and 1.20 ms warm median; C1 STEP is 36.69/28.35 ms. Raw captures and selected coverage are identified in the baseline report. These are observations on the recorded host, not guarantees or a Cadrum comparison.
