# Case manufacturing controls — 2026-10-05

Repaired the mounted Case panel against reference `5a472a9` from source base `eac47f42`. Added accepted per-part manufacturing overrides and stabilizer-fit controls through the existing mechanical settings controller. Standard thickness edits use the existing top-level configuration fields; process edits preserve unrelated targets; foam stays fixed to cut sheet/EVA. Stabilizer candidates follow the reference MX wide-key sizing, rotation, and same-pose imported-stabilizer exclusion. The controller retains its existing owner identity and stale-request guards.

The mounted UI regression was RED because the manufacturing section was absent. After implementation:

- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` — passed.
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/mechanical_settings.rs web/src/presentation/mechanical_settings_controller.rs web/src/presentation/mechanical_settings_mount.rs --result-json .scratch/dioxus-frontend-v1/evidence/case-manufacturing-repair-20261005/wasm-focused-results.json` — 16 passed, 0 failed.
- The focused run includes the mounted UI, process state transition, and nonempty stabilizer projection tests. All controller/projection assertions ran as `wasm_bindgen_test`; there is no native-test claim.

Exact RED failure, fixture barrier inventory, and implementation notes are in [implementation.md](case-manufacturing-repair-20261005/implementation.md). The positive paired Case fixture remains parent-owned at `case-manufacturing-repair-20261005/input.boardstudio` and `input.json`; packaging and browser replay are pending parent integration.

## Reviewer follow-up — settled source

The earlier 16-test count below describes the initial implementation only and is superseded by the reviewer follow-up. Plate process-thickness now shares MX family/profile/gap/default-foam synchronization with the dimension route, and Finished thickness uses the existing numeric draft behavior. Both follow-up regression tests failed before the fix; the settled focused run passed 15/15 WASM tests. See [follow-up details](case-manufacturing-repair-20261005/implementation.md) and the preserved [RED](case-manufacturing-repair-20261005/reviewer-red-results.json) and [GREEN](case-manufacturing-repair-20261005/reviewer-green-results.json) results.

Settled source SHA-256:

| File | SHA-256 |
| --- | --- |
| `web/src/presentation/mechanical_settings.rs` | `a4a15fcc995672d171f01436d9ed43a14cdb7ccc716db96705f9c07c84f44a6c` |
| `web/src/presentation/mechanical_settings_controller.rs` | `42f8779b08df623b03caefe8e1374f441ef083cbd045c97dc3fdc383912fd080` |
| `web/src/presentation/mechanical_settings_mount.rs` | `3c6b7de945a0ce1e050df6c56335b357819eac1ab54a92c59a5c307d204940cd` |
