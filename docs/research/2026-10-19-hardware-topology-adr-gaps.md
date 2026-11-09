# Hardware topology: ADR gaps

Research date: 2026-10-19. Examined HEAD: `9099b3ef7a72b0919b1572f116490ed57514d43a`.
This is a working-tree investigation, **not an atomic snapshot or acceptance evidence**.
The checkout already contained extensive concurrent changes, including untracked
GLOSSARY and ADRs 0013–0023; those texts are present policy, not committed provenance
at that HEAD. Only this report was intentionally written.

## Executive answer

**Subsequent decisions (2026-10-20):** [ADR-0024](../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md)
records shared routing with assembly-specific firmware/jumper mappings.
[ADR-0025](../adr/0025-delete-a-removed-halfs-exclusive-pcb-design.md) replaces the
retention proposal with deletion of a removed half's exclusive PCB design and
exclusive accessories in one undoable edit; shared and surviving data remain.
Findings below describe the inspected earlier implementation, not delivery of these policies.

[ADR-0026](../adr/0026-decouple-firmware-role-from-physical-side.md) subsequently
allows either physical half to be central where hardware/transport support it;
the left-central/right-peripheral restriction below is implementation debt, not policy.

[ADR-0027](../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md)
allows unfinished split designs to save and remain editable, with readiness gates
only for outputs that require the missing hardware. This is policy, not full
implementation or acceptance evidence for every incomplete configuration.

[ADR-0028](../adr/0028-convert-reversibility-explicitly-without-merging-designs.md)
settles the reversibility-control question: explicit reviewed conversion, prior user
deletion of an unwanted distinct half before sharing a design, and a new unrouted
mirrored design when separating a reversible pair. The current toggle behavior below
does not establish that conversion is implemented.

**Do not write another “boards versus halves” ADR.** A PCB design can serve two
physical instances, or a split can have distinct designs. ADR-0010 already chooses
both, their different identity/copy rules, reflection, exceptions and atomic
mirrored setup. ADR-0008 covers mechanical linkage; ADR-0023 now covers mounted
versus embedded circuitry. The prior inventory's G12 ownership gap is therefore
closed by the newly confirmed record, not still waiting for research
([0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md),
[0008](../adr/0008-linked-halves-stay-linked.md),
[0023](../adr/0023-distinguish-mounted-daughterboards-from-embedded-circuits.md),
[prior inventory](2026-10-19-adr-gap-inventory.md)).

The strongest genuinely uncovered question is **whether board-scoped electrical
authority plus instance-scoped population/firmware targets is a durable product
decision**, especially when two halves share part identities. A second, narrower
gap is the policy for **topology transitions that discard physical instances while
retaining PCB designs and electrical data**. Current behavior is recoverable;
original selection rationale is incomplete. Neither warrants inventing an accepted
ADR yet. Explain the model in architecture/glossary first; confirm these policies
before recording a new decision (implementation evidence below).

## Domain and identity ownership

