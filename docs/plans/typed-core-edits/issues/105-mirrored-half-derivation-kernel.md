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

Commits: claim `79cbef72`; implementation `f000e62f`.


## Verification

**Verdict: incomplete; status remains `implemented (awaiting verification)`.** Creation behavior is
preserved by the reviewed extraction, but the reusable kernel does not yet cover the companion/stabiliser
rules required by this ticket. This is an acceptance blocker, not a failure in the existing creation output.

Fresh independent verifier: Codex `/root/verify105`, dispatched explicitly as `gpt-6.1-sol` with `xhigh`
reasoning, separate from Luna 6 high `/root/implement105`. The dispatch is the model/effort evidence;
no runtime service-tier evidence was available, so this review does not claim priority/fast mode.
Reviewed implementation `f000e62fb` (Outcome update `ddfd81b2c`) from clean lane HEAD
`1edddb94469a81459226b17671384fa417e2d41f`, worktree
`/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`.
The central registry/prompts and original 105/19 contracts were read. Root granted the exclusive
foreground command window and confirmed that the full original acceptance remains binding.

### Acceptance blocker and smallest revision

`core/src/mirrored_half.rs:1413` calls `layout::reflected`, then lines 1414–1474 still derive stabiliser
headings, literal/duplicate/tweaked companion placement and repaired-companion fallback inside `new_board`.
Those rules read creation-wide `request.companion_mode` / `request.companion_tweak`, and the next statement
inserts the result with `set_matrix`. Matrix-member attribute retention remains coupled to document mutation
at lines 1481–1501. Fallback repair at line 1124 also mutates the draft; fallback selection is carried to this
inline derivation through `repaired_companions`. `new_board` refuses existing target boards/instances at lines
1205–1209 and mints their identities at line 1229 onward. It cannot be called to derive an existing mirror.
The unchanged `layout::reflected` is pure and retains the target matrix identity, but accepts no per-item
mode/tweak or stabiliser inputs. The new `derive` module only supplies general pose and independent-Part
projection. Thus preserving creation and proving the Part helper does not finish the requested separation.

Next bounded implementation pass, within the existing leased paths: extract the existing companion mode,
mirrored-switch-frame tweak, stabiliser heading and fallback value decisions into private pure helpers;
let callers supply established target identities and each item's effective stored mode/tweak/fallback input.
Keep identity planning, collision/refusal checks, insertion, correspondence seeding and pair creation in
`new_board`, which calls those same helpers with its creation settings. Keep matrix assemblies/materialized
parts consistent and preserve all attributes/parameters. Add direct helper cases with existing identities,
distinct item settings, axis and translated/rotated maps, stabilisers and fallback, separately from unchanged
creation equivalence. No reconciliation activation, public API, model/lib change or ticket-40 behavior.
The missing matrix/member boundary and its direct tests exceed a small clearly correct reviewer fix;
this verifier did not implement a second production pass. Minimal paths: `core/src/mirrored_half.rs`,
`core/src/mirrored_half/derive.rs` (or another private child module), creation tests only if needed, and this
ticket. `core/src/matrix/layout.rs` need not change unless a narrowly necessary existing helper requires it.

### Independent evidence

Reviewed `git show f000e62f` against its parent: the full-heading arithmetic was moved unchanged, and
independent Part construction now clones the source then replaces only its id, definition, reference and
pose. The prior creation test names/assertions were preserved. No public visibility, model/lib dispatch,
configuration, goldens, labels, modules/circuit namespace logic or runtime reconciliation hook changed.

Added the private unit test
`mirrored_half::derive::tests::part_derivation_reuses_supplied_identities_without_a_board_or_pair`.
It directly calls the helper with custom existing part/definition/reference identities and no ProjectDoc,
board or pair context; checks independently calculated axis/translated-rotated poses/headings; preserves
side, keycap, locked flag, properties and generator parameters; and checks that the source stays unchanged.
This proves the independent-Part capability, not the missing companion/matrix capability.

Commands used `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1`, sequentially in the lane:

