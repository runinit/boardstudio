# Shipped boards review (phase 1)

Read-only review of the 20 shipped projects in the requested order. Source of project data is `core/tests/fixtures/demo-projects-baseline.json`; measured inputs are pinned in `content/layouts/keyboard-layouts.json` and `content/layouts/sofle-layouts.json`. Recipe behavior is in `core/examples/demo_projects/{recipes,physical,mechanical,review}.rs`. The archive is `content/archives/reviung41-original.boardstudio`; the VIK inputs are `content/layouts/module-review.json` and the catalogue sources.

The fixtures describe BoardStudio’s adapted examples, not upstream board files: the recipes preserve measured switch locations, then add BoardStudio library switches, diodes, nice!nano placements/bays and generated outlines. Upstream routing and most peripherals are not reproduced (`content/demos/README.md`; `recipes.rs`). The recorded fixture source hashes match the layout metadata. I independently compared every stored switch pose in the baseline against the corresponding source JSON position and recipe transform: all 15 measured keyboards and all three Sofle variants matched (zero pose differences). This is a read-only calculation against saved data, not a build or physical check.

## Cross-board themes

- All 10 paired split examples still contain two recipe-authored halves. ADR-0008/0010 call for one authored half with its second half maintained as a Core reflection; every pair item stays linked unless that individual item has an explicit local/omitted exception. Ticket 20 is still `ready-for-agent`; it specifically removes manual mirror arithmetic and makes KLOR’s outline match. Treat current parts as measurement-derived, not yet as proof that the recipe satisfies the final pair contract.
- Matrix switches use `ceoloide/switch_mx` or `ceoloide/switch_choc_v1_v2`; all have `reversible:false`, generator `side:B`, and part `side:front`. Their mirrored tilt is `-r` and position mirrors on X only. Diodes use `ceoloide/diode_tht_sod123`, with `reversible:false`, generator `side:B`, part `side:back`. RGB variants add back-side SK6812 LEDs. The three companion modes are literal mirror, duplicate position, and mirror with a tweak. Other generator definitions generally omit `reversible` and a generator `side`; absence is recorded as “absent”, not interpreted as `false`. Mirroring must not rewrite either parameter or part side (ADR-0010).
- Split recipes generate nice!nano controller, reset and TRRS parts and associated bay/TRRS openings; they are adaptation placeholders, not verified source-board component placements. Sofle’s controller, encoder and mounting holes (and RGB LEDs D31–D37) sit at the measured source positions (y exact; x through the recipe’s mirror constant, checked here); its TRRS, reset and display carry recipe offsets (`recipes.rs:289-293`), so they are partly measured, not wholly generated. Unibodies get a generated nice!nano and reset switch, without TRRS. Generated outlines are live recipe outputs. Unibody examples have no case bodies, no mechanical configuration and no authored closure mounts in the baseline (the REVIUNG41 archive differs, see its entry). `content/demos/README.md` says the measured recipes’ controller and TRRS openings are local to the right half; the baseline shows `opening-1` and `opening-2` linked on all nine split fixtures that have them, so the README sentence is stale (not edited here). No fixture records explicit keep-out regions; the listed clearance values are post-hoc geometry measurements, not keep-out constraints.
- Seven measured splits (three MX, four Choc), Sofle RGB and Choc store four closure mounts per half. Sofle v2 is gasket construction and has none. These are Core-proposed closure features, not upstream mounting-hole measurements. The parts use `ceoloide/mounting_hole_npth`, front side, `hole_drill:6.6` / `hole_size:6.6`, with no explicit `reversible` parameter; their 6.6 mm hole and the 14 mm switch cutout are assumptions for clearance calculations.
- Recorded exports succeeded, but that does not establish PCB, plate, case, electrical, connector, or hardware fit. The saved keyboard fixtures carry project data rather than exported PCB/plate/case results; their paired mechanical configuration does not itself qualify generated output. VIK alone records a simple plate case body. No build/export was run for this review.

## Ten highest-priority findings

Ranked by likely impact. Severity classes are **1 — build/export blocker**, **2 — physically wrong or unresolved fit**, and **3 — cosmetic/tidy-up**. No class-1 blocker is recorded by the cited Outcomes: ticket 02 reports every split half exported. This is historical evidence, not a fresh export.