| Term | Exact current meaning and owner | Evidence |
| --- | --- | --- |
| Board / `boardId` | A PCB **design**, not a physical half. `ProjectDoc.boards` contains membership in parts, nets and outlines, thickness and routed copper. Parts/nets live in project collections; a board references their IDs. | [Glossary](../../GLOSSARY.md#mechanical-relationships); [model](../../core/src/model.rs), `Board`, `ProjectDoc`; [validation](../../core/src/validate.rs), `validate` |
| Physical instance / `instanceId` | One realization of a Board: durable ID, board reference, name, `half`, `role`, `flipped`, optional controller reference and mechanical configuration/ownership. Identity does not collapse when the Board is reused. | [model](../../core/src/model.rs#L1045-L1067), `PhysicalBoardInstance`; [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) |
| Topology | Project `hardware.topology`: `unibody` or `split`. This describes intended physical/electrical organization; the enum alone is not an exactly-one/exactly-two admission invariant. A source-half fixture legitimately starts `split` with only one instance; firmware separately requires two appropriately labelled members. | [model](../../core/src/model.rs#L1014-L1043), `HardwareConfiguration`; [synthetic fixture builder](../../core/tests/support/synthetic_half.rs#L31-L43); [Runtime firmware](../../web/crates/runtime/src/runtime.rs#L6613-L6628) |
| Transport | Project `hardware.transport`: `none`, `wireless`, `wired`. Setup selects wireless on entering split and none on entering unibody. Transport-only setup accepts wired/wireless only on split; it changes no wiring itself. Firmware maps wired to UART and consumes separate resolved plans. | [setup](../../web/crates/catalogue/src/physical_setup.rs#L345-L351), `propose_topology`, `propose_transport`; [firmware adapter](../../web/crates/runtime/src/firmware_request_adapter.rs#L332-L409), `build_request` |
| Half | A persisted **string**, used by setup for left/right/unibody labels, not a board identity or geometric transform. It is operational too: firmware currently requires central=left and peripheral=right, and reversible setup sets flips using `half == "right"`. | [instance model](../../core/src/model.rs#L1045-L1067); [setup](../../web/crates/catalogue/src/physical_setup.rs#L472-L506), `propose_reversible`; [Runtime](../../web/crates/runtime/src/runtime.rs#L6617-L6628) |
| Role | Another persisted string: setup uses central/peripheral/standalone; firmware selects members by role. It is not the module interface's host/module role, nor inherently a side. Current firmware nevertheless couples it to side. | [setup](../../web/crates/catalogue/src/physical_setup.rs#L359-L419); [Runtime](../../web/crates/runtime/src/runtime.rs#L6617-L6668); [module interface](../../core/src/modules.rs#L73-L90), `ModuleInterface` |
| Electrical board configuration | `hardware.boards`, keyed by Board ID: controller, matrix/direct mode, locks, assignments, firmware-position `key_bindings`, jumper states, protected handoff. It is **not one independent configuration per physical instance**. The instance has its own optional controller reference; Runtime forwards that reference when resolving instance-specific firmware/handoff plans. | [model](../../core/src/model.rs#L1103-L1114), `ElectricalBoardConfiguration`; [planner](../../core/src/electrical.rs#L850-L909), `resolve`; [Runtime](../../web/crates/runtime/src/runtime.rs#L5799-L5835), `resolve_pcb_handoff_plan` |
| Electrical plan | Derived board/revision/controller/optional-instance result with assignments, nets, jumpers, diagnostics and fingerprint; not a second persisted PCB design. Explicit request controller wins over board configuration; otherwise the single eligible placed controller can be selected. | [plan](../../core/src/electrical.rs#L53-L75), `ElectricalPlan`; [resolve](../../core/src/electrical.rs#L791-L909) |
| Mechanical mirror pair | Explicit pair of **instance IDs**, mapping, shared construction and linked features. Optional native `halves` extends it with source/mirror Board IDs, correspondences and exceptions. A mechanical-only link is not necessarily native whole-board reflection. | [model](../../core/src/model.rs), `MechanicalMirrorPair`, `MirroredHalves`; [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md); [link review](../../web/crates/catalogue/src/physical_setup.rs#L614-L752) |
| Layout mirror link | Separate relationship between layouts/matrices, including an X axis; it does not itself create physical instances or a mechanical pair. Native/legacy overlap is explicitly refused on affected edits rather than treated as two interchangeable authorities. | [setup](../../web/crates/catalogue/src/physical_setup.rs), `a_layout_only_pair_proposes_no_mechanical_pair_and_invents_no_instances`; [inverse layout](../../core/src/mirrored_half/inverse/layout.rs#L1-L62), `translate` |
| Effective physical view | Derived document/parts/contours for one selected instance. `physical_instance_id` is also used as a projection/context carrier; it does not create ownership of a second parts collection. Mechanical resolution uses Local, Pair or Legacy construction ownership, then linked features and defaults. | [inputs](../../core/src/mechanical/inputs.rs#L99-L164), `effective_inputs_inner`; [projection](../../core/src/mechanical/inputs.rs#L1023-L1091), `effective_document`; [configuration resolution](../../core/src/mechanical/inputs.rs#L744-L825) |

### Three transforms that must not be collapsed

1. **Same physical PCB flipped:** `reuse_board` requires an unflipped source and
   zero pair translation/rotation. The physical reflection is `(-x, y)`; it creates
   an instance and pair, not copied parts/nets. Pinned source-only modules refuse
   by name; board-owned embedded circuits need no copy
   ([`reuse_board`](../../core/src/mirrored_half.rs#L848-L921)).
2. **Different mirrored PCB design:** `new_board` creates fresh board, matrix,
   part, net and hosted-item identities with correspondences. A placed part retains
   its authored side/parameters; heading/companion rules are not a physical PCB
   flip. Copper is not reflected. These are already explicit choices, including
   exceptions for switches and stabilizers
   ([0010 amendments](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md),
   [`new_board`](../../core/src/mirrored_half.rs#L1362-L1501)).
3. **Mechanical mapping between existing instances:** review can supply a translated/
   rotated mapping, construction source and durable feature correspondences. Default
   mapping derives only from opposite flip flags, not names, part coordinates or a
   layout axis. This review/proposal is not proof that arbitrary existing boards
   have been converted into a native whole-board reflected pair
   ([`derived_mapping`, `mirror_pair_link`](../../web/crates/catalogue/src/physical_setup.rs#L516-L752)).

## Scenarios: delivered behavior versus proposal or unknown

| Scenario | Supported behavior and limits |
| --- | --- |
| One reversible board manufactured twice | Two instance IDs reference one Board ID; no duplicated board circuitry. Physical part/contour projection reflects the flipped instance. Existing regression `reuse_board_projects_authored_features_without_copying_board_data` checks the creation contract. This is not a universal qualification of reversible hardware or pin population: onboarding demands front/back connectivity evidence. ([Core test](../../core/tests/mirrored_half.rs), [inputs](../../core/src/mechanical/inputs.rs#L99-L164), [onboarding](../hardware/component-onboarding.md#onboarding-workflow)) |
| Distinct left/right PCBs | Native new-board creation remaps IDs, controller references and electrical function keys; keeps matrix numbering/pins; mirror `key_bindings` starts empty and protected handoff is not copied. Keycap legends/settings do copy. Existing Board data is not a rename-in-place topology conversion. ([`new_board`](../../core/src/mirrored_half.rs#L1784-L1859), [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)) |
| Unibody | Setup retains the selected existing instance, or a board-matching fallback, relabels it unibody/standalone and sets transport none. It replaces the **instances list**, not the Board collection. A unibody can also contain two linked layout halves without becoming a two-instance split. ([`propose_topology`](../../web/crates/catalogue/src/physical_setup.rs#L254-L449), [layout-only test](../../web/crates/catalogue/src/physical_setup.rs), `a_layout_only_pair_proposes_no_mechanical_pair_and_invents_no_instances`) |
| Enter split from ordinary unflipped unibody | Proposal requests a same-board flipped secondary and a native pair, resolves shared construction, captures explicit labels; the adapter converts it to one Core `CreateMirroredHalf` with `physical_setup`. Core applies setup plus pair on a draft, and invalid transport refuses without mutation. One Undo restores the input. ([adapter](../../web/crates/pcb/src/pcb_physical_setup/controller.rs#L614-L675), [Core](../../core/src/mirrored_half.rs#L133-L205), [test](../../core/tests/mirrored_half.rs#L153-L198), `physical_setup_and_pair_creation_are_one_atomic_core_edit`) |
| Enter split from an already-flipped primary | The proposal deliberately retains that flip; its new secondary is also flipped, so no default eligible reflection map exists. It may therefore follow the non-pair replacement path. A split label is not evidence a linked pair was created. This is explicitly tested **as proposal behavior**, not comprehensive firmware acceptance. ([setup test](../../web/crates/catalogue/src/physical_setup.rs#L983-L1043), `topology_preserves_flipped_primary_and_shared_construction_and_adds_reversible_secondary`; [dispatch](../../web/crates/pcb/src/pcb_physical_setup/controller.rs#L648-L675)) |
| Link independent existing halves | Catalogue review carries explicit mapping/source/matches and leaves the input unchanged. Core's mechanical `LinkMirrorPair` is a distinct operation; do not send arbitrary existing-board links through `into_mirrored_operation`, which constructs a fresh **ReuseBoard** request. The catalogue's distinct-board test verifies proposal contents, not that creation adapter accepting the proposal as a new half. ([review test](../../web/crates/catalogue/src/physical_setup.rs#L1521-L1603), [conversion](../../web/crates/catalogue/src/physical_setup.rs#L79-L143), [mechanical edit](../../core/src/mechanical/edit.rs), `link_mirror_pair`) |
| Switch wired/wireless | Transport-only proposal changes only that field; it neither places connectors nor rewires copper. Firmware subsequently needs valid plans; wired adapter uses UART and reverses peripheral TX/RX. Do not describe changing the dropdown as electrical certification. ([setup test](../../web/crates/catalogue/src/physical_setup.rs#L1248-L1272), `transport_changes_only_the_transport_field`; [adapter](../../web/crates/runtime/src/firmware_request_adapter.rs#L332-L409); [test](../../web/tests/firmware_request_adapter.rs), `split_wired_request_preserves_peripheral_offset_and_reverses_uart`) |
| Split back to unibody / remove member | Removed instance does not imply removed Board. Core reconciles dangling pair ownership and materializes the survivor's previously effective mechanical configuration locally. Typed paired-member removal also clears a selected removed-instance carrier. It is not a general PCB/net/keymap merge algorithm. ([reconciliation](../../core/src/mechanical/edit.rs#L1302-L1353), `remove_physical_instance`; [`reconcile_physical_topology`](../../core/src/mechanical/edit.rs#L1484-L1598); [integration tests](../../core/tests/mechanical_integration.rs#L1210-L1359)) |
| Mounted daughterboard versus embedded copy | Mounted source circuitry stays separate; an optional host-instance pin controls which physical population includes a mounted module. Embedded circuits belong to the host Board and have no instance pin. Reuse-board copies neither; new-board mirrors hosted items under fresh IDs/namespaces. These decisions are already covered, not a new topology ADR. ([0023](../adr/0023-distinguish-mounted-daughterboards-from-embedded-circuits.md), [module types](../../core/src/modules.rs#L205-L251), [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [`reuse_board`](../../core/src/mirrored_half.rs#L875-L895)) |

## Enforcement and preservation: what is actually checked

- **PCB membership, not half labels, scopes electrical planning.** `discover_keys`
  checks matrix Board ID and member parts; `resolve` selects a controller among
  this Board's placed parts and requires its reviewed profile. Missing controller,
  incompatible requested profile, unknown terminals, insufficient GPIO, conflicting
  locks/manual nets and protected-pin changes produce diagnostics. Tests include
  `resolver_scopes_board_and_honors_controller_and_locks`,
  `missing_controller_is_reported_without_inventing_one` and
  `handoff_cannot_be_silently_overridden_by_new_locks`
  ([planner](../../core/src/electrical.rs), `discover_keys`, `allocate_pins`,
  `finalize_nets`, `resolve`; [tests](../../core/tests/electrical_wiring.rs)).
- **Plans and identity remaps have boundaries.** A topology proposal clones the
  document and retains `hardware.boards`; it does not remap parts/nets or project
  keymap identities. New-board creation is different: it mints collision-checked
  identities and correspondence maps, remaps electrical locks/assignments and
  deliberately leaves mirror firmware-position bindings empty. Thus “topology
  preserves keymaps” means existing data is retained, not that two designs are
  merged or new keys automatically receive semantic bindings
  ([setup](../../web/crates/catalogue/src/physical_setup.rs#L321-L449);
  [new-board IDs](../../core/src/mirrored_half.rs#L1392-L1501);
  [electrical copy](../../core/src/mirrored_half.rs#L1814-L1859)).
- **Interface checks are separate from mechanical mirroring.** Mounted connections
  name a real host connector, module port, bus, actual MCU terminals, cable and
  rails. `connection_findings` validates the electrical connection; electrical
  resolution imports those findings and locks. Firmware gates then check the
  selected physical host, module driver qualification and embedded circuitry;
  they do not infer supported behavior from a reflected pose
  ([module contract](../../core/src/modules.rs#L187-L228), `ModuleConnection`;
  [checks](../../core/src/modules/electrical.rs), `connection_findings`;
  [planner](../../core/src/electrical.rs#L996-L1015);
  [firmware hardware tests](../../core/tests/firmware_hardware.rs),
  `invalid_or_missing_physical_context_cannot_hide_scoped_modules`,
  `rust_generated_module_connection_findings_block_functional_firmware`).
- **Firmware targets are instances even for a shared design.** Runtime resolves
  central and peripheral separately by `(boardId, instanceId)`; the adapter carries
  physical host identity while deriving board name from the plan. Core generation
  requires a distinct peripheral plan for a split transport and consistent transport
  on both halves. The output's left/right targets do not prove distinct PCB IDs
  ([Runtime](../../web/crates/runtime/src/runtime.rs#L6613-L6676);
  [adapter](../../web/crates/runtime/src/firmware_request_adapter.rs#L194-L210);
  [generation](../../core/src/firmware.rs#L449-L493)).
- **Persisted authority versus defaults:** Board/instance/pair records and explicit
  assignments are durable document data. Electrical plans, effective mechanics,
  mirrored contours and UI setup proposals are derived. Local instances own their
  mechanics; Pair instances overlay pair construction; Legacy instances can inherit
  shared construction, including the first-instance fallback. New setup instances
  are Local, not newly created Legacy records. Wireless battery defaults are reads,
  not evidence a physical battery was placed
  ([configuration resolution](../../core/src/mechanical/inputs.rs#L744-L825),
  `shared_source`, `apply_mechanical_defaults` in [inputs](../../core/src/mechanical/inputs.rs);
  [new instance](../../web/crates/catalogue/src/physical_setup.rs#L899-L923)).
- **Atomicity is specific, not universal typed migration.** Mirrored setup creation
  is one accepted Core edit/Undo step. Other physical setup intents still submit an
  accepted-state `ReplaceDocument` through the resolver when no creation operation
  is returned. Pair repair happens in Core for that replacement. Do not claim every
  topology/transport edit has become a dedicated typed Core operation
  ([resolver](../../web/crates/pcb/src/pcb_physical_setup/controller.rs#L614-L675);
  [0010 final amendment](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-physical-setup-creation-is-atomic-in-core-2026-10-14);
  [integration test](../../core/tests/mechanical_integration.rs#L1300-L1359)).

## Ranked missing-decision candidates

These are candidates for confirmation, not accepted policy. Costly/surprising/real
trade-off are the ADR admission criteria; an implementation fact alone does not
recover why the original author chose it.

### 1. Board electrical authority versus physical population and firmware identity

**Possible decision sentence:** “Persist electrical design and pin allocation per
PCB Board, but resolve physical population and firmware separately per instance;
sharing a reversible PCB does not create independent writable circuitry.”

- **Implementation/tests:** `ElectricalBoardConfiguration`, `resolve`, Runtime's
  two `FirmwarePlanTarget`s, mounted `host_instance_id`, and Board-owned embedded
  circuits establish the split. `matrix_and_direct_plans_keep_scan_rows_keys_and_bindings`
  and `split_profiled_encoders_keep_each_plans_identity_and_gpio` test adapter data;
  `scoped_module_validation_uses_the_selected_physical_host` checks population
  filtering ([model](../../core/src/model.rs#L1103-L1114),
  [Runtime](../../web/crates/runtime/src/runtime.rs#L6613-L6668),
  [adapter tests](../../web/tests/firmware_request_adapter.rs),
  [Core hardware tests](../../core/tests/firmware_hardware.rs)).
- **Historical rationale:** commit `e7f1628b8cadf947460d8d075f26bf0da742693f`
  explicitly introduces physical assemblies, automatic MCU planning, reversible
  jumper recipes, editable ZMK handoff and protection of exported mappings.
  It does **not** explain a comparison between per-instance and per-Board writable
  electrical configurations. No rejected alternatives for that choice were recovered.
  Current ADR-0010 explains reuse versus new-board; current ADR-0023 explains
  ownership of mounted/embedded copper, not all assignment/population consequences.
- **ADR test:** costly **yes** (persisted assignment and shared-ID semantics);
  surprising **yes** (two devices can share key/part IDs and Board wiring while
  requiring separate firmware plans); actual choice **visible**, original trade-off
  rationale **unknown**. Do not fabricate an earlier rejected per-instance design.
- **Coverage delta/disposition:** first add architecture/glossary explanation of
  the identity tuple and configuration ownership. A small new electrical-authority
  ADR is justified **only if confirmed as a durable scope boundary**, otherwise amend
  0010/0023 where their existing rules already suffice. **Confidence:** high current
  ownership, medium ADR value, low recovered original rationale.

### 2. Non-destructive PCB retention across destructive physical-topology changes

**Possible decision sentence:** “Changing physical topology retains PCB designs and
their wiring/bindings, but may retire physical instances; surviving mechanical
assemblies materialize their effective paired configuration before losing the pair.”

- **Implementation/tests:** setup replaces `hardware.instances`, preserves the
  electrical configuration collection and leaves project Board/part/net/keymap
  collections intact. Core materializes pair survivors. Tests
  `topology_preserves_flipped_primary_and_shared_construction_and_adds_reversible_secondary`,
  `removing_a_paired_member_materializes_the_survivor_and_clears_the_pair` and
  `a_topology_replacement_never_leaves_a_dangling_pair` cover bounded pieces, not every
  module/keymap/transport transition ([setup](../../web/crates/catalogue/src/physical_setup.rs#L321-L449),
  [proposal test](../../web/crates/catalogue/src/physical_setup.rs#L983-L1043),
  [Core tests](../../core/tests/mechanical_integration.rs#L1210-L1359)).
- **Historical rationale:** commit `155b7e01b61a2f829d35dc3ed12335125e00a722`
  explicitly explains materializing previously visible pair-derived geometry to avoid
  dangling ownership or loss during replacement. Commit
  `5221aa8ab427be3f8ac54bd396558ffc9d075b5d` explains why new halves and reviewed
  mechanical linkage had to land together rather than silently discarding the pair.
  Commit `fd2f217e1c75f3588701537c3fc55f1ab1613446` subsequently lands typed atomic
  creation; [ticket 21](../plans/typed-core-edits/issues/21-mirrored-half-web-adapter.md)
  and ADR-0010 record that supersession. No primary rationale for always replacing
  the instance list or resetting transport on topology change was recovered.
- **ADR test:** costly **yes** (retired physical identity, attachments and Undo);
  surprising **yes** (a removed half's PCB survives); real trade-off **partially
  recovered** for materialization, not the full transition policy.
- **Coverage delta/disposition:** survivor materialization belongs in an **0010
  amendment**, not a duplicate mirror ADR. The wider transition policy needs explicit
  confirmation and a bounded preservation contract before any standalone ADR.
  **Confidence:** high mechanical behavior; medium broader policy; not ready as a
  new accepted record.

## Contradictions and policy questions

1. **Labels are not merely descriptive today.** The schema permits arbitrary strings
   and creation accepts explicit labels, yet split firmware insists on left central /
   right peripheral. Decide whether right-central keyboards are intentionally outside
   product scope or whether firmware's side/role coupling is an implementation limit.
   The facts do not require the user to rediscover the code
   ([0010 labels amendment](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-companion-modes-pads-labels-2026-10-10),
   [Runtime restriction](../../web/crates/runtime/src/runtime.rs#L6617-L6628)).
2. **“Topology split” does not certify a complete two-half keyboard.** One-member split
   input is used in native creation tests, and catalogue mechanical-link tests use
   split/none. Atomic creation's explicit setup request rejects none; firmware has its
   own stricter two-member requirement. Clarify whether incomplete hardware is a saved
   design state with output-specific gates, rather than imposing creation's stronger
   contract retroactively on all imported/partial documents
   ([synthetic source](../../core/tests/support/synthetic_half.rs#L31-L43),
   [review fixture](../../web/crates/catalogue/src/physical_setup.rs#L1553-L1560),
   [Core creation](../../core/src/mirrored_half.rs#L149-L168),
   [Runtime](../../web/crates/runtime/src/runtime.rs#L6621-L6628)).
3. **Topology/reversibility controls have different effects.** Entering split sets
   `reversibleLayout=true`, but does not run the generator normalizer that explicit
   `ReversibleLayout` runs; the latter also rewrites flips from half labels. These
   are not interchangeable operations, and unchanged generator parameters are a
   deliberate mirror rule. Clarify whether global reversible setup is allowed to
   change existing linked physical placements, rather than assuming a new-board
   mirrored design should become a flipped reused design
   ([topology](../../web/crates/catalogue/src/physical_setup.rs#L424-L449),
   [reversible proposal](../../web/crates/catalogue/src/physical_setup.rs#L472-L506),
   [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-mechanical-features-and-reversible-parts-2026-10-10)).
4. **Link proposal versus accepted workflow:** catalogue's explicit existing-halves
   link review is real, but the creation conversion always requests ReuseBoard.
   Do not equate the review tests with an end-to-end distinct-PCB creation path.
   Existing-link acceptance must use the mechanical link path; this research did not
   establish complete UI availability of every reviewed mapping
   ([proposal](../../web/crates/catalogue/src/physical_setup.rs#L173-L217),
   [conversion](../../web/crates/catalogue/src/physical_setup.rs#L79-L143)).
5. **Older design language is not present authority.** The implemented-v2 layout
   brief describes two layouts beneath one Board and independent hardware/companions;
   the later native mirror ADR requires default linkage of whole halves. Keep the
   legacy layout-pair feature distinct, mark historical scope when documenting it,
   and do not use the old brief to override native pair rules
   ([brief](../design/mirrored-layout-pair.md),
   [overlap refusal](../../core/src/mirrored_half/inverse/layout.rs#L1-L62),
   [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)).

Open policy, not proven defects: what should happen to modules pinned to an instance
retired by a topology replacement; should transport changes preserve a protected PCB
handoff unchanged until explicit remap; and are independently editable key assignments
required for two instances of the same Board? The inspected proposal retains Board
electrical data and module records, while instance-specific firmware gates refuse
missing context. That establishes neither automatic attachment migration nor a
complete transition acceptance guarantee
([setup](../../web/crates/catalogue/src/physical_setup.rs#L321-L449),
[hardware gate tests](../../core/tests/firmware_hardware.rs#L149-L197)).

## Existing coverage and exclusions

- **0008 / 0010:** already cover linked-by-default mechanics, hybrid stored/reconciled
  native mirrors, reuse/new Board, full correspondence identity, exceptions, pose
  versus parameter rules, no copied copper/bindings, compatibility and atomic mirrored
  physical setup. Their actual documented alternatives include derived-only storage,
  one-time copying and recipe arithmetic; do not repackage these as a new topology ADR
  ([0008](../adr/0008-linked-halves-stay-linked.md),
  [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)).
- **0023:** closes mounted/embedded ownership, internal identity independence and
  explicit joins. **0004, 0005, 0013, 0014, 0016:** cover migration, queued intent,
  durable accepted publication and captured/export lifetime; no topology-specific
  restatement is needed ([index](../adr/README.md)).
- Compatibility is not an instruction to auto-link existing designs. Optional native
  pair fields and legacy exception records have explicit meanings in 0010; Legacy
  mechanical overlay is a compatibility path, whereas fresh setup uses Local ownership
  ([0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md),
  [setup](../../web/crates/catalogue/src/physical_setup.rs#L899-L923)).
- No new finding about strict vendor certification is made. ADRs
  [0017](../adr/0017-forgiving-vendor-previews-with-preserved-source.md) and
  [0018](../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md)
  govern forgiving previews and honest uncertainty. No CAD plan reconciliation,
  whole-firmware audit, component qualification claim or universal transport/harness
  validation belongs to this report.

## Validation and limits

- Read repo/domain instructions, glossary, ADR index and relevant records, architecture,
  onboarding and prior inventory. Used CodeGraph before implementation exploration;
  read omitted/uncovered sections directly where necessary.
- Inspected existing test bodies for atomic creation/Undo, setup parity and transport,
  electrical planning, firmware adapter identity/module gates and mechanical survivor
  materialization. The saved `synthetic-outline.json` is a one-Board/no-hardware fixture;
  the native synthetic-half builder supplies explicit split/controller/membership input.
  Fixtures demonstrate bounded contracts, not original rationale or hardware QA
  ([saved fixture](../../core/tests/fixtures/synthetic-outline.json),
  [builder](../../core/tests/support/synthetic_half.rs)).
- Verified 113 report-local links/anchors and cited line ranges, 22 named function/test
  declarations against current files, and five historical/HEAD commit objects. Git history
  supplied provenance; concurrent uncommitted docs were not assigned invented commits.
- **Not run:** builds, tests, browser checks, mutation checks, firmware compilation,
  electrical/physical bench checks. No ADR, glossary, index, ticket, production code or
  commit was created. The report is research only; untraced full transition combinations
  and historical rationale remain explicitly unknown.
