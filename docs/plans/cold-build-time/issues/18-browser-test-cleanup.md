# 18: Faster browser test bodies, then overlap and irrelevant tests

Status: ready-for-agent
Type: refactor
Blocked by: [17](17-browser-overlap.md)
Parent: [map](../map.md)

Two parts. Part A may land on its own evidence. Part B removes or moves tests only with the user's
approval, one list at a time.

## Part A: bounded waits instead of fixed sleeps

`typed-core-edits` 34 converted keymap from sleeps to bounded waits, 37.0 s to 9.66 s, and stopped there.
`layout` is now the batch floor at 33 s of bodies across 243 tests. Fixed-sleep `rendered()` and
`settle()` helpers account for about 257 calls and 11.8 s by static count, and
`layout/src/objects/layout_transform_toolbar.rs` holds 138 wait sites. These are upper bounds from
literals, not measured waits, and no per-test timing exists anywhere.

1. Time `layout` by module filter first, since nothing else reports per-test time. Work where the time is.
2. Convert with `web/crates/ui-shared/src/test_wait.rs` (`wait_for`), in order: layout, case, pcb, parts,
   ui-shared. A wait must name the condition the sleep was hoping for.
3. Replace repeat `settle_until` copies (9 definitions, six with identical bodies) with one helper there.

Do not remove a check, or weaken a condition, to make a test faster. Each converted module gets a
mutation with `scripts/mutation-check.py`: remove the production step the test waits on, and the test must
fail within the bound rather than pass or hang to the runner's 120 s limit.

Stop rule: revert a module's conversion if it adds a flake in five `--only` passes. Stop the part when
`layout` bodies stop being the batch floor, or when a further module gains under 3 s.

## Part B: refactors and test overlap, evidence for approval

Explorer output, partly sampled. Verify each item against the code before proposing it.

| Item | Evidence | Confidence |
|---|---|---|
| `web/tests/*.rs` compile under no target | `autotests = false`, no `[[test]]`, 44 `#[test]`s, 21 broken `#[path]`s | High |
| `check-wasm-tests.py:212` treats `web/tests` as native roots | Guard blind spot, follows from the above | High |
| Stale entries in `scripts/wasm-test-owners.json` | Name paths under `web/src/presentation/objects/`, which no longer exists | High |
| `core/tests` helpers copied verbatim (`scene` 14×, `edit` 12×, `open` 10×, `undone` 8×) | About 460 lines; `core/tests/support/mod.rs` holds only `kicad_command` | High |
| `adding_to_a_linked_layout_is_refused_until_ticket_20` | Body asserts creation, not refusal; name kept for the baseline | High |
| `demo_rebuild_tests` in `web/crates/library/src/demo_projects.rs` | Ticket 138 says remove board-specific tests; 11 baselined names | Medium |
| Page and crate both test `anchored_matrix_badges_its_anchor_key`, `clearing_the_selection_clears_the_badge` | Same label asserted at two layers; the mount path may be the point | Medium |
| About 154 `wasm_bindgen_test`s with no DOM or Runtime token | Keyword heuristic with known misses (IndexedDB, wasm exports) | Low |
| Isolation launches `setup_guide::`, `panels::`, `case_workspace::` | Viewport and localStorage likely; `case_workspace` cause only inferred | Medium |
| Runtime test API written twice (`runtime_test_stub.rs`, `runtime.rs` 8037-8340) | Already flagged in the rust audit; revisit per ADR 0021 | High |

Output: one reviewed list with each item's verdict and evidence. Changing a baselined test name or set
is a deliberate `--update-native-baseline` with the JSON diff shown to the user.
