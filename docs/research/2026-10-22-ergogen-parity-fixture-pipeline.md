# Ergogen parity fixture pipeline

Date: 2026-10-22. Recommendation for ticket 03, based on the standing decisions in
[ADR-0034](../adr/0034-matrices-follow-ergogen-point-layout.md) and the comparison and
dependents reports ([comparison](2026-10-22-ergogen-layout-engine-comparison.md),
[inventory](2026-10-22-matrix-layout-dependents-inventory.md)).

## Recommendation

Use an opt-in Node generator named `scripts/generate-ergogen-parity-fixtures.cjs` and commit
its JSON output under `core/tests/fixtures/ergogen-layout-parity.json`. Check in an exact-version
`scripts/ergogen-parity/package.json` and npm lockfile; `npm ci --prefix scripts/ergogen-parity`
installs the locked modules only when preparing the optional generator, with `node_modules`
ignored as usual. Keep only the Ergogen source checkout in the user cache outside the repo. The
generator should load Ergogen's `src/points.js` directly and call
`points.parse(config, units.parse({}))`;
`points.js` exports that parser and imports Maker.js, while `units.js` has no third-party
runtime import ([points.js at the pinned commit](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/points.js),
[units.js](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/units.js)).
Fixture generation is a maintainer command, not a prerequisite for ordinary tests, consistent with
the [README's Node boundary](../../README.md) and [check scheduler](../../scripts/check.py).

Pin the source to full commit
`8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37` (Ergogen 4.2.1), not a branch or tag alone. Keep
a small `scripts/ergogen-parity/package.json` and lockfile with exact `makerjs` `0.18.2` and
`mathjs` `15.2.0` versions, and install with `npm ci`; those are the versions present in the
provided cache, and both packages declare Apache-2.0 ([Ergogen package metadata](https://registry.npmjs.org/ergogen/4.2.1),
[Maker.js metadata](https://registry.npmjs.org/makerjs/0.18.2),
[Math.js metadata](https://registry.npmjs.org/mathjs/15.2.0)). The Ergogen package declares
Maker.js and Math.js as runtime dependencies; the direct points parser's imports show Maker.js
is needed for this path ([pinned package.json](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/package.json),
[pinned points.js](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/points.js)).
Keep Math.js in the deliberately small environment because the selected upstream package
declares it directly; do not install Ergogen's unrelated `hull` or `kle-serial` for the points
oracle, whose parser import graph does not use either ([package.json](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/package.json),
[points.js](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/points.js),
[anchor.js](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/anchor.js),
[point.js](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/point.js)).

Suggested preparation and invocation from the repository root:

```sh
npm ci --prefix scripts/ergogen-parity
node scripts/generate-ergogen-parity-fixtures.cjs \
  --cache "$HOME/.cache/boardstudio/ergogen" \
  --output core/tests/fixtures/ergogen-layout-parity.json
```

The generator should create the cache directory and detached checkout at the pinned commit
when absent, validate an existing checkout's `git rev-parse HEAD`, and set `NODE_PATH` to the
locked modules. It should write the full source commit and version into the fixture header and
derive expected poses only from Ergogen. This makes the provenance inspectable and prevents
silently refreshing goldens from a moving source;
the repo's current test runner registers tooling scripts explicitly in
[`TOOLING_TESTS`](../../scripts/check.py), and existing Node tooling is invoked with Node only
for those named checks ([README](../../README.md)).

## Fixture contract

Store each case's original Ergogen config beside the BoardStudio matrix input and its oracle
poses. The current Rust fixture already groups `matrix` and `expected` cells by case, and its
expected entries carry row, column, enabled state, member ID and pose ([existing fixture](../../core/tests/fixtures/matrix-projections.json),
[Rust projection assertion](../../core/src/matrix.rs)). Add explicit provenance and upstream
names so a maintainer can trace a mirrored point without reverse-engineering a generated key
ID. One case could be represented as:

```json
{
  "source": {"repository": "https://github.com/ergogen/ergogen", "version": "4.2.1", "commit": "8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37"},
  "cases": [{
    "name": "single-key-mirrored-half",
    "ergogenConfig": {"points": {"zones": {"m": {"columns": {"a": {}}, "rows": {"r0": {}}, "mirror": 120}}}},
    "boardStudioMatrices": [{"id": "left", "rows": 1, "columns": 1, "pitch": {"x": 19.05, "y": 19.05}}, {"id": "right", "rows": 1, "columns": 1, "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 240, "y": 0}, "mirror": "x"}],
    "expected": [
      {"ergogenName": "m_a_r0", "matrix": "left", "row": 0, "column": 0, "pose": {"x": 0, "y": 0, "rotation": 0}},
      {"ergogenName": "mirror_m_a_r0", "matrix": "right", "row": 0, "column": 0, "pose": {"x": 240, "y": 0, "rotation": 0}}
    ]
  }]
}
```

The values above illustrate a zero-offset mirror case. Generate and validate expected
coordinates from the pinned parser before treating a case as a golden. Keep the BoardStudio input explicit
because the parity contract compares two separately configured models; the fixture should
include all parameters needed to reconstruct each model, rather than only its output. Naming
conventions such as `m_<column>_<row>` and `mirror_` already appear in the comparison harness
and measured mirror scenario (local gitignored `compare.js` and `results.txt`; see the committed
[comparison report](2026-10-22-ergogen-layout-engine-comparison.md)).

In Rust, run the ordinary Core projection for each recorded `boardStudioMatrices` entry and
look up its projected cell by `(matrix id, row, column)`. Compare expected position x/y and
rotation as separate absolute differences `< 1e-9` (mm and degrees); do not match by array
order or pretend Ergogen's point name is a BoardStudio member ID. The existing projection test
uses `< 1e-9` for all three pose values, while the supplied comparator validates 54 legacy
cells with maximum error `7.11e-15` and has an explicit mirrored-key mapping
([assertion](../../core/src/matrix.rs), [comparison report](2026-10-22-ergogen-layout-engine-comparison.md)).
Keep `matrix-projections.json` as a BoardStudio-only regression: it also covers captured legacy
poses/saved-pose cases and the dependency inventory identifies it as an existing formula
oracle; new Ergogen fixtures should add independent parity evidence rather than erase that
coverage ([fixture](../../core/tests/fixtures/matrix-projections.json),
[dependents inventory](2026-10-22-matrix-layout-dependents-inventory.md)).

## Licences and alternatives

Ergogen is MIT-licensed; its license requires retaining its copyright and permission notice
when redistributing copies or substantial portions of the software ([pinned LICENSE](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/LICENSE)).
Maker.js and Math.js declare Apache-2.0, whose redistribution terms include a copy of the
license and preservation of applicable notices ([Maker.js metadata](https://registry.npmjs.org/makerjs/0.18.2),
[Math.js metadata](https://registry.npmjs.org/mathjs/15.2.0),
[Apache-2.0 text](https://www.apache.org/licenses/LICENSE-2.0)). The generated fixture contains
configuration and numeric pose data rather than copied implementation source; retain source,
commit, version, and license identifiers in its header for provenance, and put the full MIT
and Apache notices in a dedicated `scripts/ergogen-parity/THIRD_PARTY_NOTICES.md` if the
generator sources or dependency artifacts are committed or distributed. This avoids copying
runtime packages into the repository while keeping the applicable notices adjacent to any
redistributed tooling ([MIT license terms](https://opensource.org/license/mit),
[Apache-2.0 §4](https://www.apache.org/licenses/LICENSE-2.0#redistribution)).

Reject running the full Ergogen CLI for fixture generation: the points parser is a smaller
direct seam and the full package brings unrelated dependencies ([pinned package.json](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/package.json),
[points.js](https://github.com/ergogen/ergogen/blob/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/src/points.js)).
Reject keeping the oracle only in an outside cache because that would not make the exact
expected values reviewable or available to normal Rust tests; ADR-0034 requires committed
golden evidence. Reject deleting the old projection fixture because it records BoardStudio
behavior beyond the new upstream parity cases ([ADR-0034](../adr/0034-matrices-follow-ergogen-point-layout.md),
[existing fixture](../../core/tests/fixtures/matrix-projections.json)).

## Evidence run

Ran the supplied read-only comparator from `/tmp` against upstream commit
`8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37`, using the existing fixture and cached Node modules:
the baseline port check matched 54 cells with maximum absolute error `7.11e-15`; the 9/9
equivalence cases matched, while the expected offset, padding, cumulative shift/rotation and
world-origin differences remained visible. This was a Node harness run only; no Cargo build,
Rust test, or source edit was performed. The harness and output are local gitignored artifacts
(`.evidence/ergogen-layout-comparison/compare.js` and `results.txt`); the committed
[comparison report](2026-10-22-ergogen-layout-engine-comparison.md) describes the method.
