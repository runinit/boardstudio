# 79: Pin native unibody hardware and mounting profiles

Status: wontfix
Type: research
Blocked by: [56](56-sofle-sign-off.md)
Parent: [25](25-shipped-boards-review.md) · Evidence: [audit](../audit/shipped-boards-review.md#12-gh60)

## Sources and sequence

Measured input pins (`content/layouts/keyboard-layouts.json`, retired) identify GH60, Discipline, Mysterium, Voyager97 (103-key variant), Voyager104, Plaid, Lumberjack and measured REVIUNG41. They are rebuilt in that order after the seven splits. Their adapted fixtures have native matrix keys/diodes plus generated nice!nano/reset placements, but no authored mounting holes, mechanical configuration or case body. This absence does not approve a PCB-only result: the user requires native outline/mounting generation for unibodies too.

## Agent research: one evidence/proposal row per board

Agents inspect the pinned source revision/configuration and propose a BoardStudio adaptation; identify exact controller/reset and other required components, stable qualified library IDs, terminal/pin mapping, datum/side, artwork/pad/drill/model evidence, source licence/hash and service envelope. Do not substitute the recipe's nice!nano for the source controller without a designer choice. Recreating a full through-hole circuit or adding a missing library part is a separate bounded ticket.

Agents inspect profile/datums and propose native outline/mounting options, sourced hardware and process constraints, and numerical candidate clearances. Key JSON does not contain a source boundary; generated geometry or a newly pinned imported boundary must be deliberate. Preserve measured 19.05 mm overrides and widths, independent from the global preference.

Stabiliser configuration requires specific variant/fit evidence: GH60 4×2u + 1×6.25u; Discipline 3+1; Mysterium 4+1; Voyager97 6+1; Voyager104 7+1; REVIUNG41 1×2u; Plaid/Lumberjack none currently. Confirm changes rather than assume counts alone qualify a footprint. Lumberjack's two clusters and REVIUNG41's left/right/thumbs remain one board, not a mirror pair.

## Human design gates

Agents research source/controller/component identities, library pin/datum/side/artwork/pad/drill/model qualification and stabiliser fit evidence. Present bounded sourced adaptation proposals rather than asking the user to supply these facts. The user approves installed components and deliberate source deviations, native outline/mounting system and hardware, plate/case materials and fabrication process, numerical tolerances and minimum accepted clearances. Record approvals separately from research completion; any required unknown or unapproved choice keeps the affected implementation blocked. Physical/electrical qualification remains explicit.

## Ownership and exit

The planner owns source research and proposals; the user owns design approvals; use [component onboarding](../../../hardware/component-onboarding.md). Preserve the original REVIUNG41 archive as a historical reference; the measured recipe is the editable rebuild target. Its archived legacy gasket settings are not a mount decision for the measured example. VIK remains outside this sequence.

- [ ] Each board's source-backed component/mount/clearance evidence and genuine human choices are recorded; unknown physical/electrical fit stays explicit.
- [ ] Do not infer acceptance or physical fit from a saved project or successful export.

Record research completion separately from human approval. Tickets 80–103 are retired; ticket 25 governs ordinary saved-project delivery. Ticket 23 retains cross-board review/integration, and 104 retains original-archive preservation evidence. Preserve genuine source-backed hardware, mounting, material/process, tolerance and clearance decisions as human gates; no physical/electrical qualification is implied by saved project output.

## Comments

- 2026-10-14: User deferred built-in board work under [138](138-retire-shipped-demos.md). This ticket's board-specific work is out of scope for now; re-triage only if the user brings built-in boards back into scope. Existing findings remain historical evidence, not implementation instructions.
