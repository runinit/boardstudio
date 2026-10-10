# 39: Stabilisers keep their heading, and companions fall back to the X mirror

Status: implemented (awaiting verification)
Type: build
Blocked by: [31](31-companion-placement-modes.md), [32](32-mirrored-half-labels.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

User, 2026-10-10 (answering the ticket 31 verification follow-ups): do not rotate stabilisers; there should be no unmaterialised
parts, and the default is the X-coordinate mirror.

## What to do

- Stabilisers (the imported `STAB_MX_*` matrix members) keep their source heading in every companion mode (literal mirror, duplicate,
  tweak) and only their position is mirrored on X; a tweak's rotation never applies to them. Tests at relative heading 0, 90 and 15 degrees under an
  axis map and a rotated map, mutation-checked.
- If a cell's source part is not materialised, mode 2 and mode 3 fall back to the X mirror of the switch's companion position instead of
  returning an error. Add a test that builds such a cell; the fallback must not hide a real validation error (a missing switch is still an error).
- Never change a generator parameter, `reversible` or `side`.

## Acceptance

- [x] Stabiliser headings unchanged in all modes; fallback test passes; the earlier ticket 31 tests unchanged.

## Verification

`repo lint test typecheck`; wasm boundary parity only if the transcript changes.


## Outcome

Implemented by Codex (GPT-6-based agent), 2026-10-10, in the existing
`.worktrees/lane-mirror-core` session on `lane/mirror-core`. Claim commit: `14aa330a`.
The updated agent workflow was read from the main checkout when supplied during this
run; its snapshot names this session as ticket 39's current writer. No fresh model
dispatch or independent verification was performed. Status remains awaiting verification.

### Changes and decisions

- Both imported MX stabilisers (`STAB_MX_2u`, `STAB_MX_6.25u`) retain their source
  **world heading**, including under a rotated pair map. Their position takes the literal
  X reflection. All companion modes bypass position and rotation tweaks for stabilisers.
  The stored assembly heading cancels the mirrored switch's tilt to produce that world heading.
- Missing source companions are generated from their matrix assembly in a scratch document.
  Only the absent parts and missing board/matrix ownership IDs are copied back to the draft;
  existing source parts retain their poses, sides and generator parameters. This repairs the
  source as well as materialising the mirror, so each correspondence names two real, owned parts.
  Disabled/deleted cells are skipped. Missing switches and invalid matrices are refused.
- A repaired companion's mirror receives a per-item `literal-mirror` override with no tweak,
  while retaining the requested pair default. This is a deliberate persistence decision:
  repairing the source must not turn the fallback into duplicate/tweaked placement on later
  reconciliation. This is the only extension beyond the ticket's explicit placement wording.
- Three public Core-request tests were appended. The stabiliser test uses the real imported
  definitions, relative headings 0/90/15 degrees, a 15-degree switch, axis and 27-degree maps,
  all three modes, non-zero offsets and tweaks, authored back side, and reopen checks.
  The fallback test removes both a Sofle RGB diode and LED, with their ownership IDs either
  retained or absent, under both maps and modes 2/3. It checks exact poses within 1e-9,
  materialisation/ownership, persisted overrides, unchanged existing source matrix parts,
  reopen and one-step Undo. The missing-switch test checks atomic refusal in all three modes.
- Earlier ticket 31 tests are unchanged and passed. No generator parameter, `reversible`,
  existing authored `side`, dependency, shared type or web behavior was changed. `reuse-board`
  still copies nothing. No map, CONTEXT, ADR or generated golden was edited.

### Commands and observed results

Every check invocation was one foreground `python3 scripts/check.py STEP`, with no second
check started by this session before it finished. The standard `test` runner internally
batches native workspace, footprints and CAD commands. `uptime` was recorded before timed
runs; the table shows its one-minute load average. Loads reached about 31 during runs:
these wall times are not isolated benchmarks. Logs and the mutation wrapper remain under
this lane's ignored `target/ticket39-*` paths. Shell `time` supplied wall times; the mutation
wrapper used a monotonic clock including restoration.

| Invocation / phase | Observed result | Wall seconds | Starting load |
| --- | --- | --- | --- |
| `test`, first stabiliser test | Compile error: test referenced an unavailable `at` helper; corrected before production edits | 26.7 | 4.94 |
| `test`, stabiliser red | 557 integration passed, 1 failed: source +15 became -15 | 48.5 | 8.78 |
| `test`, stabiliser green | Passed | 81.6 | 16.21 |
| `test`, fallback red | 558 integration passed, 1 failed: `A source companion is unavailable` | 42.9 | 27.42 |
| `test`, fallback green | Passed | 79.1 | 18.14 |
| `test`, refusal and source preservation | Passed | 42.9 | 19.22 |
| `test`, stabiliser heading mutant | Stabiliser heading assertion failed | 51.9 | 13.34 |
| `test`, stabiliser X position mutant | Stabiliser position assertion failed | 52.2 | 20.08 |
| `test`, stabiliser mode bypass removed | Duplicate-mode stabiliser position assertion failed | 52.9 | 20.86 |
| `test`, fallback mode bypass removed | Missing diode's literal position assertion failed | 50.9 | 22.72 |
| `test`, missing-switch guard removed | Refusal test failed: creation incorrectly succeeded | 67.1 | 19.94 |
| `test`, fallback override removed | Stored `literal-mirror` assertion failed (`null`) | 59.5 | 26.41 |
| `lint`, restored final tree | Passed rustfmt and Clippy with warnings denied | 10.8 | 26.99 |
| `test`, restored final tree | Passed | 55.8 | 20.09 |
| `typecheck`, restored final tree | Passed WASM page compilation | 11.1 | 20.80 |
| `repo`, final ticket | Passed: 4 link-check tests, links in 183 Markdown files | 0.2 | 7.85 |

Each of the six mutants changed one site only, caused exactly one new regression test to
fail, then restored `core/src/mirrored_half.rs` byte-for-byte and touched it in a `finally`
block. The restored final native run reported **301 Core unit passed, 3 ignored; 560
integration passed**, including **52 mirrored-half tests** and both native boundary parity
tests; **CAD 51 passed, 4 ignored**. The standard ignored performance tests were not run.
`cargo fmt --all` was run after edits, not separately timed. The first attempt to use
`/usr/bin/time` did not invoke a check because that binary is absent; subsequent timings
used shell `time`.

### Limits and follow-ups

- Independent verification remains required. Reconciliation belongs to ticket 19; the
  typed operation and Inspector editing work belong to tickets 40/41. Those behaviors
  were not implemented or exercised here. These tests cover matrix-generated placements,
  not manual companion pose overrides or physical/browser inspection.
- The protected boundary transcript and demo baseline have zero diff. No bless was run.
  The explicit wasm `boundary_parity::` test was not run because the transcript is unchanged;
  the native golden transcript test passed in the final `test` step.
- Not run: `tooling`, `build`, full `browser`, `deny`, `release`, full/quick check or ignored
  benchmarks. Only `repo`, `lint`, `test`, `typecheck` were invoked. The session already had
  unrestricted filesystem access; Cargo could write the lane's external `target` symlink.
- No GitNexus commands or refresh were run, per the lane brief; the index may be stale.
  `rg` found the creation dispatch in `apply` (`core/src/lib.rs`), and `new_board` is called
  only by `create`. No graph risk/all-clear is claimed. No push or merge was performed.