- `cargo test --locked -p boardstudio_core --lib mirrored_half::derive::tests::part_derivation_reuses_supplied_identities_without_a_board_or_pair -- --exact`:
  passed 1/1, 25.04 s compile, 0.00 s test execution.
- `cargo test --locked -p boardstudio_core --test integration mirrored_half:: -- --nocapture`:
  passed 45/45, 7.18 s compile, 16.51 s test execution. This includes exact changed IDs/refusals,
  both map families, distinct switch/general headings, modes/tweaks, stabilisers/fallback, labels,
  side/parameters, modules and embedded circuits/namespaces.
- Heading mutation, one site in `derive::pose`: changed `+ 180.0` to `+ 0.0` and ran
  `cargo test --locked -p boardstudio_core --test integration mirrored_half::creation_derives_independent_part_poses_for_axis_and_rotated_maps -- --exact`.
  Expected exit 101: failed at `core/tests/mirrored_half.rs:552`, the heading assertion (18.82 s).
- Parameter mutation, one site in `derive::part`: cleared `mirrored.generator_parameters` and ran
  `cargo test --locked -p boardstudio_core --test integration mirrored_half::parts_keep_their_side_and_every_generator_parameter_exactly -- --exact`.
  Expected exit 101: failed at `core/tests/mirrored_half.rs:974`, source custom/reversible object versus
  mirror null (18.85 s). After each mutation the exact saved bytes were restored, verified, and touched;
  both restored files had SHA-256 `ae583cc7d378d87df8432e90b76082a5331510a37219c4f5432b740301177629`.

Final restored-source gate: `python3 scripts/check.py repo lint test typecheck` passed all four steps:
`repo` 0.1 s, `lint` 8.1 s, `test` 69.9 s, `typecheck` 7.8 s, total 86.0 s. Core integration passed
561/561; the native golden boundary transcript test passed within that run; CAD passed 51 with four existing
ignored benchmarks. The added private unit test compiled in the native test run. WASM page typecheck passed.
Observed warnings came from vendored cgmath, OCCT, and existing catalogue/case code. An initial direct
`rustfmt --edition 2024 --check core/src/mirrored_half/derive.rs` found one long test-call line; it was
formatted before the passing lint/final gate. `git diff --check` passed.

### Graph scope and limits

`python3 scripts/gitnexus-refresh.py --status` reported `boardstudio-lane-mirror-core: current` before
review. `list_repos` bound that exact path/index to `1edddb944` (all 13 repositories returned; no more pages).
Before the test addition/mutations, `impact` on `derive::part` reported HIGH: four impacted symbols,
one direct caller (`new_board`), through `create` to `lib.rs::apply` / the existing refusal test; ten indexed
`new_board` flows. This HIGH warning was reported before edits. `derive::pose` and its File target returned
UNKNOWN/no resolved callers. Source search confirmed `derive::pose` is called by `mirror_pose` and the
Part helper, and `derive::part` by `new_board`; UNKNOWN was not treated as proof of safety.
No index refresh or hook registration was performed. The added test is unindexed until root's next refresh;
source/diff inspection, not graph absence, establishes its test-only scope. Complete bound
`detect_changes({scope: "all", repo: "boardstudio-lane-mirror-core", worktree: <lane absolute path>})`
returned two changed files, two changed symbols (ticket sections), zero affected processes, LOW summary
risk, and no partial/truncated flag. It did not list the new test symbol, so zero affected processes is not
proof of full symbol coverage. The actual diff is only this ticket and the `#[cfg(test)]` addition to
`core/src/mirrored_half/derive.rs`; production source is byte-equivalent to the reviewed HEAD. Explicit
staging/diff inspection confirms those two leased paths; root alone refreshes after this handoff.

Not run: reconstructed pre-extraction tests (historical baseline remains the implementer's evidence), full
browser, build, release, dedicated WASM boundary parity, full batch check or performance measurement. Production behavior
and the boundary transcript did not change, so those extra runs were not required for this review. Parent19
remains open; this handoff grants no 106 dispatch. A new implementation and fresh independent verification
must close the blocker before 105 can be resolved.
