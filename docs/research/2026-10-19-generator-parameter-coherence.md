# Generator parameter coherence through export

Date: 2026-10-19. Candidate 2 research; no implementation or ticket publication.

## Source state and method

HEAD at inspection: `e326d17778524bd1704c76eb71e2c5bb7b773f10`, dated 2026-10-19 18:21:09 -0400. Evidence below refers to **current on-disk source**, not an assumption that HEAD is clean. In particular, generator export tests, footprint generator bodies, the footprint form builder, Core preview and model modules already have user edits; the glossary is untracked. These were not changed. Source links use repository-relative paths and inspected line ranges; dirty-file lines describe the inspected working tree and are not immutable commit permalinks.

Method: CodeGraph current-source exploration, focused test inventory, architecture/glossary and tracker reading, Git status and path-scoped history. No external library documentation was needed. No Cargo, browser or full checks were run. Conclusions distinguish observed code from inferred runtime effects.

## Recommendation

**Proposed decision:** narrow this candidate to making the Parts generator editor honor the existing `null`-means-absent rule for displayed defaults and candidate preparation. Keep the registry's declared kinds authoritative, Core's saved-document policy in Core, and export's electrical policy in export. Do not add a shared policy wrapper, replacement renderer, new adapter or configurable validation framework.

**Evidence:** there is already one declaration-derived acceptance predicate, used by registry declaration validation, saved-map checks and render; Core calls that check and export calls that renderer. The editor alone repeats saved/default selection in three places without filtering `null`. See [`ParamKind::accepts`](../../footprints/src/params.rs#L28-L35), [`Registry::check_parameters` and `render`](../../footprints/src/registry.rs#L124-L215), [`parameters`](../../web/crates/parts/src/parts/generator_settings.rs#L187-L219), [`apply_input`](../../web/crates/parts/src/parts/generator_settings.rs#L286-L359), and [`GeneratorSettingsEditor` initial parameters](../../web/crates/parts/src/parts/generator_settings.rs#L927-L938).

**Inference:** a definition with `hole_size: null` renders using the declared hole-size default and passes Core's saved-map check, but the editor presents a blank number and candidate preparation rejects it as not a finite number. A boolean null similarly appears unchecked even when its declaration defaults to true. This is a source-level counterexample, not a reproduced browser failure. The recommended change should begin with a failing regression proving it.

## Architectural constraints and vocabulary

- **Evidence:** generator parameters are typed saved values; omitted values use generator defaults; Core refuses new undeclared/wrong-type edits while preserving repairable legacy saves. Use this vocabulary from the current [glossary](../../CONTEXT.md#L85-L96).
- **Evidence:** [ADR-0003](../adr/0003-rust-footprint-generators-in-core.md#L5-L20) requires Rust generation inside Core and explicitly rejects reproducing JavaScript coercion quirks. [ADR-0009](../adr/0009-generator-parameters-checked-in-core.md#L14-L39) owns saved-map checking, null absence and different Open/Edit treatment. Its [form-coercion consequence](../adr/0009-generator-parameters-checked-in-core.md#L63-L65) deliberately leaves text parsing in presentation.
- **Evidence:** pending edits resolve against the accepted document, and previews do not commit; retain the existing owner/rebase behavior rather than broadening this task into edit settlement. See [ADR-0005](../adr/0005-resolve-queued-edits-at-execution.md#L12-L20) and [`generator_replacement`](../../web/crates/parts/src/parts/generator_settings.rs#L584-L647).
- **Proposed decision:** gain locality by removing repeated, inconsistent saved/default selection within the editor module. This is an internal consolidation at an existing interface, not a new cross-crate seam. The existing generator module already has depth: declarations support catalogue, normalization, checking, drawing and export through small interfaces.

## Evidence: declarations, defaults and types

1. **Evidence:** the supported kinds are exactly number, boolean, string, net and array. Kinds are explicit rather than inferred from defaults; number acceptance is JSON-number acceptance, not form-text parsing. See [`ParamKind`](../../footprints/src/params.rs#L6-L35).
2. **Evidence:** defaults belong to each declaration; no-default is permitted for nets, arrays use JSON defaults, and `side` is a string defaulting to `F`. Registry construction rejects duplicate names, invalid defaults and incorrectly typed matrix/keycap metadata. See [`DefaultValue` and `ParamSpec`](../../footprints/src/params.rs#L38-L115) and [`validate`](../../footprints/src/registry.rs#L263-L309).
3. **Evidence:** registry schema entries contain a typed kind and optional default. Core currently exposes a JSON map keyed by parameter name, serializing each registry entry and removing its redundant name. The page obtains this schema through the catalogue module. See [`ParameterSchema`](../../footprints/src/params.rs#L132-L140), [`parameter_schema`](../../core/src/generators.rs#L38-L50), and [`generator_parameter_schema`](../../web/crates/catalogue/src/catalogue.rs#L239-L246).
4. **Inference:** stringly typed schema consumption permits stale branches, but is not proof that another transport type or cross-crate interface migration is needed. The editor still contains `object`/`anchor` parsing and controls, although neither is a declared kind; see [`apply_input`](../../web/crates/parts/src/parts/generator_settings.rs#L327-L353) and [`GeneratorParameterField`](../../web/crates/parts/src/parts/generator_settings.rs#L1425-L1449). Their removal is optional follow-up cleanup, not required for the evidenced null fix.

## Evidence: saved values and render precedence

1. **Evidence:** saved-map checks skip null **before** name lookup and use the same kind predicate as render. Undeclared non-null names are reported even though render ignores them. See [`Registry::check_parameters`](../../footprints/src/registry.rs#L124-L154).
2. **Evidence:** rendering merges definition parameters first, then part parameters. Only after merging does it filter null and use the declaration default. Therefore a part-level null masks a definition's non-null value and chooses the declaration default; it does not inherit that definition value. Declared nets resolve eagerly in declaration order, and an empty/missing net gets index zero. See [`Registry::render`](../../footprints/src/registry.rs#L157-L215).
3. **Evidence:** Core checks definition, part and assembly-member maps where saved, not an effective merged map. A bad definition value remains a problem even if a placed part overrides it. See [`ParameterOwner` and `parameter_problems`](../../core/src/generators.rs#L101-L187).
4. **Evidence:** Open/ReplaceDocument refusal, introduced-value checking, existing legacy-copy allowances, owner-targeted findings and generator-switch cleanup are separate Core document policies built on registry problems. See [`document_error` through `definition_error`](../../core/src/generators.rs#L249-L344), [`parameter_findings`](../../core/src/generators.rs#L346-L389), and [ADR-0009](../adr/0009-generator-parameters-checked-in-core.md#L19-L39).
5. **Proposed decision:** preserve all these distinctions. Do not make render reject unknown names merely to match new-edit refusal; doing so would break the documented legacy-open contract.

## Evidence: actual callers through export

- **Evidence:** drawing and model discovery use Core `render_forms`, which converts document types then calls `bundled().render` with `NoNets`. Normalization delegates to the same footprint library and copies generated geometry/terminals back. See [`normalize_definition` and `render_forms`](../../core/src/generators.rs#L62-L99), [`model_asset_ids` and `model_bindings`](../../core/src/generators.rs#L392-L409).
- **Evidence:** candidate preparation parses form input, normalizes the definition, then requires a real generator drawing; it does not contain a second renderer. See [`prepare_generator_candidate`](../../web/crates/parts/src/parts/generator_settings.rs#L362-L376).
- **Evidence:** board planning builds generator jobs after electrical terminal reconciliation; standalone planning constructs a synthetic placed part. See [`prepare`](../../core/src/artifact/kicad/planning.rs#L53-L137), [`terminal_parameters`](../../core/src/artifact/kicad/planning.rs#L157-L242), and [`standalone_part`](../../core/src/artifact/kicad/planning.rs#L245-L267).
- **Evidence:** preview and export both execute `render_jobs`, which calls the same bundled renderer, then performs model-path rewriting, output splitting and legacy arc upgrades. Their difference starts in preparation/finalization, not parameter-kind acceptance. See [`render_jobs`, `preview`, `export`](../../core/src/artifact/kicad/generation.rs#L21-L74).
- **Inference:** export conversion is an adapter between serialized document shapes, not evidence of another coercion policy. Replacing it with a generic parameter-policy module would add a shallow interface without deleting the actual electrical, model or output work.

## Duplicated rules versus legitimate differences

| Concern | Finding | Decision |
|---|---|---|
| Declared kind acceptance | Already shared by validation and render | Keep existing predicate; no policy wrapper |
| Definition/part merge and stable net allocation | Renderer owns effective values and net order | Do not move into UI |
| Saved-map names and legacy compatibility | Core checks storage ownership and introduced changes | Preserve ADR-0009, not a render duplicate |
| Form parsing | Number/list text must become typed JSON; finite-number feedback is user-facing | Keep in editor; do not teach render string coercion |
| Saved/default lookup | Repeated in field projection, initial preview seeding and candidate conversion; all retain null | Consolidate locally and align null fallback |
| Terminal-net reconciliation | Rejects ambiguity/conflicts and maps document wiring | Legitimate export policy, not shared default lookup |
| Labels, groups, upload controls and side selector | Presentation choices | Keep local |

Sources: [`Registry::render`](../../footprints/src/registry.rs#L157-L249), [`parameter_problems`](../../core/src/generators.rs#L120-L187), [`already_held`/`introduced_error`](../../core/src/generators.rs#L261-L310), [`apply_input`](../../web/crates/parts/src/parts/generator_settings.rs#L286-L359), [`parameters`](../../web/crates/parts/src/parts/generator_settings.rs#L187-L219), [`initial_parameters`](../../web/crates/parts/src/parts/generator_settings.rs#L927-L938), [`terminal_parameters`](../../core/src/artifact/kicad/planning.rs#L157-L242), [`GeneratorParameterField`](../../web/crates/parts/src/parts/generator_settings.rs#L1348-L1477).

## Smallest complete scope and deletion test

**Proposed decision:** update only the editor's saved/default interpretation and its regression coverage. Displayed entries and the initial transient preview must use the same effective values; candidate preparation must not reject an absent saved value. Null selects the declaration default, or remains absent for a net without a default. A null draft value, if supplied to the existing candidate interface, also means declared default rather than falling through to an older saved non-null value. Empty text is not null and remains subject to current kind-specific parsing.

Do not silently erase unknown saved names or rewrite accepted documents when opening the editor. Preserve existing changed-parameter tracking and rebase behavior: merely displaying a default is not a user field edit. See [`request_preview` and `change_parameter`](../../web/crates/parts/src/parts/generator_settings.rs#L927-L1019) and [`generator_replacement`](../../web/crates/parts/src/parts/generator_settings.rs#L584-L647).

**Deletion test:** deleting registry validation/check/render would redistribute real declaration, merge, version and net-allocation complexity across callers, so that module earns its keep. Deleting Core's document policy would lose legacy compatibility and located findings, so keep it. A new policy wrapper would mostly forward to those existing modules: reject it. A small editor-local effective-value helper earns its keep only if it actually replaces the repeated selections rather than layering onto them; no line-savings estimate is asserted. Source for the removed repetition: [`parameters`](../../web/crates/parts/src/parts/generator_settings.rs#L194-L203), [`apply_input`](../../web/crates/parts/src/parts/generator_settings.rs#L295-L305), [`initial_parameters`](../../web/crates/parts/src/parts/generator_settings.rs#L927-L938).

## Existing tests and proposed seams

**Primary proposed seam requiring user confirmation:** the existing Parts generator settings interface, exercised by mounted editor tests using the existing candidate-provider hook and the production candidate preparation path. This is the highest existing seam that observes both the shown field and the candidate preview without inventing a new adapter. Existing controlled-provider tests must not become the only proof: retain at least one production normalization/drawing assertion.

**Necessary additional existing seam:** Core's public generator/KiCad rendering interfaces for a focused parity assertion about definition defaults, part-null masking and exported output. A mounted definition editor cannot exercise placed-part override/export semantics; do not add a new seam just to unify the harnesses. Existing Core saved-document tests remain compatibility regression coverage, not a new policy implementation.

Prior art:

- [`mounted_generator_initial_schema_default_seeds_unapplied_preview`](../../web/crates/parts/src/parts/generator_settings.rs#L2422-L2477): controlled initial candidate request, real normalization, geometry assertions and proof the accepted document is unchanged.
- [`submitted_numbers_and_lists_pass_the_core_check`](../../web/crates/parts/src/parts/generator_settings.rs#L2599-L2620) and [`text_that_does_not_parse_is_refused_before_it_reaches_core`](../../web/crates/parts/src/parts/generator_settings.rs#L2622-L2638): form conversion checked against Core.
- [`every_bundled_default_passes_its_own_check`](../../footprints/tests/parameter_check.rs#L14-L32) and [`an_undeclared_name_is_reported_and_null_is_absent`](../../footprints/tests/parameter_check.rs#L46-L56): existing declaration/null acceptance coverage.
- [`an_edit_that_writes_text_for_a_number_is_refused_and_applies_nothing`](../../core/tests/generator_parameters.rs#L168-L185), [`a_preview_that_writes_text_for_a_number_is_refused_the_same_way`](../../core/tests/generator_parameters.rs#L216-L226), and [`an_unrelated_edit_is_not_refused_for_a_legacy_name_it_does_not_write`](../../core/tests/generator_parameters.rs#L228-L247): Core ownership/legacy behavior.
- [`exports_front_and_back_switches_with_instance_nets_and_models`](../../core/tests/generator_export.rs#L150-L184) and [`exports_document_pad_net_assignments_for_terminal_groups`](../../core/tests/generator_export.rs#L186-L198): highest export prior art, including KiCad parsing. This file is already dirty; extend current work, never restore it from HEAD.

Meaningful scenarios for implementation verification:

1. Missing and saved-null number/boolean/string/list parameters show the declaration default and produce a successful candidate with equivalent observable drawing; include a true boolean default so unchecked fallback cannot pass accidentally.
2. A no-default net remains absent/no-net; null is not converted to literal text `null`, an empty numeric field, or an invalid boolean.
3. Editor mount and initial preview leave the accepted document and revision unchanged; Apply after a real edit preserves unrelated accepted metadata and existing owner/rebase guarantees.
4. A part's non-null override wins; its null override masks a non-null definition value and selects the declared default in real rendering/export. Assert parsed output properties rather than private map construction or generated IDs.
5. Numeric text is still parsed to a finite number, list JSON to an array; blank/malformed/non-finite numeric text and malformed/wrong-shape list JSON still fail visibly and cannot enable Apply.
6. Unknown saved names remain preserved and Core continues to warn/refuse according to ADR-0009. No renderer-wide unknown-name rejection is introduced.

**Proposed deterministic verification:** reuse existing mounted request completion and DOM wait helpers; do not add arbitrary sleeps or source-equality tests. Compare field values, ready/failed status, geometry/net output and accepted state. Run the repository-supported focused Parts browser and native generator/export checks during implementation; select exact commands from current README/check scheduler rather than launching competing Cargo jobs. No such checks were executed during this doc-only research.

## History and invalidated assumptions

- **Evidence:** path-scoped Git history identifies `34f6570f0` (2026-10-09, “Resolve generator-parameters ticket 04 and record its verification”) as the ADR-0009 resolution; its current [decision](../adr/0009-generator-parameters-checked-in-core.md#L14-L39) already establishes the shared predicate. Do not reopen that finished validation design as though Core has no declared-name/type check.
- **Evidence:** history includes `f57e3a4e1` (2026-10-19, “Merge branch 'footprint-engine' into dev”) and `2fe19cf42` (2026-10-19, “Let generators build KiCad forms directly; port mounting_hole_npth”). The current registry supports both text and direct forms behind one renderer; see [`Body`](../../footprints/src/registry.rs#L16-L22) and [`render` dispatch](../../footprints/src/registry.rs#L244-L249). Do not restore a separate browser worker or assume all generator bodies emit text.
- **Inference:** the initial report correctly identified a path worth investigating, but overstates independent validation/render/export drift. What is evidenced now is null-default presentation drift plus stale UI kind branches, not a need for a four-module policy layer.

## Risks, unknowns and coordination

- **Unknown:** runtime reproduction and exact number of affected user saves. Static source establishes the disagreement; a regression must establish the observable failure before editing production code.
- **Risk:** filtering null too early in definition/part merging changes precedence; leave renderer semantics alone. Filtering empty strings as though absent would also change valid text/net behavior and invalid numeric-draft feedback. See [`render` merge](../../footprints/src/registry.rs#L176-L200) and [`apply_input`](../../web/crates/parts/src/parts/generator_settings.rs#L306-L355).
- **Risk:** seeding defaults can look like an edit if changed-parameter tracking is altered. Preserve current preview-only behavior and rebasing; see [`initial preview`](../../web/crates/parts/src/parts/generator_settings.rs#L1007-L1019) and [`generator_replacement`](../../web/crates/parts/src/parts/generator_settings.rs#L610-L647).
- **Ownership:** this candidate owns editor effective-value interpretation and its tests, plus only narrowly necessary generator/export parity tests. It does not own preview geometry normalization, builder escaping, generator bodies, terminal/net export policy or model-path rewriting. Coordinate if sibling candidates modify generator normalization/render interfaces or the already-dirty generator export test file. No dependency on a sibling refactor is required.
- **Publication follow-up:** on 2026-10-19 the user approved the narrower scope and both existing test seams. Published as [ticket 183](../plans/typed-core-edits/issues/183-align-generator-editor-null-defaults.md), `ready-for-agent`. Earlier proposal language records the research stage, not an outstanding approval blocker.

## Checks actually run

Static source/doc inspection, focused symbol/test inventory, `git status --short`, HEAD timestamp/hash lookup, and path-scoped `git log`. No build, Cargo test, browser run, dependency installation, staging or commit. Only this research note and the assigned temporary spec draft were authored for this candidate.
