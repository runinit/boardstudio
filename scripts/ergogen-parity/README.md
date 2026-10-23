# Opt-in Ergogen point-layout oracle

This tooling generates `core/tests/fixtures/ergogen-layout-parity.json` for the
[layout-parity spec](../../docs/plans/ergogen-layout-parity/spec.md). It does not
activate version 7, compare Rust yet, or replace the still-active legacy oracle.

## Generate or verify

Use Node 18+ and Git. From the repository root:

```sh
npm ci --ignore-scripts --prefix scripts/ergogen-parity
node scripts/generate-ergogen-parity-fixtures.cjs
node scripts/generate-ergogen-parity-fixtures.cjs --check
```

The default source checkout is outside the repository at
`~/.cache/boardstudio/ergogen/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37`.
The first generation fetches exactly that commit; an existing checkout must have
that HEAD, no tracked modifications or untracked files, and package version 4.2.1. It is never reset or
silently updated. `--cache /absolute/external/checkout` selects an existing cache or
creates one at that location. `--output /path/to/fixture.json` selects another output;
`--check` regenerates in memory and fails on differences without writing.
An interrupted initial fetch can leave an incomplete cache. Inspect/remove only that
new cache explicitly before retrying; the generator never resets an existing directory.

Only Maker.js 0.18.2 and Math.js 15.2.0 plus their locked transitive dependencies are
installed. The local `.npmrc` also disables lifecycle scripts. Do not install the
full Ergogen CLI, hull or kle-serial. The small oracle subprocess resolves both
libraries from this locked environment before loading the pinned points parser;
a competing checkout-local installation is refused. The source and dependencies
are maintainer tooling, not app/runtime dependencies or committed vendor trees.

The offline contract check is:

```sh
node scripts/test-ergogen-parity.cjs
```

It uses Node's standard library, Git temporary-test repositories and committed data:
no npm installation, network, upstream checkout or Cargo. It is registered with the repository `tooling` checks.
Generation/`--check` are separate opt-in maintainer checks. Inspect input and expected
pose diffs before committing; never bless a different pin or moving tag accidentally.
An optional existing-cache check is `node scripts/test-ergogen-parity.cjs /path/to/checkout`;
it creates/removes only its own untracked probe to verify the cleanliness guard.

## Fixture contract

`cases.cjs` independently authors the Ergogen config and equivalent BoardStudio
parameters. It contains no BoardStudio layout evaluator. `oracle.cjs` calls upstream
`points.parse(config.points, units.parse(config.units || {}))` on cloned configuration;
the generator copies its finite x/y/r values without angle wrapping or rounding.

- `fixtureVersion: 1` versions this test-data format, not a project document.
  `targetDocumentFormatVersion: 7` identifies the planned model. The matrices are
  declarative layout inputs, not complete Open-able documents with fabricated hardware.
- `source` records the full commit/version, parser entry, MIT licence and exact direct
  dependency versions. The lockfile pins transitives. No timestamps or local paths
  enter the output.
- Each case stores `ergogenConfig`, `boardStudioMatrices`, explicit `mapping` and
  upstream `expected`. Every point name and logical matrix coordinate maps once;
  unexpected/missing points, duplicates, invalid coordinates and nonfinite poses fail.
- Expected entries have `ergogenName`, `matrix`, zero-based `row`/`column` and
  `pose: {x,y,rotation}` in millimetres/degrees. `absent: true` entries deliberately
  have no pose and require absent upstream output. Skipped values still affect later
  points and the separate anchor probe.
- Anchor `follow` arrays name the semantic subset `x`, `y`, `rotation`. `links` records
  source/mirror matrix identities and vertical `axisX` for the future fixture adapter;
  it is not a serialized physical pair or an extra project wire shape. The Rust fixture
  consumer constructs the agreed anchor context/ownership rather than ignoring it.
- `comparison: "match"` compares all present poses within `1e-6` mm/degrees.
  `intentional-divergence` stores actual upstream poses plus explicit independent
  `differences` for BoardStudio; other coordinates still match. S15 deliberately
  distinguishes additive offsets `(23,22)` from Ergogen override `(19,22)`.
- S11 and S12 supply fixed equivalent offset/Origin values; they do not introduce
  orient or world-origin rotation as BoardStudio parameters. All eight Follow subsets,
  default 19/19.05 pitch, custom/zero/negative steps, row/key composition, independent
  offsets, full mirrored headings, final-key anchors, skip and asym filtering are covered.
- `boardStudioOnly` contains literal quarter/half-turn worked examples with a `basis`,
  never claimed as upstream output. It covers non-inherited Column rotation, local Y
  mirror, and hold-still step/column-0 reflected-Origin compensation. Transition cases
  store `beforeMatrices`, `edit`, `afterMatrices`, `expectedBefore` and `expectedAfter`.
  Compare these at `1e-9`. They are geometry/edit-contract inputs, not proof that the
  production prepared edit path already handles them.

The fixture is deliberately unreferenced by the current Core projection test. Ticket
16 adds the new kernel comparator; ticket 17 coordinates activation and replacement
of the TypeScript-captured oracle. Native/browser acceptance is still required later.
Aggregate anchors are explicitly outside this build's claim.

## Attribution

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [Ergogen MIT](LICENSE-ERGOGEN)
and [Apache-2.0](LICENSE-APACHE-2.0). Do not remove dependency notices when separately
redistributing installed packages. No software dependency artifacts are vendored here.
