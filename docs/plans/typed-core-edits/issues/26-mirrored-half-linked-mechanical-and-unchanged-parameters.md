# 26: CreateMirroredHalf mechanical features as linked pair features

Status: resolved
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

[ADR-0010's 2026-10-10 amendment](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md):
the mirror's openings and mounts are linked pair features mirrored through the pair, in
`new-board` and `reuse-board`; mirroring never changes a generator parameter, including
`reversible` and `side`. Ticket 18's first implementation cleared the mirror's openings and mounts
and set `reversible` on mirrored parts. If ticket 18's verification already did this, close
this ticket with a pointer to the commits.

## What to do

- `new-board`: link the source half's openings and mounts to the mirror as `MechanicalFeature`s
  in the pair, mapped through the pair's map; do not clear or clone them.
- `reuse-board`: create the flipped instance, the pair record and the linked features
  (physical setup today clones them and records no pair): migrate physical setup to call
  this operation, so there is one path.
- Copy every generator parameter unchanged; remove any write to `reversible` or `side`. Test
  that all parameters of every reflected part equal the source's, for chiral parts (U1, TRRS,
  reset) and both a reversible and a single-sided design.
- Test linked features: an edit from either half moves both; a local exception stays local.

## Acceptance

- [x] Linked openings/mounts exist on the mirror in both modes; none cleared, none cloned. (Ticket 18's
  verification; re-seen here for `reuse-board` in the equivalence test: four linked features, mutation-checked.)
- [x] No parameter differs between source and mirrored parts (mutation-checked). (Ticket 18's verification;
  not re-run here.)
- [x] Physical setup and the Core operation agree on the reused-PCB result, with every difference pinned and
  recorded (zero rows) on the synthetic fixture. Shipped-project coverage is a follow-up.

## Verification plan

Core tests, boundary parity native and wasm, `repo lint test typecheck browser`.

## Update (ticket 18 verification, 2026-10-10)

Ticket 18's verification already made the mirror's features linked pair features in both modes
and copies every parameter unchanged (commit after `af4c329fd`). Remaining here:

- The equivalence test in `web/crates/catalogue`: on one accepted document, compare
  `propose_with_mirror_pair` + `into_mirrored_operation` against
  `CreateMirroredHalf` `ReuseBoard` (instances, pair, features, generated `case-closure/*` parts).
- Report every difference the comparison exposes (at the time, the `halves` record, topology/transport
  and relabelling, a derived versus supplied map). The 2026-10-14 decision in ticket
  [21](21-mirrored-half-web-adapter.md) supersedes the adapter-owned topology/transport finding:
  `CreateMirroredHalf` now owns applying those adapter-selected values atomically. Ticket 21 updated
  this comparison for the new inputs in commit `fd2f217e`; the live comparison now has no
  differences. Retain the old results below as historical evidence.

## Outcome

Implemented by Claude Haiku 5.5. Claimed in `ddd1a396e`, which carries a Sonnet 5.5 trailer as the brief asked;
the harness attribution for this work is Haiku 5.5, so the implementation commit uses that trailer. Not verified;
status is for the verifier. Scope was the equivalence test only: no Core change and no physical-setup change (the
two mutations below were reverted), and physical setup was not moved onto the Core operation (ticket 21).

### What changed

- `web/crates/catalogue/src/physical_setup.rs`: 230 lines added, none removed, all inside the existing `mod tests`
  (219 before `cargo fmt`).
  New helpers `reversible_split_start`, `physical_setup_split`, `reuse_board_half`, `json_differences`, `show`, and
  the test `physical_setup_and_reuse_board_agree_on_a_reversible_split_pair`. The test lives there so it can reuse
  `document`, `mechanical`, `make_instance`, `keep_definition` and `mounting_hole_template`.
- `cargo fmt --all` reformatted only my new test lines after the first `lint` run failed. No other file changed.

### The fixture (one accepted document)

- One board `board-a`, `reversibleLayout` = true, no pairs, no `shared_construction`.
- One unibody instance `left-new` (half unibody, role standalone, name Keyboard, unflipped) with mount `mount-1`,
  opening `opening-1` and closure mounts `closure-1`, `closure-2`.
- A = `propose_with_mirror_pair(Topology { split, selected left-new, new right-new })`, then
  `into_submission(Some(&mounting_hole_template()))`.
