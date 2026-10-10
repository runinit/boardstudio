# 46: Pin the hardware inputs for Sofle v2, RGB and Choc

Status: ready-for-human
Type: research
Blocked by: none
Parent: [25](25-shipped-boards-review.md) · Evidence: [audit](../audit/shipped-boards-review.md#1-sofle-v2)

## Source evidence and settled direction

[sofle-layouts.json](../../../../content/layouts/sofle-layouts.json) pins Josef Adamcik's SofleKeyboard at `fb294f7c58d0f91c379a9ef339159d4d056643e9`, separate source paths/hashes for v2/RGB/Choc, measured switches, outline points and TH1–TH7. [recipes.rs](../../../../core/examples/demo_projects/recipes.rs) chooses library peripherals and datum offsets. The user has selected native board-specific assemblies, real parts, native mounting/outline generation and duplicate-position shipped diodes. This packet does not reopen those choices.

## Agent research: source facts and library qualification

The user has selected **BoardStudio adaptations**, not reproduction of the upstream hardware. Keep every measured key position, tilt and width fixed for each Sofle variant. The user will specify the target installed hardware/BOM; that input is pending and agents must not choose it. Screws/spacers may be added. Gasket, plate and tolerance must use BoardStudio’s existing case-generation tool. Assemblies, components and mounts/cases may adapt within those directions; do not ask the user to reselect them.

A read-only planning researcher owns the following facts for a separate row per variant. Cite pinned primary sources and exact library definitions; record unknowns and missing onboarding rather than asking the user to research them.

| Research input | Required evidence and proposal |
| --- | --- |
| Source components | Identify the pinned source controller/socket, encoder, display, reset, TRRS, diode, switch/socket and LED variants, terminals and sides. Distinguish source identity from a candidate for the adaptation. |
| Datum/headings | Explain source/library pad and body datums, including U1/encoder/TH measurements, J2 +4 mm, reset +1 mm, J3 (+3.81, −16.7) mm recipe offsets and v2 source U1 270° versus recipe 0°. Do not remove offsets solely because origins differ. |
| RGB | Identify matrix versus D31–D37 package, pins, DIN/DOUT/power mapping and installed orientation. Current matrix LEDs use reverse_mount:true, D31–D37 false, and Choc has no D31–D37 bank; qualify these facts against the pinned source. Propose any needed companion-mode exceptions. |
| Existing profiles | Inspect exact native assembly and MX/PG1350/PG1353 mechanical profiles, footprint parameters, socket/plate datums and supported thickness. Current v2/RGB use MX; Choc uses V1 on measured 19.05 mm relationships. Propose compatible candidates while preserving all measured keys. |
| Library and mirror suitability | Inventory candidate library IDs and inspect pins, artwork, pads, drills, models, sides and reversibility. Separate qualified evidence from unverified claims. Core uses 180-r for independent parts, -r for switches and adds no pad-symmetry finding; physical suitability needs evidence. Missing exact entries require separately scoped onboarding. |
| Mount proposals | Inventory the existing native case generator’s gasket, plate, tolerance, mount and clearance capabilities and propose supported options. Use its workflow and parameters rather than importing upstream manufacturing specifications or inventing independent geometry. TH1–TH7 are source holes, distinct from generated closures; current v2 gasket/empty closure list and RGB/Choc closure proposals are historical evidence, not selections. Provide numerical candidate envelopes/clearances with sources and limitations. |

## Human design gates: approve concrete adaptation proposals

After research, present a short per-variant proposal with recommendation, alternatives and unresolved evidence. The user approves only choices source inspection cannot decide:

- The user supplies exact target installed hardware/BOM. Agents then qualify matching library entries and compatible assemblies, identifying any deliberate source deviation or reversible substitution that needs approval. No automatic BOM choice or nice!nano selection from the current placeholder.
- Any still-open mounting choice and installed fastener/spacer dimensions, with screws/spacers allowed. Present supported case-generator options and the genuine remaining material/process choices; derive gasket and plate geometry through that tool.
- Any genuinely unresolved case-generator tolerance setting or minimum closure/component/switch/service margin, including an explicitly accepted limitation. Inventory existing defaults and supported settings first; use the case generator for tolerance rather than demanding unrelated upstream fabrication specifications.

Research can proceed while these gates are open. Hardware-dependent board implementation remains blocked until the relevant proposal is approved and its required evidence exists. Successful generation does not establish physical fit or electrical qualification.

## Ownership and exit

The planning researcher records sourced facts and proposals here only after output ownership is assigned; the user records approval of design choices. Keep research completion and design approval as separate records (or link an existing qualified library entry). Read [component onboarding](../../../hardware/component-onboarding.md); a missing exact library entry needs a small separately numbered onboarding ticket, owned before the board component slice is dispatched. Do not assume a nice!nano is interchangeable with an unidentified source controller.

- [ ] Agent source/library research is complete per variant, with exact evidence, proposed adaptations and unresolved qualification gaps.
- [ ] The user approves the required installed-hardware/mount/process/numerical design choices per variant, or unresolved rows keep the affected tickets `needs-info`.
- [ ] Mount/profile/clearance choices are numerical, sourced and per variant; no physical fit is claimed from export success.

Docs-only `repo` when answers land. This packet unblocks 47–55 only when the required variant's answers are complete; use separate decision follow-ups if one variant would otherwise stall the others. The Sofle user sign-off in ticket 56 still gates every later board.
