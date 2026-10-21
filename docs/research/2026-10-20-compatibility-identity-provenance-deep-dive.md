# Generator compatibility, input identity and portable provenance: deeper evidence

**Date:** 2026-10-20. **Examined HEAD:** `9099b3ef7a72b0919b1572f116490ed57514d43a`.
**Research only:** no policy acceptance, specification, tickets, claims or implementation.

**Subsequent user decisions (2026-10-20):** [ADR-0003's amendment](../adr/0003-rust-footprint-generators-in-core.md#amendment-existing-projects-use-current-generators-2026-10-20)
permits current generator output without freezing old geometry or requiring per-project
adoption. [ADR-0032](../adr/0032-recreated-inputs-start-unassigned.md) makes recreated
inputs start unassigned. [ADR-0033](../adr/0033-include-license-information-in-exported-packages.md)
requires `license.md` in exported packages. These settle the corresponding policy
questions below; they do not turn the source traces or pending experiments into
implementation-acceptance evidence. ADR-0032's later amendment retains prior bindings
for temporary disable/re-enable across save/reopen; deleted/recreated inputs remain
unassigned. ADR-0033's later amendments make modification notes optional, support
independent inner KiCad packages with their own root notice, and allow export with
explicit licence-unknown disclosure rather than a mandatory acknowledgement or
refusal solely for unavailable terms. These are not legal-compliance conclusions.

The working tree was extensively dirty throughout inspection. In particular, current ADRs
0013–0031 and `GLOSSARY.md` were untracked; production and documentation files also contained
concurrent edits. HEAD identifies Git evidence, **not an atomic version of this working tree**.
Only this report was intentionally written. Current-source links identify symbols and inspected
line ranges; recheck them before implementation. Historical versions below are explicit Git objects.

## Executive delta: what is newly established

This follows, rather than republishes, [D1/D2/D4](2026-10-20-adr-blind-spots-and-sustainability.md#ranked-next-decisions-and-exact-coverage-deltas)
and [C2/C3/C5](2026-10-20-adr-coverage-and-candidates.md#ranked-further-decision-candidates).

1. **Compatibility has real precedents, not just a hypothetical moved pad.** The Vishay diode
   change added different land geometry behind a default-false option while retaining
   `bundled-1`. The earlier JavaScript→Rust transition changed omitted-side execution semantics
   and explicitly migrated old back-side instances. Existing golden and migration tests cover
   these boundaries. Also, **Open is not a universal generator-normalization pass**: retaining
   saved pad data and executing today's generator during export are distinct behaviors
   ([A](#a-generator-compatibility)).
2. **The suspected cleanup difference survives a connected trace, but must be scoped.** For an
   ordinary unpaired matrix, generic shrink/disable/`RemoveParts`/whole-matrix removal retain
   binding maps; common post-edit cleanup only prunes outline dependencies. Serialized Open
   does not universally remove those bindings. Structural row/column/cell deletion instead
   explicitly remaps/prunes exact input IDs. Native mirrored membership has additional cleanup,
   so the unpaired result must not be generalized to every paired edit. Qualified `/push` IDs
   expose a further exact-key limitation ([B](#b-input-identity-and-binding-lifecycle)).
3. **A real portable archive demonstrates the provenance boundary.** Read-only inspection of
   committed `U1.boardstudio` found two embedded vendor models with verified, unchanged original
   bytes but no asset licence or revision metadata. Neither sampled model contains a licence/source
   URL notice; a third sampled KiCad model does contain one. Separately, reviewed marbastlib
   catalogue provenance is stored beside the definition, then omitted by the runtime catalogue
   decoder; a mechanical source URL survives inside the definition, but its licence record does
   not. This is more precise than saying that all provenance is lost—or that original bytes
   necessarily carry sufficient notices ([C](#c-portable-provenance)).

**Still undecided:** which generator changes require reviewed adoption; which removal/recreation
operations mean a new logical input; and which source, notice and authored-change information a
recipient must receive offline. No blanket executable/pin freezing, automatic keymap migration,
hardware-revision schema, historical-generator framework or unknown-licence export ban is chosen.

## Method and authority

Read [AGENTS](../../AGENTS.md), [domain instructions](../agents/domain.md), [GLOSSARY](../../GLOSSARY.md),
[PRODUCT](../../PRODUCT.md), [DESIGN](../../DESIGN.md), [architecture](../architecture.md),
[ADR index](../adr/README.md), and current 0003/0004/0009/0010/0013/0019/0020/0023–0028/0030/0031.
Read the installed domain-modeling skill and its ADR format: a separate ADR is justified by a
durable, surprising, actually chosen trade-off, not by discovering another module. Relevant
orientation includes the [generator plan](../plans/generator-parameters/map.md),
[matrix map](../plans/matrix-layout-consistency/map.md), [matrix spec](../plans/matrix-layout-consistency/spec.md),
[Rust-port investigation](../investigations/footprint-generators-rust.md) and
[component onboarding](../hardware/component-onboarding.md).

Evidence labels used below:

- **Source-established:** follows from inspected connected branches, with stated preconditions;
  not a fresh runtime result or recovered historical intent.
- **Inspected test:** the named body asserts the behavior; **no Rust/browser tests were run**.
- **Executed read-only observation:** Python standard-library ZIP/JSON/byte/hash inspection and
  Git-object reads only. No generated archives, build products, fixtures or source copies.
- **Pending:** requires an application/export run or qualified interpretation not performed here.

Policy constraints remain intact. [0030](../adr/0030-export-firmware-as-an-editable-repository-template.md)
means editable firmware source with external compilation, including disclosed unsupported optional
peripherals when the base keyboard is valid. [0031](../adr/0031-preserve-pcb-handoff-protection-across-undo.md)
starts protection after successful packaging, current ownership and a saved protection commit,
before delivery; Undo or download failure/cancellation does not erase it. [0024–0028](../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md)
do not permit shared copper to stand in for actual per-half assembly mapping. Their acknowledged
delivery gaps are debt, not alternatives to re-decide here.

The earlier four wording conflicts are not repeated as open findings: current [keycap reference](../features/keycaps-and-keymap.md)
uses mesh/STL scope; [0004](../adr/0004-document-format-version-and-id-rename.md#follow-up) distinguishes
the framed format-6 transition from later optional counters; [DESIGN](../../DESIGN.md#layout) separates
prepared manufacturing inputs from disposable previews; [architecture](../architecture.md) and
[0006](../adr/0006-replace-document-for-import-and-recovery.md) exclude user-authored scripting.
No scripting-feature research is inferred from retained implementation calls. Forgiving/partial
vendor previews and honest collision uncertainty remain governed by
[0017](../adr/0017-forgiving-vendor-previews-with-preserved-source.md) and
[0018](../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md), not universal certification.

## A. Generator compatibility

### A1. Data ownership is not executable ownership

| Boundary | Current behavior and compatibility consequence | Primary evidence |
| --- | --- | --- |
| Catalogue → project | Project definitions win by ID. Matrix setup clones a definition, changes its ID and extends selected parameters; it does not materialize every default or capture generator code. | [`merge_project_overrides`](../../web/crates/catalogue/src/catalogue.rs#L167-L191); [`snapshot_definition`](../../web/crates/catalogue/src/matrix_setup_operation.rs#L338-L355), diode overrides at L218–228. |
| Load migration → Open | Migration orders schema/ID repairs; current-version JSON is returned unchanged. Open prepares layout/mirror relationships and validates parameter values, then retains the prepared document. It does **not** call `normalize_definition` for every saved generator. | [`migrate_json`](../../core/src/migrate.rs#L51-L61); [`CoreEngine::handle`, Open](../../core/src/lib.rs#L241-L283); [`prepare_open`](../../core/src/mirrored_half/reconcile/prepare.rs#L26-L102). |
| Explicit normalization | Re-renders current generator geometry; retains saved pad ID/net **at the same index**, regenerates terminal membership, and preserves valid explicitly authored courtyard/keycap envelopes. This is not geometric freezing. | [`Registry::normalize`](../../footprints/src/definition.rs#L82-L169); [`normalize_definition`](../../core/src/generators.rs#L62-L76). |
| Definition + placed-part parameters | Part map overwrites definition map. Missing/null after merge selects the current declared default. Thus a **part-level null masks an explicit definition value**, rather than falling back to that definition value. Current compiled body is called. | [`Registry::render`](../../footprints/src/registry.rs#L163-L249); inspected `part_null_overrides_definition_true_with_the_declaration_default` in [diode tests](../../footprints/tests/diode_sod123_qualification.rs#L345-L360). |
| Export terminal mapping | Captured saved terminal→pad associations are used to derive generator net parameters, with conflict checks. The job then renders current Rust code; stable saved IDs do not prove unchanged terminal semantics. | [`terminal_parameters`](../../core/src/artifact/kicad/planning.rs#L157-L243); [`render_jobs`](../../core/src/artifact/kicad/generation.rs#L23-L59). |
| Model selection → bytes | Generator output determines model ID and datum; explicit filenames/transforms can override it, including empty filename omission. A document-owned asset wins over a bundled provider entry. An unembedded bundled ID resolves through the current build's catalogue/digest. | [`model_bindings`](../../footprints/src/models.rs#L101-L150); [`BundledModel`](../../web/crates/runtime/src/bundled_models.rs#L1-L15); [`load_model_files`](../../web/crates/runtime/src/pcb_handoff.rs#L220-L275); [portable round-trip test](../../web/crates/runtime/src/portable_archive.rs#L464-L503). |
| Manufacturing generation | Captured project data and revision guards protect a job's ownership, but `render_jobs` invokes today's registry. There is one supported version string, `bundled-1`, not a selection of historical bodies. | [`GENERATOR_VERSION`](../../footprints/src/registry.rs#L13-L14), `render` above; [`generation::export`](../../core/src/artifact/kicad/generation.rs#L69-L74). |

**Important limit:** this establishes that changed defaults/body/model-provider contents *can*
change a subsequent render without a document-format bump. It does not show that any sampled
release silently damaged a user's board. Conversely, saved pad geometry surviving Open does not
prove that the next generated manufacturing artifact reproduces those saved coordinates. The
inspected [`open_preserves_generator_and_copper`](../../core/tests/core.rs#L178-L230) even retains
an old `builtin:mx-switch` record; it asserts preservation, not successful rendering of that generator.

### A2. Real historical case: additive diode geometry, deliberately unchanged legacy path

The Git change is **`4607c26667bbe73f11914f1495e55c6e98ebcdc3` →
`d246d4d98f27c30dab8fec4f17dd24f657ee38ad`**. Both versions of
`footprints/src/generators/ceoloide/diode_tht_sod123.rs` were inspected with `git show`;
the former is the latter's parent. The commit message states “Preserve legacy defaults,”
but is implementation rationale, not evidence of a general user-approved compatibility policy.

| Input/configuration | Before | After / current | What follows, versus what is pending |
| --- | --- | --- | --- |
| Legacy ordinary F/B SMD diode; new flag omitted | Pad 1 at −1.65 mm, pad 2 at +1.65 mm; size 0.9 × 1.2 mm; `to` on 1, `from` on 2. | Same legacy branch. New boolean `use_vishay_1n4148w_land_pattern` defaults false. | Source establishes preservation for this branch. Existing golden/frozen comparison tests provide bounded oracles, not a run in this investigation. |
| Flag explicitly false or null | Name undeclared; old rendering loop only visits declared parameters. | Same default-false legacy branch. | New null/false equivalence is asserted by `absent_null_false_and_malformed_side_keep_legacy_rendering`. Old Open/edit admission of an undeclared value must not be confused with old renderer ignoring it. |
| Flag true, supported ordinary SMD settings | No selectable pattern; renderer has no corresponding branch. | Centers −1.675/+1.675 mm, each land 0.85 × 0.85 mm; same two pad numbers/order and `to`/`from` roles. | Real changed geometry, opt-in rather than a changed default. At zero rotation the center moves 0.025 mm outward and size changes; no need to invent a 0.3 mm example. |
| Flag true with reversible/THT/hole-pad/traces option | No opt-in validation branch. | Rejected with the named configuration requirement. | Compatibility includes admission/error behavior, not only successful geometry. |
| Model parameters omitted | Infused-Kim diode model; offset `[0,0,0.7]`, rotation `[-90,0,0]`. | Same defaults in this change. Explicit model override/empty omission remain supported. | This particular geometry update did not require a different model datum. It is not evidence that future model changes are harmless. |

Sources: the two Git objects above; [current declaration/body](../../footprints/src/generators/ceoloide/diode_tht_sod123.rs#L16-L73),
[land branches](../../footprints/src/generators/ceoloide/diode_tht_sod123.rs#L297-L335),
[`opt_in_lands_match_vishay_callouts_and_fixed_pose_oracles`](../../footprints/tests/diode_sod123_qualification.rs#L213-L246),
[`opt_in_requires_a_front_or_back_nonreversible_smd_configuration`](../../footprints/tests/diode_sod123_qualification.rs#L248-L281),
[`model_binding_keeps_emitted_defaults_overrides_and_empty_omission`](../../footprints/tests/diode_sod123_qualification.rs#L362-L435).
The qualification test distinguishes manufacturer callouts from BoardStudio's chosen left-land
height/centered origin; this report does not relabel all dimensions as manufacturer-mandated.

**Representative real saved data:** the committed [U1 archive](../../cad/test/fixtures/performance/U1.boardstudio)
contains `assembly-preset-mx-solder-south-matrix-1-0/definition/diode`, with source
`ceoloide/diode_tht_sod123`, version `bundled-1`, and parameters
`{"include_tht":false,"reversible":false,"side":"B"}`. It omits the new flag, model filename and
model transform. Read-only ZIP/JSON inspection confirmed this. The committed
[diode golden](../../footprints/tests/golden/generators/ceoloide__diode_tht_sod123.json) has the same
parameter shape in case `demo-definition:sofle-v2:assembly-preset-mx-hotswap-south-left-keys-0/definition/diode`;
its `default` case uses `{}`. Explicit false/true/null cases exist in the qualification bodies
above; they are not invented saved user projects. None of these archive reads executed generation.

### A3. Second historical case: omitted `side` changed meaning, with migration

At **`851875a0e2e42c7651ee02e6cbe997c291f3a6ad:ergogen/src/index.ts`**, `render` explicitly did
`p.side = inputs.side ?? (mirrored ? 'B' : 'F')`. At
**`d0c795766c38c2f8944fd19a0bbb9a4aced83193:footprints/src/registry.rs`**, Rust resolved
declaration defaults; `context.rs::side` read that resolved text. All generators declaring
default `F` means that a back-side part alone no longer supplies the omitted `side` value.
The [recorded deviation](../investigations/footprint-generators-rust.md#deviations) explicitly
describes this change; these source objects, not the prose alone, were inspected.

The concrete saved examples in [`migrate::tests::version_1`](../../core/src/migrate.rs#L392-L429)
cover omitted side on a back part, explicit `side:"F"` on a back part, a definition already
saving side, and an assembly member. [`state_back_sides`](../../core/src/migrate.rs#L193-L240)
writes `B` only where the old omitted-side behavior needs preserving; explicit saved side wins.
[`migrates_a_version_1_document`](../../core/src/migrate.rs#L433-L492) asserts each distinction.
[`the_version_1_step_states_the_back_side_and_leaves_text_for_version_4`](../../core/src/migrate.rs#L820-L837)
and `version_3_to_4_keeps_unknown_names_and_unconvertible_values` distinguish this from later
numeric-text migration. These are inspected assertions, not fresh two-version export results.

**Lesson supported by evidence:** a semantic executable change has previously been accommodated
by a narrow data migration rather than shipping all historical executables. It does not establish
that every future geometry correction can be solved by saving more parameters: hardcoded geometry,
pad order, terminal-role meaning and model-byte replacement are different compatibility dimensions.
The pad-order migration requirement is already explicit in [`Registry::normalize`'s comment](../../footprints/src/definition.rs#L82-L88),
and [onboarding](../hardware/component-onboarding.md#generator-parameters) already requires migration for kind changes.

### A4. Unsupported generator versus source-backed footprint

| Case | Current boundary, without substituting saved pads silently |
| --- | --- |
| Registered source, unsupported version | Registry render/normalization rejects `UnsupportedVersion`. Parameter checking is source/schema based, not a version renderer check, so an otherwise acceptable document can Open before a render/export later refuses. |
| Missing/unknown source | `normalize` errors when a generator record names an unregistered source. `render_forms` without a generator returns `MissingGenerator`. Open's parameter walk skips unknown sources, retaining repairable data rather than proving exportability. |
| Unknown generator inside `ceoloide/` or `infused-kim/` namespace | Export classifies by namespace, schedules a job and reaches registry rejection. An unsupported generator outside those namespaces reaches authored compilation, which refuses any generator-backed fallback. |
| No generator, retained `kicadSource` | Normalization returns unchanged. Export uses source import/patching, not a current generator body. Source format version must be 1; placement/net/UUID/model adjustments still mean exported text is not simply the untouched original file. |

Evidence: [`Registry::render`](../../footprints/src/registry.rs#L163-L175),
[`generator_of`/parameter walk](../../core/src/generators.rs#L78-L94),
[`parameter_problems`](../../core/src/generators.rs#L125-L147),
[`is_generated`/`is_imported`](../../core/src/artifact/kicad.rs#L449-L457),
[`planning::prepare`](../../core/src/artifact/kicad/planning.rs#L39-L137),
[`compile_authored`](../../core/src/artifact/compile.rs#L4-L20),
[`validate_source_version`](../../core/src/artifact/mod.rs#L227-L240),
[`patch_footprint`](../../core/src/artifact/source.rs#L123-L147).
Inspected tests: [`normalization_leaves_non_generator_definitions_alone_and_rejects_unknown_ones`](../../footprints/tests/framework.rs#L289-L308),
[`retired_generators_cannot_fall_back_to_saved_pad_geometry`](../../core/src/artifact/compile.rs#L237-L250).

### A5. Verification seams, pending A/B and policy alternatives

Existing coverage is substantial but not a universal release guarantee:

- [`bundled_generators_match_the_golden_baseline`](../../footprints/tests/generators.rs#L1069-L1100)
  consumes recorded forms, geometry, normalization, model bindings, terminal and export expectations.
  [Golden provenance](../../footprints/tests/golden/README.md) pins the JavaScript recording revision
  and lists intentional deviations. It is not an untouched oracle for every later product release.
- [`saved_pad_ids_and_nets_follow_index_and_extra_pads_get_fresh_ids`](../../footprints/tests/framework.rs#L344-L363)
  and [`normalization_regenerates_pads_and_keeps_saved_ids`](../../core/src/generators.rs#L440-L457)
  assert positional identity preservation, not safety of an arbitrary reordering.
- The manual `diode_sod123_baseline_capture`, `diode_sod123_baseline_matches_frozen_expected_bytes`
  and `diode_sod123_explicit_false_matches_frozen_outputs` tests at
  [L564–757](../../footprints/tests/generators.rs#L564-L757) are **ignored**, require explicit
  baseline environment paths, and one captures files. They were
  not run: neither a frozen baseline's availability nor their historical reported passing results
  is a new executed receipt here.

**Small pending experiment, no new generic harness:** use the existing public
[`artifact_request` line driver](../../core/examples/artifact_request.rs) and the literal
`{"id":"export","kind":"export-pcb","request":{"snapshotToken":"snapshot","expectedRevision":...,"document":...,"target":{"kind":"standalone-footprints","definitionIds":[...]},"contours":[],"modelPaths":...}}`
seam already exercised by [`export`/`export_footprint`](../../core/tests/kicad_integration.rs#L26-L82).
With separately authorized, source-bound builds of the diode parent and change commits, feed the
**same** legacy definition from the golden, once omitted and once explicit false. Supply the same
model path map, and compare pad number/order/coordinates/layers, terminal nets, model path/datum and
error results, not only ZIP hashes. Expect unchanged legacy geometry. On the changed build only,
explicit true should yield the literal ±1.675/0.85 mm lands. For placed-part null precedence, use
the existing qualification test or a board-target request containing the placed Part, **not**
standalone export: null should select false even over a true definition. The existing qualification
tests are the smallest place for extra assertions if later authorized. Full-board `ExportTarget::Board` adds saved pin assignments/copper
and is the next bounded check, not a prerequisite for establishing the literal land difference.

For the side transition, old/new direct rendering without migration should distinguish omitted
back-side input; migration followed by rendering should restore B, while explicit F stays F.
Current [migration tests](../../core/src/migrate.rs#L433-L492) are the nearest public-data oracle.
**Neither two-build comparison was executed.** No prebuilt executable's source provenance was
established, and builds/worktrees/source copies were prohibited here. A current protected PCB
baseline must not be treated as a geometry fingerprint: [0031](../adr/0031-preserve-pcb-handoff-protection-across-undo.md)
protects assignments; a separate changed-geometry review rule is not accepted by that ADR.

Feasible **unselected** policies:

| Alternative | Benefit | Cost / distinction needing agreement |
| --- | --- | --- |
| Current executable, compatibility-preserving migrations/options | One implementation; diode and side transitions demonstrate bounded mechanisms. | Requires classifying changes and preserving legacy defaults/meaning intentionally; not all hardcoded fixes are expressible as old parameters. |
| Current executable, changed-result disclosure and reviewed adoption for specified classes | Can fix mistakes without indefinitely retaining all bodies. | Needs a comparison/reference and clear stale/updated status; merely naming a generator revision does not calculate a geometric delta. Decide whether old output remains available. |
| Retain a resolved source artifact per adopted definition | Can preserve the chosen footprint independent of current generator bodies. Existing source-backed export is a concrete contrasting path. | Added persisted data, editing/refresh authority and migration questions; retaining footprint text alone does not pin model bytes or all board objects/nets. |
| Retain historical executables | Can regenerate historic parameter combinations. | Multiple implementations/security fixes/default schemas and support window; no evidence that this broader framework is necessary or previously selected. |

**Documentation disposition:** prefer an amendment to [0003](../adr/0003-rust-footprint-generators-in-core.md)
once compatibility expectations are chosen, cross-referencing 0004/0009 and an ordinary change-class
reference. A separate ADR is warranted only if the owner chooses a broader persisted artifact/variant
ownership boundary with meaningful reversal cost. No candidate count or implementation framework is prescribed.

## B. Input identity and binding lifecycle

### B1. Four identities that must not be conflated

1. **Physical placement** is a pose, matrix row/column and physical instance/face. Moving it need
   not change the input ID. Matrix member IDs are coordinate-derived `matrix/{matrix}/r{row}c{column}`;
   structural compaction can rename a survivor to a formerly occupied string
   ([`member_id`](../../core/src/matrix.rs#L77-L104), [`remove_axis`](../../core/src/matrix/deletion.rs#L378-L465)).
2. **Typed logical bindings** are document-level layer maps keyed by input ID, and sensor maps keyed
   by encoder ID. **Legacy bindings** are separate Board electrical `key_bindings`. The legacy edit
   also updates the typed base layer when a typed map already exists; neither makes layers instance-scoped
   ([types](../../core/src/keymap.rs#L64-L95), [`SetKeyBinding`](../../core/src/keycaps/edits.rs#L5-L60)).
3. **Press versus rotation** is profile/scan-mode dependent. A matrix encoder's press ID is its Part ID;
   a direct standalone encoder/press uses `<part>/push`; rotation uses the Part ID in sensors
   ([`inputs::profile`/`scan_mode`](../../core/src/inputs.rs#L50-L106),
   [`electrical_peripherals::describe`](../../core/src/electrical_peripherals.rs#L59-L130)).
4. **Firmware position/order** is supplied by the current electrical plan's assignment sequence,
   followed by auxiliary pushes; the keymap source looks up those IDs in that order. Missing base
   bindings fall back to supplied legacy text/`&none`, higher layers to `&trans`. Old map entries
   are not emitted merely because stored, but can become active if an ID returns
   ([adapter](../../web/crates/runtime/src/firmware_request_adapter.rs#L90-L171),
   [`keys_for_bindings`](../../web/crates/runtime/src/firmware_request_adapter.rs#L449-L469),
   [`keymap::source`](../../core/src/keymap.rs#L472-L501)).

[PRODUCT](../../PRODUCT.md#capabilities-and-constraints) and [DESIGN](../../DESIGN.md#wiring-and-physical-assemblies)
disclaim automatic keymap migration/deleted-key tombstone promises. Their “new hardware/keymap
revision” wording is **not evidence of a persisted hardware-revision object**. Shared reversible
PCB design, per-half assembly mapping and independent central role remain the separate decisions
in [0024](../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md) and
[0026](../adr/0026-decouple-firmware-role-from-physical-side.md).

### B2. Connected cleanup trace: the missing universal pass was checked

For **unpaired, nonscripted ordinary matrix data**, the public Core flow is:

- `SetMatrix` → authored-companion placement wrapper → `matrix::set_matrix` → legacy link sync.
  Shrink drops out-of-bounds cell overrides, parts, net pins and constraints, but does not visit
  typed bindings/sensors, keycap configuration maps or electrical binding maps
  ([dispatch](../../core/src/lib.rs#L1943-L1970), [`set_matrix`](../../core/src/matrix.rs#L714-L998)).
- `SetMatrixCellsEnabled` → `edit_selected_cells(Some(enabled))` → the same setter.
  A disabled cell remains a cell, but has no actual Part; `deleted` cells are refused by that
  public enable route ([deletion module](../../core/src/matrix/deletion.rs#L470-L544)).
- `RemoveParts` marks matrix cells deleted, removes physical/dependency membership and syncs
  legacy companions; it does not call the richer `remap_parts`. `RemoveMatrix` delegates to
  `RemoveParts` and deletes the matrix/empty owned nets
  ([dispatch](../../core/src/lib.rs#L1805-L1885), [`removed_members`](../../core/src/matrix.rs#L108-L164)).
- `RemoveMatrixRow`/`Column`/`Cells` instead call `remap_parts`, which explicitly transforms typed
  binding and sensor maps, keycaps, nets, constraints, board/layout membership and legacy electrical
  bindings; row/column survivor renaming is simultaneous, not iterative overwrite
  ([`remap_parts`](../../core/src/matrix/deletion.rs#L119-L262), [`remove_cells`](../../core/src/matrix/deletion.rs#L321-L369)).
- After dispatch, `CoreEngine::edit` validates map **values**, resolves constraints and legacy
  links, then calls `prune_removed_components`. That common function only delegates to outline
  source pruning. Scene/recompute do not create a general binding-pruning pass. Open validates
  the typed map and prepares relationships but does not reject or prune every binding key absent
  from `parts` ([edit](../../core/src/lib.rs#L598-L664),
  [`prune_removed_components`](../../core/src/lib.rs#L1473-L1483),
  [`keymap::validate`](../../core/src/keymap.rs#L226-L292), [`prepare_open`](../../core/src/mirrored_half/reconcile/prepare.rs#L26-L102)).

This supports a **bounded retained-state/reactivation outcome**, not just “one function lacks
cleanup.” Save serialization preserves these map fields; migration has no input-binding retirement
step ([model types](../../core/src/keymap.rs#L69-L83), [`migrate_value`](../../core/src/migrate.rs#L18-L49)).
The exact end-to-end shrink/save/reopen/regrow workflow still lacks an executed observation here.

The UI has real, different producers: [selected-key actions](../../web/src/presentation/layout_key_selection_actions.rs#L335-L356)
submit structural selected-cell deletion versus enable/disable; [matrix delete resolver](../../web/crates/layout/src/objects/matrix_inspector_controller.rs#L178-L195)
submits `RemoveMatrix`; [transform operation builder](../../web/crates/layout/src/matrix_transform_operation.rs#L208-L257)
chooses row/column/cell deletion by selection. Therefore a future fix must name user operations,
not assume every visible Delete action already traverses one shared cleanup function.

### B3. Compact scenario matrix

Unless stated otherwise, outcomes below concern an **ordinary unpaired matrix**, valid binding
values and no unrelated blocker. “Returns” means the stored value is eligible for lookup when a
current input again has the same ID; it does not promise that hardware planning/export succeeds.

| Scenario | Source-established outcome | Named inspected test proving a subset | Still unobserved / important qualification |
| --- | --- | --- | --- |
| Remove interior row 1 of 3; bound row-2 key survives as row 1 | Survivor reference, Part keycap size and exact-key binding metadata move together; removed slot does not donate its metadata. Constraints/net pins/direct wiring are remapped. Column removal uses the same axis algorithm. | `deleting_matrix_row_preserves_surviving_part_and_binding_identity`; `deleting_matrix_row_remaps_nets_constraints_keycaps_and_direct_wiring`; `deleting_matrix_column_compacts_both_halves_and_preserves_local_assemblies` in [Core tests](../../core/tests/core.rs#L3762-L4066). | Complete all-layer/legacy/sensor/push + firmware-order + save/reopen cross-product not executed. Tests do not prove a generic semantic migration promise. |
| Structural `RemoveMatrixCells`, then recreate coordinate without Undo | Exact key entries are pruned; cell has `deleted:true`, disabled, assemblies cleared. Ordinary enable refuses “Matrix cell was deleted.” Recreating requires a different operation that clears/replaces cell state, e.g. a valid `SetMatrix`; exact pruned values do not return. | `deleting_matrix_cells_propagates_tombstones_and_survives_reopen` [L3882–3931](../../core/tests/core.rs#L3882-L3931). | This test checks structural absence/reopen, not every binding map. Do not advertise a product keymap tombstone: `deleted` prevents geometric rematerialization. |
| Generic `RemoveParts` of a matrix key, then valid `SetMatrix` clearing deleted state | Physical key/companions removed and cell marked deleted; ordinary unpaired typed/legacy/keycap maps are retained. Reusing the exact ID can reactivate retained entries. | `open_preserves_generator_and_copper` is not a deletion oracle; there is **no named full workflow oracle established here**. | Pending direct public workflow. UI bulk Delete uses structural removal, so this is not a claim that all UI Delete actions retain bindings. [Dispatch](../../core/src/lib.rs#L1845-L1885). |
| Disable → enable | Setter removes then recreates the Part under the same coordinate ID. Typed/legacy bindings and configured keycap overrides remain eligible. Part-owned fields such as an explicit `Part.keycap`, reference or generator override are not archived by disable and may be regenerated/lost. | `matrix_cells_preserve_survivors_and_companions` [L2732–2820](../../core/tests/core.rs#L2732-L2820) covers surviving existing Parts, not full disable restoration. | Need direct disable/reenable assertions separating configured keycap metadata from Part-owned fields. Not equivalent to Undo. |
| Shrink 3→2, serialize/save, fresh Open, regrow 2→3 | Out-of-bounds cells/Parts and their net pins/constraints are removed; typed/legacy bindings and configured keycap map entries persist on the traced unpaired route. Fresh Open has no universal retirement pass; regrowth reuses the coordinate ID and can reconnect the old binding. | `matrix_shrink_discards_out_of_bounds_edits` [L2649–2700](../../core/tests/core.rs#L2649-L2700) covers cell/offset cleanup. | Binding reactivation is source-derived, **not a run**. Native mirrors have a separate membership path; do not extrapolate this row to them. |
| Remove the only row/column versus delete whole matrix | Last-axis removal first calls `remap_parts`, pruning exact current key entries, then `RemoveMatrix`; direct whole-matrix deletion delegates directly to generic `RemoveParts`, so the traced unpaired binding retention differs. Matrix-level keycap config is not itself in the Part remap map. | `deleting_last_matrix_row_removes_matrix_and_key_metadata_and_is_undoable` [L3933–3970](../../core/tests/core.rs#L3933-L3970). | Test seeds one typed key only; not all orphan config, disabled previously absent keys or fresh UI ID allocation. |
| True Undo → Redo | Document snapshots restore removed Parts, cell state and binding data, then redo the accepted state; revision increments. Existing PCB protection is specially preserved. Open clears history. | Last-row test above; `bindings_create_defaults_preserve_geometry_and_support_history` [keymap tests](../../core/tests/keymap_edits.rs#L42-L64). | Reopen is not Undo; [0013](../adr/0013-save-before-publication-and-explicit-recovery.md) does not promise history survival after reopen/executor replacement. [History source](../../core/src/lib.rs#L868-L912). |

**ID-allocation nuance:** a public `SetMatrix` can explicitly reuse a free matrix ID, but the ordinary
new-matrix helper checks remaining owned definition prefixes as well as current matrices/parts.
After removal it may therefore choose a different ID, leaving retained entries dormant rather than
reactivating them. This is why direct Core reuse and an ordinary UI “new matrix” must be tested
separately ([`next_matrix_id`](../../web/crates/catalogue/src/matrix_setup_operation.rs#L107-L138)).

### B4. Suffixes, mirrors, legends and queued ownership

- **Qualified pushes are not automatically covered by exact-key remapping.** `remap_map` transforms
  only keys present in `PartChanges`. `cell_changes` includes primary and actual companion Part IDs,
  not every synthetic `<part>/push`; only electrical `direct/` and `link/` function keys are specially
  expanded. A matrix encoder's own press uses the bare Part ID and does benefit. A standalone/direct
  push or a direct press on an attached component needs a separate suffix scenario. This is a
  source-established coverage limit, not an executed report of a broken device
  ([remapping](../../core/src/matrix/deletion.rs#L220-L248), [cell changes](../../core/src/matrix/deletion.rs#L283-L302),
  [press identity](../../core/src/electrical_peripherals.rs#L73-L96)).
- **Legend/configuration is not one field.** `keycaps.keys[id]` overrides and typed/legacy-derived
  legends are looked up for current Parts; explicit blank legend differs from inheritance. Structural
  deletion remaps that map; generic removal does not. Separately, `Part.keycap` belongs to the removed
  Part, not the retained map ([resolution](../../core/src/keycaps.rs#L124-L216),
  [`matrix_and_key_edits_merge_fields_and_distinguish_blank_from_inherited`](../../core/tests/keymap_edits.rs#L87-L129)).
- **No compensating universal mirrored cleanup was inferred.** Native distinct-board pairs use
  [`uses_prepared_transaction`](../../core/src/lib.rs#L1412-L1433) and
  [`prepare_transaction_with_ledger`](../../core/src/mirrored_half/reconcile/prepare.rs#L619-L710).
  Their membership kernel expressly runs target-side `remove_axis`/`remove_cells` and
  `remove_parts_by_id`, with Local detachment/omission handling
  ([kernel](../../core/src/mirrored_half/reconcile.rs#L6413-L6489), [target cleanup](../../core/src/mirrored_half/reconcile.rs#L6736-L6768)).
  Thus the unpaired retention result cannot be called a universal bug. Electrical reconciliation
  deliberately restores the mirror Board's local bindings/protection rather than copying source
  bindings ([L3563–3651](../../core/src/mirrored_half/reconcile.rs#L3563-L3651)); [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)
  keeps key bindings nonmirrored. Complete source-and-target shrink/regrow metadata outcomes,
  Local/Omitted combinations and same-board reuse remain pending, not resolved by this slice.
- **A stale queued UI target is a different hazard from dormant persisted bindings.** The inspected
  [`queued_key_resize_retires_when_row_delete_reuses_the_selected_id`](../../web/crates/layout/src/objects/keycap_size_tests.rs#L1073-L1139)
  drives real Runtime/Core behind a held reply and expects a `Failed` settlement containing
  “matrix topology changed”; despite its name, its assertion is not silent `Retired`. Selected-key
  actions also capture topology. Neither is proof that all keymap controls guard every reused ID
  ([action guard](../../web/src/presentation/layout_key_selection_actions.rs#L116-L127)).

### B5. Unmade domain decisions and minimum verification

The user-facing alternatives are **not** “which helper should own cleanup?” They are:

| Decision scenario | Genuine alternatives and costs, not accepted policy |
| --- | --- |
| Temporarily disable an input | Resume binding/appearance on enable, or treat enable as a new unassigned input. The former supports layout exploration but requires clearly defining which Part/configuration data survives; the latter discards useful temporary work. |
| Shrink away a row, save/reopen, regrow without Undo | Restore the old meaning, create fresh unassigned inputs, or present recovered values for explicit review. Coordinate reuse makes restoration cheap today but can silently inherit unintended behavior; retirement/reset is simpler semantically but loses recoverable work. |
| Delete and deliberately recreate an input | A replacement should start blank, inherit by position, or offer explicit carry-over. This must be distinguished from survivor compaction and true Undo; a “no automatic migration” disclaimer does not select all these outcomes. |
| Compact survivors | Keep bindings with the surviving physical input, or intentionally reset/review the affected ordering. Existing remapping strongly supports survivor-following implementation, but does not authorize moving old deleted bindings onto a different survivor. |

**Pending smallest public experiment:** extend existing [Core matrix tests](../../core/tests/core.rs)
later, not a new harness. Seed two layers, a legacy base binding, a keycap legend override, a
Part-owned size, a net pin and a constraint. Exercise each table row through `CoreEngine::handle`;
between shrink and regrow serialize into a **fresh** Core Open. Inspect stored maps while absent,
then current IDs and generated keymap order. Add one matrix encoder bare press and one direct
`/push` case separately. Contrast true Undo/Redo in the original engine with reopen's empty history.
Expected observations are the table's scoped results; a differing result must amend this research,
not be forced to match it. Repeat the smallest shrink/compaction cases through the native-pair route
before any universal cleanup proposal. Existing [keymap workflow tests](../../core/tests/keymap_workflow.rs#L67-L142)
cover serialization/edit/history and invalid values, **not this full lifecycle matrix**.

No application probe was executed: no provenance-bound current prebuilt Core driver was established,
and building/running a test against an active target would violate this task's constraints.
If the owner chooses a durable same-input/replacement contract, this is a plausible separate domain
ADR; otherwise first document actual operation semantics and add bounded regressions. Do not invent
keymap tombstones, per-half schema or semantic migration from a structural cell marker.

## C. Portable provenance

### C1. Representative assets and what their original bytes actually say

This portfolio uses existing files, not a newly fabricated “mixed package.” The real U1 archive
contains the first two models; the KiCad model and marbastlib footprint are additional source cases.
No new combined PCB ZIP was produced or claimed inspected.

| Item | Repository-owned source/revision/licence evidence | Original-file evidence and its limit |
| --- | --- | --- |
| Generated `ceoloide/switch_mx` + KiSwitch `SW_Cherry_MX_PCB.stp` | Generator is MIT, author @ceoloide, upstream `48935f54b456ff1503d78d6b17d9d146b54e8ade` in [footprints NOTICE](../../footprints/NOTICE.md). [Model manifest](../../ergogen/library/manifest/kiswitch.json) pins KiSwitch `aefcf65038d48d2666ff14530d482be3c350fa6e`, hash `ddf0fb3a776faa6105303efb98e39645e044c43b3ac03cbfe75d8eda28e297f4`, and its [separate licence/exception](../../ergogen/library/vendor/kiswitch/licenses/LICENSE). | [STEP header](../../ergogen/library/vendor/kiswitch/3d_models/SW_Cherry_MX_PCB.stp#L1-L20) names FreeCAD, kicad StepUp/ksu MCAD, timestamp and an original local filename. Whole-file search found no `Copyright`, licence, author or HTTP URL match. It does not identify the pinned Git revision/licence; generator MIT must not be applied to this model. |
| Generated `ceoloide/diode_tht_sod123` + Infused-Kim `Diode_1N4148W.step` | Generator [Rust header](../../footprints/src/generators/ceoloide/diode_tht_sod123.rs#L1-L6) retains copyright/SPDX/authors; [NOTICE](../../footprints/NOTICE.md) records Ceoloide and Infused-Kim upstream pins. [Sources manifest](../../ergogen/library/manifest/sources.json#L39-L47) pins Infused-Kim `bb80a207d8a6fa7b9245caad2c2d97e2adc2f612`; model hash is `abe50072b241c6ce69f8e79d787644ab3ff54d1ea09d74ee8025fb2bf4228722` at L112. | [Model header](../../ergogen/library/vendor/infused-kim/3d_models/Diode_1N4148W.step#L1-L13) actually says `MMSZ5226B-7-F.STEP`, blank author/organization, SolidWorks 2017. Whole-file notice/URL search found no matches. Renamed file identity is not reconstructible from this header alone. [Vendor LICENSE](../../ergogen/library/vendor/infused-kim/LICENSE) says BY-NC-SA; [vendor README](../../ergogen/library/vendor/infused-kim/README.md#L158-L172) says BY-SA. Preserve this unresolved source inconsistency, not a guessed legal resolution. |
| KiCad `PinSocket_1x05_P2.54mm_Vertical.step` | [Manifest](../../ergogen/library/manifest/kicad.json#L1-L27) pins `e62ed1fc7862da83f789bd562671b5e4b82afcdf`, upstream path and hash; [collection licence](../../ergogen/library/vendor/kicad/LICENSE) states CC-BY-SA plus exception. It is a declared model of `display_nice_view` in [default-models](../../ergogen/library/manifest/default-models.json#L18-L27). | [The STEP header itself](../../ergogen/library/vendor/kicad/3d_models/PinSocket_1x05_P2.54mm_Vertical.step#L1-L41) contains KiCAD copyright, licence URL, exception, disclaimer and upstream licence-page URL. Those original bytes can carry meaningful attribution even under a hash filename. They do **not** carry the manifest's exact source commit. |
| Source-backed marbastlib `STAB_MX_2u`, reusable/custom Part kind | [Import manifest](../../catalogue/parts/import-manifest.json#L4-L15) records repository, `6b0a9a73f579e377816d60b58eac2b3252de7868`, source path/hash and CERN-OHL-P-2.0 with retained [licence](../../catalogue/parts/sources/LICENSE). [Catalogue README](../../catalogue/parts/README.md) distinguishes normalized drawing from authoritative source. | [Original footprint](../../catalogue/parts/sources/STAB_MX_2u.kicad_mod#L1-L12) contains description and reference/value text, but whole-file licence/URL/copyright/author search found no match. Exact source is stored in `kicadSource`; a separate mechanical profile source URL names the pinned upstream. Licence is in sibling provenance, not in footprint text. |

**Executed read-only observation:** `zipfile.ZipFile` read
[`cad/test/fixtures/performance/U1.boardstudio`](../../cad/test/fixtures/performance/U1.boardstudio)
without extraction. Its bytes matched `git show HEAD:<path>`. Exact entries were:

```text
project.json
archive.json
assets/ddf0fb3a776faa6105303efb98e39645e044c43b3ac03cbfe75d8eda28e297f4
assets/abe50072b241c6ce69f8e79d787644ab3ff54d1ea09d74ee8025fb2bf4228722
```

`archive.json` is `{"embedUsedModels":true}`; format version is 6. The asset records retain
bundled IDs, original display filenames, media types and SHA-256, with `source:"bundled footprint library"`
and **no licence field**. The two payloads were 2,769,044 and 149,936 bytes respectively;
`hashlib.sha256` matched each asset record, and byte comparisons matched the current vendored
files in the table. The archive contains no `kicadSource` definitions and no notices inventory.
These are observations of this existing fixture, not a fresh Core pack/unpack acceptance run or
an assertion about all saved archives.

### C2. Propagation: richer source evidence is not automatically portable metadata

| Stage | Present | Not carried automatically / consequence | Primary source |
| --- | --- | --- | --- |
| Rust generator specification | SPDX and authors; file header copyright; NOTICE holds upstream repositories/commits and licence texts. | `CatalogueEntry`/`PartGenerator` does not serialize `GeneratorSpec.license`, copyright, upstream revision or authored-change record. Generator source/version is not the upstream revision. | [`GeneratorSpec`](../../footprints/src/registry.rs#L24-L50); [`CatalogueEntry`](../../footprints/src/definition.rs#L58-L76); [`PartDefinition`/`PartGenerator`](../../core/src/model.rs#L130-L209). |
| Reviewed imported catalogue | Generated rows contain `{definition, diagnostics, provenance}` including marbastlib licence/pin/hash. | Runtime `ImportedPart` decodes only `definition`. This is a concrete loss boundary before placement, not an archive bug. Mechanical profile's source URL survives independently. | [`import_parts`](../../scripts/import-kicad-parts.py#L33-L95); [`ImportedPart`](../../web/crates/catalogue/src/catalogue.rs#L71-L79); [actual catalogue](../../catalogue/parts/imported-parts.json). |
| Project-owned definitions/customization | Name, generator parameters, source text, mechanical/hardware profiles and models can travel in the document. Optional `hardwareProfile.source` can carry richer source evidence when supplied. | Plain `PartDefinition` has no universal upstream/author/change-log/licence record. Renaming or changing parameters does not itself record who changed what relative to which upstream. Existing source/profile URLs are not erased merely because universal metadata is absent. | [model](../../core/src/model.rs#L124-L209); [`HardwareSource`/`HardwareProfile`](../../core/src/hardware.rs#L4-L52); [matrix snapshot](../../web/crates/catalogue/src/matrix_setup_operation.rs#L338-L355); [import writer](../../scripts/import-kicad-parts.py#L75-L94). |
| Model staging/build metadata | Stable bundled ID, source-relative path, emitted path, media type, source digest in Rust metadata. Original model bytes copied/symlinked. | Stager scans model extensions only; runtime model rows lack licence, copyright and source revision. Vendor manifests/notices are not embedded by this row construction. | [`catalogue`/`stage_models`](../../scripts/stage-ergogen-models.py#L26-L105); [runtime build generator](../../web/crates/runtime/build.rs#L13-L90); [`BundledModel`](../../web/crates/runtime/src/bundled_models.rs#L1-L9). |
| Project portable archive | `project.json`, optional `archive.json`, content-addressed `assets/<hash>`. Local `Asset` supports optional licence/source and existing records are cloned unchanged. Newly embedded bundled assets get filename/type/hash and generic source. | Newly embedded bundled metadata uses `license:None`; no per-generator notice inventory is added. Preserving original bytes is not a structured source/revision record. | [`prepare_archive`](../../web/crates/runtime/src/portable_archive.rs#L10-L96); [`Asset`](../../core/src/model.rs#L639-L649); [`archive::dispatch`](../../core/src/archive.rs#L44-L117). |
| PCB handoff | Inner `<board-name>-kicad.zip`: first Core board file plus `models/<hash>.<ext>`. Outer handoff: inner ZIP, `wiring-report.json`, `ASSEMBLY.md`, applicable `jumpers/assembly-N/part-N.svg`. Models hash-checked and copied as bytes. | No automatic project.json, original display-name/source/licence inventory or licence-text files. Wiring/jumper reports can retain some generator source IDs, not a full provenance ledger. | Complete [`build_handoff`](../../web/crates/runtime/src/pcb_handoff.rs#L65-L218), [`load_model_files`](../../web/crates/runtime/src/pcb_handoff.rs#L220-L275), [`assembly_instructions`](../../web/crates/runtime/src/pcb_handoff.rs#L340-L397). |

The PCB contents row is **structural source inspection**, not inspection of a produced PCB ZIP.
Hash filenames identify byte contents, but a recipient cannot reverse a hash into a repository,
revision or licence without an independent catalogue. In contrast, the sampled KiCad STEP header
survives because the model loader does not rewrite those bytes. Both claims can be true simultaneously.

**Source-backed/custom case:** Python JSON inspection of `imported-parts.json` confirmed that
the marbastlib `kicadSource.source` equals the retained original footprint text, while
`definition.mechanicalProfile.source` contains the exact pinned GitHub blob URL. The outer row
retains its six provenance fields, but the runtime decoder omits them. Source patching can preserve
an authored/vendor property already inside footprint source; it cannot synthesize a missing licence
or a customization attribution. The synthetic `RICH_SOURCE` regression explicitly retains
`"Vendor note" "café 東京 & Ω"` through import/export; it is a source-retention example, **not a
vendor licence or a real customized user project**
([`native_source_import_projects_asymmetric_pads_and_retains_exact_kicad_source`](../../core/tests/kicad_integration.rs#L750-L772),
[`native_board_export_preserves_untouched_source_spans_and_kicad_10_parses_and_plots_both_sides`](../../core/tests/kicad_integration.rs#L774-L831)).
That latter test invokes KiCad when run; it was only read here.

### C3. Embedded versus linked and offline discoverability

- **Embedded bundled models:** U1 demonstrates self-contained original bytes and original filenames
  in project metadata. It does not provide the two models' licence texts, repository URLs or exact
  revisions. Someone with only this archive, offline and without the checkout, cannot recover
  those missing fields from the sampled headers. They can identify a vendor-like namespace and
  filename; that is weaker than reproducible provenance (U1 observation above; source table C1).
- **Embedding disabled:** Runtime skips generated bundled-model resolution and preserves local
  assets; it does not convert unembedded bundled IDs into immutable remote revision references.
  The archive is smaller, but bundled bytes/provenance still depend on another installed app/catalogue.
  Already document-owned models remain packaged; the option is not “strip every model”
  ([`pack_document`](../../web/crates/runtime/src/runtime.rs#L7227-L7273),
  [round-trip disabled/owned cases](../../web/crates/runtime/src/portable_archive.rs#L441-L503)).
- **PCB model payloads:** hashed paths lose the original display filename mapping unless available
  elsewhere in contained data/header. They do not remove notices already inside original model bytes.
  The loader does not separately copy adjacent vendor LICENSE files
  ([`load_model_files`](../../web/crates/runtime/src/pcb_handoff.rs#L220-L275)).
- **App notices:** inspected [web/assets/licenses/README.txt](../../web/assets/licenses/README.txt)
  describes CAD dependencies/fonts; [footprints NOTICE](../../footprints/NOTICE.md) and vendor licences
  live elsewhere. Build copies `web/assets` and provider/model assets, not the whole source notice tree
  ([`copy_runtime_assets`](../../scripts/build-web.py#L70-L91)). These inspected notices/build paths do
  not supply an output-contained offline lookup from U1's two hashes to pinned vendor records.
  This is a factual discoverability limit, **not a verdict on the app's complete legal compliance**.

### C4. Licence facts versus questions requiring qualified interpretation

Primary retained texts differ materially:

| Primary text | What it literally provides | What this research does not decide |
| --- | --- | --- |
| [MIT generator licence](../../footprints/licenses/MIT-ceoloide-ergogen-footprints.txt#L3-L13) | Copyright/permission notice and notice-retention condition for copies/substantial portions of Software. | Whether a particular generated footprint contains such a portion; do not automatically assign generator MIT to every model/output. |
| [Infused-Kim BY-NC-SA text](../../ergogen/library/vendor/infused-kim/LICENSE#L151-L184), [§3](../../ergogen/library/vendor/infused-kim/LICENSE#L242-L300) | NonCommercial grant; conditional retention of supplied attribution/copyright/licence/disclaimer/URI; modification indication; ShareAlike conditions for Shared Adapted Material. | Which model/source/output is within a licensor's rights, the README mismatch, whether a particular generated footprint is Adapted Material, or commercial-use legality. |
| [KiCad licence/exception](../../ergogen/library/vendor/kicad/LICENSE#L1-L19), [KiSwitch exception](../../ergogen/library/vendor/kiswitch/licenses/LICENSE) | Express waiver language concerning electronic designs/generated files; KiCad separately discusses redistribution as a library collection. | Whether each portable archive, inner model bundle or customized source is a design output versus library redistribution under the applicable exception. |
| [CERN-OHL-P §3/§4](../../catalogue/parts/sources/LICENSE#L76-L110) | Notice retention, modified-source date/description notice, and conditions on conveying source/products. | Classification of each exported footprint/package, what is Covered Source, or the sufficiency/location of notices for a user's distribution. |

These texts justify preserving source-specific evidence and asking narrow interpretation questions.
They do **not** justify declaring a violation, offering legal assurances, relicensing mixed assets,
requiring universal certification, or blocking every user export with unknown licence metadata.
The existing [onboarding requirement](../hardware/component-onboarding.md#onboarding-workflow) to record
source revision/licence is engineering policy; the complete distribution guarantee remains unchosen.

### C5. Minimal proposed scope and pending package evidence

**Possible minimum, for later agreement:** an output-reachable mapping from each used definition/model
to upstream repository/path/revision (or explicitly unknown), exact byte identity, supplied creator/
copyright/licence/exception notices, and a distinction between upstream material and authored changes.
Include an original display filename when an output path is hashed. Do not invent an author/revision
from a STEP exporter name or infer licensing from a vendor namespace. A change description is not a
generic source copy/manifests framework; the needed record and delivery location are still choices.

| Alternative | Trade-off |
| --- | --- |
| App/source-repository notices only | Least duplication; insufficient for the demonstrated offline recipient with only U1 or a PCB ZIP. |
| Per-project provenance plus output-used notice inventory | Keeps source/variant context through edits and copies only relevant notices to recipients; needs propagation and customization rules. Does not alone settle legal interpretation. |
| Output-only inventory generated from current bundled catalogue | Smaller saved schema change, but cannot faithfully recover old/custom/imported lineage once current catalogue entries have changed or provenance was already discarded. |
| Full bundled notice catalogue in every output | Easier coarse copying, larger/noisier outputs; can obscure which terms/versions apply and still does not record authored custom changes. |

**Pending smallest experiment:** with later authorization, use existing Runtime PCB handoff ports
and existing archive pack/unpack APIs to package a small board using the U1 MX+diode definitions,
one source-backed marbastlib definition, and the KiCad socket model. Inventory both ZIP levels;
compare original bytes/headers and all metadata fields against C1/C2. Expected current result:
model bytes/contained headers survive, hashed paths appear, no separate notice inventory appears,
and project-only mechanical source/provenance fields are not automatically emitted into the PCB
package. Add a user-edited footprint source property and a locally supplied Asset licence/source
to distinguish source-span preservation from synthesized metadata. Do not modify fixtures just
to make this report's expectation true.

The best existing seams are [`prepare_archive`](../../web/crates/runtime/src/portable_archive.rs#L10-L96)
and its inspected `pack_round_trip_keeps_local_bytes_and_optionally_adds_opaque_model_bytes`,
[`build_handoff`](../../web/crates/runtime/src/pcb_handoff.rs#L65-L218),
[`catalogue_digest_matches_the_exact_staged_vendor_source_file`](../../web/crates/runtime/src/bundled_models.rs#L266-L280),
and the two source-preservation integration tests cited above. The archive round-trip uses opaque,
even empty, payloads in one case; it is not attribution acceptance. No new browser/ZIP generation
or test was performed here.

**Documentation disposition:** an [0020](../adr/0020-validate-portable-project-packages-in-core.md)
amendment plus onboarding/output reference is enough if the owner merely extends portable-record
requirements. A separate ADR becomes useful if they choose a durable distribution/variant ownership
boundary across both project and PCB handoff outputs. Keep integrity, provenance, original-byte
preservation, authored changes and legal obligations distinct; do not create one ADR per licence/vendor.

## Findings classification: debt is not an unmade policy

| Finding | Classification / action boundary |
| --- | --- |
| Snapshot/default/current-body distinction, including two real historical changes | Compatibility choice still unmade; existing side migration and diode opt-in are evidence of techniques, not universal acceptance criteria. |
| Unpaired structural versus generic removal map retention | Source-established behavioral divergence; desired disable/shrink/delete/recreate semantics still unmade. Not automatically a regression against a universal cleanup promise that does not exist. |
| Exact-key remapping and synthetic `/push` identities | Concrete bounded verification gap; qualify by actual input scan mode and operation before labeling a user-visible bug. |
| Rich imported-row provenance omitted by runtime decoding; bundled embedding gets generic metadata | Concrete propagation gaps relative to reproducible/offline source discovery. Onboarding already asks for evidence, but no complete per-output provenance policy or legal verdict is accepted here. |
| Per-half mapping, exclusive removed-design/accessory deletion, role/side decoupling, incomplete save/output readiness and explicit reversibility conversion | Accepted 0024–0028 delivery debt where absent; not grounds to retain unwanted PCB designs, merge independent routing or invent arbitrary firmware overrides. |
| Optional unsupported peripheral blocks whole otherwise-valid firmware handoff | Known [0030 amendment](../adr/0030-export-firmware-as-an-editable-repository-template.md#amendment-unfinished-optional-peripheral-support-2026-10-20) delivery debt, not a new universal support gate. |
| Handoff protection after package/current ownership/saved commit and across Undo | Settled [0031](../adr/0031-preserve-pcb-handoff-protection-across-undo.md), not a decision reopened by compatibility/provenance research. |

## Ranked questions for the next domain / to-spec conversation

Facts are answered above; these ask for **expected user outcomes**, not code archaeology.

1. **Generator correction on an existing design:** when an app update changes resolved footprint
   geometry but preserves saved parameter types and pad identities, should the next manufacturing
   handoff use the correction immediately, require reviewed adoption, or preserve the previous
   result until explicitly upgraded? Does the answer differ for known-dangerous geometry, changed
   defaults and changed model datum/bytes? The diode opt-in and side migration are real contrasting
   precedents, not a reason to select a historical-executable framework.
2. **Absent input returning without Undo:** should disable/enable restore prior bindings and
   appearance, while shrink/save/reopen/regrow or delete/recreate starts fresh—or should either
   offer recovered values for review? Agree separately on survivor compaction, temporary absence,
   replacement, qualified push/rotation identities and true Undo. Current routes differ.
3. **Recipient with only the output, offline:** must they be able to discover exact upstream
   identity/revision, supplied notices and authored modifications for every included used asset,
   or only a declared subset? Decide the project/PCB handoff boundary and unknown-metadata
   disclosure, not whether all exports should be prohibited. Get qualified interpretation of
   the sampled licence/exception and README mismatch before claiming legal sufficiency.

**Conditional later spec/user-story coverage, not publication:**

- If compatibility adoption is chosen: “As a designer reopening a saved project after an update,
  I can distinguish retained data from updated resolved manufacturing geometry and deliberately
  accept changes where required.” Cover omitted/explicit/null values, current/unsupported/missing
  generators, imported source, pad/net mapping, models, Undo/reopen and existing handoff protection.
- If input lifecycle is chosen: “As a designer changing a layout, I can predict whether a surviving,
  temporarily absent or replacement input keeps its bindings.” Cover B3's compact matrix through
  public Core, serialization/fresh Open, typed/legacy/sensor/push data, one mirrored route and
  firmware ordering; do not add automatic semantic migration or a hardware-revision schema by implication.
- If portable provenance is chosen: “As a collaborator receiving only a portable project or PCB
  handoff, I can identify included source materials and deliberate custom changes without this
  checkout.” Cover mixed licences, headers present/absent, source-backed customization, local
  Asset metadata, embedded versus unembedded models and both nested ZIP levels; do not promise
  legal clearance or universal fabrication qualification.

## Verification and explicit limits

- **Executed:** read-only Git inspection; local JSON/ZIP/header inspection; SHA-256 and byte
  equality of the two existing U1 asset payloads against their records/current source; U1 bytes
  compared with HEAD. No application behavior was executed. No source/hash manifest was written.
- **Inspected, not executed:** all named generator, migration, matrix/keymap, queued Runtime,
  portable-archive and KiCad test bodies. Their names/paths were checked; assertions establish
  coverage intent, not a current passing result. In particular, reading a test with `kicad-cli`
  does not constitute running KiCad, and an ignored frozen-baseline test was not silently enabled.
- **Document checks:** local file/heading/line-range links and named test declarations were checked;
  five cited local commit objects resolved, HEAD remained unchanged, and report whitespace was clean.
  These checks are document integrity evidence, not application verification.
- **Historical Git evidence verified:** HEAD; diode parent/change objects
  `4607c26667bbe73f11914f1495e55c6e98ebcdc3` and `d246d4d98f27c30dab8fec4f17dd24f657ee38ad`;
  old provider `851875a0e2e42c7651ee02e6cbe997c291f3a6ad`; Rust cutover
  `d0c795766c38c2f8944fd19a0bbb9a4aced83193`. Retrieve exact versions with
  `git show <commit>:<path>` as named in A2/A3. Vendor revision strings are inspected manifest
  evidence, not a claim that those upstream commits are objects in this local repository.
- **Pending:** source-bound two-build exports, full unpaired binding lifecycle observation,
  native-pair/shared-board lifecycle cross-product, a generated mixed-source PCB ZIP, and
  qualified interpretation of applicable distribution terms. No complete lifecycle, licensing,
  security, hardware, CAD or application audit is claimed.
- No builds, tests, browser automation, dependency changes, commits, subagents, worktrees,
  source copies, fixtures, scripts or generated persistent artifacts. No ADR/glossary/index/
  ticket/spec/claim was edited. Only this new report was intentionally changed; concurrent
  modifications were preserved.
