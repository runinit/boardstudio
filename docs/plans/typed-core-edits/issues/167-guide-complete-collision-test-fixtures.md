# 167: Guide authors to the complete collision-test fixture

Status: ready-for-agent
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [Fixture onboarding research](../../../research/retro-collision-fixture-onboarding.md)

## What to build

Give an agent assigned a complete Runtime collision-check test a short, conditional
navigation entrypoint to the existing fixture, publication helper, controlled worker
and independent operand oracle. The author should be able to preserve the current
ownership and readiness prerequisites without reconstructing them across unrelated
modules. Put the guidance in a module-level note beside the collision browser tests,
with an inbound link from the existing collision-ownership paragraph in the
architecture reference. Task handoff text alone does not deliver this change.

The preceding test lane spent 18m45s reconstructing context, with 53 command
executions and 34 file reads. This is a session observation, not a measured savings
target or evidence that the model itself needs replacing. Research found an
existing reusable fixture; this ticket does not introduce another fixture framework.

## Acceptance criteria

- [ ] The module-level note identifies the existing complete-check fixture,
  preview publication helper, controlled worker and literal operand oracle through
  portable file links and symbol names rather than fragile line anchors. The
  architecture's existing collision-ownership paragraph links to this note, making
  it reachable from established project navigation without global instruction growth.
- [ ] Guidance explains matching preview owner/lease and delivered rows, a current
  accepted scene and view, and no pending model delivery before a check. It points
  to the enforcing source rather than creating a second authoritative contract.
- [ ] Guidance identifies the current literal seven-operand, 21-unique-pair oracle,
  including duplicate/self-pair rejection, and explains why expected membership
  must not be derived from observed results.
- [ ] Controlled-worker coverage is explicitly distinguished from actual provider
  geometry qualification and packaged-page acceptance; Unknown remains Unknown.
- [ ] Start a representative complete-check test walkthrough from the existing
  architecture reference, follow its new inbound link, and locate all required
  helpers and prerequisites. Do not start at the new note or claim a time saving.
- [ ] Run the repository documentation-link check, manually verify the module note's
  symbol references and links not covered by the Markdown check, and read the diff. No
  runtime behavior, visibility changes, new worker abstraction or retired workflow
  instructions are needed; if scope expands, identify that before implementation.

## Comments

Created at the user's request from retro item 2 after primary-source research.
Independent of the other three retro follow-ups.

Revised after Claude Opus audit: name the local landing point and inbound link,
and verify discovery from an established reference rather than from the new note.
