# Sofle adaptation: hardware candidates and case-generator evidence

Research date: 2026-10-10. Read-only planning research by Codex, Sol 6.1 max; report assembled by the Sol 6.1 medium coordinator from that research. Related work: [shipped-board audit](shipped-boards-review.md). The per-variant hardware packet is [ticket 46](../issues/46-sofle-hardware-inputs.md) in the reviewed phase 2 plan, now synced into this worktree.

## Settled direction and scope

These are BoardStudio adaptations. Preserve every measured key position, tilt and width independently for Sofle v2, RGB and Choc. The user will specify the target installed hardware/BOM. No controller, switch, LED, connector or other installed component is selected by this report. Screws/spacers may be added. Gasket, plate and tolerance must use BoardStudio's existing case-generation tool; an unsupported setting requires a bounded tool-capability follow-up rather than independent geometry.

Pinned upstream hardware is comparison evidence. It does not require reproducing upstream components, circuit or mounting geometry. Existing library entries and defaults are candidates, not approval of hardware or physical fit.

## Source pins and variant evidence

[sofle-layouts.json](../../../../content/layouts/sofle-layouts.json) pins Josef Adamcik's SofleKeyboard revision `fb294f7c58d0f91c379a9ef339159d4d056643e9`. The research downloaded the three pinned PCBs and confirmed their SHA-256 values against that metadata:

| Variant | PCB path within the pinned repository | SHA-256 |
| --- | --- | --- |
| v2 | `Sofle_v2/PCB/SofleKeyboard.kicad_pcb` | `e34c4981c039bd5c0ab8613c7454422cae117f0dc320b99cb37fd26592d36cce` |
| RGB | `Sofle_RGB/PCB/SofleKeyboard.kicad_pcb` | `2162c58ea07f378e4352421c9d2bed31b774a08539114df97515d9f06c058739` |
| Choc | `Sofle_Choc/PCB/SofleKeyboard.kicad_pcb` | `f1714d5d75b3c6d103e865020a45cb0941ddcd59ac5dbd47a35eb5224bce7189` |