- B = `CoreRequest::Edit` of `CreateMirroredHalf { pair_id: "pair/right-new", source: left-new,
  ReuseBoard { instance_id: right-new }, mapping: zero (the flip), BoardPrefix, mounting_hole: template }` on the
  same starting document.
- Limit: this is a synthetic fixture. The shipped Corne and other projects are reachable only from Core's test
  support (`core/tests/included_keyboards.rs`), not from the catalogue crate. The comparison therefore does not yet
  cover matrices, parts, outlines or modules.

### Diff table (JSON pointers; A = physical setup, B = CreateMirroredHalf ReuseBoard)

Observed in the final run. All other JSON is equal.

| # | JSON pointer | A | B | Class |
| --- | --- | --- | --- | --- |
| 1 | `/hardware/instances/0/half` | `"left"` | `"unibody"` | expected, A only (topology relabels the primary) |
| 2 | `/hardware/instances/0/name` | `"Left half"` | `"Keyboard"` | expected, A only |
| 3 | `/hardware/instances/0/role` | `"central"` | `"standalone"` | expected, A only |
| 4 | `/hardware/instances/1/half` | `"right"` | `"unibody"` | **finding F1**, B side |
| 5 | `/hardware/instances/1/name` | `"Right half"` | `"Keyboard mirror"` | **finding F1** |
| 6 | `/hardware/instances/1/role` | `"peripheral"` | `"central"` | **finding F1** |
| 7 | `/hardware/pairs/0/halves` | absent | `{sourceBoardId: board-a, mirrorBoardId: board-a}` | expected, B only |
| 8 | `/hardware/sharedConstruction` | object | `null` | expected, A only |
| 9 | `/hardware/topology` | `"split"` | `"unibody"` | expected, A only |
| 10 | `/hardware/transport` | `"wireless"` | `"none"` | expected, A only |

Asserted equal (named asserts, plus the exact-set check): pair `id`, `members`, `mapping` (A derives the zero flip and
B is given it; the value is equal, and the provenance is not stored), `constructionSource` (left-new),
`construction`, `features` (4 linked: mount-1, opening-1, closure-1, closure-2); instance 1 `flipped` true and
`controllerPartId` null; both instances `constructionOwnership` `pair`; both instances' `mechanical`; the
`case-closure/*` part ids (non-empty on A and equal on B).

The pinned difference set is exact: a row that appears or vanishes fails the test with its JSON pointer.

### Findings

- **F1 (unexpected in the unlabelled comparison, B side).** `CreateMirroredHalf` ReuseBoard labels the mirror from its source's label. In
  `core/src/mirrored_half.rs`, `mirror_half_label` (about line 499) keeps every label except left and right, and
  `mirror_instance` takes the role from `half == "right"`. With a unibody source, the mirror becomes half
  unibody, role central, name "Keyboard mirror" (rows 4 to 6). Physical setup's reference is right, peripheral,
  "Right half". This synthetic comparison omitted the optional labels that ticket 21 already requires the adapter
  to pass. The 2026-10-14 contract supplies the physical setup's source and mirror labels atomically, so these
  historical rows are not a separate Core refusal or relabelling requirement. Ticket 21 updates the comparison
  with those inputs.
- **Expected at the time of this test.** `topology`, `transport` and `sharedConstruction` were set by the adapter, not by the
  Core operation (the 2026-10-10 ADR-0010 amendment). The 2026-10-14 ticket 21 decision moves topology and transport
  application into `CreateMirroredHalf`; this table remains the pre-change baseline until ticket 21 updates the
  comparison. Physical setup relabels the primary (rows 1 to 3). `halves` is present only on B (row 7). Both sides
  store the same pair `construction` and features, so the link itself agrees.

### Mutation checks (sources restored, verified by hash)

- **(a) B clears the linked features.** Changed the `mirror_configuration(...)` argument in `reuse_board`
  (`core/src/mirrored_half.rs`) to clear `mounts` and set `openings` and `closure_mounts` to empty. Result: FAILED at
  `/hardware/pairs/0/features differs` (A has four features, B has none). Restored from a saved copy;
  `core/src/mirrored_half.rs` sha256 `b3c362e4…`, the committed content, `git diff` empty.
