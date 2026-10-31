# 16: Run more crate launches at once; prebuild the page test

Status: resolved
Type: build
Blocked by: none
Parent: [map](../map.md)

## Outcome

**Corrected 2026-10-31.** Part 1 is kept with a default of `min(6, cpu_count)`, and part 2 stays dropped.
The rejection below was wrong: a stall of about 300 s hit every browser run from the Pi agent's shell,
including at 4 jobs and with no runtime change, and no run from the Claude shell. From the Claude shell,
jobs=6 had a 59 s batch, jobs=8 63 s and jobs=4 76 s, with no stall. See [17](17-browser-overlap.md).

Original outcome: Part 1 rejected: the default remains 4. At 8 jobs, launches took 301–334 s
(`.evidence/build-time/t16-j8-1.log`); at 6, case took 301 s and keycaps 285 s,
with a 284.8 s batch versus 76.9 s at 4 (`.evidence/build-time/t16-j6-1.log`).
The 6-job pass was terminated by the user, not counted as a test failure. Test bodies
were normal; the delay was outside them, in startup or teardown. CPU and memory did
not explain it. The cause is unknown. Two later 8-job passes had already been started
before the stop instruction was received. Three focused 2-job passes each for
`ui-shared` and `layout` passed, refuting sibling overlap as the explanation
(`.evidence/build-time/t16-sibling-*.log`). No live process/fd snapshot was captured.

Part 2 rejected after the prescribed single approach. Only `web/Cargo.toml` has a
`default` feature among the selected packages. A fresh isolated target prebuild with
`--lib --bin boardstudio-web --no-default-features` and the page features took 102 s;
the subsequent page `--list` launch took 16 s and still compiled `boardstudio-web`
in the dev profile (9.34 s). This fails the freshness requirement; no cold gain is
claimed. Logs: `.evidence/build-time/t16-page-fresh-{prebuild,launch}.log`.

## Evidence

Single cold run at `62a274623` (`.evidence/build-time/single-build.log`): the crate batch takes 76.9 s
for 14 launches. Launch overhead is about 194 s of wall against 77 s of test bodies, and launches no
longer compile, so the build lock no longer limits them. `layout --skip setup_guide::` (54 s) is the
floor. The `--list` launch takes 20.4 s, about 13 s of that compiling the page test binary.

History: `check-speed` 11 found jobs=6 no better than 4 and jobs=8 timing out in PCB and Keycaps, with
no error text kept. Those launches were still compiling, so the result needs retesting, not trusting.

## Work

1. `check_jobs()` default in `scripts/check.py`: try 6 and 8, capped by `os.cpu_count()` so a 4 vCPU
   runner keeps 4. Read every `check_jobs()` caller first: the value also gates other lanes.
2. Add the page test binary (`web --bin boardstudio-web`, page features) to `BROWSER_PREBUILD` if one
   invocation can produce the unit the launch finds fresh. `--no-default-features` and `--lib` apply to
   every selected package, so check that first. If it cannot be done in one invocation, drop this part.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -20 to -35 s (estimate) | none | same, every run |

## Verification

```sh
python3 -B scripts/test-check.py
python3 scripts/check.py browser   # three consecutive clean passes at the chosen default
```

Then one full cold run with `python3 scripts/build-time/cold.py <name>`. Counts must hold: 693 launch
tests with the same names, 112 page tests, 2378 native tests, 15 drag checks, 364 CAD browser checks.

## Stop rule

Keep the highest job count with three clean passes that gains at least 10 s of batch wall over 4. Any
timeout means step down, and if 6 also fails, keep 4. Drop part 2 if it gains under 8 s.
