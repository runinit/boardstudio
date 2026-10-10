# 45: Decide whether reversible-variant substitution needs a separate workflow

Status: ready-for-human
Type: research
Blocked by: none
Parent: [25](25-shipped-boards-review.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Decision already made

[Human decision 5](../audit/shipped-boards-review.md#human-decisions-2026-10-10): flip only an explicitly reversible footprint or a geometry-symmetric part whose flip is safe. A reversible substitution is a separate, visible choice; mirroring preserves generator parameters and side. ADR-0010 does not add pad-symmetry checks. A non-polar component's class/name alone does not prove its installed footprint is symmetric.

## Narrow open question

Does per-board, explicit selection of a verified reversible library variant satisfy the current rebuild, or should a later user-facing operation propose substitutions before pairing? The latter was to be considered, not already mandated. This ticket is not a dependency of the Sofle or later board rebuilds; those can choose qualified variants explicitly.

## Work and ownership

A planner/user owns this ticket only. Record one decision: defer a general workflow, or request a separate build plan with exact eligible parts, whether substitution changes existing document identities, what the user reviews, and the required Undo/reopen/parameter tests. If requested, split UI choice, conversion/edit behavior and hardware qualification into bounded tickets; do not silently add them to 20, 23 or a running board ticket. The orchestrator owns any ADR proposal.

## Acceptance and verification

- [ ] The decision distinguishes footprint selection/substitution from the Core reflection operation and states whether more work is wanted now.
- [ ] Per-board hardware packets retain exact reversible/symmetry evidence and uncertain physical cases remain gated; no automatic side or parameter mutation is approved by this decision ticket.

Docs-only `repo` after an answer. No implementation, browser/build or hardware claim is required here. Hardware compatibility remains per part and board revision.
