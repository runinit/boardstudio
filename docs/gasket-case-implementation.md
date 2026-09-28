# Internal gasket Rust and CAD implementation

This checkout implements the Rust and CAD phase of
[gasket-case-redesign.md](gasket-case-redesign.md). The editor conversion and
hardware catalog selection remain separate work. Existing documents keep their
legacy construction unless they explicitly select `internalGasket.version:
internal-v1`.

## Implemented contract

Rust owns the independent PCB-based exterior, nominal plate and local tabs,
matched upper/lower foam stacks, support tracks, closure positions, screw
selection, blind seats, optional downward bosses, and material/hardware lists.
The top retains the persisted `retainer` body ID.

Custom hardware supplies explicit dimensions, screw-length datum, installation
method, and available or fixed lengths. Unresolved datums and incompatible
processes block generation. Adhesive thickness is not compressed. Unknown foam
compression data produces a warning rather than an export approval gate.

Generated supports can be repaired or removed by a count change. Saved
user-positioned supports and adopted closure locations remain protected.
Split-region closures and linked support placement are handled independently.

Prepared CAD carries typed support prisms, cylindrical seats, and conical seats.
Native construction checks complete support footprint ownership, fuses additions,
cuts seats and openings, and rejects disconnected feature results. Feature
geometry participates in cache identity and bypasses the planar shortcut.

## Validation scope

Public Rust tests cover geometry, persistence, travel, process compatibility,
screw selection, adhesive behavior, and all 13 specified foam sizes. Native CAD
fixtures cover rectangular, countersunk, downward-boss, rotated-concave, and split
cases. They check connected solids, STEP reimport volume, tower material, and
plate/PCB clearance at both travel limits. These fixtures use synthetic Custom
hardware dimensions, not verified product presets.

Validation completed so far:

- Core Rust suites: 271 tests passed, including 21 internal-gasket regressions.
- Renderer Rust suite: 25 tests passed.
- Native CAD: 20 tests passed; four diagnostic benchmarks remain intentionally
  ignored by the normal suite.
- CAD JavaScript/export suite: 44 tests passed, including all five new fixtures.
- Application unit suite: 399 tests passed across 67 files.
- Repository structure, generated contracts, and runtime import checks passed.

The production build, CAD type check, and native/WASM boundary parity checks
passed. The full `pnpm run check` returned failure at the browser suite:
**193 passed, 8 failed**. The failures were:

| Test | Observed failure |
| --- | --- |
| `cad-loading.spec.ts:4` | Clicks old “Generate” label |
| `electrical-handoff.spec.ts:8` | Expects old “Generate” label |
| `workbench.spec.ts:81` | Clicks old “Generate” label |
| `workbench.spec.ts:465` | Clicks old “Generate” label |
| `case-preparation.spec.ts:4` | Expects profiling measures without `?cadMetrics=1` |
| `parts-catalog.spec.ts:6` | Expects 37 entries; catalog contains 39 |
| `parts-catalog.spec.ts:45` | Expects 37 entries; catalog contains 39 |
| `workbench.spec.ts:403` | Expects hidden “Saved locally” text to be visible |

The corresponding UI label (“Update preview”), profiling guard, catalog, and
save-status UI are unchanged from baseline commit
`49f88a36776a95a5288edb2366803fc61771ab05`. This is a source comparison, not a
separate baseline browser run. No frontend test or production UI changes were
made in this phase. The separate `pnpm run test:e2e:pages` was run after the
main command stopped and also failed: `e2e-pages/deployment.spec.ts:3` waits
for the old “Design” tab. There are nine browser failures across both runs.

These checks do not establish installed-app behavior, physical fit, or
pointer-to-paint timing.

## Remaining work

- Complete verified catalog families and automatic family ranking/defaults.
- Integrate editor controls, new-project defaults, and explicit undoable legacy
  conversion.
- Run the specification's live interaction and performance acceptance sessions.
- Validate actual selected hardware and foam against their product data before
  claiming those catalog entries complete.

The original checkout's performance experiments remain untouched. Its historical
Phase 3 execution record is named in the design but is not copied into this
isolated implementation.

## Independent review

Standards review found no remaining hard violations or concrete correctness
issues after fixing support ownership and full CAD footprint validation.

Spec review found no additional reproducible defect in this Rust/CAD slice.
Catalog selection, editor conversion, and live acceptance remain incomplete as
listed above. An initially suspected saved-closure cap collision could not be
reproduced independently of existing rejection checks, so no speculative change
was added.

The branch is an implementation checkpoint, not a claim that the full redesign
or the full repository acceptance gate is complete.
