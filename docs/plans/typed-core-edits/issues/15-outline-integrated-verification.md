# 15: Integrated outline verification and independent reviews

Status: ready-for-agent
Type: task
Blocked by: 14
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Contract: [03 Answer](03-decide-outline-version-intents.md#answer)

## Goal

Verify the complete typed/linked outline contract, not individual helper completion.
Use real Session/Core pending edits, portable archive and browser persistence, mounted
presentation, exact selected-output gates and the combined native/WASM transcript.
Record independent Standards and Spec reviews before accepting tickets 11-14.

## Required cases

1. Matrix/mechanical changes and outline actions in both queue orders use current
   accepted geometry and one Undo step per accepted action. Fixed alternatives stay
   fixed; linked alternatives follow source placement/geometry/inclusion changes.
2. Component removal cleans only dependent outline choices, recalculates bridges and
   restores all of it on one Undo. Recreated entities do not receive old pending edits.
3. Active-version/relevant-source changes retire pending shape edits. Unrelated edits
   survive; malformed input errors atomically; no-op field edits add no history.
4. Save/reopen and browser writeback preserve link status, IDs, settings, active choice
   and exceptions. Freeze keeps both linked and edited versions with correct ownership.
   Legacy projects do not gain links or move geometry.
5. Findings identify affected board/version/choice/location; inactive alternatives and
   unrelated boards do not block valid exports. Saving and repair remain available.
6. Append typed outline requests to the combined native/WASM transcript without
   silently replacing prior matrix/mechanical evidence. Bless only reviewed semantic
   changes at the final integrated source; use one transcript owner.
7. Run the actual product: accept a refinement, move/rotate/add/remove source components,
   change settings, Freeze/edit a copy, Undo/Redo and reopen. Verify the visible boundary,
   bridges, ownership indicators and output gates, not merely serialized enum values.

## Gates

- `CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target python3 scripts/check.py`
  (all steps), plus affected Core/application and mounted Layout/browser suites.
- One coordinated Chrome owner. Native Rust tests alone do not validate WASM UI.
- A non-nested validation worktree is allowed for Cargo's separate CAD workspace
  discovery; retain pinned OCCT provisioning and existing CAD assertions/tolerances.
- Refresh the graph and run complete change analysis against integrated source before
  commits. Partial/truncated evidence is not a clean gate; document static-analysis
  limits instead of treating zero callers/processes as safety.
- Independent Standards and Spec review, fixes and affected rechecks. Review the whole
  confirmed Answer, not just the latest implementation diff.

## Outcome

Record integrated commits, per-case evidence, executed checks and review dispositions.
Keep build tickets open for any unmet criterion. Ticket 03 records design approval only;
these Outcomes establish implementation acceptance. Mechanical 02 and the separate
keyboard/matrix follow-ups retain their own required gates and scope.
