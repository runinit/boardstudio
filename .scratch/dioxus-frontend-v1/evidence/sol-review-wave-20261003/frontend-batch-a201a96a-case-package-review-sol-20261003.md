# Combined candidate: Standards carry-forward, package verification, Case paired journey and isolated repair

Reviewer: independent Sol 6.1 High, 2026-10-03. Frozen served source: `a201a96a76c0d908580793e36e4c7d315155fbb3`, build `frontend-candidate-batch-20261003`, served at `http://127.0.0.1:34740/` and `/boardstudio/`. This report supplements the existing consolidated Standards review and preserves its findings/history. It does not accept all62 parents or authorize cutover.

| Axis | Disposition | Exact scope |
|---|---|---|
| Standards, combined source | HOLD carried forward | Existing a201 report identifies the Keycaps Part finding/Layout Inspector mismatch and known Parts accepted-refresh dirty draft loss. This browser leg neither clears nor re-reviews those repairs. |
| Package integrity and selected actual serving | CLEAR | Independently checked pinned provenance, command receipts, source inputs, every disk asset in both routes, and served key assets/headers. |
| Spec, actual Case paired leg at a201 | HOLD | New reproducible P2: blocked edit → Undo restores current exact Plate but keeps obsolete generation-block message. |
| Isolated repair readiness | Frozen for next root join | `39ecc81f74c82ffbcc70d09877efd87d0e34e19b`, focused production-selector red/green and WASM type check pass. Actual served repair recheck remains pending. |

## Pinned package evidence

Provenance file is `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-candidate-batch-20261003/provenance.json`; independently calculated SHA256 `a94e2c6989ea98664f31e5dd2f57d62a853b40fcdce348c701f0c80d40eafc7f`, source exact a201. All22 recorded commands have exit0 and retained command logs. At audit time, all1354 recorded source input hashes matched the integration working files, with zero drift. Mutable HEAD may subsequently advance; the immutable package remains pinned to provenance.

All145 manifest assets per route matched packaged disk bytes, zero mismatches. Actual HTTP responses for the page JS/WASM, native CAD WASM, CSS and index, plus core-worker entry/JS/WASM, CAD-worker entry/JS/WASM, renderer JS/WASM and service-worker script, matched manifest hashes at both root and subpath (14 selected responses per route). Index routes returned200. Recorded responses include COOP `same-origin`, COEP `require-corp`, cache-control `no-cache`; fetched WASM uses `application/wasm`. Retained JSON includes full URLs, hashes and headers. This verifies the package and sampled serving; it does not replace parent workflow acceptance or assert that every served asset was independently fetched.

No fullbuild was repeated. Root's combined strict all-target WASM Clippy passed at61ed and a201 according to coordinator evidence. The earlier isolated Case85 review truthfully retains unproven strict-command attribution; flags are not inferred from stdout, and that history is unchanged. Fresh combined strict Clippy remains the coordinator gate for the next repaired candidate.

## Paired mounted Case evidence

Reference React: `http://127.0.0.1:5173/`, reference checkout `5a472a9426e6e38993361da402cd4ec730feb369`, relevant Case frontend files clean at capture. Candidate: exact a201 package above. Fresh isolated named profiles `case-batch-sol-react-20261003` and `case-batch-sol-candidate-20261003`, agent-browser0.38.1, same1280×960 viewport. Both imported the original layered-Sofle archive, SHA256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

Observed paired public actions:

1. Import fixture, enter Case before enabling mechanical stack: Update preview disabled in both.
2. Configure mechanical stack: generation begins automatically without clicking Update preview. Both expose Plate, Plate foam, Bottom foam, Bottom case and PCB in the physical tree/stack; decoded board-model count reaches90/90.
3. Select Plate, observe1.5mm, fill2.0 and press Enter: both accept the blocked edit and retain previous geometry. Candidate Inspector shows accepted2mm alongside previous resolved1.50mm.
4. Candidate Assembly settings during that block: mechanical diagnostic projection says `Mechanical diagnostics · 0No current mechanical findings.`, has0 Show callbacks, while previous resolved stack remains visibly labeled `Previous revision`. This demonstrates stale display retention is permitted without exposing prior-scene diagnostic/Show authority.
5. Reselect Plate and Undo: React says `Geometry current · export available with warnings to review.` and resolves1.50mm. Candidate removes previous labels, restores accepted1.5mm/resolved1.50mm, but still says `Case generation blocked: mechanical findings block case generation`. Actual snapshots/screenshots preserve the mismatch.
6. Reload and reopen Case: both start/recover output without manual Update; candidate eventually reports exact geometry ready and90/90 decoded models. Live preview off/on is observed in both; manual Update remains available while paused.

