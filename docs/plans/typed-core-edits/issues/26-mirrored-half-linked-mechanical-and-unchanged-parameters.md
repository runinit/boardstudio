# 26: CreateMirroredHalf mechanical features as linked pair features

Status: implemented (awaiting verification)
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

- [ ] Linked openings/mounts exist on the mirror in both modes; none cleared, none cloned.
- [ ] No parameter differs between source and mirrored parts (mutation-checked).
- [ ] Physical setup and the Core operation agree on the reused-PCB result.

## Verification

Core tests, boundary parity native and wasm, `repo lint test typecheck browser`.

## Update (ticket 18 verification, 2026-10-10)

Ticket 18's verification already made the mirror's features linked pair features in both modes
and copies every parameter unchanged (commit after `af4c329fd`). Remaining here:

- The equivalence test in `web/crates/catalogue`: on one accepted document, compare
  `propose_with_mirror_pair` + `into_submission(Some(&mounting_hole_template()))` against
  `CreateMirroredHalf` `ReuseBoard` (instances, pair, features, generated `case-closure/*` parts).
- Report every difference the comparison exposes (expected: the `halves` record, topology/transport
  and relabelling, a derived versus supplied map). Do NOT move physical setup onto the Core operation
  here: that is part of the web adapter, ticket [21](21-mirrored-half-web-adapter.md). Scope of 26 is the
  equivalence test and its findings only.

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

- **F1 (unexpected, B side).** `CreateMirroredHalf` ReuseBoard labels the mirror from its source's label. In
  `core/src/mirrored_half.rs`, `mirror_half_label` (about line 499) keeps every label except left and right, and
  `mirror_instance` takes the role from `half == "right"`. With a unibody source, the mirror becomes half
  unibody, role central, name "Keyboard mirror" (rows 4 to 6). Physical setup's reference is right, peripheral,
  "Right half". Recommendation, a decision for the user and not fixed here: the ticket 21 adapter must relabel the
  source to left, central, "Left half" before calling ReuseBoard, as the topology step does. Core should either
  refuse a ReuseBoard whose source half is neither left nor right, or label a reused board's mirror as the right
  half, since a reused board is by definition the second half of a split.
- **Expected, no fix needed.** `topology`, `transport` and `sharedConstruction` are set by the adapter, not by the
  Core operation (ADR-0010 amendment). Physical setup relabels the primary (rows 1 to 3). `halves` is present only
  on B (row 7). Both sides store the same pair `construction` and features, so the link itself agrees.

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

- F1 above (the mirror's label rule for a non-left/right source). Decide whether a reused board's mirror is always
  the right half, and whether ReuseBoard should refuse a unibody source.
