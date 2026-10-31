# 16: Run more crate launches at once; prebuild the page test

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [map](../map.md)

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
