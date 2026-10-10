# 57: Pin per-board hardware for the seven measured splits

Status: ready-for-human
Type: research
Blocked by: [56](56-sofle-sign-off.md)
Parent: [25](25-shipped-boards-review.md) · Evidence: [audit](../audit/shipped-boards-review.md#4-corne)

## Settled direction and sources

After Sofle sign-off, rebuild Corne, Lily58, KLOR, Ferris Sweep, Chocofi, TOTEM and Cantor in that order. [keyboard-layouts.json](../../../../content/layouts/keyboard-layouts.json) pins every repository/revision/path/hash and key pose. Ticket 20 owns native half construction, right-authored Lily58/KLOR and KLOR outline reflection. The adapted recipes' generated U1/RST/TRRS and fixed bay coordinates are not source peripheral evidence.

## Agent research: one evidence/proposal row per board

For each board, agents inspect the pinned source and propose a BoardStudio adaptation; identify exact switch/socket/diode/controller/reset/transport components, library identities and terminal/pin mapping; source datum/side/reversibility and actual artwork/pad/drill/model qualification. Confirm whether an assumed TRRS/controller/reset actually belongs to that target. Pin any approved safe variant substitution; a missing library match gets its own component-onboarding ticket.

Corne/Lily58/KLOR currently use MX 19.05 mm. Sweep/Chocofi/TOTEM/Cantor currently use Choc V1 at 18 × 17 mm; preserve these measured overrides. Agents inspect the existing switch families and plate profiles/datums and propose a compatible assembly and component service envelope with sources. Native Core has separate MX/Choc V1/V2 profiles with provenance; the old clearance test's 14 mm square does not qualify hardware.

Agents propose native outline and mounting options with sourced fastener/gasket/support dimensions, process constraints and numerical candidate clearances. The layout JSON contains keys, not a sourced board boundary; generated native geometry is an explicit design choice, and importing a different boundary needs a source pin. Old four-closure coordinates are not requirements. Record KLOR's 1.42 mm overlap and tight 0.295/0.495/0.048/0.031 mm KLOR/Sweep/TOTEM/Cantor gaps as historical comparison only.

## Human design gates

Agents own source identities, pin/datum/side facts and library/artwork/pad/drill/model qualification; these are research tasks, not questions for the user. Present a concrete sourced proposal per board. The user approves the installed component configuration and deliberate source deviations, native outline/mounting choice, actual hardware, plate/case materials and fabrication process, numerical tolerances and minimum acceptable clearances. Keep these approvals separate from research completion. Missing evidence or approval keeps the affected implementation blocked; source facts alone cannot select hardware or qualify fit.

## Ownership, exit and verification

The planner owns source research and proposals in this packet; the user owns design approvals. Use [component onboarding](../../../hardware/component-onboarding.md) and preserve ADR-0010: duplicate-position shipped diodes, per-item overrides when evidenced, switches -r, independent headings 180-r, no mirror side/parameter mutation or Core pad check.

- [ ] All seven rows are complete or their components/mechanics tickets remain `needs-info`; no unknown becomes a fabrication claim.
- [ ] Questions are per-board and source-backed. If answers arrive separately, split this packet before claiming it so one answer cannot falsely unblock all seven.

Record agent research completion and human design approval separately. Docs-only `repo` after updates. Tickets 58–78 own the small implementation slices and ticket 23 retains cross-board integration/reviews/full check. No remaining-board implementation starts before 56 resolves.
