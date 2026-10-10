# 105: Reusable mirrored-half derivation kernel

Status: implemented (awaiting verification)
Type: build
Blocked by: [18](18-core-create-mirrored-half.md), [26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md), [27](27-mirror-modules.md), [28](28-mirror-embedded-circuits.md), [31](31-companion-placement-modes.md), [32](32-mirrored-half-labels.md), [39](39-stabilisers-and-companion-fallback.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Extract the reflection rules already used by creation into reusable, private derivation helpers.
Creation behavior stays unchanged. This ticket does not activate reconciliation.

## Evidence and scope

Source evidence: committed `2478fb151`, `core/src/mirrored_half.rs` (`create`, `new_board`)
and `core/src/matrix/layout.rs` (`reflected`). Confirm symbols against root's synchronized
baseline before implementation. Existing creation performs fresh allocation and collision
checks; it cannot be rerun against an existing mirror as the reconciliation algorithm.

- Separate derivation of mapped values from allocation, insertion and pair creation.
- Let later slices supply established correspondence identities instead of minting replacements.
- Preserve switch versus independent-part heading rules; rotated maps; 31's per-item modes
  and mirrored-switch-frame tweaks; 39's stabiliser headings and fallback; 32's atomic labels.
- Preserve generator parameters, side, and all existing creation refusals/changed IDs.
- Do not add public APIs or activate a post-edit/Open hook. Do not add ticket 40 behavior.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs`, narrowly required existing
reflection helpers in `core/src/matrix/layout.rs`, and creation-focused tests in
`core/tests/mirrored_half.rs`. Private child modules under `core/src/mirrored_half/` are
permitted if they reduce this module's complexity; no visibility widening. Any other
source path requires root to assign ownership before editing.

## Test-first and acceptance

- [ ] Add focused derivation cases for an axis and translated/rotated map before extracting;
      preserve the existing creation test suite and its failure assertions.
- [ ] Creation results and changed IDs remain equivalent, including modules/circuits,
      labels, modes/tweaks, stabilisers, side and parameters.
- [ ] A helper can derive into caller-supplied existing identities without allocating
      another board or pair; no runtime reconciliation is enabled.

Independent verifier: mutation-check a reused heading/mapping rule and retention of an
item tweak or unchanged generator parameter, with restored sources touched afterwards.
A behavior-preserving extraction need not invent a failing production bug: record the
pre-extraction cases and the mutation failures that demonstrate their sensitivity.

## Execution and verification gates

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
The entire 19 execution batch runs **alone**: no independent implementation or heavy checks.
Root must first finish 37, integrate/sync the verified prerequisite lanes, complete the batch
check, refresh the bound lane index, and explicitly grant this slice and its exclusive files.
Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
a fresh Sol 6.1 xhigh independently verifies/mutation-checks, then alone may set `resolved`.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Request every test/build/browser/performance command slot from root before running it.
Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when its
behavior/transcript changes; targeted red/green and verifier mutations also need slots.
No full check per slice, no `release`, no background checks; Cargo uses 12 jobs.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.

## Outcome

Implemented by Luna 6 high in `lane/mirror-core`. Added a private `derive` child module with pure pose and part
projection helpers. `new_board` keeps identity planning, collision refusal, and insertion; it supplies the chosen
mirror part id, definition id, and reference to the derivation helper. `layout::reflected` remains unchanged: it
already derives a matrix into a caller-supplied target identity and is used by shared edit paths.

Added a focused creation case for axis and translated/rotated maps. It checks mapped pose and heading, side, and
generator parameters. The focused case passed before and after extraction; the preserved creation suite passed
45/45, including refusal and changed-id assertions. Existing tests continue covering companion modes/tweaks,
stabilisers, labels, modules, circuits, and parameters. No reconciliation hook or public API was added.

Checks (all with `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1`): `python3 scripts/check.py repo` passed (0.1 s);
`lint` passed (9.1 s); `test` passed (74.1 s; its final CAD binary reported 51 passed, 4 ignored); `typecheck`
passed (8.5 s, including the `wasm32-unknown-unknown` page check). The post-extraction focused test passed (17.43 s). The first focused runs exposed test setup
mistakes (the fixture part id and reusing a stale engine revision); both were corrected before the passing
pre-extraction baseline. The earlier full creation run also passed 45/45. Clippy printed warnings from vendored
`cgmath`; typecheck printed existing warnings in the catalogue and case crates. No browser run was needed because
the boundary transcript did not change. Full browser, build, release, and boundary-parity runs were not run.

GitNexus impact before edits: `mirror_pose` was HIGH (10 impacted, 6 direct callers, one `new_board` process);
text search confirmed its production callers and four internal test sites. `layout::reflected` was CRITICAL (21
impacted, 5 direct callers, 8 affected processes); its direct source callers were `new_board`, `sync`, two layout
tests, and the Core resolver. Its implementation was not edited. The initial `new_board` resolution was ambiguous;
the Core candidate was LOW with enrichment skipped, and source search confirmed `create` as its caller. GitNexus
was current when the work began. Before commit, complete `detect_changes` returned no partial/truncated flag,
`changed_files=4` (all four staged leased files, including the new `derive.rs`), `changed_count=8`, `affected_count=10`,
and HIGH risk; all 10 listed processes were `new_board` flows. The delta did not list the new helper symbol. The
freshness check showed the index at `3570e8d94` while this lane was at claim commit `79cbef72b`; therefore the graph
delta is provisional, not a clean or full-symbol-coverage result. Staged-diff inspection and source search confirmed
the new helper and its only caller in `new_board`; the changed path stays within the four leased files. This lane did
not refresh the index; root owns refresh after verified merge.

Commits: claim `79cbef72`; implementation commit ID will be added in the final handoff update.