- **(b) A changes the mapping.** Changed `derived_mapping` in `physical_setup.rs` to return rotation 180 instead of 0.
  Result: FAILED at `/hardware/pairs/0/mapping differs` (A rotation 180, B rotation 0). Core accepted the rotated
  pair, so the comparison, not Core, caught it. Restored from a saved copy of the file with the new test and no
  mutation (sha256 `dab502f7…`). `cargo fmt` then changed it to its current sha256 `7e644481…`, which is the file
  committed with this ticket. The `rotation: 180.0` text that remains is in an existing test, not the mutation.
- Impact: GitNexus `impact` returned "not found" (risk UNKNOWN) for `derived_mapping` and `reuse_board`, so the index
  is stale for these symbols. Callers were confirmed by text search: `derived_mapping` is used only in
  `physical_setup.rs` (lines 350, 560, 675); `reuse_board` only from `create` in `core/src/mirrored_half.rs`.
  The mutations were temporary and not committed.

### Commands (sequential, foreground, one at a time)

- `cargo test -p boardstudio-web-catalogue physical_setup:: -- --nocapture`, first run before the closure fixture
  was added: 23 passed, 1 failed on the closure-part precondition. The fixture had no closure mounts, so no
  `case-closure/*` parts existed on either side and the earlier "no diff" for parts was vacuous. Fixed by adding two
  closure mounts and expecting four linked features. A second run failed only on row order; fixed by listing rows in
  pointer order. Final `physical_setup::` run: 24 passed, 0 failed, about 1 s incremental.
- `python3 scripts/check.py lint`: first run exit 1, rustfmt diffs in the new test (three hunks, all mine, no other
  file). `cargo fmt --all`, then `lint`: exit 0, 9 s. Clippy warnings were from dependency crates (`unexpected cfg
  simd`, `#[inline]` on macro calls); none point at `physical_setup.rs`.
- `python3 scripts/check.py typecheck`: exit 0, 8.5 s (wasm32 `cargo check` of the page).
- `python3 scripts/check.py test`: exit 0, 87.2 s. The step runs `cargo test --workspace --locked --exclude
  boardstudio-footprints`, `cargo test -p boardstudio-footprints`, and `cad/scripts/test-cadrum.py`, and printed
  `all steps passed: test`. The new test ran and passed (`physical_setup::tests::
  physical_setup_and_reuse_board_agree_on_a_reversible_split_pair ... ok`).
- `python3 scripts/check.py repo`: exit 0, 0.1 s (165 Markdown files, doc links).
- Not run, per the brief: `browser`, `build`. `release` was never run.
- GitNexus `detect_changes` (scope all, before commit): 2 changed files, 0 changed symbols, 0 affected processes,
  risk low. The index is stale (last indexed at 4b372b2) and the new test functions are not in it, so this zero is
  not a clean check.

### Unverified

- Only one synthetic fixture. NewBoard mode, single-sided (non-reversible) designs, matrices, parts, outlines and
  modules are not compared. A fixture built from shipped data (for example the Corne project) is the next step,
  before ticket 21 moves the adapter.
- The catalogue test runs natively. It is not run as wasm (`browser` step), although it uses only core and
  `serde_json`.
- The GitNexus index is stale, so `impact` and `detect_changes` are not clean checks for this change.

### Follow-ups for the verifier

- F1 above is historical behavior when labels are omitted. Ticket 21 supplies explicit physical-setup labels, so no
  separate decision about Core's generic label fallback is needed.

## Verification

Sonnet 5.5, 2026-10-10, `tce-matrix` worktree on top of `b3f4cbd41`. Reviewer pass, not a second implementation. No
source file was changed; only this ticket and ticket 21 were edited. Acceptance holds, so the ticket is resolved. The
GitNexus index does not know `reuse_board` or `derived_mapping` (`impact` returned "not found", risk UNKNOWN); callers
were taken from the implementer's text search and the mutations were temporary, so this is not a clean graph check.

### Claims reproduced

- **Both paths start from one document and run as the product does.** A is `propose_with_mirror_pair` with the
  `Topology` intent and the same normaliser the app uses for topology intents (`catalogue.rs:373-377` returns the
  definition unchanged, as `keep_definition` does), then `into_submission(Some(&mounting_hole_template()))`, which
  opens `CoreEngine` and applies `LinkMirrorPair` through `ApplyMechanicalEdit` (the same call the controller makes at
  `pcb_physical_setup/controller.rs:710`). B opens the same document in a fresh `CoreEngine` and applies the real
  `CreateMirroredHalf` `ReuseBoard` edit with the zero map, the same pair id and the same template. Neither side
  shortcuts the engine.
