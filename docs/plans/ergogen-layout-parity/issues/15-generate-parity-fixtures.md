# 15: Generate pinned parity fixtures and replacement oracle inputs

Status: ready-for-agent
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
