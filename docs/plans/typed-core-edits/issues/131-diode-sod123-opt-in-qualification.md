# 131: Add an opt-in Vishay 1N4148W land pattern

Status: implemented (awaiting verification)
Type: hardware
Lane: F
Execution gate: root producer dispatch issued; sequential foreground command window required. Root records the claim before further implementation.

The reviewed plan uses the existing generator and assets, preserves all legacy defaults and output, and explicitly extends its parameter schema. The independently verified 43-case pre-change baseline is frozen, and its helper is checkpointed on lane F at `4607c266`. Baseline preparation and producer/qualification work use separate session assignments. No schema test may be weakened to accommodate the addition.

Planner: `/root/sofle_adaptation_research`, Sol 6.1 max. Independent plan reviewer: `/root/diode_plan_independent_review`, Sol 6.1 max, final amended plan PASS including actual registry input-contract confirmation. Producer implementer: `/root/diode_131_producer_implementer`, Luna 6 high, fresh session. A distinct fresh Sol 6.1 xhigh verifier follows the stopped writer. Bounded producer implementation and independent qualification passed; integration and full browser batch remain open.

Source packet: `/home/chris/.cache/boardstudio-baselines/ticket131/plan-packet.md`, SHA-256 `ebf03905161a0dc98ee101b9ce8bbd9546fcdc2dc948a5032e984124453449e9`. This ticket preserves the complete technical plan below; cache references are evidence, not a substitute for acceptance.

## 2. Primary source and precise attribution