| Variant | Pinned source facts | BOM-conditional adaptation candidate |
| --- | --- | --- |
| v2 | U1 is `ArduinoProMicro-ZigZag-DoubleSided`, at `(100,56.71,270°)`. Guide specifies Pro Micro/clone, MX Kailh hotswap sockets and SOD123 1N4148W; no RGB. Supplier and installed header/socket height are unpinned. [PCB U1](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_v2/PCB/SofleKeyboard.kicad_pcb#L1044), [guide](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/docs/build_guide.md#L19). | Existing `mx-hotswap` members are a possible seed after the supplied BOM is qualified. |
| RGB | U1 is `ProMicro`, at `(101,56,0°)`, with alternative 2×12 header patterns. Each half has 29 combined MX/SK6812MINI footprints and seven surface LEDs. Source LEDs are MINI, not MINI-E; guide warns about different package pin markings. [PCB](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_RGB/PCB/SofleKeyboard.kicad_pcb#L6123), [guide](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/docs/build_guide_rgb.md#L50). | Existing `mx-hotswap-rgb` uses MINI-E. That substitution needs BOM, electrical and installed-face qualification. |
| Choc | ProMicro/peripheral pattern follows RGB. Source requires PG1350 Choc v1 hotswap, 29 MINI-E LEDs per half, no D31–D37 bank. Guide installs diodes, sockets and reverse-facing LEDs on the back, peripherals on the front. [PCB](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_Choc/PCB/SofleKeyboard.kicad_pcb#L7102), [guide](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/docs/build_guide_choc.md#L28). | Existing `choc-hotswap-rgb` enables V1 and disables V2. Qualify matching members after BOM input; retain the measured Sofle relationships, including its 19.05 mm grid. |

Source footprint names identify geometry, not exact purchasable hardware. Common source names include reset `TACT_SWITCH_TVBP06`, TRRS `MJ-4PP-9` despite guide calls for PJ-320A, generic `OLED_4Pin` SSD1306 display and a modified Alps EC11E footprint. Source encoder push switches also have an additional diode; generic key assemblies alone do not describe that circuit. See the pinned PCB/guide links above and the peripheral details below.

## Source versus catalogue qualification

The current [recipes](../../../../core/examples/demo_projects/recipes.rs) select nice!nano, EC11/12, SSD1306, PJ320A and PTS636-style reset candidates. U1/encoder use zero offsets; OLED uses `(+3.81,−16.7)`, TRRS `(+4,0)` and reset `(+1,0)`. Those are candidate substitutions and placements. Do not automatically retain nice!nano, remove offsets because origins differ, or copy upstream angles onto different library geometry.

| Component or datum | Observed difference | Required qualification after BOM input |
| --- | --- | --- |
| OLED | Source datum is the first header hole, with holes at `0,2.54,5.08,7.62`. Library holes are centred at `−3.81…+3.81`, local `y=16.7`. [Source J3](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_v2/PCB/SofleKeyboard.kicad_pcb#L4824), [generator](../../../../footprints/src/generators/ceoloide/display_ssd1306.rs). | This explains the existing offset. Check target module pin order, header/socket height, side and body envelope. |
| Reset | Source hole centres 6.5 mm, drills 1.3 mm; library centres 6.4 mm, drills 1.2 mm. [Source](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_v2/PCB/SofleKeyboard.kicad_pcb#L960), [generator](../../../../footprints/src/generators/ceoloide/reset_switch_tht_top.rs). | Match actual terminals/body/access before retaining the candidate or its +1 mm offset. |
| TRRS | Source and library differ in post, slot and terminal geometry. [Source](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_v2/PCB/SofleKeyboard.kicad_pcb#L960), [generator](../../../../footprints/src/generators/ceoloide/trrs_pj320a.rs). | +4 mm does not establish equivalence. Match exact jack, contact use, insertion/cable and installed body envelope. |
| Encoder | Source common is C; library common is B. Source signal row is local `x=−7.5`; library row `y=+7.5`. Source signal drills 1 mm round and mounting slots 2.8×1.5 mm; library defaults signal slots 1×0.5 mm, mounting slots 2.3×1.5 mm. [Source](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_v2/PCB/SofleKeyboard.kicad_pcb#L5097), [generator](../../../../footprints/src/generators/ceoloide/rotary_encoder_ec11_ec12.rs). | Dimension overrides exist, but require explicit terminal mapping and transformed-pad evidence for the supplied encoder, including push-switch circuit and knob/service space. |
| Source TH holes | TH1–2 use plated 2.2 mm drills; TH3–7 plated 4.1 mm drills. RGB/Choc TH3–6 have local offsets up to 0.275 mm, so measured module origins are not all drill centres. Recipe substitutes centred 2.2 mm NPTH holes. [RGB holes](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_RGB/PCB/SofleKeyboard.kicad_pcb#L6938), [generator defaults](../../../../footprints/src/generators/ceoloide/mounting_hole_npth.rs). | Keep source holes, adapted host drills and generated closure mounts distinct. TH references/positions do not select fasteners or require upstream hole reproduction. |

## LED electrical and installed-face evidence

The RGB guide maps source MINI pins `1=DOUT, 2=GND, 3=DIN, 4=VCC`. PCB nets establish the bank order **D37 → D33 → D32 → D31 → D34 → D35 → D36**, then J5/per-key lighting. D37 receives controller data through J6 and matches the guide's single front indicator; the other six are rear drop lights. [Guide](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/docs/build_guide_rgb.md#L81), [PCB bank/J6](https://github.com/josefadamcik/SofleKeyboard/blob/fb294f7c58d0f91c379a9ef339159d4d056643e9/Sofle_RGB/PCB/SofleKeyboard.kicad_pcb#L319).

Current recipes put all seven MINI-E candidates on the back with `reverse_mount:false`; matrix companions use `reverse_mount:true`. MINI-E mapping is `P1=VCC, P2=DOUT, P3=GND, P4=DIN`, as recorded in [Core peripheral mapping](../../../../core/src/electrical_peripherals.rs). If the user-specified adaptation retains source lighting roles, propose a separately qualified D37 face/definition exception and explicit chain. Do not equate MINI and MINI-E or silently reproduce the source circuit.

## Existing native assembly seeds

[Preset construction](../../../../web/crates/catalogue/src/matrix_setup_operation.rs) uses `generator:ceoloide/switch_mx` or `switch_choc_v1_v2`, `diode_tht_sod123`, and optionally `led_sk6812mini-e`. Seeds place the back diode at `(7.4,−1.5),90°`, reverse-mounted back LED at `(0,−4.75),180°` for MX or `(0,−4.7),180°` for Choc. [Matrix cell definitions](../../../../core/src/model.rs) support per-cell definitions, offsets, rotations and sides.

These reusable members are not qualified Sofle-specific assemblies. Current shipped diode direction remains duplicate-position under the reviewed plan. Core's heading rules (switches `−r`, independent parts `180−r`) preserve side/parameters but do not prove mirrored pad suitability. The eventual profiles must keep measured key positions/tilts/widths and qualify companion placement and both installed halves against the supplied BOM.

## Existing case-generation capabilities and defaults

Use the tool's existing workflow and numerical settings for proposed case geometry. Values below are review baselines, not selected hardware or qualified supplier tolerances.

| Capability | Observed support/default and limit | Source |
| --- | --- | --- |
| Native fit profiles | Nominal 14×14 mm openings. MX plate 1.5 mm (supported 1.4–1.6), nominal underside-to-PCB gap 3.5 mm. PG1350 plate 1.3 mm (1.2–1.4), gap 2.2 mm. PG1353 plate 1.5 mm (1.4–1.6), gap 3.5 mm. Match the supplied switch; never respace measured keys to fit a profile. | [Profile/datums](../../../../core/src/mechanical.rs), `builtin_profile`, research line 2322. |
| Process and mount | FR-4, printed PLA/ABS, CNC aluminium and cut-sheet acrylic; tray, rigid or gasket mounts; shell/sheet bottoms and optional middle frame where supported. Gasket forces shell construction and disables integrated plate frame. | [Case controls](../../../../web/crates/case/src/mechanical_settings.rs), research line 3350. |
| General dimensions | Defaults: wall 2 mm, clearance 0.3 mm. | [Mechanical edit defaults](../../../../core/src/mechanical/edit.rs), research line 2170. |
| Gasket initialization | Public `SetMount(Gasket)` initializes travel to 0.1 mm. Internal gasket defaults: foam 2 mm, compression 0.15, minimum wall 2 mm, numeric tolerance 0.05 mm. Do not substitute the lower-level fallback travel 0.3 mm for the public initialization value. | [Mount initialization](../../../../core/src/mechanical/edit.rs), research line 2774; [gasket defaults](../../../../core/src/mechanical.rs), research line 132. |
| Mounts and hardware | Clearance-tested proposals/adoption, fixed closure positions and editable screw/insert geometry. Added screws/spacers are allowed but not selected here. | [Mount controls](../../../../web/crates/case/src/mechanical_settings.rs), research line 2435. |
| Component/access space | Component clearance polygons/volumes and access openings. Current controller 20×36 mm and TRRS 14×12 mm bays are historical rectangles, not measured installed hardware envelopes. Derive replacements through the tool from component stacks, cable/connector and knob/service bounds. | [Historical bays](../../../../core/examples/demo_projects/mechanical.rs), research line 185. |

The inspected [public edit vocabulary](../../../../core/src/mechanical/change.rs) exposes clearance/opening allowance and **text** hardware/critical-fit tolerance annotations. Research found no public numeric setter for `internal_gasket.tolerance`, gasket compression or travel. Retaining supported generator defaults is possible; different requested numeric settings require a scoped capability follow-up. Text annotations do not change solver geometry. The [internal gasket solver](../../../../core/src/mechanical/gasket/internal.rs), research line 1235, warns that foam compression/material tolerances are unverified.

## Qualification limits and implementation gates

The historical [alignment manifest](../../../../ergogen/library/manifest/alignment.json) records front/back and 0°/90° evidence for MX/Choc, diode, MINI-E, PJ320A, SSD1306 and nice!nano configurations. Nice!nano coverage excludes socket/header solids and extra pins although the recipe enables extra pins. Encoder and THT reset lack default models in the inspected [coverage manifest](../../../../ergogen/library/manifest/coverage.json). Arbitrary rotations and whole-board inspection remain outside the recorded coverage. No exact standalone ProMicro, source MINI package, TVBP06 or MJ-4PP-9 entry was found in the inspected catalogue; this is a bounded search finding, not proof that no possible compatible candidate exists.

Before the relevant hardware implementation slice can proceed:

- Obtain the user's exact installed BOM, then source drawings and qualify library entries; missing entries need separately owned component-onboarding tickets.
- Pin electrical terminal mappings and check pad/drill/artwork/model geometry on both installed halves, including actual thumb angles, connector contact use and LED faces/chain.
- Produce native tool-derived gasket/plate/case/closure geometry and numerical component, switch, source-hole, stabiliser and service clearances.
- Keep supplier/material limits and genuinely unsupported generator settings explicit; do not silently replace defaults or invent independent geometry.

Successful generation/export does not qualify electrical operation, manufacturing tolerances or physical fit. Follow [component onboarding](../../../hardware/component-onboarding.md). No native edit, component, case or hardware change was made in this research.

## Short user decision packet

1. **Immediate input:** supply the target installed BOM separately for v2/RGB/Choc: controller and header/socket stack, switch/socket/keycap, encoder/knob, display/header, reset, TRRS/contact use, diode and exact LED variants/faces. Agents then source drawings and qualify matching entries; no default nice!nano selection.
2. **After BOM matching:** review the concrete proposal's still-open native mount style and plate/case process, including any added fastener/spacer dimensions. Present matching profiles and supported generator options only.
3. **After tool output exists:** review displayed clearances/service margins and unresolved supplier/material limits. Start from supported generator defaults and distinguish geometry-changing settings from annotation-only tolerance.

## Research and validation record

The Sol 6.1 max research session inspected pinned primary PCB/guide sources, measured metadata, recipes, footprint generators, assembly construction/cell contracts, mechanical profiles/edit vocabulary, case controls and alignment/coverage manifests. The source hashes were compared in that research. The coordinator transcribed its findings here without claiming a second independent verification.

The complete GitNexus repository listing contained ten indexes and none for `boardstudio-lane-boards-review`; checkout-bound query/context returned repository-not-found. No other checkout graph was substituted and no refresh ran. Code facts therefore use `rg` and direct source inspection, not a clean graph-impact result. Implementation needs fresh bound graph analysis and fallback confirmation where unresolved.

No cargo, check.py, build, browser, export, timing, graph refresh or hardware qualification ran in the research/report-writing sessions. Lane F drafts, code, maps, ADRs and generated goldens remained untouched by that pass.

Root subsequently synced the green integration batch into this managed worktree at `64510cb0`, preserving this report,
and ran `python3 scripts/check.py repo`: 262 Markdown files passed. Root registered the bound
`boardstudio-ts-boardstudio2` index; its analyzer reports process/call-resolution coverage caps, so absent flows are
not proof of safety. These synchronization/documentation gates do not constitute independent verification of the
hardware findings, BOM qualification or physical/electrical fit. The report remains evidence for ticket 46 and human choices.
