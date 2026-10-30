# 01: Land round one, then the audit

Status: in-progress
Type: task
Blocked by: none
Parent: [map](../map.md)

## Question

In what order do the uncommitted round one and the separate audit's manifest edits land, so the
measured gain stays attributable and the browser launches stay unified?

## Answer to plan against

Round one first, audit second, in two commits on separate branches, landed close together.

- Round one was measured at `510861b11`, and the audit branch `audit/rust-dev-20261030` starts from
  the same commit. Landing round one first keeps its 1273.8 s result true for what lands.
- The audit's own earlier estimate applies: `ci-wall-time` ticket 08 found about zero wall from
  these removals, because every candidate stays reachable through another edge. It is hygiene with
  a lockfile change, so it should not sit underneath a performance measurement.

## Evidence

- The two diffs overlap in only three files: `Cargo.lock`, `web/crates/catalogue/Cargo.toml` and
  `web/crates/runtime/Cargo.toml`. The manifest hunks do not touch the same lines. Round one adds
  wasm dev-dependencies and machete metadata; the audit removes `boardstudio-application` from
  catalogue and `serde-wasm-bindgen` from runtime.
- `Cargo.lock` conflicts: round one adds `boardstudio-web-ui-shared` to four dependency lists, and
  the audit deletes 23 lines around the same stanzas. Regenerate it instead of merging by hand.
- The audit also moves source (`web/crates/pcb/src/pcb_wiring.rs`, `pcb_wiring/mode_owner_tests.rs`,
  new `pcb_wiring/test_fixtures.rs`, `pcb/src/lib.rs`), and has untracked `docs/plans/rust-audit/`
  that must be committed with it. Its findings file says the removals are unverified until
  `cargo check` runs after removal.
- Launch unification depends on identical dependency features across launches. The removed direct
  `boardstudio-web-host` `page` edges in keycaps, keymap, layout and parts stay covered, because
  `web/crates/ui-shared/Cargo.toml:15` enables `page` and each of those crates depends on ui-shared.
  Inferred from the manifests; verify below.
- Rust-cache keys include the lockfile hash, and the Actions cache already holds 10.59 GB in eight
  entries against a 10 GB limit (`gh api .../actions/cache/usage`, 2026-10-30). Two separate landings
  mean two partial-restore transitions, each saving about 2.9 GB and evicting older entries.

## Gain

None new. Round one is already counted; the audit is expected to be about zero (ticket 08 above).

Effort: 1 hour plus one combined gate. Risk: low. A silent change to launch features would bring
back the duplicate Core builds, which the check below catches.

## Work

1. Commit round one on `perf/reduce-cold-build-times`. Note that this checkout is on that branch,
   not `t3/3c97e5c4` as the brief says.
2. Rebase the audit branch onto it. Take round one's `Cargo.lock`, re-apply the manifest removals,
   then regenerate with `cargo update --offline -w`, and the same for
   any other affected lockfile. Do not hand-merge lock stanzas.
3. Run the gate below on the rebased audit branch before landing either.
4. Land both inside one push window so `dev` saves one new cache, not two.

## Verification

```sh
git diff --stat HEAD~1 -- Cargo.lock        # only the expected dependency lines changed
python3 scripts/check.py repo tooling lint test build browser > .evidence/cold-build-time-01.log 2>&1
grep -E '^check: (native tests|step times|all steps)' .evidence/cold-build-time-01.log
# one wasm32 debug Core per browser run, as round one achieved:
# recorded by ticket 02's report, `boardstudio_core` wasm32/debug count must be 1
```

Every count in the map's count floor must hold.

## Stop rule

Revert the audit's offending edge, not round one, if the audit changes the count of `wasm32/debug`
Core builds from 1 or any gate fails. Revert a round-one change only on its own evidence.
