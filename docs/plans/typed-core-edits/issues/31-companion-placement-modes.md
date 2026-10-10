# 31: Companion placement modes for mirrored switches

Status: resolved
Type: build
Blocked by: [29](29-mirror-switch-and-companion-headings.md), [30](30-independent-parts-use-the-full-heading-rule.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-companion-modes-pads-labels-2026-10-10)

## Request

User, 2026-10-10: a switch's diode and LED on the mirror have three placement modes: (1) literal mirror,
the default; (2) duplicate position, the same side as the source (what the shipped right halves do); (3) literal
mirror with an offset or tweak. Ticket [29](29-mirror-switch-and-companion-headings.md) left this open and
documented Core's current behaviour (literal mirror: local x -7.40 against the shipped +7.40).

## What to do

- Store the mode: a per-pair default (`companionMode`) and a per-item override on the correspondence or exception
  record (extend the ticket 17 `halves` types; absent means the default, so v5 documents stay valid), plus an
  optional tweak `{x, y, rotation}` in the switch's local frame for mode 3. Validation: mode names, finite tweak,
  tweak only with mode 3.
- `CreateMirroredHalf` takes the default mode in the request (default literal mirror) and applies it to every
  switch companion (diode, LED, RGB LED) at creation. Mode 2 places the companion at the source's local offset and
  heading relative to the mirrored switch; mode 3 adds the tweak to the literal mirror.
- Tests per mode with the Corne and the Sofle RGB fixtures: exact poses (1e-9), a rotated pair map, one Undo step,
  invalid combinations refused without mutation. Mutation-check each mode.
- Ticket 19 (reconcile) must keep each item's mode and tweak; note the contract in ticket 19.
- Never change a generator parameter, `reversible` or `side`.

## Acceptance

- [x] Three modes implemented, stored and validated; literal mirror is the default.
- [x] Shipped-fixture mirrors under mode 2 reproduce the shipped right-half diode placement within 1e-9.

## Verification

`repo lint test typecheck`; wasm boundary parity if the transcript changes. Part of the batch after Batch A.

### Verified

Claude (Sonnet 5.5), 2026-10-10, in `.worktrees/lane-mirror-core` on top of `b49904cad`, own `target`. Codex
implemented; Claude verified. Reviewer pass, not a second implementation. Acceptance holds: resolved.

- **Storage and validation (read, not only run).** `halves.companionMode` (kebab-case values, default literal mirror,
  `skip_serializing_if` default so old v5 documents serialise byte-identically and the boundary golden does not move),
  optional per-item `companionMode` and `companionTweak {x,y,rotation}` on correspondences and exceptions, all camelCase,
  `serde(default)`. `MirroredHalves` has no `deny_unknown_fields`, so an older v5 build would silently drop the new fields;
  v5 is unreleased on this branch (no tag), and ADR-0004 versions only shape changes, so **no version bump or migration** is
  needed. `validate_companion` is shared by the creation request and by Open for every correspondence and exception
  (effective mode = item override or pair default): tweak only with mode 3, finite x/y/rotation.
