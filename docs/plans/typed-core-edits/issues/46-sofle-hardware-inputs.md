# 46: Pin the hardware inputs for Sofle v2, RGB and Choc

Status: ready-for-agent
Type: research
Blocked by: none
Parent: [25](25-shipped-boards-review.md) · Evidence: [audit](../audit/shipped-boards-review.md#1-sofle-v2)

## Source evidence and settled direction

sofle-layouts.json (`content/layouts/sofle-layouts.json`, retired) pins Josef Adamcik's SofleKeyboard at `fb294f7c58d0f91c379a9ef339159d4d056643e9`, separate source paths/hashes for v2/RGB/Choc, measured switches, outline points and TH1–TH7. recipes.rs (`core/examples/demo_projects/recipes.rs`, retired) chooses library peripherals and datum offsets. The user has selected native board-specific assemblies, real parts, native mounting/outline generation and duplicate-position shipped diodes. This packet does not reopen those choices.

## Agent research: source facts and library qualification

The user has selected **BoardStudio adaptations**, not reproduction of the upstream hardware. Keep every measured key position, tilt and width fixed for each Sofle variant. The user delegated best-fit BOM selection. The supported nice!nano v2 remains accepted, and SuperMini is also accepted; do not introduce a new platform. Screws/spacers may be added. Gasket, plate and tolerance must use BoardStudio’s existing case-generation tool. Assemblies, components and mounts/cases may adapt within those directions; do not ask the user to reselect them.

A read-only planning researcher owns the following facts for a separate row per variant. Cite pinned primary sources and exact library definitions; record unknowns and missing onboarding rather than asking the user to research them.

| Research input | Required evidence and proposal |
| --- | --- |
| Source components | Identify the pinned source controller/socket, encoder, display, reset, TRRS, diode, switch/socket and LED variants, terminals and sides. Distinguish source identity from a candidate for the adaptation. |
| Datum/headings | Explain source/library pad and body datums, including U1/encoder/TH measurements, J2 +4 mm, reset +1 mm, J3 (+3.81, −16.7) mm recipe offsets and v2 source U1 270° versus recipe 0°. Do not remove offsets solely because origins differ. |
| RGB | Identify matrix versus D31–D37 package, pins, DIN/DOUT/power mapping and installed orientation. Current matrix LEDs use reverse_mount:true, D31–D37 false, and Choc has no D31–D37 bank; qualify these facts against the pinned source. Propose any needed companion-mode exceptions. |
| Existing profiles | Inspect exact native assembly and MX/PG1350/PG1353 mechanical profiles, footprint parameters, socket/plate datums and supported thickness. Current v2/RGB use MX; Choc uses V1 on measured 19.05 mm relationships. Propose compatible candidates while preserving all measured keys. |
| Library and mirror suitability | Inventory candidate library IDs and inspect pins, artwork, pads, drills, models, sides and reversibility. Separate qualified evidence from unverified claims. Core uses 180-r for independent parts, -r for switches and adds no pad-symmetry finding; physical suitability needs evidence. Missing exact entries require separately scoped onboarding. |
| Mount proposals | Inventory the existing native case generator’s gasket, plate, tolerance, mount and clearance capabilities and propose supported options. Use its workflow and parameters rather than importing upstream manufacturing specifications or inventing independent geometry. TH1–TH7 are source holes, distinct from generated closures; current v2 gasket/empty closure list and RGB/Choc closure proposals are historical evidence, not selections. Provide numerical candidate envelopes/clearances with sources and limitations. |

## Current hardware-selection contract (2026-10-11)

The user delegated best-fit Sofle BOM selection. The supported nice!nano v2 remains an accepted choice; SuperMini is also accepted. Do not make hardware selection a user gate or add a platform. Record actual source, library, terminal, geometry, electrical, and physical-fit evidence gaps as evidence work; use an existing-library equivalent first and normal onboarding for an adapted reusable footprint. Keep measured Sofle layout data fixed and preserve genuine electrical/physical qualification limits.

## Ownership and exit

The planning researcher records sourced facts, BOM selection, and qualification gaps here after output ownership is assigned. User approval is not a BOM gate. Keep research completion and design approval as separate records (or link an existing qualified library entry). Read [component onboarding](../../../hardware/component-onboarding.md); a missing exact library entry needs a small separately numbered onboarding ticket, owned before the board component slice is dispatched. Do not assume a nice!nano is interchangeable with an unidentified source controller.

- [ ] Agent source/library research is complete per variant, with exact evidence, proposed adaptations and unresolved qualification gaps.
- [ ] Record selected supported BOM and sourced qualification gaps per variant; unresolved physical/electrical evidence remains explicit.
- [ ] Mount/profile/clearance choices are numerical, sourced and per variant; no physical fit is claimed from export success.

Docs-only `repo` when answers land. The prior 47–55 implementation tickets are retired. Ticket 56 retains the separate human review and sign-off gate before other boards.
