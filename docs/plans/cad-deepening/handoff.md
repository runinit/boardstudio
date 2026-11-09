# Handoff: CAD deepening

Branch `t3/cad-case-generation`, based on `dev` at `9099b3ef7` (2026-10-20). Nothing is pushed
and no PR exists. Plan and tickets: [map.md](map.md).

## Done

| Ticket | Outcome |
|---|---|
| Boss rule (`beb634350`) | A **Case boss** must be wider than its hole, in Core and the layered kernel; term added to [GLOSSARY.md](../../../GLOSSARY.md). |
| [001](issues/001-collision-query-contract.md) | Contracts share the collision leaves: STEP import options, sha256 check, placement-transform check. Query and operand types stay separate by design. |
| [002](issues/002-polygon-predicates.md) | 15 containment copies became `path_contains` / `polygon_contains`. |
| [003](issues/003-tests-through-build-case.md) | The second profile builder is gone; kernel tests build through `build_case`. |
| [004](issues/004-qualification-mesh-with-evidence.md) | Meshing stopped naming qualification types; finished by 012. |
| [005](issues/005-remove-cadrum-leftovers.md) | Cadrum leftovers removed or commented where kept on purpose. |
| [006](issues/006-review-follow-ups-spec.md) | Spec for the review follow-ups 007–012, with the combined review notes. |
| [007](issues/007-kicad-cli-under-appimage.md) | `kicad-cli` tests drop a leaked AppImage environment and pass from T3 Code's terminal. |
| [008](issues/008-kernel-coordinate-limit.md) | `MAX_COORDINATE_MM` lives in contracts; `build_case` returns an error, not a panic, for bad ring points. |
| [009](issues/009-transform-error-display.md) | One `Display` wording for placement-transform errors (user-visible text changed). |
| [010](issues/010-polygon-area.md) | One signed polygon area; f64 copies migrated bit-identically, exact and f32 copies kept. |
| [011](issues/011-kernel-tests-one-fixture.md) | Kernel tests share one prepared-case fixture; `tests/common` deleted. |
| [012](issues/012-proof-modules-under-qualification.md) | Proof modules moved under `qualification/`; meshing takes an `EmissionObserver`. |
| [013](issues/013-kernel-mount-coordinate-limit.md) | The coordinate check also covers mount positions. |
| [014](issues/014-full-check-green-locally.md) | `footprints/` reformatted (dev was genuinely unformatted under the pinned 1.98.0); the missing baseline test recorded. |

Gate-binary containment copies in `cad/monstertruck-qualification` stay as independent oracles,
commented.

## Verification state

Last complete run, before 013 and 014: `repo`, `tooling`, `build` and `test` passed (2,096
baselined tests); native and wasm32 Clippy clean. Since then each ticket ran its own crates
(kernel 294 passed, 1 ignored; CAD worker 57; `check.py test` 2,097; `cargo fmt --all --check`
clean). A final full run was interrupted, so **re-run first**:

    python3 scripts/check.py repo tooling build test lint

Run it with the sandbox lifted: inside it `wasm-pack` cannot install `wasm-bindgen` (read-only
filesystem). The `browser` step is not run locally (see 015).

## Next

1. Re-run the full check above and fix anything it reports.
2. [016](issues/016-rust-browser-harness.md), `ready-for-agent` spec: replace Node Playwright
   in the drag smoke, CAD runtime capture and Graphite acceptance with a Rust harness on
   chromiumoxide (own manifest, same receipts). Slice it with `/to-tickets`; it supersedes 015.
3. Not yet ticketed: `CaseFeature` positions (such as round seats) and opening points reach the
   kernel without the coordinate check (noted in 013). Ticket it like 013 if wanted.
4. Push the branch and open a PR against `dev` when you are ready.

## Environment notes

- T3 Code's terminal leaks `APPDIR`, `APPIMAGE`, `LD_LIBRARY_PATH` and `GSETTINGS_SCHEMA_DIR`;
  tests now cope, but other tools spawned from it may not.
- `.gitmodules`, `.bashrc`, `.zshrc`, `.gitconfig`, `.mcp.json`, `.vscode` and `.idea` in the
  worktree root are empty read-only sandbox placeholders. `/.gitmodules` is in the local
  `info/exclude`; delete the files after the session from outside the sandbox.
- `boardstudio-cad` (`cad/monstertruck`) and the CAD worker (`cad/wasm`) are separate manifests:
  use `--manifest-path`. Kernel tests are not in the native-test baseline.
- Open shortcut debt (`shortcut:` markers): STEP mapping nesting capped at 128, outline candidate
  work can be quadratic, and a vendored seam check samples rather than proves. None has hit its
  trigger.
