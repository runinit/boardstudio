# Remaining-plan OCCT screening

Historical raw artifacts and experimental sources now live in the performance
worktree. See Development worktrees (historical record in Git at `323967ff`) for their location.

This continues E2/E3/E5/E8/E9 from the original optimization plan. The earlier
engine comparison did not complete those experiments. All changes here are in
staged diagnostic crates; production CAD, the case design, tolerances, and result
ownership copies remain unchanged.

## Five-session results

Fresh-control timings for the original bottom regression, including construction
and mesh generation:

| Variant | Median / p95 | Median change |
| --- | ---: | ---: |
| Control | 109.7 / 119.5 ms | — |
| No wrapper history relay | 109.3 / 117.1 ms | −0.4% |
| No kernel history / relay | 108.9 / 116.0 ms | −0.7% |
| OBB filtering | 113.2 / 120.9 ms | +3.2% |
| Dedicated multi-tool cut | 105.4 / 113.4 ms | −3.9% |
| Non-destructive, copies retained | 142.8 / 150.8 ms | +30.2% |
| Combined mounting/opening cuts | 104.8 / 110.5 ms | −4.5% |
| Analytic mounting holes in profile | 104.5 / 113.8 ms | −4.7% |
| Final cleanup | 114.1 / 120.4 ms | +4.0% |

The rotated stress case changes the relative cost of OBB: median 141.8 ms versus
143.9 ms control, a 1.5% reduction. Dedicated cuts, combined cutters, and profile
holes improve that fixture by 4.7%, 5.6%, and 4.9% respectively. Non-destructive
mode and final cleanup remain slower by 32.7% and 5.9%.

The three construction candidates show consistent modest gains in paired
session medians; they do not reach the plan's 10% targeted-latency promotion
threshold individually. No production adoption follows. Their effects cannot be
added without testing combinations, which remains pending. History suppression
has little impact here; do not enable OBB globally or retain non-destructive/final
cleanup as speed changes on this evidence.

Session five slowed across variants: control session medians were 108.5, 109.4,
109.2, 108.5, and 115.3 ms. All samples are retained. Pairing reduces the risk of
mistaking that drift for an optimization; it does not establish its cause or
replace a full-app acceptance run. Do not compare these absolute numbers with
the prior batch's separately built diagnostic control.

The run contains **2,250 warm observations**, plus 135 cold/warmup and 225
instrumented observations. All **45 browser meshes** passed finite buffers,
nondegenerate triangles, normals, sampled occupancy, and welded edge closure.
There were zero occupancy mismatches, with about 31,100 samples per captured
bottom and 47,100 per rotated mesh. These checks supplement the exact native and
STEP checks below; they do not prove arbitrary input eligibility.

Raw results, provenance, and per-session analysis (`cad/bench/results/phase3-occt-round2/README.md` in the performance worktree)
are retained separately from the first comparison.

## Independent variants

- `no-relay`: suppress only Cadrum's unused bridge history maps.
- `no-history`: suppress bridge maps and OCCT `SetToFillHistory(false)`; this is
  separate from merely timing the bridge's history relay.
- `obb`: enable OCCT oriented bounding-box filtering with unchanged operations.
- `multi-cut`: use a dedicated batched `BRepAlgoAPI_Cut` for one DNF clause with
  exactly one positive operand; mixed expressions retain CellsBuilder. Preserve
  batching, history relay, error checks, and post-operation deep copying.
- `non-destructive`: enable OCCT non-destructive processing while retaining every
  ownership copy. This does not test or authorize copy removal.
- `combined-cuts`: subtract mounting and existing reduced opening cutters in one
  operation per region. Preserve the existing rectangular remainder reduction.
- `profile-holes`: put analytic circular mounting holes in the extrusion profile,
  then apply the existing opening cuts. This differs from the slower height-band
  construction in the first comparison.
- `final-clean`: run one final `Solid::clean()` before meshing, including its cost
  in construction timing.

The OCCT 8.0.1 rev2 installed headers were checked for the APIs above. Context7
provided upstream API descriptions but did not establish version-specific 8.0.1
coverage. Current-source compilation and geometry checks establish applicability
for the tested build; API availability alone is not evidence of a speedup.

## Protocol and correctness

Four unchanged captured bottom fixtures plus a **synthetic** 0.31-radian rotated
copy of the regression bottom. Rotation is rigid and all original fixture files
remain untouched. It is useful stress coverage, not a substitute for testing real
rotated projects and difficult contact cases before adoption.

Nine variants including control passed all five native material/volume/bounds/
STEP-round-trip comparisons: **45 variant/fixture cases**. All 45 exported STEP
files also passed independent libcascade validity, two-solid count, and both
directional material differences below 0.01 mm³. The 16 ordinary native tests
passed, with five diagnostic tests ignored in the ordinary run; the new ignored
comparison was separately invoked for every fixture.

Browser measurements use the same quiet-host harness, five fresh workers,
ten warm samples per fixture/variant/session, alternating forward/reverse order,
and separately instrumented attribution samples. The fresh control includes the
same disabled diagnostic hooks. It is not the byte-identical production bundle.
No whole-app latency, full-assembly export, or production acceptance is implied.

The staged browser build initially failed because a host-only target-cache symlink
was not resolvable inside its container. The newly created symlink was removed and
a normal isolated build succeeded. No timing samples came from the failed build.

## Remaining scope

The coverage table (historical record in Git at `323967ff`) keeps E0–E9 open at the
appropriate granularity. In particular:

- E4's actual two tabbed-plate regions each have 28 orthogonal outer vertices,
  140 hole vertices, and 390 candidate grid cells. The existing cap is 4,096.
  Implementing and validating the guarded mesher extension remains pending.
- E6 has confirmed stable inputs: 10 of 12 mounting tools and 12 of 16 openings
  retain their definitions across six edits. Reuse speed, clone cost, invalidation,
  Undo/eviction, and memory bounds have not been measured.
- E5 spatial grouping/ordering and tool reuse remain pending.
- E9 release/LTO/wasm-opt comparisons remain pending. The existing diagnostic
  WASM module declares an unshared memory; threaded-build and deployment
  feasibility are not established by flipping an OCCT option.
- Broader eligibility/fallback, contact, multi-body, cold/burst, interaction,
  export and memory-soak checks remain required before any production adoption.

Reproduction scripts (`cad/experiments/gasket/README.md` in the performance worktree) preserve both experiment
rounds and provide configurable variant/fixture lists. The first-round results
are retained separately rather than overwritten.
