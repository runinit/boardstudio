# Module deepening integration acceptance — 2026-10-08

Final panel/Rust/fixture source: `eebc703d3f1530c654f82fe92fa282c9617fe378` on
`codex/module-deepening-integration`, compared with local dev `30e922ac6`.
CAD container tooling is committed separately at `bf3e26af`; documentation-only
acceptance commits follow these source pins.

## Preserved history and behavior

Parts merge `629ced642` retains all 11 commits through external `43bbdc266`.
Layout ancestry merge `4ce44add6` and working-source snapshot `b0a59b3e4`
preserve external `9bd5ab125` and its 27-file diff without altering either checkout.
Previously accepted Case/Keymap/Keycaps/Library and PCB changes remain intact.
No history is rewritten. PCB's non-building intermediate `63e051463` remains a
historical bisectability limitation. Protected electrical remap is unchanged.

## Executed final gates

Root ran `python3 scripts/check.py lint typecheck test browser build` on the final
source with `CARGO_BUILD_JOBS=2` and the toolchain shim. Repo and tooling steps
passed separately; repo/doc-links are checked again after documentation changes.
The initial production build failed at the CAD container boundary: contracts
inherits workspace lints, but the mounted container tree had no workspace manifest.
Commit `bf3e26af` repairs that boundary with a temporary contracts-only workspace
manifest retaining root lints/resolver and excluding independent CAD crates. Its
Python regression reproduced the missing-root failure; all 17 CAD tooling tests
and the full tooling step pass. Root Cargo.toml/build script changes invalidate
the cache, and explicit job limits reach the container. Independent incremental
Standards/Spec review reports no blockers. The real CAD container and complete production site build pass.
Root executed `python3 scripts/check.py tooling build` with exit 0; the Dioxus
client bundle and final site were generated successfully. Logs are preserved as
`cad-container-build-check.log`.

- Lint and WASM typecheck pass.
- Full native test step passes, including KiCad and CAD: CAD 51 passed / 4 ignored,
  Parts 42, PCB 23, Layout 51 / 1 ignored, UI-shared 8.
- Full browser step passes: CAD provider 15 checks; Host 6; Runtime 29; UI-model 4;
  UI-shared 26 plus panels 1; Keycaps 30; Library 16; Keymap 40; Catalogue 12;
  Case 77; Parts 65; PCB 48; Layout 131 plus setup guide 1; page groups 51.
- The page result JSON reports complete coverage, zero failed tests, missing
  terminal outcomes or duplicates. No test is skipped to mask a defect.

The broader runner uses the repaired explicit 120-second batch budget. The final
Keymap suite completes together; ChromeDriver SIGKILL no longer blocks the gate.
KiCad tests run with the native process environment isolated from AppImage APPDIR.

## CAD repair

Commit `77fe9a8e9` explicitly authors the six historical closure-mount positions in
the rotated-concave fixture. The expected volume and tolerance are unchanged.
A prior Core boundary-predicate change intentionally changed fresh automatic
placement; the old fixture silently depended on those earlier positions. A real
Core regression proves adopted mount positions survive adjacent outline floats.
It does not claim fresh automatic placement is float-invariant. Production
selection, kernel code and the geometry assertion are unchanged. The original
failure was reproduced before the fix; the original CAD volume test passes now.
Exploratory standalone CAD Clippy still reports eight pre-existing lints outside
the added regression; the required workspace lint step passes.

## Reviews and graph coverage

Final Standards review pins base `30e922ac6` through `eebc703d`: zero documented
hard breaches. Two nonblocking heuristics remain: resolver-builder submit names,
and geometry-script tuple JSON drafts. Parts Spec review reports zero blockers at
`eebc703d`; Layout Spec reports zero at `8f013928`, with no later Layout/page changes.
An independent CAD review finds no blockers. Review agents did not run checks;
the executed gate evidence above belongs to root.

The worktree index includes Parts via the tracked `.gitnexusignore` exception.
The complete panel-source comparison at `eebc703d` covers 42 files, 461 symbols and 56 flows,
aggregate CRITICAL risk, with full arrays and no partial/truncated flags.
Risk was reported and reviewed; it was not waived. Static graph analysis still
cannot prove every RSX/dynamic dispatch path, so unresolved callers were verified
against current source. Published flow analysis has analyzer limits; this is not
a claim that every execution path was indexed. The refreshed analyzer reports
570 flows but omits 3,204 candidate entry points and 2,075 callees under global
budgets, with 24 cut walks. The later source-plus-docs/build comparison covers
52 files, 506 symbols and 56 flows (CRITICAL), again with complete returned arrays.
Use the
[worktree CLI workflow](gitnexus-worktree-coverage.md) for new branch symbols.

## Evidence and retained work

Detailed unfiltered logs and graph JSON live in root's ignored
`.scratch/module-deepening-integration/verification/round2/`; final source checks
are `accepted-source-check.log` and the copied `browser.json`.
Root's five unrelated uncommitted files and the external Layout binary diff are
hash-checked before and after integration. External worktrees are retained.
Parent-scoped dynamic row Signal allocations last until parent unmount; disposed
row bindings and metadata are pruned. This is a documented lifetime limit, not a
second settlement implementation.