- **The pinned difference set is exact (10 rows).** The test compares the full JSON of both documents (every leaf, by
  JSON pointer) and `assert_eq!`s the observed rows against a literal list of 10, so any new or vanished row fails.
  Ten rows reproduced on a fresh run.
- **Mutations, each run once, then restored (`git checkout` of the file, then `touch`; sha1 of both files equal to
  `HEAD` afterwards, `git status` clean):**
  - (a) `reuse_board` gives the mirror configuration empty mounts, openings and closure mounts: the test FAILS at
    `/hardware/pairs/0/features differs` (A has four linked features, B has none).
  - (b) `derived_mapping` in `physical_setup.rs` returns rotation 180 instead of 0: FAILS at
    `/hardware/pairs/0/mapping differs` (A 180, B 0). Core accepts the rotated pair, so only the comparison catches it.
  - (c) the `/hardware/transport` row removed from the pinned set: FAILS with "the differences changed" and the
    observed list contains the row.
- **Commands (sequential, foreground, each once, on the restored sources):** `check.py lint` 8.9 s, `check.py test`
  90.0 s (workspace, footprints and cadrum steps; the new test among them), `check.py typecheck` 8.5 s, `check.py
  repo` 0.1 s (after the doc edits); all passed. Mutation runs: `cargo test -p boardstudio-web-catalogue
  physical_setup_and_reuse_board`, about 4 s each. `browser`, `build` and `release` were not run (a test-only change in
  a native test module).

### Findings

- **F1 judgment at the time: an unlabelled-source result, addressed for physical setup by ticket 21's explicit labels.**
  `mirror_half_label` (`core/src/mirrored_half.rs`) keeps any label other than left and right, so a `unibody` source
  gets a mirror with half `unibody`, role `central` (the role rule is `half == "right"`) and the name "Keyboard
  mirror". The result is a pair of two `central` instances with the same half, which no physical setup produces.
  The comparison omitted labels; ticket 21 supplies explicit source and mirror labels on the
  `CreateMirroredHalf` request, resolving the physical-setup case. These rows remain evidence of
  behavior when labels are omitted, not a Core defect or a separate product decision. Ticket 21's
  updated parity test should assert the explicit labels.
- **Rows 1 to 3 and 7 to 10 were expected in this pre-change comparison** (adapter-set topology, transport, shared
  construction, relabel of the primary; `halves` only on B). Ticket 21 revises topology and transport ownership.
  No difference was found in instances' `mechanical`, `flipped`, `constructionOwnership`,
  the pair `id`, `members`, `mapping`, `constructionSource`, `construction`, `features`, or the `case-closure/*` parts.

### Gaps in the test (not fixed; follow-ups)

- **Fixture is synthetic.** One board with one instance, one mount, one opening and two closure mounts. Not compared:
  matrices, parts other than `case-closure/*`, outlines, modules, wiring. Rerun the comparison on such a project when ticket
  [21](21-mirrored-half-web-adapter.md) or ticket [22](22-mirrored-half-integrated-verification.md) can reach a shipped
  project from the catalogue crate (today it is only reachable from `core/tests/included_keyboards.rs`); then rerun the
  comparison on it. If that is not reachable, 22 should assert the same equivalence in a mounted test.
- **Object-valued rows are pinned by presence, not content.** `show` renders any object as "object", so rows 7
  (`halves`) and 8 (`sharedConstruction`, A) would still pass if the object's content changed. A small hardening: render
  compact JSON for these two rows. Not needed for the acceptance.
- **Only the `ReuseBoard` mode.** `NewBoard` has no physical-setup counterpart, so there is nothing to compare; the
  reversible/flip designs and a single-sided design are covered by ticket 18's tests.
- **B is given the zero map; A derives it.** The test shows the values are equal, not that Core would derive one.
  Adequate: ADR-0010 has Core validate a supplied map against the flip.
- The test runs natively. It uses only core and `serde_json`; it is not run as wasm.
