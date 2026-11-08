# 020: Export split firmware with either half as central

Status: resolved
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [016: Audit ADR decisions](016-audit-adr-decisions.md), [ADR-0026](../../../adr/0026-decouple-firmware-role-from-physical-side.md), [ADR-0024](../../../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md), [Plan map](../map.md)

## What to build

Make split firmware export honor ADR-0026: either physical half may be the central
when its hardware and the selected transport support it, without renaming or
mirroring a PCB, swapping halves or changing logical keymap positions. The user
asked on 2026-11-06 for this to be updated after 016's initial audit found the gap.
At ticket creation ADR-0026 described this as intended behavior, not delivered;
its dated amendments and the completion record below supersede that limitation.

Historical pre-implementation source facts (superseded by the implementation below):

- `web/crates/runtime/src/runtime.rs` (`firmware_bytes`, around lines 6753-6764)
  rejects split export unless the `central` instance is half `left` and the
  `peripheral` instance is half `right`.
- Core's generator is positional, not physical: `core/src/firmware.rs` names the
  central shield `boardstudio_left` and the peripheral `boardstudio_right`
  (around lines 497-580, 804, 844), adds the peripheral's keys after the
  central's rows with ids prefixed `right/` (around lines 479-495), and
  `firmware_request_adapter` passes primary then secondary. Removing the
  runtime guard alone would therefore put a physical right-half central's keys
  first and so change logical keymap positions, which ADR-0026 forbids.
- Which transports and hardware support an either-half central is not recorded
  here; ADR-0026 says "when its hardware and selected transport support that role".

## Acceptance criteria

- [x] RED first at the real boundary: a split design whose central is the right
  half (and one whose central is the left half, as today) exported through the
  runtime path. The right-central case failed before implementation with "Split firmware requires
  a left central and a right peripheral assembly".
- [x] Either role assignment exports. Logical key positions, matrix pins and the
  keymap layout are those of the physical halves, not of the roles: swapping the
  central role must not renumber or reorder keys.
- [x] Generated names, ids and files do not hard-code that left is central. Role
  and physical half are both recorded where a reader needs them, and the build
  matrix and Kconfig role flags follow the role.
- [x] Unsupported combinations (a transport or hardware that cannot make the
  chosen half central) fail with a message that names the actual limitation, not
  a side convention. Record which combinations exist; do not invent support.
- [x] Existing left-central output is unchanged for existing designs, or any
  difference is explained and approved by the user.
- [x] ADR-0026's "intended behavior, not delivered" note is updated with a dated
  amendment when this lands; 016's 0026 finding is cross-referenced.

## Verification and completion

The new runtime-level tests plus the focused Core firmware and
`web/tests/firmware_request_adapter.rs` tests; record exact commands and results.
If implementation shows independent seams (runtime guard, adapter ordering, Core
generator naming, transport support) that need separate decisions, re-scope before
coding instead of widening this ticket.

## Comments

- 2026-11-06: The user confirmed that shield names identify physical sides:
  retain `boardstudio_left` and `boardstudio_right` and assign their role flags
  according to the selected central/peripheral roles. A split design assigning
  both halves as central must be rejected with a clear error. Preserve existing
  left-central output as required above. Hardware and transport support still
  need evidence; no new capability model is implied by this decision.
- 2026-11-06: Created from 016's initial audit finding at the user's request
  ("split firmware export needs to be updated"). Routed with `ask-matt`: the
  policy is already decided by an accepted ADR, so no grilling step; it looks
  like one session of work if transport support is already known, otherwise run
  `/to-spec` then `/to-tickets` first.