Both profiles report no browser errors. The initial reopen candidate screenshot was taken while `Generating case…`; the additional `candidate-reopen-auto-settled` capture records completion and is the readiness evidence. Request counts, race/soak guarantees, export, all responsive/accessibility variants and the entire Case parent set were not tested by this bounded leg. The current diagnostics counts differ between the two frontends in this fixture (candidate75 vs reference2); this report records that broader parity observation without treating it as newly diagnosed by the status repair. No full visual or geometry-identity parity is claimed: the React current-Plate capture still showed a preparing-3D caption alongside resolved stack/models.

## New P2 and frozen correction

At a201, `web/src/cad_presentation.rs` gives any terminal `GenerationStatus::Blocked` precedence over the ready exact current scene. GenerationStatus retains the last job outcome. Runtime legitimately rebinds completed exact geometry when Undo returns to the same physical inputs; automatic admission then sees reusable output and does not submit another job. The old blocked outcome remains in the model, so presentation incorrectly continues to report a current block.

Correction is frozen at `39ecc81f74c82ffbcc70d09877efd87d0e34e19b` on `codex/case-current-result-status-repair-20261003`, worktree `/home/chris/.local/share/boardstudio/worktrees/case-current-result-status-repair-20261003`, base exact a201, clean after commit. Two files changed: CasePanel delegates its actual production title selection to a small crate-local selector in the existing Case generation lifecycle module. Active Preparing/Running retains priority; strictly reusable current exact geometry then establishes readiness before an older terminal outcome. Missing, previous or preview-only geometry continues to surface the real blocked reason. Failure/cancellation without current exact output keep their prior messages.

The existing reusable predicate is unchanged: exact plus matching scope, accepted token, captured document revision and prepared revision. No Session/Core authority, worker protocol, contracts, cache ownership or diagnostic/Show predicate changed. No public visibility widening, lint suppression, dependency, root source or immutable build mutation. This is a defect correction under current CONSTRAINTS authorization; the root integrates and reviews the next complete candidate without an extra per-child approval gate.

Actual regression calls the same production selector used by CasePanel. Red run, before priority correction, exited101 with7 passed/1 failed: received `Case generation blocked: mechanical findings block case generation`, expected `Exact case geometry ready.`. Green run exited0 with8/8 Case lifecycle tests passing, including active Preparing/Running and blocked missing/previous/preview cases. WASM frontend bin `cargo check` passed. Changed Rust files were formatted with rustfmt and `git diff --check` passed. Native harness emits existing unrelated dead-code warnings; the WASM check does not. This is not represented as fresh strict all-target Clippy or the affected crate's complete tests.

Commands use the older isolated Case target cache only:

```text
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/case11-live-preview-review-repair-20261002/web/target cargo test --locked --manifest-path web/Cargo.toml --bin boardstudio-web case_generation_lifecycle::tests -- --nocapture
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/case11-live-preview-review-repair-20261002/web/target cargo check --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web
```

## Durable evidence and remaining gates

Evidence directory: `/home/chris/.local/share/boardstudio/reviews/case11-batch-a201-20261003`. `evidence-index.json` pins36 artifacts with hashes (including retained paired snapshots/screenshots, package audits, browser errors, red/green and WASM logs); its SHA256 is `ccd84b557251699ccbb8af8cee782044e934f636aaf03b1fe50db5738bb89522`. `reference-receipt.json` preserves the initial baseline capture history separately. The report and index do not overwrite prior source HOLD/CLEAR records.

Prior independent consolidated Standards report: `/home/chris/.local/share/boardstudio/reviews/frontend-batch-a201a96a-standards-review-sol-20261003.md`, SHA256 `5542d97a98d861f2aabfa35d9e012096939cc891bc1fb623ae1d9b31cb8d8dce`. Prior bounded Case85 source report: `/home/chris/.local/share/boardstudio/reviews/case11-live-preview-source-review-85c90bfb-sol-20261002.md`, SHA256 `5183bb06ecdd2f693ef0fdb857b9c209b9685ac96d6b89832bf4bd15cb6a86f7`.

RF handoff: append this evidence to existing RF-015 (viewer/presentation reads can disagree with rendered scene) and RF-009 acceptance accounting as appropriate. Confirmed instance: last job terminal outcome and current rebound exact scene diverge after Undo. Current mitigation is the narrowly tested current-output-first presentation selector; later assess an explicit owner envelope for terminal generation outcomes if broader lifecycle ambiguity remains. Do not add a duplicate RF ID or close structural findings from the bounded fix. RF-001–015 and all62 canonical parent records are preserved; root owns ledger incorporation.

Next root candidate must include the repair, pass its affected integrated checks including strict all-target WASM Clippy, and repeat this exact public Plate2.0→Undo1.5 journey at the served immutable source. Package CLEAR here remains specific to a201. Native repair green does not clear the a201 paired Spec HOLD or complete the Case parent, save/reopen/export, responsive, performance or full-integration gates.
