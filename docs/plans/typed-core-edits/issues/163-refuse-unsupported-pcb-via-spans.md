# 163: Refuse unsupported via spans in physical PCB geometry

Status: claimed
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [CAD 013](../../cad-backend-replacement/issues/013-layered-case-construction.md), [CAD 014](../../cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md)

## What to build

Physical PCB preparation must preserve the authored meaning of drilled features.
Blind, buried and microvias must produce a named unsupported-source result until
their depth can be represented faithfully. They must not silently become cuts
through the entire PCB in exact exports or collision analysis.

The 2026-10-18 audit at `f955050c3` submitted through, blind and microvias through
the public Core board-source operation. All three returned the same round-hole
description without an error, despite the latter two spanning only the front and
first internal copper layers. The physical builder applies that description
through the board thickness.

## Acceptance criteria

- [ ] Public Core regressions reproduce the current incorrect acceptance of blind
  and microvias with partial layer spans. Include a buried span and an ordinary
  front-to-back through-via control.
- [ ] Inspect both via type and layer span before emitting a physical drill.
  Unsupported or unresolved depth produces a diagnostic naming the source feature
  and reason, using existing source-geometry errors or mechanical gates.
- [ ] Through-vias retain their authored center and diameter, including transformed
  PCB sources. Existing round pad drills and slots retain their behavior.
- [ ] Direct board-source reads and module/physical-PCB preparation propagate the
  refusal. No affected source can become an exportable exact PCB or a successful
  collision operand with an invented through-hole.
- [ ] Preserve original source data and existing independent layout/preview
  availability where the current mechanical-gate policy permits it. The fix requires
  no new partial-depth geometry schema or public API.
- [ ] Run affected Core source/module-import tests and native physical-PCB tests.
  Add focused Runtime or worker verification for any changed propagation boundary.
- [ ] Complete one combined Standards+Spec review of the finished geometry behavior
  under the CAD review policy; record checks and supported via limits.

## Comments

Created from audit finding 2 at the user's request. The smallest delivery is an
explicit refusal for unsupported depth, with ordinary through-holes preserved.

### CURRENT — 2026-10-18 implementation complete; parent verification pending

- Public Core now refuses blind/buried (`blind`), micro and unknown via types, and
  partial, missing, unknown, wildcard or ambiguous layer spans, naming the via and
  reason. Physical drills require an omitted via type and exactly one F.Cu/B.Cu
  endpoint pair (either order); legacy `locked` flags remain supported. Via identity
  retains `tstamp` or `uuid`. No public API/member visibility or pad/slot parser changed.
- Public reads/module imports cover refusal, transformed through-via centre/diameter,
  retained slots, source provenance and independent layout/circuit availability.
  Existing mechanical gates suppress exact module geometry. Runtime browser tests
  prove physical preparation refuses a collision operand and exact export refuses
  without nominal fallback. Source bytes and Unknown thickness policy are unchanged.
- RED: `flock /home/chris/.cache/boardstudio-target/audit162-165-root.lock cargo test -p boardstudio_core --release --test integration direct_board_source_refuses_unsupported_via_depths`
  failed as expected: blind via returned `read-board-source` with an invented round
  drill (`red-core.log`). The same command then passed (`green-core-via-types.log`).
- GREEN: `flock /home/chris/.cache/boardstudio-target/audit162-165-root.lock cargo test -p boardstudio_core --release --test integration module_import::`
  passed all 22 tests (`green-core-module-import-final.log`). The initial group run
  exposed a new test's incorrect assumption that an empty `roundDrills` field was
  serialized; asserting the returned typed definition fixed that test assumption.
- GREEN: `cargo test --manifest-path cad/monstertruck/Cargo.toml --release --lib layered::pcb_tests`
  passed all 14 native physical-PCB tests (`green-cad-pcb.log`).
- GREEN: `flock /home/chris/.cache/boardstudio-target/audit162-165-root.lock env CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target wasm-pack test --headless --chrome web/crates/runtime --lib -- source_pcb_export_tests::`
  passed both browser tests (`green-runtime-source-pcb.log`). Targets were inspected
  before builds; native targets are worktree-local, browser compilation used the
  shared root target under the shared lock. Logs live in `.evidence/ticket-163/`.
- `rustfmt --check --edition 2024 core/src/artifact/mechanical_extract/board_source.rs core/tests/module_import.rs web/crates/runtime/src/source_pcb_export_tests.rs`
  and `git diff --check` passed. Three native test names added; none removed/renamed.
- Pending parent gates: integrate the commit, run the final full native suite and
  combined Standards+Spec geometry review, then determine ticket resolution.
  No independent reviewer was spawned and acceptance boxes remain open.