- 2026-11-06: Implementation evidence (agent). Historically left open; the
  limitations recorded below are superseded by the 2026-11-08 completion record.
  - Design: Core `FirmwareRequest` gains `central_half` (`SplitHalf`); `generate` reorders to physical
    left-first before anything positional, so keys, sensors, row offsets, shield files and `.conf`
    follow the physical half and only `Kconfig.defconfig` role flags follow the role. The adapter
    derives `central_half` from the central plan's instance `half`. The runtime guard
    (`validate_split_assignment`) accepts one central plus one peripheral on opposite sides and
    rejects the rest, e.g. "...both halves are assigned central". Shield names stay physical.
  - RED (runtime, real Core, wasm): `split_export_*` failed: right-central exported 0 files with
    "Split firmware requires a left central and a right peripheral assembly"; log
    `.evidence/ui-integration-recovery/020-red-runtime.log` (mixed tree with 018 at the time).
    Core and adapter RED seen as failing assertions before their code. GREEN on the isolated tree
    (HEAD da752ad5c plus only 020 edits): `firmware_export_tests` 7 passed (wasm, via
    `scripts/check.py`-equivalent runner); Core `firmware::` 13 passed; adapter 9 passed
    (`cargo test -p boardstudio-web --test firmware_request_adapter`, newly registered in
    `web/Cargo.toml` because `autotests = false` had left that file unrun).
  - Left-central unchanged: byte-identical `dump_firmware` output (wired and wireless variants)
    before and after the Core change (`diff -r`), plus the existing split tests. The right-central
    README gains one "Central half: right" line; left-central README text is unchanged.
  - Full check: `check.py` repo, tooling, build, lint and native tests passed (2384 baselined tests
    executed; baseline not updated, 40 unrecorded names include 018's and this ticket's).
    `check.py browser --keep-going`: every crate wasm launch and 132 page tests passed.
    Final `check.py repo lint test` after review fixes: passed.
  - Combinations: no transport/hardware side restriction is recorded or invented. ZMK docs call
    left-central a convention and set the central role per shield by Kconfig; wired split is
    described as experimental there (docs from `main`, not checked against the pinned v0.3.0).
    Wireless is exercised through the real Runtime; wired UART only by an adapter test and Core
    tests, and the role-based tx/rx reversal is carried over unchanged. No real hardware verified.
  - Review: fresh Standards and Spec delegates. No hard standards breaches; fixes applied for the
    stale `web/Cargo.toml` comment, a mis-named test helper, duplicated offset matching and ADR
    over-claims (swapping roles also changes per-half transport-direction settings). Open
    judgement calls left as is: `"left"`/`"right"` instance strings still parsed in two places.
  - Historical limitations (superseded 2026-11-08): `check.py browser` failed at
    `scripts/run-drag-browser.py` (Part drag, "WebSocket protocol reset"; logs under
    `web/target/test-results/drag-browser/`), reproduced twice. At that time a clean-HEAD run
    was unavailable, so it was not shown to be pre-existing. Wired right-central and
    encoder-bearing right-central through the real Runtime were not yet tested.

## Completion record (coordinator, 2026-11-08)

- Candidate `HEAD`: `82b97c5d`; implementation `ca065e679` (2026-11-06, ancestor).
  The candidate adds the Runtime fixture transport/encoder parameters and proof-gap tests.
- Executed now: wasm `split_export` filter, 4/4 passed in
  `.evidence/ui-integration-recovery/020-proof-gap-repopulate.log`, including
  `split_export_wired_right_central_preserves_physical_layout_and_uart` and
  `split_export_encoders_stay_physical_left_then_right_with_right_central`.
  These are proof-gap GREEN on the existing implementation, not a new RED claim.
  `cargo test -p boardstudio-web --test firmware_request_adapter`: 9 passed
  (`020-proof-gap-adapter.log`); `cargo test -p boardstudio_core firmware::`: 13 passed,
  including the `right_central_*` and `wired_*` cases (`020-proof-gap-core.log`).
  Both logs are under `.evidence/ui-integration-recovery/`; the Core log retains an initial
  wrong-package-name attempt (`boardstudio-core`); the corrected final run is the pass cited.
- Reused: original real-Runtime RED/GREEN in `020-red-runtime.log` /
  `020-green-runtime.log` under the same evidence directory satisfy AC1. The full gate in
  `.evidence/update-draft-safety/015-coordinator-full-gate-fdlock.log` is GREEN
  (browser drag PASS at line 4939; split-export wasm at lines 5899–5900; all steps at
  line 6873). Its applicability rests on the empty firmware diff `ca065e679..6bf22d5d`,
  verified during exploration; this is reused evidence, not a new full-gate execution at
  `82b97c5d`. The old browser limitation is resolved by the [008](008-restore-drag-save-undo-proof.md)
  / [009](009-verify-reload-persistence.md) lifecycle work, not waived.
- Combined review (Sol), implementation endpoints `ca065e679^..ca065e679` plus the test
  delta at `82b97c5d`: Spec **PASS**, Standards **PASS**, acceptance **READY**;
  coordinator checkpoint `review-020`. Accepted dispositions are recorded in Answer below.
- No unmet ticket gates or retained cleanup. Evidence retained is logs only. Next action:
  coordinator integration; this closure does not close 010/011. Experimental wired split
  and the absence of real-hardware verification remain explicit limits, not new support claims.

## Answer

Resolved: either physical half can be central without changing physical key/sensor order.
The real Runtime now proves wired UART and encoder-bearing right-central exports as well
as wireless role swapping and invalid-role rejection; the historical browser limitation
and Runtime proof gaps are superseded by the receipts above. ADR-0026's dated delivery
amendment cross-references 016's finding, with a new evidence note for these Runtime paths.

Accepted review dispositions:

- AC3: `build.yaml` still builds both physical shields for either role assignment; this
  invariance is intentional because Kconfig, not the build matrix's shield list, carries role.
- AC4: recorded absence of transport/hardware side restrictions satisfies the criterion;
  none are invented. Existing electrical-plan readiness checks remain. Wired split is
  experimental and no real hardware has been verified.
- AC5: left-central byte-identical output is a historical `diff -r` claim without a retained
  artifact, corroborated by identity normalization and unchanged tests; it is not a newly
  reproduced byte comparison.
- Parsing side strings at two sites is an accepted validation-versus-translation judgement
  call. No dedicated central-half selector UI exists: document roles drive firmware, and
  the ticket's ACs do not require a selector; this is an observation, not an AC gap.
