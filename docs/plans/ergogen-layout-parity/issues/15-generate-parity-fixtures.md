# 15: Generate pinned parity fixtures and replacement oracle inputs

Status: resolved
Type: build
Blocked by: [10](10-slice-the-build.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Implement the opt-in generator from decisions 03/04. Commit independently generated
expected poses and declarative new-model inputs before activating new application math.
Normal app/native test runs must not execute or download Node/Ergogen.

## First RED / GREEN slice

Make a generator self-check fail for one known translated/turned key and its explicit
Ergogen-name → `(matrix,row,column)` mapping. Add the minimal pinned invocation and
fixture writer to pass it; then add the next behavioral case, not all tests up front.

## Subsequent acceptance

- Implement `scripts/generate-ergogen-parity-fixtures.cjs`; pin Ergogen full commit
  `8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37`, exact Maker.js/Math.js and a lockfile,
  external cached checkout and install-script refusal. Retain required notices.
- Record upstream config/version, equivalent column/row/cell values, named expected
  position/heading and mirror mappings. Reject duplicate/missing mappings explicitly.
- Cover default/custom steps, Shift+Bend order, offsets after Bend, relative origins,
  mirrored full local headings, key anchors/Follow, skipped output and carried values.
  Reproduce the agreed comparison scenarios, with additive divergence intentional.
- Include BoardStudio-only scenarios for Column rotation, hold-still, local Y mirror
  and other extended contracts with independent worked expectations, not a new oracle
  copied from the implementation under test. No aggregate coverage claim.
- Use `1e-6` upstream and `1e-9` BoardStudio-only tolerances. Separate source provenance
  from execution evidence; a generated fixture is not Rust acceptance.

## Scope and checks

Tooling/fixtures/notices only; do not activate a schema or delete the currently used
oracle before its replacement comparison is wired. Source anchors: decision 03's
research report, decision 04's scenarios and `core/tests/fixtures/matrix-projections.json`.
Run generator self-checks, reproducibility/diff inspection and the tooling checks selected
by `python3 scripts/check.py quick --files <changed-paths>`. Ordinary test execution
must consume committed JSON without the upstream runtime.

## Answer

Implemented on 2026-10-23, without activating the new application schema/math or
changing the currently consumed TypeScript projection oracle.

- Generator: `scripts/generate-ergogen-parity-fixtures.cjs`, with external pinned-source
  checkout creation/validation and atomic JSON output. Existing wrong-HEAD, modified
  or untracked-content caches are refused, not reset. Output cannot overwrite the
  active projection oracle or modify the source checkout. `--check` is read-only.
- Tool environment: `scripts/ergogen-parity/package.json`, npm lockfile and
  `ignore-scripts=true`, with Maker.js **0.18.2** and Math.js **15.2.0** only as direct
  dependencies. The isolated parser loader verifies actual module resolution/versions
  before executing the pinned `points.parse` entry. No full Ergogen CLI/hull/kle-serial.
- Source scenarios: `scripts/ergogen-parity/cases.cjs`. Committed output:
  `core/tests/fixtures/ergogen-layout-parity.json`, fixture version 1, targeting the
  planned document model 7 rather than pretending to be an Open-able document.
- **40 upstream cases, 340 emitted points and 3 explicit absences** cover the agreed
  scenarios, default/custom/zero/negative steps, carried row/key composition,
  independent adjustment order, all eight Follow subsets, final-key/skip anchors,
  asym filtering and complete mirrored headings. Every name/coordinate maps exactly
  once; missing/extra/duplicate/nonfinite output is refused.
- S15 explicitly stores the intentional additive-offset difference, not a false parity
  assertion. Four separate, literal BoardStudio-only worked cases cover Column rotation,
  local Y mirror and this/next-step plus column-0 reflected-Origin hold compensation.
  They contain their derivation basis, not copied implementation output.
- `scripts/test-ergogen-parity.cjs` is registered in `scripts/check.py` tooling. Normal
  invocation uses standard Node/Git and committed data, with no npm installation,
  upstream checkout or network. An explicit cache argument additionally tests untracked
  provenance rejection by creating/removing only its own probe.
- Usage/schema notes and full MIT/Apache licence text plus Math.js NOTICE are adjacent
  in `scripts/ergogen-parity/`; no dependency artifacts are committed or bundled into
  the app. Upstream/BoardStudio-only tolerances remain `1e-6`/`1e-9` respectively.

### Evidence and limits

The known translated/turned point, duplicate/missing mappings, absence/finite checks,
catalog/validation/CLI guards and the two reviewed safety fixes have RED/GREEN logs
under `.evidence/ergogen-layout-parity-15/` (gitignored). Actual pinned-parser generation
produced the known `(10,20,30°)` point. A fresh external source fetch produced the same
fixture as the existing cache; reproduction passed, deliberate fixture mismatch did
not write, and tracked/untracked dirty-cache refusal was exercised. Atomic output's
regression preserves an unrelated pre-existing temporary-name collision.

Tooling and documentation checks pass. The combined spec/standards review used Luna
Fast/high: its untracked-cache finding was fixed and verified, and the subsequent
atomic-writer ownership delta reviewed clean. Review itself is not execution evidence.

No Cargo build, Rust comparison or browser/application acceptance was run or claimed.
The fixture remains unconsumed by the current projection test; ticket 16 wires the new
kernel comparison before later activation. The app's current format remains 6 and the
legacy oracle is unchanged. Interrupted initial fetch may leave a cache to inspect/
remove manually; existing cache directories are never silently reset.
