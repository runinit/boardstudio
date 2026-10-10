# 33: Make the native tests faster (audit phase 2, part 1)

Status: implemented (awaiting verification)
Type: build
Blocked by: [24](24-test-audit.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

User, 2026-10-10: implement all the audit's recommendations ([section 7 speed options and section 6 items 1, 3, 4](../audit/test-audit.md)).
The native `test` step is 74.5 s because the shipped-keyboard recipes are built twice in debug.

## What to do (measure each; keep only what is a net win)

- Share one build: `demo_projects::native_recipes_preserve_all_examples_and_embedded_model_bytes` and
  `included_keyboards::projects()` build the same recipes twice (55 s and about 50 s). Build once per test binary
  (one `OnceLock` shared through `core/tests/support`) and make `included_keyboards` read from it; build the
  reused-PCB project itself only where needed.
- Make `prepare()` cheaper: try per-fixture parallelism and a dev-profile `opt-level` for the core crate; keep a change only
  if `check.py test` and `build` both get faster and `lint` does not get slower. Record before/after numbers.
- `generator_export::every_bundled_generator_exports_a_board_kicad_parses` (15 s): one board containing all generators and one
  `kicad-cli` run that still reports which generator failed.
- `included_keyboards::closure_proposals_on_a_paired_half_...`: 9 projects down to 2 (the synthetic closure tests in
  `mechanical_integration` cover the rule).
- `kicad_parses` passes silently when `kicad-cli` is missing: fail with a clear message unless
  `BOARDSTUDIO_ALLOW_NO_KICAD=1` is set.
- Merge `source_right_splits_reproduce_the_shipped_left_half` into `every_shipped_split_mirror_follows_the_switch_and_companion_rules` and trim
  the Corne reproduction test only where an identical assertion survives (audit D1, D2).
- Do NOT delete the recipe-reflection tests (audit R1): they retire with ticket 23.

## Acceptance

- [ ] `check.py test` wall time before/after recorded (target: roughly 25-35 s, measured not assumed).
- [ ] Every removed or merged assertion names the surviving test; mutation-check that coverage of the behaviour survives.

## Verification

`repo lint test`; full before/after timing run of `test`.

## Outcome

Implemented by Haiku 5.5 in `.worktrees/tce-matrix` (branch `typed-core-edits/mechanical`). Commit trailers name
Sonnet 5.5, as the brief required. Not verified by a second agent yet.

Commits, in the order made (item 4 landed before item 3 because item 3 was still uncommitted at that point; the
two touch disjoint files):

| Commit | Item |
| --- | --- |
| `929a7c0ba` | claim (status `claimed`) |
| `120f33bf9` | 1: one shared build of the ten splits per test binary |
| `7ee6ad0af` | 2: recipes built in parallel, one engine per job, results in input order |
| `448aa3f53` | 2: `[profile.dev.package.boardstudio_core] opt-level = 1` |
| `4797d48af` | 4: closure-proposal loop from nine projects to two (Corne, Sofle RGB) |
| `aeee22e16` | 3: one board with all 36 generators, one `kicad-cli` run; kicad-check fails loudly |
| `5ed6ebeb1` | 5: source-right reproduction merged; two Corne assertions dropped |

### Timings (warm, sequential, 16 CPUs; wall seconds unless marked)

Machine load was high at times (load average 17 during the last integration run), so differences of a few seconds
are within noise.

| Measure | Before | After 1 | After 2 (parallel) | After 2 (opt-level) | Final |
| --- | ---: | ---: | ---: | ---: | ---: |
| `check.py test` | 69.9 | 72.8 | 35.0 | 31.3 | 30.5 |
| `demo_projects::` (test body) | 54.8 | 59.3 | 17.7 | 14.0 | 13.4 |
| `included_keyboards::` (15 tests) | 53.0 | 52.9 | 17.5 | n/r | 10.9 |
| `generator_export::` (8 tests) | 13.8 | 13.9 | n/r | n/r | 0.7 |
| `integration` binary (tests) | 64.0 (546) | 64.9 | 25.1 | 22.0 | 23.2 (545) |
| `check.py lint` | 5.7 | n/r | n/r | 2.9 warm (17.0 cold) | 8.5 (after test edits) |
| `check.py build` | n/r (audit: 206.8 at the Batch A boundary, a different state) | n/r | 50.7 | 45.2 warm (258.7 cold) | not rerun |

"n/r" means not run at that state. Each row's "before" for `check.py test` is the first run on the claim commit
(`baseline-test.log`); the audit's 74.5 s was a different run. The `demo_projects` test body in the "before" column is
the `finished in` time; wall time was 58.4 s.

The first run after each profile or test-binary change includes recompilation: test 148.7 s cold after the
`opt-level` change, and `build` 258.7 s cold. Each checkout that shares `target/` pays that once.

### What was kept, and what was not

- **Item 1 (shared build): kept, but wall-neutral alone.** Serial sharing moved the single build onto the critical
  path (`check.py test` 72.8 s warm), and CPU fell by one build (about 45 to 50 s). It is kept because item 2 makes
  that one build parallel, which is where the wall win is.
- **Item 2a (parallel recipes): kept.** `split_projects` and the unibody boards run as scoped-thread jobs. Packing and
  asset writes stay sequential (pack writes shared asset files, so parallel writes would race). Fixture order is the
  same as before (provenance order checked from the build output).
- **Item 2b (`opt-level = 1` for `boardstudio_core`): kept.** It is faster for `test` (35.0 to 31.3) and `build`
  (50.7 to 45.2) and not slower for `lint`. The brief's drop rule was not triggered.
- **Item 3: kept.** One board, one `kicad-cli` run (0.7 s for `generator_export::`, from 13.8 s).
  `kicad_parses` now panics without `kicad-cli` (or with a version other than KiCad 10) unless
  `BOARDSTUDIO_ALLOW_NO_KICAD=1`, as `kicad_integration.rs` does. Previously it silently passed. The variable is not yet
  documented in README.
- **Item 4: kept.** Corne and Sofle RGB are the two projects left in the loop. The other seven splits are covered
  by `mechanical_integration::closure_proposals_come_from_the_recorded_construction_source` and
  `..._fall_back_to_the_first_member_without_a_recorded_source`, and by
  `included_keyboards::every_split_half_exports_to_kicad_with_its_own_closure_holes` (all ten export).
- **Item 5: kept.** `source_right_splits_reproduce_the_shipped_left_half` is gone. Its checks run inside
  `mirrored_half::every_shipped_split_mirror_follows_the_switch_and_companion_rules` behind `source == "right"`:
  the `U1`/`RST`/`TRRS` match, `constructionSource`, the controller halves and roles, closure mount positions and
  closure holes. Its switch and reflection checks were already there and were not repeated.
  `corne_second_half_reproduces_the_shipped_right_half` drops only the two assertions the every-split test runs with
  the same inputs: `assert_same_switches` on the right half and `assert_reflects_every_part`. Its bay, source-half
  and closure-feature checks have no surviving equivalent, so they stay.
- **Not done: recipe-reflection tests (R1) and web/wasm tests and goldens (34, 35).** Untouched, as the brief said.
- **Reused-PCB** stays in the shared map. The generator and mirrored-half loops that iterate `projects()` include it,
  and dropping it would drop that coverage. Its build cost sits inside the shared build and was not measured alone.

### Removed and merged assertions, with surviving tests

- Ten splits to two in the closure loop: surviving tests are the two `mechanical_integration` closure tests named
  above and `included_keyboards::every_split_half_exports_to_kicad_with_its_own_closure_holes`.
- `source_right_splits_...` merged: surviving test is `mirrored_half::every_shipped_split_mirror_follows...`.
- Two Corne assertions: surviving test is the same every-split test.
- Generator: 36 per-generator board exports and KiCad runs are one combined check. A per-generator run happens only
  when the combined run fails, so a failing generator is still named.
- Test count: 546 to 545. The one removed is the merged `source_right_splits_...` test.

### Mutation checks (temporary edit, restored with `git checkout`, then `touch`)

- **Closure proposals (item 4):** `link_pair_closure_proposals` takes `members[1]` as the source. Caught by
  `included_keyboards::closure_proposals_on_a_paired_half_are_made_once_from_the_construction_source`:
  "corne: proposed once, from the construction source".
- **Generator naming (item 3), (a):** unbalanced footprint text for `ceoloide/switch_mx`. Core's parser rejects it
  first ("unexpected eof") in `export board`, so the KiCad path is not reached. Caught by 4 generator tests.
- **Generator naming (item 3), (b):** a valid `gr_line` on an unknown KiCad layer for `ceoloide/switch_mx`. Caught by
  `every_bundled_generator_exports_a_board_kicad_parses`: "generators it rejects alone: [\"ceoloide/switch_mx\"]".
  This is the path the brief asked for.
- **Missing kicad-cli (item 3):** with `kicad-cli` absent from `PATH`, `exports_front_and_back_switches...` panics with
  the KiCad message. With `BOARDSTUDIO_ALLOW_NO_KICAD=1` it passes.
- **Source-right merge (item 5):** `constructionSource` set to the mirror. Core refuses the pair at `create`, so 23
  tests fail there, including `every_shipped_split_mirror_follows...` and `corne_second_half...`. The merged
  source-right branch itself was not reached, so its assertions are **not isolated by a mutation**.

### Checks run on the final state

- `repo`, `lint`, `typecheck` pass (0.1 s, 8.5 s, 32.9 s). `lint` after the test edits was a recompile.
- `build` passed at the profile and prepare state of `448aa3f53` (45.2 s warm). Later commits touched tests only, so
  the build inputs did not change after that run. It ran with the sandbox lifted, for `wasm-pack` and the CAD
  container. It regenerated the untracked, gitignored `web/assets/fixtures` and `web/target/site`.
- `check.py test` passes on the final state (30.5 s warm).
- `cargo fmt --all --check` clean. `cargo clippy -p boardstudio_core --all-targets -- -D warnings` clean at the
  parallel-recipe commit; the final state's `lint` passed.

### Unverified and open

- **`browser` and the wasm tests were not run.** They are not in the brief's step list, but the dev-profile change
  also applies to wasm-dev builds.
- **GitNexus index is stale** (indexed at `4b372b2`). Impact was `UNKNOWN` for `prepare`, `projects`, `kicad_parses`,
  `export` (LOW, 4 impacted) and `link_halves`. Text search found the callers (`prepare`: the example and the
  demo test; `projects`: `included_keyboards`, `generator_parameters`, `mirrored_half`; `kicad_parses`: only
  `generator_export.rs`; `link_halves`: two calls in its file). `detect-changes` ran but reported no overlap for the
  `Cargo.toml` hunks.
- **Before-ticket `build` was not measured.** Only the post-change values above exist.
- **Concurrent commits on this branch** from another session: `fa789ba1a` (GitNexus refresh script) and `8df541d1c`
  (browser-step plan docs). Not touched by this ticket.
- **Load-sensitive timings:** one integration run showed 27.6 s under load average 17. Rerun: 23.3 s.
- **`BOARDSTUDIO_ALLOW_NO_KICAD`** is not documented in README.

### Models

- Haiku 5.5 implemented, measured and wrote this Outcome.
- Sonnet 5.5 is named in the commit trailers, as the brief required. The implementing agent is Haiku 5.5.
- Verification: pending, Sonnet 5.5 at high effort, per the verification process.
