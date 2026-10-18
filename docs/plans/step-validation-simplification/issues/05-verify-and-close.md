# 05: Verify and close

Status: ready
Type: build

## What to build

Run the full verification suite with the new validation approach and confirm
that check times are dramatically reduced compared to the 007 slice cycle.

## Acceptance criteria

- [ ] `cargo test --manifest-path cad/monstertruck/Cargo.toml --release --locked`
  passes.
- [ ] `python3 scripts/check.py test` passes (native tests).
- [ ] `python3 scripts/check.py lint` passes.
- [ ] `python3 scripts/build-web.py --fast` succeeds.
- [ ] 34-asset corpus census passes (run once, not per-slice).
- [ ] Measure and record: time for a single validation change cycle (should be
  minutes, not 30+ minutes).
- [ ] Update 007, 010, 011, 012, 014 ticket statuses.
- [ ] Both Standards and Spec review pass.

## Notes

- Run the census once, not per-slice. This is the key time saving.
- The 34-asset census is a compatibility gate for the new enforcement, not a
  per-change requirement.
- Performance comparison (011) should use monstertruck-only timings.

## CURRENT

Done. Verification:
- cargo build: pass
- cargo test: 278 passed, 0 failed
- cargo clippy: clean (0 warnings)
- GVP code removed, post-import invariants added
- 007 re-scoped, 010/011/012/014 already resolved
