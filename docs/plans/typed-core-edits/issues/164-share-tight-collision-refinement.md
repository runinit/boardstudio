# 164: Share tight collision refinement across repeated model placements

Status: claimed
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [CAD 010](../../cad-backend-replacement/issues/010-production-cache-worker-browser.md), [CAD 014](../../cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md)

## What to build

An explicit tight collision check should refine each distinct vendor STEP asset
once for its import settings and reuse that analysis mesh for all its placements.
Each placed component must keep its own identity, transform and source witness.

The 2026-10-18 audit at `f955050c3` traced the Case loop awaiting a fresh import for
every delivered model row. Sixty identical switch placements therefore issue sixty
serial imports of the same bytes and settings before pair checks begin. This was
established from the production call path; whole-keyboard latency was not measured.

## Acceptance criteria

- [ ] A regression with repeated placements fails because the current tight-check
  path imports the same source/settings more than once. Count preparation requests
  rather than asserting wall-clock timing.
- [ ] Within one current check, reuse refinement by verified source digest and the
  complete import settings, including repair budget and target deflection. Different
  digests or settings never share a refined asset accidentally.
- [ ] Preserve placement IDs, final transforms, face provenance and pair results;
  sharing geometry must not collapse distinct component occurrences.
- [ ] Preserve existing cancellation, stale-snapshot, worker-generation and evidence
  recovery rules. Reuse cannot activate retired evidence or bypass the existing
  bounded retry/refusal policy.
- [ ] A failed shared preparation settles affected placements with the existing
  explicit uncertainty/error behavior. It does not leave waiters pending or cause
  another import for every row in the same check.
- [ ] A focused Case/Runtime test exercises explicit tight checking with repeated
  rows. Verify the real provider/worker boundary and existing default/tight behavior
  using the established focused browser checks.
- [ ] Run affected lifecycle and collision tests. Apply the CAD closure review policy
  once for the completed slice and record preparation counts and validation limits.

## Comments

Created from audit finding 3 at the user's request. Reuse existing asset delivery
and evidence ownership mechanisms; this ticket does not introduce a new scheduler
or broaden collision qualification.

## CURRENT — 2026-10-18

Implemented private Runtime refinement reuse through the existing token-owned
`ModelDeliveryAdapter`: verified SHA plus complete repair/deflection settings,
shared success/failure, original placement IDs/transforms/source faces, and separate
bounded missing-evidence recovery. No API visibility, protocol or producer changes.

RED: fresh Runtime repeated-placement test observed **2 preparations, expected 1**
(`.evidence/ticket-164/red-repeated-valid-fixture.log`). GREEN: that test and all
five new Runtime regressions pass (settings/digest isolation, shared failure,
cancellation/new check, and one separate evidence recovery without a retry loop).
Fresh real-provider Case page: **21 default pairs, 21 tight pairs, 1 tight import**;
the 0.01 mm request remains uncertain for insufficient source precision. The earlier
real-page RED used an existing package; the GREEN page was rebuilt in this worktree.

Executed commands/results (root builds/tests use `flock /home/chris/.cache/boardstudio-target/audit162-165-root.lock`):

- `CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome web/crates/runtime --lib -- vendor_refinement`: 5 passed.
- `cargo test --manifest-path cad/wasm/Cargo.toml --release --locked --lib collision`: 10 passed.
- `python3 cad/scripts/build-cad-wasm.py --no-opt`: passed; `node cad/wasm/tests/collision-analysis.cjs "$PWD/cad/wasm/pkg/boardstudio_cad_wasm.js" --web` and `node cad/wasm/tests/qualified-analysis.cjs "$PWD/cad/wasm/pkg/boardstudio_cad_wasm.js" --web`: passed.
- `python3 scripts/build-web.py --fast --routes subpath`: passed. Served its packaged page on port 4195 and ran `scripts/qualification-page-fixture.cjs` through Playwright: passed with the counts above.
- `rustfmt --check --edition 2024` on the changed Rust files, `node --check scripts/qualification-page-fixture.cjs`, `python3 scripts/check-wasm-tests.py`, and `git diff --check`: passed.
- `cargo test -p boardstudio-web-runtime --release --locked --lib model_delivery::tests`: 25 passed; `cargo test -p boardstudio-web-runtime --release --locked --lib case_model_lifecycle::tests`: 5 passed.
- `python3 scripts/check.py repo`: passed.
- `CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome web/crates/case --lib -- case_collision::browser_tests`: 3 passed; `CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome web/crates/runtime --lib -- collision_browser_tests`: 12 passed (including the five new regressions).

Logs are in `.evidence/ticket-164/`. No test names removed or renamed; no latency
benchmark claimed. Parent still owns integration, `python3 scripts/check.py`, and
the single combined Standards/Spec closure review. Review pending; keep claimed.
The requested Rust skill was unavailable in installed locations; TDD and repository
guidance were read and applied.
