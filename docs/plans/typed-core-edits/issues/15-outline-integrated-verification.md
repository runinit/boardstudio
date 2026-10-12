# 15: Integrated outline verification and independent reviews

Status: resolved
Type: task
Blocked by: 14
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Contract: [03 Answer](03-decide-outline-version-intents.md#answer)

## Goal

Verify the complete typed/linked outline contract, not individual helper completion.
Use real Session/Core pending edits, portable archive and browser persistence, mounted
presentation, exact selected-output gates and the combined native/WASM transcript.
Record independent Standards and Spec reviews before accepting tickets 11-14.

## Required cases

1. Matrix/mechanical changes and outline actions in both queue orders use current
   accepted geometry and one Undo step per accepted action. Fixed alternatives stay
   fixed; linked alternatives follow source placement/geometry/inclusion changes.
2. Component removal cleans only dependent outline choices, recalculates bridges and
   restores all of it on one Undo. Recreated entities do not receive old pending edits.
3. Active-version/relevant-source changes retire pending shape edits. Unrelated edits
   survive; malformed input errors atomically; no-op field edits add no history.
4. Save/reopen and browser writeback preserve link status, IDs, settings, active choice
   and exceptions. Freeze keeps both linked and edited versions with correct ownership.
   Legacy projects do not gain links or move geometry.
5. Findings identify affected board/version/choice/location; inactive alternatives and
   unrelated boards do not block valid exports. Saving and repair remain available.
6. Append typed outline requests to the combined native/WASM transcript without
   silently replacing prior matrix/mechanical evidence. Bless only reviewed semantic
   changes at the final integrated source; use one transcript owner.
7. Run the actual product: accept a refinement, move/rotate/add/remove source components,
   change settings, Freeze/edit a copy, Undo/Redo and reopen. Verify the visible boundary,
   bridges, ownership indicators and output gates, not merely serialized enum values.

## Gates

- `CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target python3 scripts/check.py`
  (all steps), plus affected Core/application and mounted Layout/browser suites.
- One coordinated Chrome owner. Native Rust tests alone do not validate WASM UI.
- A non-nested validation worktree is allowed for Cargo's separate CAD workspace
  discovery; retain pinned OCCT provisioning and existing CAD assertions/tolerances.
- Refresh the graph and run complete change analysis against integrated source before
  commits. Partial/truncated evidence is not a clean gate; document static-analysis
  limits instead of treating zero callers/processes as safety.
- Independent Standards and Spec review, fixes and affected rechecks. Review the whole
  confirmed Answer, not just the latest implementation diff.

## Outcome

Record integrated commits, per-case evidence, executed checks and review dispositions.
Keep build tickets open for any unmet criterion. Ticket 03 records design approval only;
these Outcomes establish implementation acceptance. Mechanical 02 and the separate
keyboard/matrix follow-ups retain their own required gates and scope.

Resolved 2026-10-09 at integrated `77b9548b8` (parent branch `typed-core-edits/mechanical`).
Lane work: transcript `39c0a6c06`, queue orders `80147bf69`, lock `b8bcc687a`, merged at
`de45d0e7d`; the ticket 14 fix round (Refine control, retired-id guard) merged at `75a761e34`.
The parent then fixed what the independent reviews found: Core guards and linked-choice
cleanup `6804c12db`, web fixes and tests `8fa4b3b4c`, dispositions `77b9548b8`.

**Boundary transcript.** Append-only: 28 `apply-outline-edit` entries, +20607/-0, prior
matrix and mechanical entries byte-identical, 60 core entries (about 1.0 MB). Red before
bless, with every semantic assertion green first. Native `boundary_parity` 2 passed, WASM host
`boundary_parity::` 1 passed, and both still pass after the review fixes (the fixes changed no
wire behavior). The transcript exercises `apply`; `evaluate`/retirement is covered by web-native
tests. A future trim needs its own bless.

**Required cases.** 1: `application/tests/outline_queue_orders.rs` runs matrix, mechanical and
outline actions in both orders, plus a linked refinement following a queued placement while its
fixed alternative keeps its saved shape, in both orders, with extents worked by hand. 2: component
removal cleans only dependent choices on every removal path (`remove-parts`, matrix key deletion,
matrix removal), in the same edit and Undo step (`core/tests/outline_linked_resolution.rs`).
3, 4, 5: the Core, persistence and mounted tests named in the case map (retirement and no-op
history, save/reopen and writeback parity, legacy reopen without links, Freeze, located findings
and export gating). 6: above. 7: the product gesture pass was partial at the first
verification (no entry point for linked refinements, Copy appeared to do nothing). Ticket
14's second round added the Refine-outline control and found Copy's cause (a stale core-worker
provider asset served by `scripts/dev-web.py`, which rebuilds providers only when a file is
missing), and re-drove those gestures live (screenshots, save/reopen read-back from IndexedDB).
Not re-driven live after the review fixes: linked-refinement settings editing, cleanup on matrix
key deletion and the gap review guard (all covered by Core, native and mounted tests above), and
the material-bridge-with-authored-connection gesture, which the bundled demos cannot produce.

**Full check at `77b9548b8`**, split across worktrees. Parent: `repo tooling lint typecheck`
passed; `build browser` passed (browser suites 8/29/5/26+1/30/16/40/12/81/65/49/146+1 and 60 page
tests, 0 failed; 15 CAD browser checks). `/tmp/boardstudio-tce-verify`: native `test` passed
(core 271 + 463, CAD 51 passed / 4 ignored). `deny` passed. `security` was not run. The `build`
step cannot run in the verify worktree (its `cad/.cache` symlink dangles inside the container and
cadrum then tries a blocked OCCT download), so the parent covers it. Timings with 12 cargo jobs
(`.cargo/config.toml`, `471e3dc11`): `test` 123 s, `build browser` 979 s, `lint typecheck` 25 s.
`build browser` is still slow: `wasm-opt` on the two release workers, one Chrome launch per
`wasm-pack test` call and the forced core rebuild; this is tooling work outside the outline lane.

**Review dispositions.** Two fresh-context read-only reviews (Standards, Spec) over
`71e159d91..471e3dc11`. Fixed: Core now validates settings ranges (non-finite or negative values
are refused with no mutation); a version's base contour cannot be removed; linked choices are
cleaned on every removal path; a linked refinement's margin, bridge width, automatic cleanup and
maximum gap span are editable (a fixed copy offers none); protecting a gap carries the reviewed
source shape; a linked/fixed queue-order test; the false "no adapter re-derives identity" claim
and the `planner.rs` comment; non-durable `/tmp` report references replaced with the tests that
carry the evidence. Accepted deviations: Copy, Refine and connection ids and the default name are
still chosen in the adapter and validated by Core (ticket 14); removed feature, connection and gap
ids are not reserved against recreation, only version ids (ticket 13); connection anchors and
`Copy`/`Refine` ids are captured when the action is built rather than when the queued edit runs;
the last-valid display is the persisted projection from creation or the last prune (ticket 12);
the queue-order test covers two orders of three actions, not all six; the transcript has no source
move after a refinement is created (Core tests cover it). Deferred to
[docs/backlog.md](../../../backlog.md): duplicated version lifecycle and identity-namespace code,
repeated contour re-id and version-push code, `SettingsField` mirroring, the
`unreachable!` in the malformed-admission path, byte-versus-UTF-16 name length, the missing
CONTEXT.md terms, the hardcoded linked finding id, and the other nits. The reviewers' note that the
`[build] jobs = 12` default is unrelated to the outline work stands, and it was kept on purpose at
the user's request in its own commit.

Tickets 11-14 are accepted with the deviations recorded in their Outcomes and above. Keyboard
ticket 04 stays blocked on matrix 11; matrix 04-11 are separate efforts.
