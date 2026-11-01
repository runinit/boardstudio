# 09: One feature set for the CAD owner tests

Status: ready-for-agent
Type: build
Blocked by: [04](04-cad-test-lanes.md)
Parent: [map](../map.md)
Related: [15](15-cad-test-dev-deps.md)

## Question

The qualification tree builds much of itself twice. Can its seven selections share one build without
changing which tests run?

## Evidence

- `OWNER_TESTS` (`cad/scripts/cad_build.py:37-45`) runs `-p monstertruck-io` four times and
  `-p monstertruck-meshing` three times. Each package unifies features separately, which gives two
  builds of most things.
- In `cad/monstertruck-qualification/target`: 45 crates have more than one variant, and those crates
  take 186 of the tree's 308 CPU-s. `monstertruck_meshing` has 3 variants (27.1 CPU-s),
  `monstertruck_io` 2 (25.6), `regex_automata` 2 (13.4), `monstertruck_geometry` 2 (12.1).
- `research/native-warm` proposed this as F2 with a 62 s CI estimate; unmeasured.
- Workspace-wide feature unification would remove this but is nightly-only, and that is rejected.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -5 to -30 s (more if this lane stays last after 04) | -80 to -100 | CI only: fewer vendored rebuilds |

Effort: 3 hours. Risk: medium, all of it in test identity. Using the same `-p` list everywhere widens a
`--lib` filter across both packages. For instance, meshing's `--lib repair_` would also match
`monstertruck-io`'s `repair_movement_sink` tests. A union feature set also compiles tests with features
they did not have before, such as `vtk`.

## Work

1. List the tests each selection runs today, from the reference log, as the expected names.
2. Use `-p monstertruck-io -p monstertruck-meshing` on all seven commands, keeping filters.
3. If a filter widens, tighten it or use a separate `--no-run` then run with an exact filter, whichever
   keeps the guard's "recognized binary" capture valid (`check_native_tests.py`). Do not relax that guard.
4. Do not weaken the per-selection check at `cad_build.py:172`. Its regex accepts any non-zero result
   line in the output, so with two packages per command it can pass when the intended package ran
   nothing. Make it name-based for the expected tests of that selection.

Test-only slice under the [CAD closure rule](../../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user).

## Verification

```sh
python3 -B cad/scripts/test-cad-build.py
python3 cad/scripts/test-cad.py > .evidence/owner-after.log 2>&1
grep -E '^test .* ok$' .evidence/owner-after.log | sort > .evidence/owner-after.names
diff .evidence/owner-before.names .evidence/owner-after.names   # empty, or only intended additions
python3 scripts/check.py test > .evidence/owner-gate.log 2>&1
python3 scripts/build-time/cold.py owner test && python3 scripts/build-time/report.py rebaseline owner --lanes
```

RED: remove one test from a selection with `scripts/mutation-check.py`; the selection check must fail on
that selection's own name.

## Stop rule

Revert if CPU savings fall under 50 CPU-s, or if any test name leaves the executed list, or if a test
behaves differently under the union feature set. Duplicate executions of a test also mean revert unless
the guard accepts them by design.