Official manufacturer PDF: [Vishay document86356](https://www.vishay.com/docs/86356/1n4148w.pdf). Official product page: [Vishay1N4148W](https://www.vishay.com/en/product/86356/).

Captured PDF: 122766 bytes, six pages, SHA256 a39b522cdd61e128c49240a6608d162b6e9297dcb2b482b967b2587024525916. Datasheet Rev1.0,16-Nov-2023; package drawing S8-V-3910.01-003(4), Rev01,18-Jan-2022. Appended2026 legal material is not a package-drawing revision. Pages1 and4 were rendered and independently inspected as pixels in memory. The PDF is not persisted here because source-asset writes were excluded.

Manufacturer facts:

- Selected commercial part1N4148W-E3-08 is SOD-123, marking AH,3000/reel.
- Body2.7±0.15 by1.55±0.15 mm; overall terminal-tip span3.7±0.15; height1.175±0.175.
- Transverse terminal width0.55±0.10. Left foot length0.35±0.10 is explicitly led; do not transfer it to the right foot. Right0.5 is a reference projection.
- Page1 terminal1 is ANODE and terminal2 is CATHODE. The package drawing's cathode bar is on the opposite side of the page orientation; do not equate page left/right.
- Recommended lands explicitly label **both widths0.85**, **inner-edge gap2.50**, and **RIGHT height0.85 mm**. LEFT height is undimensioned. No recommended-land tolerance is supplied.

BoardStudio choices, not manufacturer facts: choose LEFT height0.85 mm, center the origin at the gap midpoint, and retain the existing cathode direction toward local negativeX. Derived pitch is3.35 and outside span4.20 mm. Do not infer undimensioned equality, symmetry or an absolute datum.

## 3. Existing producer and coverage

[Producer](../../../../footprints/src/generators/ceoloide/diode_tht_sod123.rs): PARAMS16-36, SPEC38-50, body52-322.

Existing source ID: ceoloide/diode_tht_sod123; version bundled-1; CC-BY-NC-SA-4.0. Registered through footprints/src/generators/mod.rs:38. Public Registry::render invokes the stored callback at registry.rs:238. No unused/safe inference follows from missing graph edges.

Existing ordinary SMD pads on either face: generatorpad1 at(-1.65,0), size0.9x1.2, netto; generatorpad2 at(+1.65,0), size0.9x1.2, netfrom. Local positions and IDs are the same in F/B blocks; only layers differ. Existing inner gap2.40 and right height1.20 differ from the manufacturer's recommendation. Outer span4.20 happens to match and cannot be the sole oracle. This demonstrates a recommended-pattern gap, not physical incompatibility.

Polarity crosswalk:

| Generator identity | Electrical role | Manufacturer terminal |
| --- | --- | --- |
| pad1, to | K / cathode | terminal2 |
| pad2, from | A / anode | terminal1 |

Never assume equal terminal numbers across producer and manufacturer. Keep this independent from duplicate-position diode assemblies and mirror behavior.

Existing model: ${KIPRJMOD}/models/boardstudio/infused-kim/Diode_1N4148W.step, offset[0,0,0.7], rotation[-90,0,0], scale[1,1,1]. The retained asset's previously verified SHA256 is abe50072b241c6ce69f8e79d787644ab3ff54d1ea09d74ee8025fb2bf4228722. Vendor provenance is infused-kim bb80a207d8a6fa7b9245caad2c2d97e2adc2f612, CC-BY-NC-SA-4.0; ceoloide source pin48935f54b456ff1503d78d6b17d9d146b54e8ade. See [NOTICE](../../../../footprints/NOTICE.md), [model manifest](../../../../ergogen/library/manifest/sources.json). These captured provenance facts were not reread during cache writing.

Historical alignment checks are generic and coverage says alignment unverified. Binding tests here prove emitted filename/transforms/overrides/omission only. They do not prove exact Vishay solid dimensions, contacts, cathode model marking, underside seating, soldering, installed F/B datum, case clearance or physical fit.

## 4. Exact additive parameter contract

Root selected the parameter route and explicitly accepted additive schema expansion.

Append AFTER to:

    ParamSpec::boolean("use_vishay_1n4148w_land_pattern", false)

PARAMS length becomes17. Existing16 names, kinds, values and declaration order remain unchanged. Source ID/version, SPEC identity/licence, callback signature and net allocation order stay unchanged. No Registry/Core API or member-visibility change.

Exact expected declaration sequence and defaults:

| Order | Name | Kind | Default |
| --- | --- | --- | --- |
| 1 | side | string | F |
| 2 | designator | string | D |
| 3 | reversible | boolean | false |
| 4 | include_traces_vias | boolean | false |
| 5 | trace_distance | number | 1.2 |
| 6 | trace_width | number | 0.25 |
| 7 | via_size | number | 0.6 |
| 8 | via_drill | number | 0.3 |
| 9 | include_tht | boolean | false |
| 10 | include_thru_hole_smd_pads | boolean | false |
| 11 | diode_3dmodel_filename | string | existing exact model path above |
| 12 | diode_3dmodel_xyz_offset | array | [0,0,0.7] |
| 13 | diode_3dmodel_xyz_rotation | array | [-90,0,0] |
| 14 | diode_3dmodel_xyz_scale | array | [1,1,1] |
| 15 | from | net | no default net; serialized value omitted |
| 16 | to | net | no default net; serialized value omitted |
| 17 | use_vishay_1n4148w_land_pattern | boolean | false |

In footprints/tests/generators.rs, add a source-specific **exact17-entry contract**. Assert exact length/names/order; assert every old field/default through the existing documented historical corrections; assert the new record exactly {name:"use_vishay_1n4148w_land_pattern",type:"boolean",value:false}. Do not filter new/unknown names, skip schema checks, accept arbitrary additions or loosen any other generator's contract.

The historical fixture has43 cases and16 parameter names. Its old side/vector historical corrections remain precisely bounded. Root owns the fixture; no edit or regeneration is authorized or required by this plan. Record this additive contract narrowly in docs/investigations/footprint-generators-rust.md if root grants that documentation path.

## 5. Input contract established by literal current source

Source window read HEAD2d4b03c9 with no runtime experiments. Literal excerpts and full-file hashes are in raw/source-branches.literal.txt and raw/source-and-binding-manifest.json.

Registry::render merges part parameters over definition parameters, discards supplied null, checks non-null values against declared kinds, then uses declaration defaults. flag() returns a resolved boolean. side() simply calls text("side"); there is no F/B enum validation at that accessor or string ingress.

Assume all other inputs valid:

| Input | Established behavior | Reaches existing body? |
| --- | --- | --- |
| reversible absent | defaultfalse | yes |
| reversible null | null discarded, defaultfalse | yes |
| reversible false/true | unchanged bool | yes |
| reversible wrong non-null type | InvalidParameter; parameterreversible must be true or false | no |
| side absent/null | defaultF | yes |
| side F/B | unchanged string | yes |
| malformed STRING side, e.g.X | accepted as text, no enum check | yes |
| wrong non-null side type | InvalidParameter; parameterside must be text | no |

Part-null overrides a definition value before filtering, so it selects the DECLARATION default rather than falling back to the definition. Apply this same established generic rule to the new boolean: definitiontrue plus partnull resolvesfalse.

Exact existing error messages, from error.rs:56-60 and params.rs:18-24:

    ceoloide/diode_tht_sod123: parameter reversible must be true or false.
    ceoloide/diode_tht_sod123: parameter side must be text.

A malformed string reaches the legacy producer. For nonreversible X, neither ordinary F/B SMD/silk branch is selected. Full downstream malformed-side results were not run; absent/false must preserve them rather than inventing an existing rejection.

Existing framework tests assert string-to-boolean rejection and null-default handling for a string parameter. Boolean-null and malformed-side behavior are established by source, not newly reproduced tests. All runtime assertions remain pending.

## 6. Producer behavior and validation order

Read the new flag through the existing boolean accessor. **ONLY effective TRUE** enters new eligibility validation, immediately BEFORE the original through-hole-SMD guard.

True is eligible only when:

- effective side is exactlyF orB;
- reversible=false;
- include_tht=false;
- include_thru_hole_smd_pads=false;
- include_traces_vias=false.

Reject every unsupported true combination with GeneratorError::rejected, SPEC.source and the opt-in parameter named. Proposed deterministic rejection text for ticket materialization:

    diode_tht_sod123: use_vishay_1n4148w_land_pattern requires side F or B, reversible: false, include_tht: false, include_thru_hole_smd_pads: false, and include_traces_vias: false

This is a new opt-in producer error, not a shared ingress change. Wrong non-null boolean/side types already fail before body; do not duplicate/coerce them. Absent, null/effectivefalse and explicitfalse must preserve ALL legacy validation order, errors, malformed-string-side paths and literal output templates.

Leave the old include_thru_hole_smd_pads/reversible guard, legacy pad strings and all existing output paths untouched. Avoid a generic numeric-template refactor of the false branch.

The true branch substitutes ONLY ordinary F/B SMD land strings: pad1 local(-1.675,0), pad2(+1.675,0), bothsize0.85x0.85, existing angle expression and pad/net roles, exact selected-face Cu/Paste/Mask layers. Preserve silk/model handling. Inactive trace/via numerical settings may remain set while their enabling flag isfalse; they do not authorize generated routing.

## 7. Independent qualification oracles

Proposed new test path:

/home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2/footprints/tests/diode_sod123_qualification.rs

Use existing public registry/Expr/geometry/net/model interfaces; no API widening or shared helper edits.

Geometry:

- Test both independent width callouts0.85, inner gap2.50, RIGHT height0.85.
- Test LEFT height0.85 as explicit configuration, not manufacturer attribution.
- Derive centers from widths/gap and the chosen origin, not producer coordinates. Derived K-to-A pitch3.35 and outer span4.20 are additional checks.
- Use nominal numerical epsilon1e-9 mm, separate from physical manufacturing tolerance. No tolerance for the recommended lands was published.

Pose/face:

- Use synthetic0°,90°,120° fixtures; they are test configuration choices, not changes to measured Sofle keys.
- Require exact F.Cu/F.Paste/F.Mask or B.Cu/B.Paste/B.Mask, no unintended opposite-face pads, unchanged local pad IDs/centers.
- Assert root pose independently. For the established clockwise board transform, fixed K-to-A vector oracles are (3.35,0), (0,-3.35), (-1.675,-2.901185102678).
- Do not derive those expected vectors by invoking producer/context transform helpers.

Polarity:

- Use distinct named, nonzero A/K nets and check names, indices and pad identities separately.
- Assert generatorpad1/to/K corresponds to manufacturerterminal2 and generatorpad2/from/A to manufacturerterminal1.
- Do not conflate this with companion/mirror assembly placement.

Silk cathode marking:

- Independently identify the existing two collinear bar segments: (-0.35,0)→(-0.35,+0.55) and (-0.35,0)→(-0.35,-0.55), stroke0.1.
- These fixed coordinates are the EXISTING GRAPHIC contract, not a manufacturer-dimensioned silk recommendation.
- Require the selected F.SilkS/B.SilkS layer and independently transformed endpoints at every pose.
- Require the midpoint on the geometric pad1/K side, nearer pad1 than pad2: distances1.325 versus2.025 mm.
- Identify/locate the bar without using net assignments; then independently verify pad1 carries to/K.

Model:

- Assert existing filename/offset/rotation/scale, explicit overrides and empty-filename omission.
- Keep emitted binding evidence separate from installed fit and exact manufacturer/model alignment.

Ingress/rejections:

- Test absent/null/false/true flag behavior, wrong non-null boolean type, non-string side ingress error, malformed string side under false versus true, every prohibited true flag independently and combined.
- Test definitiontrue overridden by partnull→declarationfalse.
- Assert the old through-hole guard's exact error survives false/absent.
- Preserve legacy malformed-string output/errors, whatever their observed baseline results are.

## 8. Forty-three-case byte baseline protocol

All captures and runtime commands require a later root window. **Nothing here has run.**

1. Add any needed test-only capture/compare support BEFORE changing the producer, within the specifically granted generators.rs test scope. Reuse that file's existing private historical-input adaptation; do not widen helpers or blindly re-render unadapted historical inputs.
2. Capture all43 cases against the untouched producer, keyed by original caseID/order. Record adapted effective inputs, public serialized forms, net allocation, exported footprint/object strings and error variant/fields/message as applicable.
3. Use exact UTF-8 bytes with explicitly fixed serialization/delimiters and per-record SHA256. Cache baseline outside the repo under ticket131; root owns it. Record producer/full-file hash before capture.
4. Freeze the baseline. After implementation, run the same43 adapted cases both with flag absent and explicitfalse; require byte-for-byte equality of public render/export outputs and identical nets/errors.
5. Keep the existing43-case golden file byte-identical, hash5454079a9bbd5f9e49a8b9003df3738a8049625fd505e7e4378546567232031a.
6. Also run existing golden comparison, exact17-entry schema contract and new true qualification tests. No baseline may be regenerated from changed production output.
7. Existing golden tests normalize whitespace by parsing forms. Distinguish canonical public serialized-byte equality from raw private body-string whitespace. The false branch's literal templates must remain unchanged; do not claim raw private callback bytes were captured by the public integration API.
8. Root must confirm the actual capture/compare invocation and newly created qualification target before execution; no such pre-existing capture command is known from retained evidence.

## 9. Separate meaningful mutation proofs

Only after separate explicit temporary producer leases:

- Geometry mutant: selected true width0.85→0.95, compiling and failing the intended width assertion.
- Polarity mutant: swap the true branch's to/from assignments, compiling and failing the intended A/K assertion with distinct nets.

After EACH mutant: preserve failure output, restore exact production bytes, touch the restored file so cargo cannot reuse the mutant build, verify restored hash, rerun intended tests and legacy baseline. Do not leave a mutant staged/committed. Independent verifier reproduces both; its session differs from planner and implementer.

No physical-tolerance relaxation, expectation adjustment or current-output oracle may make a mutant pass.

## 10. Prospective file ownership

Root must grant exact paths before dispatch. Proposed production/test/documentation scope:

1. /home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2/footprints/src/generators/ceoloide/diode_tht_sod123.rs — PARAMS plus true-only guard/SMD branches; false templates unchanged.
2. /home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2/footprints/tests/diode_sod123_qualification.rs — new dedicated public-boundary qualification tests.
3. /home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2/footprints/tests/generators.rs — exact additive schema contract and specifically approved private byte-baseline support.
4. /home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2/docs/investigations/footprint-generators-rust.md — narrow additive-schema deviation entry, only if granted.
5. Ticket131 issue path — ROOT TO MATERIALIZE/ASSIGN; no path guessed or created here.

Root owns historical/generated goldens, central registry, maps and integration. No registration, Registry/Core API, member visibility, Core behavior, schema framework, recipe, assembly, model/asset, case or measured-key edit.

A named generator variant was considered but not selected; it would change registry membership, fixture obligations and explicit catalogue counts36. Do not expand into that route.

## 11. Known checks and unresolved invocations

Future commands run sequentially in the exact F worktree with explicit absolute workdir and root command grant. **All pending.** Existing known Cargo target names:

    cargo test --manifest-path footprints/Cargo.toml --test generators
    cargo test --manifest-path footprints/Cargo.toml --test framework
    cargo test --manifest-path footprints/Cargo.toml --test parameter_lint
    cargo test --manifest-path footprints/Cargo.toml --test library_assertions

Known repository check entrypoints (root chooses affected scheduling; full check once per integration batch):

    python3 scripts/check.py repo
    python3 scripts/check.py lint
    python3 scripts/check.py test
    python3 scripts/check.py build
    python3 scripts/check.py typecheck
    python3 scripts/check.py browser

Root-confirmed invocations still needed:

- Newly created diode_sod123_qualification target and its exact test filters.
- Pre-change43-case baseline capture/compare and output files.
- Affected Core parameter-schema/catalogue test selectors.
- Affected browser generator-setting/apply/preview/export selectors and WASM validation schedule.
- Mutation/restoration/hash protocol commands under separate leases.

Do not invent missing test names, selectors or scripts. Native tests alone do not validate browser/WASM behavior. No dependency change or deny step is planned.

## 12. Bound graph evidence and unresolved risk

Binding: boardstudio-ts-boardstudio2, exact F path, indexedc2c04f3fb5bf037d0878f99f14607b19db0b42cf at2026-10-11T00:31:31.590Z, three commits behind capturedHEAD2d4b03c9. Prior bounded diff found only reset130 ticket/test changes between them; audited production paths were unchanged.

| Target | Actual reported impact | Limits |
| --- | --- | --- |
| diode body | UNKNOWN;0 upstream callers | lower-bound;25 receiver-typing gaps |
| diode PARAMS/SPEC | UNKNOWN;0 callers | zero is unresolved, despite returned exact indexed-edge envelope |
| registration SPECS | UNKNOWN;0 callers | source confirms registration/bundled use; not safe/unused |
| Registry::catalogue | UNKNOWN;0 callers | lower-bound;1 receiver gap |
| Registry::parameters | **HIGH**;14 symbols,d1=3,d2=9,d3=2 | lower-bound;3 receiver gaps; directcheck_parameters/render/schema-order test; normalization flows |
| Coreparameter_schema | **CRITICAL**;23 symbols,8 direct | indexed-edge envelope exact; migration/catalogue/settings/preview;8 PartsPreviewPanel flows |
| Registry::render context | lower-bound |8 receiver gaps; not a new impact verdict |
| RenderContext::side context | raw response unavailable | capped response could not be JSON-parsed; direct current source resolved this one contract |

HIGH/CRITICAL warnings remain binding. riskSharedAxes LOW/MEDIUM does not waive them. Source confirmation does NOT relabel UNKNOWN as LOW. No Registry/Core behavior edit is proposed, but declared schema data propagates to these consumers.

Raw MCP envelopes were not retained in memory for cache persistence; raw/graph-evidence-ledger.json is explicitly a NARRATIVE ledger, not a recreated raw response. See availability manifest. Only root refreshes the bound index; future pre-edit and pre-commit graph gates remain required.

## 13. Remaining limitations

Runtime tests,43-case byte capture/comparison, meaningful mutations, affected downstream checks and independent implementation verification are pending. No physical-fit, installed-model, fabrication-ready, full-BOM or whole118 claim follows.

Older builds' treatment of a saved new opt-in is not qualified; current render ignores undeclared names. This unit makes no backward-client promise and changes no document/API version.

No additional hardware selection, catalogue search or source acquisition belongs to this unit. The selected manufacturer, left-height/origin choices and exact limited qualification boundary are explicit.


## Bounded implementation and independent verification checkpoint

Luna 6 high implemented the opt-in producer, qualification tests, strict 17-field schema contract and explicit-false comparison. A distinct fresh Sol 6.1 xhigh verifier independently reviewed official PDF pages and tightened small qualification assertions, then passed 51 focused tests (3 ignored), both 43-case absent/false comparisons, four Core selectors and repo/lint/typecheck. Separate producer mutations for width 0.85 to 0.95 and A/K net swapping compiled and failed the intended assertions; each restored the exact producer bytes and passed qualification and both compatibility checks. The generated golden, frozen baseline/provenance and original absent comparator remain unchanged.

Complete independent evidence: `/home/chris/.cache/boardstudio-baselines/ticket131/producer-verification/verification.json`, SHA-256 `2c21a5399158561639d8b5789581113dadee9e8b31ac34771e2d82ae39d20fb7`. Producer SHA-256 `134975444c53722e2ebd761d8077bcb6437e257d856bff4d2fb4b1a91283a3c6`; qualification SHA-256 `fa203f8ea8bbc2bcf6250349360b7db2acc9ad06cdeec9f37d97ca150e9847fd`.

This is bounded independent PASS. Producer commit: `d246d4d9`; baseline-helper commit: `4607c266`. Root integration and full browser batch remain; the ticket stays open. No physical solderability, exact STEP alignment, installed fit, electrical/current, firmware or case qualification is claimed. Ticket 118 remains open. HIGH Registry and CRITICAL Core-schema warnings are retained; no shared API/ingress change was made.
