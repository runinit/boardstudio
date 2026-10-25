# 37: Outline and Net dead-surface triage

Status: ready-for-agent
Type: build
Blocked by: None (all dependencies resolved; the live outline and net surfaces are established, so each dead variant can be judged against them)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Triage every Core edit variant with no live surface caller, and for each one
either wire it to a real Inspector or canvas surface or delete it, so no
dead outline or net variant remains in the Core contract.

## First RED / GREEN slice

Fail a mounted test: the triage table in this ticket names every dead
variant in the SelectOutline family, SetNet, and the superseded mechanical
and outline operations, with a wire-or-delete verdict for each, and follow-up
work proves the table is empty by the ticket's close.

## Subsequent acceptance

- [ ] Each SelectOutline-family variant is either reachable from a real
      surface with acceptance proof or deleted with dependents updated.
- [ ] SetNet is either reachable from a real surface with acceptance proof or
      deleted with dependents updated.
- [ ] Each superseded mechanical and outline operation is either reachable
      from a real surface with acceptance proof or deleted with dependents
      updated.
- [ ] No dead Core variant remains: enumeration of the Core edit contract
      shows every surviving variant has a live caller or an explicit test.

## Checks

Triage table plus mounted browser tests for each wired surface (or deletion
proof with dependents updated); affected quick/browser checks.