- **Acceptance 2, reproduced independently.** New test
  `duplicate_placement_reproduces_every_shipped_right_half_companion_one_to_one`: for Corne and Sofle RGB, mode 2 with the
  shipped pair mapping gives a one-to-one match (position within 1e-9, heading modulo 360) of every shipped right-half
  diode and LED (only the Sofle RGB fixture has LEDs; at least 21 companions per fixture), by pose, not by id. The default literal mirror matches **zero**
  diodes (local -7.40 against the shipped +7.40) and all LEDs (ticket 29's RGB observation). Codex's own shipped check
  used "any shipped diode matches", which cannot detect two created diodes landing on one shipped pose; mine consumes each
  shipped companion once. Killed by the `y` mutant below.
- **Mutation checks, each disabled alone in this worktree, `mirrored_half::` filter, restored with `git checkout` and
  touched** (36 tests at baseline): literal `offset.x` negation removed (at least 8 tests failed; output capped), literal rotation negation removed
  (at least 8), duplicate `y`, `x` (rotated-key frame) and relative-heading mutants (2 each), tweak `x`, `y`, `rotation` dropped
  (2 each), tweak seeding on the correspondence dropped (2), validation "tweak only with mode 3" removed (1), finite check
  without `rotation` (2), per-item validation call in `halves.rs` fed `None` (1). All die. My new test also dies under the
  duplicate-`y` mutant (3 failures in that run).
- **Other claims.** Rotated pair map (27 degrees) and axis map covered for duplicate and tweak on both fixtures; one Undo
  step restores the source; refused requests (bad mode name, tweak with mode 1 or 2, non-finite via the typed boundary,
  `reuse-board` with a tweak) leave the document byte-identical (`refused` compares serialised snapshots). The
  transcript golden and every file under `core/tests/fixtures` are untouched by `b49904cad` (0 diff lines), so the wasm
  `boundary_parity::` run was not needed and was not run. The native `boundary_parity` test passes in `test`.
- **Commands and load (shared machine, load average 7 to 30 from other agents; small timing differences mean nothing).**
  `cargo test -p boardstudio_core --test integration mirrored_half::` baseline 31.8 s wall, 36 passed (load 5.6); mutation
  runs 23 to 35 s each (load 7 to 17). `python3 scripts/check.py lint` passed 9.1 s (load 8.3); `test` passed, 61.7 s total
  (load 8.2 to 28; CAD 51 passed, 4 ignored); `typecheck` passed 13.6 s (load 29.8); `repo` passed, see the commit.
  Not run: `tooling`, `build`, `browser`, `deny`, `release`, wasm parity, ignored benchmarks. Cargo needed the sandbox
  lifted to write the lane's `target` symlink (read-only otherwise).
- **GitNexus** not used (stale); callers checked with `rg`: `create` has one non-test caller (`apply` in `core/src/lib.rs`).

### Findings and follow-ups (none blocks acceptance)

1. **Mode 2 and 3 apply to every matrix assembly, including stabilisers and other non-electrical assemblies.** Codex says
   so ("other matrix assembly companions"), and the plain LED test pins it. Duplicate placement keeps the relative heading, so a
   stabiliser (`STAB_MX_2u`, mirror-symmetric but not half-turn symmetric, see ticket 29) at a relative heading of 90
   degrees would keep +90 where the literal mirror gives -90 (a 180-degree difference). No fixture has a stabiliser
   assembly, so it is untested. Decision for the user, at ticket 19 or 20: skip the `SYMMETRIC_DEFINITIONS` in modes 2
   and 3, or accept it.
2. **No way to change a mode after creation, and `reuse-board` stores the default but no per-item record** (Codex's
   decision, tests pin it). Mode edits and reconciliation belong to ticket 19, whose contract text Codex extended; I read
   it and it matches the code.
3. Mode 2 reads the source key and companion **parts** (`doc.parts`) rather than the source assembly's offset, so it fails
   with "A companion's source switch is unavailable" if a matrix cell's parts are not materialised. A one-line message in
   a corner no fixture reaches; left as is.


## Outcome

Implemented by Codex (GPT-6-based agent), 2026-10-10, only in `.worktrees/lane-mirror-core` on
`lane/mirror-core`. Claim committed as `f54303ec2`. This is an implementation report; independent
verification has not been performed.

### Changes and decisions

- `halves.companionMode` stores the pair default: `literal-mirror`, `duplicate-position`, or
  `literal-mirror-with-tweak`. Absent means literal mirror and serializes without an added default
  field, preserving existing v5 documents and the boundary golden. Correspondences and exceptions
  have optional `companionMode` overrides and `companionTweak: {x, y, rotation}`. A missing item mode
  inherits the pair default; a missing tweak is zero. Unknown names fail deserialization. Tweaks
  require effective mode 3 and finite coordinates/rotation; errors identify the record location.
- `CreateMirroredHalf` accepts `companionMode` and an optional initial `companionTweak`. The latter
  seeds every companion correspondence, rather than storing a pair-wide tweak. Mode 2 derives the
  source companion's actual offset and heading relative to its switch. Mode 3 adds mm/degrees in the
  **mirrored switch's local frame** to the literal reflection. Both stored matrix assemblies and
  generated part poses use these settings. Companions are identified by matrix assembly membership,
  including a plain/imported LED with no generator; no footprint-name allowlist is used. This also
  applies the selected default to other matrix assembly companions.
- `reuse-board` still copies no parts and stores the requested default. It refuses an explicit
  `companionTweak`, which otherwise had no per-item record to retain it. This refusal is an extra
  consistency decision beyond the ticket's explicit list, covered by a red/green test.
- Seven public-boundary tests were added. Corne and Sofle RGB cover duplicate and tweaked placement
  under axis and 27-degree maps, with 15/-20-degree source keys, absolute 1e-9 pose tolerances,
  persistence and one-step Undo. Duplicate placement matches shipped right-half diode positions
  and headings (headings modulo 360). Existing literal-mode coverage remains and an explicit literal
  mode is also tested. A synthetic plain LED has a custom definition with no generator or pads:
  it tests placement, not real single-colour footprint geometry. Other tests cover invalid requests
  and records, inherited/overridden settings in linked/local/omitted records, and NaN/+Inf/-Inf in
  every tweak coordinate at the typed creation and document-validation boundaries.
- Ticket 19 now records the exact inheritance, frame, persistence and matrix/part consistency
  contract. Reconciliation, a mode-edit UI/API for existing companions, and recipe changes remain
  with later tickets. No generator parameter, `reversible`, authored `side`, ADR, map, CONTEXT or
  generated golden was changed. The sole web edit is two default fields in an existing typed test
  constructor; mounted web behavior was not changed.

### Commands and observed results

All check invocations were sequential, in this worktree, with no second check started while one
was running. Each test invocation was `python3 scripts/check.py test`; its normal runner internally
batches native workspace, footprint and CAD commands. Successful durations below are the runner's
printed step totals. Failed-run timings are the printed workspace / footprint / CAD subcommand
seconds, because the runner does not print an aggregate on failure. These are not isolated benchmarks;
Cargo printed package-cache/build-lock waits in some runs. Logs are retained under the lane's ignored
`target/ticket31-*.log`.

| Test run | Observed result | Printed seconds |
| --- | --- | --- |
| Initial cold duplicate red | Exit 1; output truncated, so repeated with a log; total timing not retained | not retained |
| Logged duplicate red | 545 integration passed, 1 failed: stored mode absent (`null`) | 34.7 / 14.9 / 36.9 |
| First duplicate implementation | Compile failure: private `member_id`, missed internal `create` argument; corrected without widening visibility | 21.0 / 6.2 / 10.3 |
| Duplicate iteration 2 | 545 passed, 1 failed: incorrect test count threshold (Corne has 21 companions) | 99.8 / 6.2 / 48.7 |
| Duplicate iteration 3 | 545 passed, 1 failed: test loop selected a non-part correspondence; added kind filter | 37.4 / 12.2 / 36.3 |
| Duplicate green | All native checks passed | 42.6 |
| Tweak red | 546 passed, 1 failed: literal offset instead of tweaked offset | 50.3 / 14.8 / 48.3 |
| Tweak green | All native checks passed | 122.6 |
| Validation red | 547 passed, 1 failed: invalid tweak accepted | 46.2 / 9.9 / 39.9 |
| Validation green | All native checks passed | 64.4 |
| Plain LED red | 548 passed, 1 failed: duplicate mode still reflected the custom LED | 41.1 / 9.3 / 37.8 |
| Plain LED green | All native checks passed | 60.3 |
| Reuse tweak red | 551 passed, 1 failed: tweak accepted and discarded; typed finite/override persistence tests passed | 36.5 / 9.7 / 30.9 |
| Green before mutations | All native checks passed | 56.9 |
| Literal mutation | Flipped reflected local X again: 545 passed, 7 failed, including existing switch/companion and plain LED assertions | 53.9 / 7.4 / 39.1 |
| Duplicate mutation | Negated duplicated local X: 550 passed, 2 failed (duplicate and plain LED tests) | 52.7 / 7.3 / 37.8 |
| Tweak mutation | Dropped tweak rotation: 550 passed, 2 failed (tweak and plain LED heading assertions) | 52.6 / 8.7 / 37.8 |
| Restored final tree | All native checks passed, including strengthened shipped diode heading comparison | 55.3 |

Each mutation was made only in this worktree, restored byte-for-byte from a saved good copy, and the
source touched after restoration. Final `cmp` against that copy passed. Final native output:
Core unit tests **301 passed, 3 ignored**; integration **552 passed, 0 failed**, including **44 mirrored-half
tests** and `boundary_parity::native_transport_reproduces_the_golden_transcript`; CAD **51 passed,
4 ignored**. The standard matrix performance target reported **0 passed, 1 ignored**; its ignored
release benchmark was not run. Existing literal behavior already passed before the change; its mutation
provides failure evidence. Typed finite and override persistence coverage passed when first added.

- `cargo fmt --all`: run after edits; not separately timed.
- `python3 scripts/check.py lint`: passed, **57.5 s** (rustfmt and Clippy with warnings denied).
- `python3 scripts/check.py typecheck`: passed, **84.3 s** (WASM page compilation).
- `python3 scripts/check.py repo`: passed, **0.1 s**; 4 link-check tests and links in 180 Markdown files. Repeated after recording this Outcome.

### Limits and gates

- No boundary transcript change and no re-bless (`BOUNDARY_BLESS=1` was not used). The explicit
  wasm `boundary_parity::` command was **not run**, as instructed when the transcript is unchanged.
- Not run: `tooling`, `build`, full `browser`, `deny`, `release`, full/quick check or ignored benchmarks.
  Only the requested `repo`, `lint`, `test`, `typecheck` steps were used. No sandbox override was requested.
- No GitNexus refresh or graph commands, per the lane brief; the index may be stale. Caller searches used
  `rg`: creation dispatch is `apply` in `core/src/lib.rs`; its typed constructor also appears in the
  catalogue equivalence test. Halves validation is called by creation, Open and mechanical feature
  validation. No graph risk/all-clear is claimed.
- The exact real single-colour footprint, mounted browser behavior, per-item re-derivation on later
  edits, and independent verifier mutation review remain unverified here. Ticket 19 owns ongoing
  reconciliation; the web adapter owns UI. No push or merge was performed.