1. **[2] KLOR closure/bay overlap persists on both halves by 1.42 mm.** Ticket 06 says linked closure positions carry the overlap to both halves; ticket 02 records that Core does not report a half’s own closure mount against a Case opening. Reproduced from the baseline (see Verification). [Ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md), [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
2. **[2] Sofle RGB and Choc closure holes sit 0.10 mm from the linked controller bay on both halves.** Ticket 06’s post-link table gives 0.10 mm for both halves; the −3.30 mm right-half overlap is ticket 06’s “before” value (the old local right bay) and is superseded, like the Lily58, Sweep and Chocofi overlaps below. Reproduced from the baseline: hole edge to `opening-1` is 0.10 mm on the left board; the opening is linked, so the right is the same by construction. This is a near-contact, not an overlap, but the 0.10 mm margin is below any plausible tolerance. [Ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).  
   *(Corrected in verification: this item said “overlap” and −3.30 mm on the right.)*
3. **[2] Ten paired recipes still author both halves rather than using Core's native mirror.** Their current data is a measured stopgap and can drift from the accepted pair rules. Ticket 20 is ready for an agent and includes the KLOR outline repair. [Ticket 20](../issues/20-recipes-build-one-half.md), [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md).
4. **[2] Tight closure-to-switch-cutout clearances:** Cantor 0.031 mm (both after linkage), TOTEM 0.048 mm (both), Sofle Choc 0.109 mm (both), KLOR 0.295 mm (both), Sweep 0.495 mm (both). Sofle RGB is 1.066 mm on both halves after linkage. These are geometry calculations with assumed cutout dimensions, not measured manufacturing clearance. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
5. **[2] KLOR’s generated outline is not reflective:** up to 6.38 mm boundary distance at right x 316–334, y −61 to −69. Ticket 06 attributes this to gap repair filling a 19.36 mm concave pocket on one side only, not to the measured keys, bay, reset, TRRS or diodes. Ticket 20 requires a native mirrored half and removes the 6.5 mm allowance. [Ticket 06 Outline mirror](../../generator-parameters/issues/06-paired-half-closure-proposals.md#outline-mirror), [ticket 20](../issues/20-recipes-build-one-half.md).
6. **[2] Sofle v2 closure/mount decision remains open.** It has a gasket and zero closure mounts; keeping gasket means none, changing to tray means mounts must be proposed. Its gasket geometry was regenerated when the pair construction changed and needs fit review. [Ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 04 Outcome](../issues/04-refresh-included-keyboards.md).
7. **[2] The current diode mirror rule changes diode position by 14.8 mm from every shipped right-half diode.** Shipped recipes keep the same switch-local `(+7.40, -1.50)` offset; ADR-0010 currently specifies literal mirrored position. Three modes exist (literal mirror, duplicate position, mirror plus tweak). The Core side is decided: the user chose three modes with literal mirror as the default (ADR-0010 amendment of 2026-10-10; [ticket 31](../issues/31-companion-placement-modes.md), `ready-for-agent`, whose acceptance includes reproducing the shipped diode placement under mode 2). What remains is which mode each shipped board should use, a PCB/hardware question; ticket 29 recommends no hardware guess. [ADR-0010 companion amendment](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ticket 29 Verification](../issues/29-mirror-switch-and-companion-headings.md#verification).
8. **[2] Independent-part heading mismatch is recorded for Corne and all three Sofles, and the baseline shows it on all seven measured splits.** Ticket 30’s scratch comparison covered only Corne among the measured splits. In the baseline, `U1`, `RST` and `TRRS` are at rotation 0 on both halves of Corne, Lily58, KLOR, Sweep, Chocofi, TOTEM and Cantor, so the 0→180 difference applies to all seven (verified here, see Verification). Ticket 30 reports right-half MCU/reset/TRRS on Corne at 0→180, and Sofle v2/RGB/Choc right MCU, encoder, display, TRRS, reset and seven round mounting holes at the listed headings. RGB/Choc additionally have LED hand placements. Recipe headings still use `-r`/the authored angle; ADR-0010 requires `180-r` over 360° for independent parts. [Ticket 30 Outcome](../issues/30-independent-parts-use-the-full-heading-rule.md).
9. **[2] Correct heading alone does not geometrically mirror off-axis pads, but the Core rule on this is decided.** ADR-0010’s ticket-27 note says MCU, TRRS, display and encoder pads off their heading axis land with local `y` reversed when a part is only rotated. The later amendment “companion modes, pads, labels” (2026-10-10) records the user’s answer: a part’s pads are not a Core concern and Core adds no pad-symmetry finding. What stays open for these boards is physical: whether each shipped right-half MCU, TRRS, display and encoder is acceptable with its pads reversed about the heading axis (not answerable from the repository). [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ticket 27 Verification](../issues/27-mirror-modules.md#pad-layout-is-rotated-not-mirrored-the-on-grid-claim-is-wrong).
10. **[3] All generated peripheral placements need component/fit qualification.** Source metadata expressly says peripherals/routing are not reproduced; bays, reset/TRRS placements and controller positions on the seven measured splits and the unibodies are recipe values (`recipes.rs:125-135`, fixed `y` of −15, −45, −65; `demo/…` definitions). On Sofle the controller, encoder, mounting holes and LEDs follow the measured source and only the TRRS (+4 mm), reset (+1 mm) and display (+3.81, −16.7 mm) carry recipe offsets (`recipes.rs:289-293`; the recipe also fixes the headings of U1, J2, RSW1 at 0, 90, 90 whatever the source says). Decide whether to keep these demonstration placeholders, replace them with verified part identities/poses, or leave the example explicitly schematic. [README](../../../../content/demos/README.md), [recipes](../../../../core/examples/demo_projects/recipes.rs).

## Decisions for the user

1. **Diodes on each shipped board:** the Core question is closed (three modes, literal mirror default; ticket 31). Per shipped board, choose (a) literal mirror, moving each diode to the other side of its switch; (b) duplicate position, which reproduces the shipped data (every shipped right half has the diode at switch-local (+7.40, −1.50), heading +90, as on the left); or (c) literal mirror with a per-item tweak. **Recommendation:** (b) for the shipped boards until a target PCB shows otherwise, because it keeps the data users already have; confirm against one upstream PCB (for example the Corne or Sofle source board) which side its right-half diodes are on. Do not encode one mode as universal. [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ticket 31](../issues/31-companion-placement-modes.md).
2. **Sofle v2 construction:** (a) keep the gasket, with no closure mounts; or (b) move to tray and review generated closure positions. **Recommendation:** decide the mount before changing holes; the current source contains no evidence to choose. [Ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
3. **Peripheral placeholders:** (a) retain as illustrative generated parts/openings; (b) qualify and replace with exact target components and measured placements; or (c) remove peripherals and leave the example layout-only. **Recommendation:** (b) only with the exact hardware/board revision and fit evidence; otherwise (a) must remain clearly documented as a demo. No hardware facts are inferred here.
4. **Independent-part mirror geometry (physical question only):** the Core choice is already made (heading `180-r`, pads not a Core concern, no pad-symmetry finding). Remaining: for the MCU, TRRS, display and encoder on the right halves, (a) accept the rotated footprint as shipped, or (b) pick a footprint variant or flip per part. **Recommendation:** (a) for demonstration use, with a note that the right-half pads are not a mirror image; (b) only per part, against the real board, before anyone orders fabrication. [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-companion-modes-pads-labels-2026-10-10).
5. **Clearance findings:** decide acceptable minimum clearance and whether to move closure mounts or revise bay/outline geometry. **Recommendation:** require target hardware and fabrication/assembly tolerances before accepting sub-millimetre values.

## Part-list and footprint key

The baseline stores `definitionId`, generator source/parameters, part `side`, pose and model assets; generated definitions do not carry a separate KiCad footprint-name string. Therefore the “footprint” identifiers below are the exact BoardStudio library source IDs, not a claim that their fabrication footprint/model is qualified. Generator code is under `footprints/src/generators/ceoloide/`; imported Marbastlib footprint files and manifest are under `catalogue/parts/`. `reversible` and generator-side values below are the stored recipe parameters; a parameter marked absent is not explicit in the definition. All references, counts and positions come from the expected baseline fixture.

| Recipe/profile | Part list and library footprint source | Stored side / settings |
|---|---|---|
| Measured MX matrix (Corne, Lily58, KLOR) | N `ceoloide/switch_mx`; N `ceoloide/diode_tht_sod123`; where present 2 `ceoloide/mcu_nice_nano`, 2 `ceoloide/reset_switch_tht_top`, 2 `ceoloide/trrs_pj320a`; closure features are generated `case-closure/*` assemblies | Switch: front, `side:B`, `reversible:false`, hotswap; diode: back, `side:B`, `reversible:false`, no THT; peripherals front, `include_extra_pins:true` on MCU, other mirror/side/reversible params absent. Closure hole source uses 6.6 mm normalized hole/drill settings. |
| Measured Choc matrix (Ferris Sweep, Chocofi, TOTEM, Cantor) | N `ceoloide/switch_choc_v1_v2`; N `ceoloide/diode_tht_sod123`; 2 each `mcu_nice_nano`, `reset_switch_tht_top`, `trrs_pj320a`; 8 `mounting_hole_npth` closure holes | Switch: front, `side:B`, `reversible:false`, hotswap, `choc_v1_support:true`, `choc_v2_support:false`, `include_choc_v1_led_cutout_marks:true`; diode as the MX profile. Matrix pitch is 18 × 17 mm (`recipes.rs`, `choc` branch; baseline `matrices[].pitch`), not 19.05 mm. Added in verification. |
| Sofle v2 | 58 `switch_mx`, 58 `diode_tht_sod123`; two each nice!nano, EC11/EC12 encoder, SSD1306 display, PJ-320A TRRS, reset; 14 NPTH measured TH1–TH7 per half | Switch/diode as above; all peripherals front, holes front; no RGB LED. `reversible` absent on peripheral definitions. Gasket, no closure assemblies. |
| Sofle RGB | 58 `switch_mx`, 58 matrix diodes, 58 matrix SK6812 LEDs, 14 measured SK6812 LEDs D31–D37 per half, plus Sofle v2 peripheral/hole profile and eight closure parts | Matrix LED back, `side:B`, `reversible:false`, `reverse_mount:true`; measured D31–D37 back, `side:B`, `reversible` absent, `reverse_mount:false`. Switch/diode as above. |
| Sofle Choc | 58 `switch_choc_v1_v2`, 58 matrix diodes, 58 matrix SK6812 LEDs, plus Sofle v2 peripheral/hole profile and eight closure parts | Switch front, `side:B`, `reversible:false`, hotswap, Choc V1 support, V2 off; matrix LED back with `reverse_mount:true`; diode back. No separate measured D31–D37 set in fixture. |
| Measured unibody common | N `switch_mx`, N `diode_tht_sod123`, one nice!nano and one reset switch; optional Marbastlib MX stabilisers (one 6.25u on GH60, Discipline, Mysterium, Voyager97/104; 2u counts per board below; one 2u on REVIUNG41) | Switch front, generator `side:B`, `reversible:false`, hotswap; diode back, `side:B`, `reversible:false`; controller/reset front. Stabiliser side front; reversible absent. No authored mounting-hole or closure parts in these fixtures. |
| VIK module review | 13 solder MX switch source `ceoloide/switch_mx`; imported `thqwgd001:rotation-reversible`, `:c-2pin-reversible`, `:c-4pin-reversible`; source horizontal VIK host connectors; embedded Haptic DRV2605L components; mounted VIK splitter, haptic and EC11 modules | Switches/connectors front except J_VIK2 back; haptic passives/components retain their embedded front/back faces; imported encoder definitions have no instance `reversible` parameter (reversible identity is in the definition/source). Not a shipped keyboard. See `review.rs`, `catalogue/modules/imported-modules.json`, `catalogue/parts/imported-parts.json`. |

## Board-by-board record

The profile key above gives each part’s library source and stored side/settings. Counts are fixture part counts, including the generated closure assemblies where present; switches/diodes are matrix instances, not distinct definitions. Switch positions were checked against source JSON as described above. The named controller/reset/TRRS and bay/opening are generated by the recipe and should be treated as placeholders pending the user decisions above.

### 1. Sofle v2

- **Source/positions:** 58 measured switches (4×6 keys + 1×5 thumbs per half), 19.05 mm matrix pitch; all 58 switch poses matched `content/layouts/sofle-layouts.json` `layouts.v2` through `recipes.rs`. Source: `Sofle_v2/PCB/SofleKeyboard.kicad_pcb`, revision `fb294f7c`, hash in the layout JSON. 140 parts, 70 per board.
- **Parts:** v2 profile: MX switches/diodes, nice!nano, EC11 encoder, SSD1306 display, TRRS, reset, and TH1–TH7 NPTH per half. `reversible:false` on switches/diodes; absent on peripherals/holes. No closure mounts. Refs (ids `left/…`, `right/…`): U1 MCU, SW25 encoder, J3 display, J2 TRRS, RSW1 reset, TH1–TH7. Positions of U1, SW25 and TH1–TH7 equal the source JSON through a constant mirror in x (224.73 − x) and constant y offset; J2 is 4 mm, RSW1 1 mm and J3 (3.81, −16.7) mm off the source (`recipes.rs:291-293`); J3 sits about 1 mm from U1, i.e. over the controller. Recipe rotations (U1 0, J2/RSW1 ±90) override the source’s (v2 U1 is 270 in the source JSON; unexplained, hardware question).
- **Mounts/keep-outs/placeholders:** 7 authored through-holes each half; no recorded closure holes. Two linked openings (a pair feature each, resolving on both halves): `opening-1` the controller/display bay, `opening-2` the TRRS; reset has no Case opening. No measured Case window for encoder/display. Shared gasket construction replaces the old one-half legacy gasket and was regenerated.
- **Findings:** gasket fit/construction unresolved; the three boards’ independent parts are in ticket 30’s heading-disagreement set. The independent pads on MCU/TRRS/display/encoder also need the ticket-27 decision. [Ticket 04 Outcome](../issues/04-refresh-included-keyboards.md), [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 30](../issues/30-independent-parts-use-the-full-heading-rule.md).

### 2. Sofle RGB

- **Source/positions:** 58 switches, 4×6 + 1×5 per half, 19.05 mm matrix pitch; source-matched. Source hash/revision as above, `layouts.rgb`. 220 parts, 110 per board.
- **Parts:** 58 MX switches, 58 diodes, 58 matrix SK6812 LEDs, 7 additional measured LEDs per half, plus v2 peripherals/THs and eight closure parts. Matrix LEDs store back/`side:B`/`reverse_mount:true`; measured LEDs back/`side:B`/`reverse_mount:false`. See RGB profile.
- **Mounts/keep-outs/placeholders:** 7 authored holes per half; 4 closure holes each half. Closure-to-bay gap 0.10 mm on both halves (the −3.30 mm right-half overlap is ticket 06’s superseded “before” value; baseline reproduces 0.10). Closure-to-switch gap 1.066 mm on both halves after linkage. Controller/TRRS bay values are generated placeholders.
- **Findings:** heading mismatches (right U1, SW25, J3, J2, RSW1, TH1–TH7; LED headings as enumerated in ticket 30); 0.10 mm closure-to-bay margin (both halves); pad geometry (physical question only); diode companion-mode choice per board. [Tickets 02, 06 and 30](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 3. Sofle Choc

- **Source/positions:** 58 switches, 4×6 + 1×5 per half; source-matched to `layouts.choc`. 206 parts, 103 per board.
- **Parts:** 58 Choc switches, 58 diodes, 58 SK6812 matrix LEDs, plus two each nice!nano/encoder/display/TRRS/reset, 14 authored TH holes, eight closure parts. Settings in Sofle Choc profile.
- **Mounts/keep-outs/placeholders:** 7 authored holes per half; 4 closure holes per half. Bay gap 0.10 mm on both halves (the −3.30 mm right overlap is ticket 06’s superseded “before” value); switch gap 0.109 mm on both halves after linkage. Choc switches sit on the 19.05 mm Sofle grid here (baseline `matrices[].pitch` 19.05); the 14 mm cutout used for the gap is the MX size.
- **Findings:** 0.10 mm closure-to-bay margin; tight 0.109 mm switch gap on both halves; ticket 30 heading mismatches for independent components; ticket 29 diode mode differs from shipped same-side placement. No separate measured D31–D37 LED bank. [Tickets 02, 06 and 30](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 4. Corne

- **Source/positions:** 21 measured keys (3×6 + 3 thumbs), 19.05 mm pitch; all 42 switch poses matched. Source `foostan/crkbd`, revision `1f56ad19`, `corne-cherry/pcb/corne-cherry.kicad_pcb`. 98 parts, 49 each half.
- **Parts:** 42 MX switches + 42 back diodes; two each MCU/reset/TRRS; eight generated closure-hole parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Closure switch gap is 0.664 mm on both halves after linkage; no hole within 0.5 mm of bay. Bay/reset/TRRS generated. Ticket 30 reports right U1/RST/TRRS heading 0→180. Shipped diode positions use same-side placement, unlike current literal-mirror rule. [Tickets 02, 29, 30](../issues/30-independent-parts-use-the-full-heading-rule.md).

### 5. Lily58

- **Source/positions:** 29 measured keys (4×6 + 5 thumbs), 19.05 mm pitch; all 58 switch poses matched. Source `kata0510/Lily58`, `Pro/PCB/Lily58_Pro.kicad_pcb`, revision `58c64979`, hash in `keyboard-layouts.json` (corrected: the first draft said `josefadamcik/lily58`). 130 parts, 65 per half.
- **Parts:** 58 MX switches + 58 back diodes; two each MCU/reset/TRRS; eight generated closure parts. Measured MX profile. Recipe input is right-authored; `recipes.rs` records this special source frame.
- **Mounts/placeholders/findings:** no authored TH parts; four closure holes each half. Ticket 02 records the earlier right closure/bay overlap of 1.35 mm; ticket 06’s later linked position is 25.74 mm clear. Switch gap is 1.118 mm on both halves after linkage. Generated bay/reset/TRRS. Right `U1`/`RST`/`TRRS` are at heading 0 like the left (baseline), so the ticket 30 0→180 difference applies here too although ticket 30 lists only Corne. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 6. KLOR

- **Source/positions:** 21 keys, 3×6 + 4 thumbs, 19.05 mm pitch; all 42 switch poses matched, including rotated columns/thumbs. Source repository/revision/path/hash in `keyboard-layouts.json`. 98 parts, 49 each half; right-authored source frame.
- **Parts:** 42 MX switches + 42 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes per half. Closure/bay overlap is 1.42 mm on both; switch gap 0.295 mm on both after linkage. Outline boundary differs by as much as 6.38 mm because the 20 mm pocket-gap repair fills only one concavity. Recipe fixes cannot address that; ticket 20 requires creating the mirrored half from one source. Right `U1`/`RST`/`TRRS` heading 0 as on the left (ticket 30 difference applies). [Ticket 06 Outline mirror](../../generator-parameters/issues/06-paired-half-closure-proposals.md#outline-mirror), [ticket 20](../issues/20-recipes-build-one-half.md).

### 7. Ferris Sweep

- **Source/positions:** 17 keys per half (3×5 + 2 thumbs), matrix pitch 18 × 17 mm (Choc), measured spacing about 17 mm; all 34 switch poses matched. Source `davidphilipbarr/Sweep`, `Sweep v2.2/sweepv2.kicad_pcb`, revision `8ba72a25`, hash in `keyboard-layouts.json`. 82 parts, 41 each half. (Corrected: the first draft said 3×6 + 5 thumbs, 19.05 mm and a `qmk/…` source.)
- **Parts:** 34 Choc switches (`switch_choc_v1_v2`) + 34 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured Choc profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Ticket 02 records the earlier right closure/bay overlap of 1.10 mm; ticket 06’s later linked position is 27.68 mm clear. Switch gap is 0.495 mm on both halves after linkage. Bay/reset/TRRS generated. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 8. Chocofi

- **Source/positions:** 18 keys per half (3×5 + 3 thumbs), matrix pitch 18 × 17 mm (Choc); all 36 switch poses matched. `pashutk/chocofi`, `pcb/chocofi.kicad_pcb`, revision `273676d1`, hash in `keyboard-layouts.json`. 86 parts, 43 each half. (Corrected: first draft said 3×6 and 19.05 mm.)
- **Parts:** 36 Choc switches + 36 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Ticket 02 records the earlier right closure/bay overlap of 0.54 mm; ticket 06’s later linked position is 35.48 mm clear. Switch gap is 1.381 mm on both halves after linkage. Generated bay/reset/TRRS. [Tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 9. TOTEM

- **Source/positions:** 19 keys per half (15 main, one outer, 3 thumbs), matrix pitch 18 × 17 mm (Choc); all 38 switch poses matched. `GEIGEIGEIST/TOTEM`, `PCB/totem_0-3/totem_0_3.kicad_pcb`, revision `410e566d`, hash in `keyboard-layouts.json`. 90 parts, 45 each half. (Corrected pitch.)
- **Parts:** 38 Choc switches + 38 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Switch gap is 0.048 mm on both halves after linkage, with no closure within 0.5 mm of bay. Generated bay/reset/TRRS. Diode side is a pending companion-mode decision. [Tickets 02 and 29](../issues/29-mirror-switch-and-companion-headings.md).

### 10. Cantor

- **Source/positions:** 21 keys per half (3×6 + 3 thumbs), matrix pitch 18 × 17 mm (Choc); all 42 switch poses matched. `diepala/cantor`, `Cantor_Classic/keyboard_pcb.kicad_pcb`, revision `13f8bc03`, hash in `keyboard-layouts.json`. 98 parts, 49 each half. (Corrected: first draft said 3×10 and 19.05 mm.)
- **Parts:** 42 Choc switches + 42 back diodes; two each MCU/reset/TRRS; eight closure parts. Measured MX profile.
- **Mounts/placeholders/findings:** no authored TH holes; four closure holes each half. Switch gap is 0.031 mm on both halves after linkage; no closure within 0.5 mm of bay. Generated bay/reset/TRRS. The 0.031 mm side is a fit concern, not a hardware verdict. [Ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).

### 11. REVIUNG41 measured

- **Source/positions:** 41 measured switches, 19.05 mm pitch, split into left/right key clusters and thumbs on a single unibody main board; all 41 poses matched. Source `gtips/reviung`, revision `110154f5`, `reviung41/pcb/ver1.3/reviung41.kicad_pcb`. 85 parts.
- **Parts:** 41 MX switches + 41 back diodes, one nice!nano and reset, and one front `STAB_MX_2u` stabiliser (85 parts = 41 + 41 + 3; the first draft omitted the stabiliser). Measured unibody profile; no TRRS.
- **Mounts/placeholders/findings:** no authored mounting holes, closure holes, mechanical setup, or case body in this measured fixture. Controller bay and reset are generated examples, not sourced peripheral placements. [Layout metadata](../../../../content/layouts/keyboard-layouts.json), [fixture baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 12. GH60

- **Source/positions:** 61 measured keys; all 61 matched at 19.05 mm matrix pitch with measured widths retained. Pinned upstream revision/path/hash in `keyboard-layouts.json`. 129 parts.
- **Parts:** 61 MX switches + 61 back diodes; one nice!nano/reset; one 6.25u and four 2u Marbastlib stabilisers. Stabiliser source IDs `kicad:marbastlib/STAB_MX_6.25u` and `.../STAB_MX_2u`, front side.
- **Mounts/placeholders/findings:** no authored TH or closure parts/case body; generated controller/reset, no TRRS. Stabiliser placement follows switch width/reference mapping in `physical.rs`; no hardware validation recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 13. Discipline

- **Source/positions:** 68 measured keys; all 68 matched at 19.05 mm base pitch. Pinned source revision/path/hash in `keyboard-layouts.json`. 142 parts.
- **Parts:** 68 MX switches + 68 back diodes; one nice!nano/reset; three 2u and one 6.25u Marbastlib stabilisers (front; baseline: 3 × `STAB_MX_2u`, 1 × `STAB_MX_6.25u`, total 142 parts). No TRRS. (Corrected: first draft said four 2u.)
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body. Generated controller/reset placeholders only. No physical-fit findings are recorded for the unibody recipe; no fit is established. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 14. Mysterium

- **Source/positions:** 87 measured keys across function, keys, navigation and arrows matrices (no numpad matrix; baseline matrices: main-function, main-navigation, main-keys, main-arrows); all 87 matched. Pinned source path/revision/hash in `keyboard-layouts.json`; 19.05 mm base pitch. 181 parts.
- **Parts:** 87 MX switches + 87 back diodes; one nice!nano/reset; four 2u and one 6.25u Marbastlib stabilisers, front (corrected from five 2u).
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset, no TRRS. No recorded physical fit qualification. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 15. Voyager97

- **Source/positions:** 103 measured keys across function/navigation/arrows/numpad/keys matrices; all 103 matched, 19.05 mm base pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 215 parts.
- **Parts:** 103 MX switches + 103 back diodes; one nice!nano/reset; six 2u and one 6.25u Marbastlib stabilisers, front.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset only, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 16. Voyager104

- **Source/positions:** 104 measured ANSI keys across function/navigation/arrows/numpad/keys matrices; all 104 matched, 19.05 mm base pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 218 parts.
- **Parts:** 104 MX switches + 104 back diodes; one nice!nano/reset; seven 2u and one 6.25u Marbastlib stabilisers, front.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset only, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 17. Plaid

- **Source/positions:** 48 measured keys in 4×12 matrix, all matched at 19.05 mm pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 98 parts.
- **Parts:** 48 MX switches + 48 back diodes; one nice!nano/reset. No stabilisers in the fixture.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 18. Lumberjack

- **Source/positions:** 60 measured keys split into left/right 5×6 matrix clusters; all matched at 19.05 mm pitch. Pinned path/revision/hash in `keyboard-layouts.json`. 122 parts.
- **Parts:** 60 MX switches + 60 back diodes; one nice!nano/reset. No stabilisers recorded.
- **Mounts/placeholders/findings:** no authored TH/closure holes or case body; generated controller/reset, no TRRS. No physical qualification recorded. [Source layout](../../../../content/layouts/keyboard-layouts.json), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 19. REVIUNG41 original archive

- **Source/positions/parts:** the packed archive contains project JSON `REVIUNG41`, revision 3, 85 parts on one main board. Its source URL/revision/hash match the measured layout entry (`gtips/reviung`, `110154f5`, `reviung41/pcb/ver1.3/reviung41.kicad_pcb`); the 41 switch positions match the same 41-key measured input. This inventory was read from the ZIP member in memory; the archive was not modified or extracted.
- **Footprints:** 41 `ceoloide/switch_mx` (front, generator `side:B`, `reversible:false`, hotswap); 41 `ceoloide/diode_tht_sod123` (back, `side:B`, `reversible:false`, no THT); one front `kicad:marbastlib/STAB_MX_2u`; one front nice!nano (`include_extra_pins:true`, no `side`/`reversible` setting); one front reset switch (no side/reversible setting). No TRRS or authored mounting-hole parts.
- **Mounts/keep-outs/placeholders:** one generated outline, no explicit keep-outs, no case body. The single main instance carries legacy internal-gasket settings: 4 auto supports, 2 mm minimum wall, 0.05 mm tolerance, 80×3×2 mm gasket, 0.15 compression, 0.1 travel, and an empty closure-mount list. The controller bay and reset remain generated placeholders. No physical fit is established. Archive and measured recipe share parts/layout but differ in the archive’s stored mechanical configuration. [Archive](../../../../content/archives/reviung41-original.boardstudio), [baseline](../../../../core/tests/fixtures/demo-projects-baseline.json).

### 20. VIK module review

- **Source/positions:** synthetic 3×5, 19.05 mm MX host-switch matrix in `review.rs`; this is a module demonstration, not a measured keyboard. 33 parts and three mounted module examples: VIK splitter above and below, EC11 rotary; plus an embedded Haptic DRV2605L circuit. Matrix switch pose follows a regular grid, not board measurements.
- **Parts/footprints:** 13 solder `ceoloide/switch_mx` (front, `hotswap:false`, `solder:true`), three imported THQWGD001 encoder identities (rotation-only, 2-pin tactile, 4-pin tactile), horizontal VIK host connectors and embedded haptic components from the catalogue. The review includes front/back instances and source STEP model assets in the fixture. Exact imported component footprint references are in `catalogue/modules/imported-modules.json` and `catalogue/parts/imported-parts.json`, with source PCB at `catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/`.
- **Mounts/keep-outs/placeholders/findings:** no mounting holes or keep-outs are recorded; a 3 mm plate case body with 0.5 mm clearance is in the review fixture. The mounted module definitions contain real imported circuit/component data, but their placement is illustrative (`x=29/57/85`, `y=20`) and review fixture metadata does not establish host connector compatibility, case clearance, fastening or electrical fit. Ask the user to choose a target host and intended module variant before treating these positions as qualified. [Review recipe](../../../../core/examples/demo_projects/review.rs), [module review metadata](../../../../content/layouts/module-review.json), [module catalogue](../../../../catalogue/modules/imported-modules.json).

## Findings by severity and traceability

### 1 — Blocks a build or export

- **None recorded in the reviewed outcomes.** Generator-parameters 02 says all 20 split halves exported to KiCad after restoring closure proposals. This review did not run export/build, so it does not certify the present branch.

### 2 — Physically wrong or fit unresolved

- KLOR closure/bay overlap and non-reflective outline gap repair: [ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).
- Closure-to-bay: KLOR overlaps by 1.42 mm on both halves; Sofle RGB and Choc sit 0.10 mm from the bay on both halves. The Lily58, Sweep, Chocofi (and Sofle right −3.30 mm) overlaps in ticket 02 are superseded by ticket 06’s linked positions and no longer apply: [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).
- Closure-to-switch clearances below 0.5 mm on KLOR, Sweep, TOTEM, Cantor and Sofle Choc: [tickets 02 and 06](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md).
- Sofle v2 gasket geometry and no closure mounts: [ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md), [ticket 04](../issues/04-refresh-included-keyboards.md).
- Shipped independent part headings versus ADR-0010: [ticket 30](../issues/30-independent-parts-use-the-full-heading-rule.md).
- Diode position differs between shipped same-side layout and current literal mirror mode: [ticket 29 Verification](../issues/29-mirror-switch-and-companion-headings.md#verification).
- Pose-only reflection does not reflect off-axis pad geometry: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ticket 27 Verification](../issues/27-mirror-modules.md#pad-layout-is-rotated-not-mirrored-the-on-grid-claim-is-wrong).
- All generated peripheral placeholders and unibody mechanical/hardware fit are unqualified (the top-ten list ranks the placeholder item [3]; the two lists disagree and the user chooses): [content/demos README](../../../../content/demos/README.md), [ticket 04 Outcome](../issues/04-refresh-included-keyboards.md).
- VIK review mounting/electrical/mechanical compatibility remains unknown without selecting target host hardware: [VIK review recipe](../../../../core/examples/demo_projects/review.rs), catalogue sources linked above.

### 3 — Cosmetic or tidy-up

- Replace or clearly label demo-only controller/reset/TRRS/encoder/display placements if the library is intended to communicate actual buildable hardware; do not present generated examples as upstream component placements. [recipes.rs](../../../../core/examples/demo_projects/recipes.rs), [content/demos README](../../../../content/demos/README.md).
- KLOR’s gap repair is a data/geometry parity issue with physical consequences; it is tracked as severity 2 above, not merely cosmetic. [ticket 06](../../generator-parameters/issues/06-paired-half-closure-proposals.md).

## Scope and verification record

Read: `AGENTS.md`; `docs/agents/verification-process.md`; `docs/plans/typed-core-edits/parallel-lanes.md`; ticket 25 and `docs/agents/issue-tracker.md`; ADR-0008 and ADR-0010; generator-parameters tickets 02 and 06; typed-core-edits tickets 04, 20, 27, 29 and 30; `content/demos/README.md`; both measured layout JSON files and `module-review.json`; `core/tests/fixtures/demo-projects-baseline.json`; `content/archives/reviung41-original.boardstudio` (ZIP member `project.json`, inspected in memory); `core/examples/demo_projects/recipes.rs`, `physical.rs`, `review.rs`, and excerpts of `mod.rs`. `core/examples/demo_projects/mechanical.rs`, catalogue imported definitions and footprint generator files were located/searched but not read in full; source paths are linked as provenance, not independently qualified.

Could not determine from repository sources alone: whether generated MCU/reset/TRRS/encoder/display placements correspond to target hardware; whether the closure and switch clearances are acceptable for actual parts/processes; intended diode mode; whether Sofle v2 should remain gasket-mounted; whether pose-only reflection is acceptable for footprints with asymmetric off-axis pads; and whether the archived gasket/placeholder placements fit real hardware. No hardware decision is guessed.

## AI review record

The AI review compared this report's clearance claims against the later linked-proposal measurements in ticket 06, the hand-placed heading inventory in ticket 30, and the accepted rules in ADR-0010. It corrected the Corne, Sofle RGB and Sofle Choc switch-gap values to ticket 06's post-link values; removed the obsolete Sofle RGB right-side 0.009 mm gap from current findings; and replaced a historical-only item in the top ten with the still-open two-authored-half recipe mismatch. Ticket 02's pre-link figures remain useful history and are not treated as current values.

This is a source-data review, not a complete footprint qualification. The report records library definition IDs, stored side and available `side`/`reversible` settings. It does not enumerate or independently inspect every pad, drill, courtyard, 3D model, wiring assignment, PCB export, plate or case output. Those checks remain outside what this review can determine; no build/export was run under the lane constraint. Target hardware/board revision and fabrication evidence are needed for the hardware-dependent choices above.

No build, cargo command, export, browser run, recipe/fixture/footprint/code edit, GitNexus refresh, or release check was run. The only verification command for this document is the requested `python3 scripts/check.py repo` run, recorded by the lane agent at completion.

## Proposed tickets per board

Added in verification. Titles only; none is created. "Lane" names the parallel-lanes lane that could own it. Recipe or baseline edits wait for ticket 20 (lane B, re-records the baseline), then golden edits go last (lane D). Items already inside tickets 20, 23, 30 or 31 are marked and need no new ticket.

**Sofle first**

1. *Sofle v2: decide gasket or tray, then propose closure mounts or record that none are wanted* (decision 2; user, then keyboards/recipes after 20). Also *Sofle v2: fit-review the regenerated gasket (4 supports, 0.1 mm travel)* (hardware qualification; user).
2. *Sofle (v2, RGB, Choc): place TRRS, reset and display from the measured source, or record the offsets as intentional* (`recipes.rs:289-293`: J2 +4 mm, RSW1 +1 mm, J3 (+3.81, −16.7) mm; recipe headings U1 0, J2/RSW1 90 versus source v2 U1 270 and Choc RSW1 270). Lane: keyboards/recipes after 20. Needs the user for which is correct.
3. *Sofle RGB and Choc: closure hole 0.10 mm from the controller bay on both halves* (move the proposal, resize the bay, or accept; needs an assembly-tolerance decision). Lane: generator-parameters follow-up.
4. *Sofle Choc: closure hole 0.109 mm from a switch cutout; use the real Choc cutout size* (merge with the Choc item below).
5. *Sofle (all three): right-half independent parts take the ticket-30 headings* — inside tickets 20 and 23; add the 19-part (RGB) and 12-part (v2, Choc) lists to ticket 23’s acceptance.
6. *Sofle RGB: LED D31–D37 and matrix LEDs use different `reverse_mount` (false versus true)* — check against the source board; user (hardware).

**Choc splits (Ferris Sweep, Chocofi, TOTEM, Cantor) and Sofle Choc**

7. *Choc boards: recompute closure-to-switch gaps with the Choc cutout instead of the MX 14 mm assumption* (ticket 06 uses 14 mm for all; the repository records no Choc cutout size). Lane: generator-parameters follow-up. Affects the tightest values (TOTEM 0.048, Cantor 0.031, Sweep 0.495, Sofle Choc 0.109).

**Other measured splits**

8. *KLOR: resolve the 1.42 mm closure-to-bay overlap* (move the mount, widen the bay, or accept). Lane: generator-parameters follow-up. The outline gap repair is already in ticket 20.
9. *TOTEM and Cantor: move or justify closure mounts 0.048 mm and 0.031 mm from a switch cutout.* Same lane; needs a minimum-clearance decision (decision 5).
10. *Corne, Lily58, KLOR, Sweep, Chocofi, TOTEM, Cantor: right `U1`/`RST`/`TRRS` headings* — inside tickets 20 and 23; ticket 30 listed only Corne, so name all seven there.
11. *Fix stale paragraph in `content/demos/README.md`* (says the measured recipes’ openings are local to the right half; the baseline has them linked). Docs, trivial.

**Unibodies and others**

12. *Unibodies (GH60, Discipline, Mysterium, Voyager97, Voyager104, Plaid, Lumberjack, REVIUNG41 measured): decide whether shipped unibodies carry a mechanical configuration (gasket or closure mounts) or stay PCB-only.* Decision (user), then recipes. Separately, *unibodies: controller and reset are recipe placements; qualify or label them* (decision 3).
13. *REVIUNG41: reconcile the archive (gasket, 4 supports, no case body) with the measured recipe (no mechanical configuration)* — the two share all 85 part poses. Lane: keyboards/recipes; user chooses which is the shipped reference.
14. *VIK module review: choose the target host and module variants before treating the placements as qualified.* Decision (user).

## Verification

Sonnet 5.5 reviewer pass, 2026-10-10, in `lane-boards-review` on top of `f39bd5ca9` (the other app’s AI-review pass, which corrected the Corne, Sofle RGB and Sofle Choc gaps and added the ticket 20 item; it landed while this pass was reading, so claims below were first checked against the earlier text and rechecked against it). Read-only: no build, no cargo, no fixture, recipe, ADR or code edits. Evidence was computed with throwaway Python over `core/tests/fixtures/demo-projects-baseline.json`, `content/layouts/*.json`, the archive ZIP and the ticket text.

### Coverage

All 20 projects have an entry and the three Sofle variants, then Corne, Lily58, KLOR, Sweep, Chocofi, TOTEM, Cantor come in the requested order. Two order differences: the entry for REVIUNG41 measured comes first among the unibodies (requested: last of them, before the archive), and the entries follow the baseline’s names. Not covered, so the review is not the full checklist of ticket 25: (a) pads, drill sizes, courtyards, model assets (the baseline stores only a subset; the document says so); (b) matrices and wiring: net and pin assignments (`hardware.boards[*].assignments`) are not reviewed; (c) generated PCB, plate and case results and any export; (d) controller, connector, diode and LED placement is checked for Sofle only here (positions against source JSON), and for the other boards only the diode offset and rotations were checked; (e) per-board ADR-0010 differences are listed only where ticket 30 lists them (extended here to all seven measured splits); (f) no overlap check between peripherals and switches, because the baseline stores no courtyards.

### Claims checked

| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | Part counts of all 20 projects (Sofle 140/220/206, Corne 98, Lily58 130, KLOR 98, Sweep 82, Chocofi 86, TOTEM 90, Cantor 98, REVIUNG41 85, GH60 129, Discipline 142, Mysterium 181, Voyager97 215, Voyager104 218, Plaid 98, Lumberjack 122, archive 85, VIK 33) | CONFIRMED | `fixtures.<name>.expected.parts` lengths; per-half 70/110/103/49/65/49/41/43/45/49 from `boards[*].partIds` |
| 2 | Sofle v2 is a gasket with no closure mounts; 14 NPTH (TH1–TH7 per half); one bay and one TRRS opening, linked | CONFIRMED (wording corrected) | `fixtures.sofle.expected.hardware.instances[*].mechanical.mount == "gasket"`, left `closureMounts: []`, right has no key; `pairs[0].features` = `opening-1`, `opening-2`; 14 `ceoloide/mounting_hole_npth`, none named closure |
| 3 | Sofle RGB and Choc: 4 closure holes per half, 14 + 8 = 22 hole parts | CONFIRMED | 22 `mounting_hole_npth` in both fixtures; 4 `case-closure` parts per board |
| 4 | Sofle RGB closure-to-switch gap 0.009 mm (right) and 1.066 (left) | WRONG as a current value | After linkage both halves are 1.066 mm; 0.009 is ticket 06’s “before” value. Recomputed from the baseline (hole radius 3.3, 14 mm cutout) |
| 5 | Sofle RGB and Choc: closure hole overlaps the right bay by 3.30 mm | WRONG as a current value | Recomputed: hole edge to `opening-1` is 0.10 mm; the opening is a linked pair feature. −3.30 is ticket 06’s “before” column |
| 6 | KLOR closure-to-bay overlap 1.42 mm on both halves | CONFIRMED | Recomputed −1.42 mm; ticket 06 lines 193 and 203. (The “Core does not report” remark is ticket 02:153, not 06) |
| 7 | Switch gaps: Cantor 0.031, TOTEM 0.048, Sofle Choc 0.109, KLOR 0.295, Sweep 0.495, Corne 0.664, Lily58 1.118, Chocofi 1.381 | CONFIRMED | Recomputed to the stated precision for both halves of each |
| 8 | Lily58, Sweep, Chocofi bay clearances 25.74, 27.68, 35.48 mm | CONFIRMED | Recomputed 25.74, 27.68, 35.48 (also Corne 44.99, TOTEM 23.52, Cantor 47.17) |
| 9 | Sweep, Chocofi, TOTEM, Cantor use MX switches at 19.05 mm | WRONG | `ceoloide/switch_choc_v1_v2` in the baseline; `matrices[].pitch` 18 × 17; `keyboard-layouts.json` `choc: true`; `recipes.rs:119`. Corrected, profile row added |
| 10 | Key counts and sources: Sweep “3×6 + 5 thumbs”, Chocofi “3×6”, Cantor “3×10 + 3”, Lily58 and Sweep repository | WRONG | `keyboard-layouts.json` keys and matrices: Sweep 15 + 2, Chocofi 15 + 3, Cantor 18 + 3; repositories `kata0510/Lily58`, `davidphilipbarr/Sweep`. Corrected |
| 11 | Stabilisers: Discipline “four 2u”, Mysterium “five 2u” and a numpad matrix; REVIUNG41 measured has none | WRONG | Baseline definition counts: Discipline 3 × 2u + 1 × 6.25u, Mysterium 4 + 1 (matrices: function, navigation, keys, arrows), REVIUNG41 1 × 2u. GH60 (4 + 1), Voyager97 (6 + 1), Voyager104 (7 + 1) CONFIRMED. Corrected |
| 12 | All switch poses of the 15 measured boards and three Sofles equal the source | CONFIRMED (by a weaker test) | Pairwise distances between sourced switches equal the source’s to under 1e-4 mm, and \|rotation\| equals the source’s, on every cluster of every board, with no unmatched reference. Sofle (left half, 29 switches each): fixture x + source x and fixture y + source y are one constant pair per variant (224.73 and 35.55 for v2; 225.47 and 35.0 for RGB and Choc) and rotation is the negated source (RGB and Choc: negated source minus 180). The right halves were checked only through ticket 06’s and ticket 30’s statements. Absolute coordinates were not recomputed through the recipe |
| 13 | Sofle controller, encoder, mounting holes are measured; TRRS, reset, display are placeholders | PARTLY WRONG | `recipes.rs:289-293`: U1, SW25, TH1–7, D31–D37 at source position (y exact, x = constant − source x: 224.73 for v2, 225.47 for RGB and Choc); J2 +4 mm, RSW1 +1 mm, J3 (+3.81, −16.7) mm offsets. Rotations are recipe constants |
| 14 | Shipped right-half diodes sit at switch-local (+7.40, −1.50), heading +90 like the left | CONFIRMED | Corne, Lily58, KLOR, Cantor, Sofle v2 and RGB, both halves: same local offset and +90 |
| 15 | Ticket 30’s heading list (Corne U1/RST/TRRS; Sofle U1, SW25, J3, J2, RSW1, TH1–7) | CONFIRMED, and wider | Sofle v2: left J2 and RSW1 −90, right +90, all others 0 on both halves. Right `U1`/`RST`/`TRRS` are 0 on all seven measured splits (ticket 30 lists Corne only) |
| 16 | Part side, `side`, `reversible`, `reverse_mount` settings | CONFIRMED | Definitions in the baseline: switches `side:B`, `reversible:false`, hotswap; diodes `include_tht:false`; matrix LEDs `reverse_mount:true`; D31–D37 `reverse_mount:false`, no `reversible`; Choc `choc_v1_support:true`, `choc_v2_support:false`; closure holes `hole_drill`/`hole_size` 6.6 |
| 17 | REVIUNG41 archive: revision 3, 85 parts, gasket 80 × 3 × 2, compression 0.15, travel 0.1, 4 supports, 2 mm wall, 0.05 tolerance, empty closure list, no case body, no TRRS | CONFIRMED | `project.json` in the archive; all 85 part poses and sides equal both the `reviung41` and `measured-reviung41` baseline fixtures |
| 18 | VIK: 33 parts, 13 solder MX, three module placements at x 29/57/85, y 20, 3 mm plate body with 0.5 mm clearance, J_VIK2 on the back | CONFIRMED | `fixtures.vik-module-review.expected`; two of the three THQWGD001 identities occupy matrix cells SW1 and SW2 (the document does not say so) |
| 19 | Ticket 20 `ready-for-agent`, removes the 6.5 mm KLOR allowance; KLOR pocket 19.36 mm, 6.38 mm at x 316–334, y −61 to −69 | CONFIRMED | ticket 20 acceptance; ticket 06 lines 210 and 220 |
| 20 | README: “Unibodies… no pair”; openings local to the right half on the measured recipes | UNVERIFIED as stated, README is STALE | Baseline: `opening-1` and `opening-2` are linked on all nine split fixtures with openings |

### Corrections made

Items 4, 5 (top-ten 2, 4; Sofle RGB and Choc entries; severity list), 6 (attribution), 9 and 10 (profile row, four board entries), 11, 13 (Sofle entries, top-ten 10), 15 (extended to seven splits), plus the Sofle v2 opening wording, the ADR status of decisions 1 and 4 (below) and the order note above.

### Ranking and decisions

Severity: tier 2 is defensible for the KLOR overlap, the sub-millimetre closure gaps, the Sofle v2 mount decision and the heading mismatches. Two judgement calls left to the user: top-ten 3 (two authored halves) is a process mismatch rather than “physically wrong” and could be tier 3; the placeholder item is tier 3 in the top ten but tier 2 in the severity list. Sofle RGB and Choc at 0.10 mm remain tier 2 (near-contact, not overlap). Decisions: 2 (Sofle v2), 3 (placeholders) and 5 (clearance) are real hardware decisions with options and a recommendation, though 3 and 5 recommend collecting evidence rather than choosing. Decision 1 (diodes) and decision 4 (pads) were no longer Core decisions: the user answered them in the ADR-0010 amendment of 2026-10-10 (ticket 31 for diodes); they are reworded above to the physical question that is left.

### Could not determine

Whether any placement fits real hardware (nice!nano, TRRS, display, encoder, Choc cutout); the Choc cutout size (the repository records none); why the Sofle v2 source has the MCU at 270; which diode side the real right-half PCBs use; courtyard or pad overlaps (baseline has no courtyards); wiring and pin assignments; exported PCB, plate and case output; the README’s intended meaning if it describes a different build. The first draft’s first-person statement that every pose was compared with the recipe transform was not reproduced through the recipe itself, only by the distance and rotation test in claim 12.
