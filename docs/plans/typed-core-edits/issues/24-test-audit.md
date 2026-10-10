# 24: Audit the tests after the typed-edit, matrix and mirrored-half changes

Status: claimed
Type: task
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

User, 2026-10-10: "we have a lot of tests which are a bit pointless after all the changes we
made. We need a full audit." Over this branch we replaced the matrix placement meanings
([ADR-0007](../../../adr/0007-matrix-gestures-edit-named-parameters.md)), the closure rules, the
mirror arithmetic and the document versions. Some tests pin replaced mechanisms, some
duplicate others, and a few fail on a version number alone (boundary transcript lines,
literal-version fixtures). The full check is about ten minutes, so each test must earn its time.

## What to do

- Inventory every test (Rust native and wasm, Python tooling, browser) and its time:
  `cargo test -- --report-time` or per-binary timings from `check.py`, wasm suite timings,
  the 15 CAD checks. Record the slowest 30 and each suite's total.
- Classify each test or group: **protects real behaviour**, **duplicates** another, **pins a
  replaced mechanism**, **pins incidental output** (golden noise), **trivial** (asserts what
  the compiler or a type already guarantees), **slow for what it proves**.
- Propose, per class, delete / merge / rewrite as a behaviour test / keep. Do not delete a
  test that is the only guard of a real behaviour; for goldens (boundary transcript, demo
  baseline, footprint goldens) decide what the minimum pinned surface is.
- Execute the approved deletions and merges in small commits, each with before/after counts
  and time, mutation-checking that coverage of the behaviours it protected survives
  (disable the code under test, confirm some test still fails).
- Look for tests that exist only because of the old design: matrix toolbar stagger paths,
  per-key closure suggestions, recipe mirror arithmetic checks, version-literal assertions.

## Acceptance

- [ ] A written audit (counts, time, classification, decisions) committed with the ticket.
- [ ] Removed or merged tests listed with the behaviour each protected and what still protects it.
- [ ] Full check time before and after recorded; no behaviour loses its last guard.
- [ ] The user approves the deletion list before it is executed (HITL checkpoint).

## Verification

Per [verification-process.md](../../../agents/verification-process.md); this is a batch-boundary
ticket, so run the full `check.py` (never `release`) before and after.

Order: schedule after 19 so the new mirrored-half tests are part of the audit, and before 22.
