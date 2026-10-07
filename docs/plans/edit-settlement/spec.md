# Spec: Edit settlement

Status: ready-for-agent
Source: [architecture review, candidate 1](../../investigations/architecture-review-2026-10-06.html#c1),
[layout part-editing investigation](../../investigations/layout-part-editing.md),
[ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md).
Map: [map.md](map.md). Tickets: [issues/](issues/).

## Problem Statement

A user editing a keyboard can lose an edit without being told. If two edits are
committed in quick succession, for example X then Y in the Layout Inspector, the
second can quietly put back the value the first had just changed. Both edits report
success, and Undo then shows two steps for what looked like one change. This affects
every panel that builds an edit from the accepted document it last saw, not only the
Inspector. About half of the product's edits replace the whole document, so a queued
edit can undo any change accepted after it was prepared.

The same weakness shows up in the code. Each panel works out separately whether its
edit "really" landed, using its own mix of snapshot tokens, revisions and save
states. Each also writes its own failure messages. There is no single place to fix
or test edit behaviour, and the mounted browser tests for the Inspector stop
before the edit reaches the document engine, so they cannot catch the overwrite.

## Solution

Each pending edit carries intent: what the user asked for, such as "set X of the
selected parts to 60". Application resolves that intent against the accepted document
immediately before the edit runs, so every edit applies on top of the ones accepted
before it. Application reports exactly where each edit landed (the accepted revision
it produced or confirmed). It retires an edit whose target has disappeared, with a reason.

Presentation uses one edit ticket module to submit an edit and read its
settlement: pending, landed, failed or retired. While an edit is pending, the field
keeps the user's draft value. If it fails or is retired, the field shows the
accepted value again with an inline explanation, and nothing retries automatically.

The browser runtime gains an in-process document engine and persistence adapter
for tests, so mounted forms can be exercised all the way through acceptance, saving
and Undo with controlled timing.

## User Stories

1. As a keyboard designer, I want to type X, press Tab, type Y and press Enter quickly, so that the part ends up at both coordinates I entered.
2. As a keyboard designer, I want one Undo after a rapid X/Y entry to remove only the Y change, so that history matches what I did.
3. As a keyboard designer, I want edits made in two different panels in quick succession to both apply, so that working fast never silently discards a change.
4. As a keyboard designer, I want a pending edit to keep showing the value I typed, so that the field doesn't flicker back while it saves.
5. As a keyboard designer, I want a field to show the accepted value again when my edit fails, so that what I see matches the saved keyboard.
6. As a keyboard designer, I want an inline explanation when my edit fails or can't apply, so that I know whether to retry.
7. As a keyboard designer, I want an edit to a part that was deleted before my edit ran to be retired with a clear reason, so that nothing is applied to the wrong part.
8. As a keyboard designer, I want an edit that would change nothing to complete quietly, so that I'm not shown errors or empty Undo steps.
9. As a keyboard designer, I want edits made while saving is in recovery to be refused with the recovery reason, so that I don't lose work behind a failed save.
10. As a keyboard designer, I want edits I committed before opening another project to be dropped, not applied to the new one, so that projects never mix.
11. As a keyboard designer, I want group edits (several selected parts) to still move as one translation anchored on the first selected part, so that group behaviour is unchanged.
12. As a keyboard designer, I want locked and relationship-driven parts to stay protected in queued edits, so that resolution doesn't bypass eligibility rules.
13. As a keyboard designer, I want untouched two-decimal fields to keep their precise geometry when they lose focus, so that viewing values never rounds my layout.
14. As a keyboard designer, I want Enter followed by blur to commit once, so that one action never creates two edits.
15. As a keyboard designer, I want Escape to discard my draft value, so that I can abandon an edit before committing it.
16. As a keyboard designer, I want a panel I've closed or navigated away from to stop reacting to its pending edit's result, so that stale messages never appear elsewhere.
17. As a keyboard designer, I want edits from panels other than the Inspector (matrix, keymap, keycaps, PCB wiring, outline, mechanical, parts, project and board names) to follow the same pending/landed/failed behaviour, so that the whole app feels consistent.
18. As a keyboard designer, I want failure messages worded consistently across panels, so that I learn one set of meanings.
19. As a keyboard designer, I want drag gestures to behave exactly as before, so that the change doesn't disturb direct manipulation.
20. As a keyboard designer, I want PCB handoff and electrical export commits to keep their strict "changed while exporting" protection, so that exports never use a document they didn't check.
21. As a developer, I want one application-level interface for submitting an edit as intent, so that new panels can't reintroduce the overwrite.
22. As a developer, I want each settlement to state the revision an edit landed at, so that I never infer success from tokens and save states.
23. As a developer, I want one web edit ticket module that turns outcomes into pending/landed/failed/retired, so that each controller only supplies its owner liveness and feature wording.
24. As a developer, I want to test edit resolution natively against the real Session and document engine, so that the overwrite case is a fast regression test.
25. As a developer, I want to mount a form against an in-process engine and in-memory persistence with controllable completion, so that browser tests assert accepted results and Undo, not just captured events.
26. As a developer, I want the feature-specific test interception modes removed from Runtime, so that tests use the same path production does.
27. As a developer, I want queued whole-document replacements resolved against the accepted document, so that they're correct even before they become typed edits.
28. As a developer, I want a list of which whole-document replacements remain after migration, so that choosing typed Core edits next is evidence-based.
29. As a maintainer, I want the investigation's reproducer to be a permanent regression test, so that the overwrite can't silently return.
30. As a maintainer, I want the per-controller `Pending*` lifecycle structs gone, so that settlement logic has one home.

## Implementation Decisions

- **Application owns admission and resolution.** A new Session event submits an
  edit as intent: an owner-captured resolver that receives the accepted snapshot
  and returns one of *submit this commit command*, *unchanged* or *retire with
  reason*. Session calls it only when the edit reaches the head of the queue, and
  only while no Core request or save is in flight, so the snapshot it passes is
  exactly the document Core will apply the command to. The command's base revision
  is set from that snapshot. Resolvers must be pure functions of the snapshot and
  the values they captured: they must not read signals or Runtime state. Resolved
  commands are always commit phase, never preview.
- **Existing `Event::Edit` stays during migration** with its current
  base-revision refresh. After migration it is reserved for payloads that are valid
  against any revision (field-scoped commands). Whether to enforce that at the type
  level is an open map decision.
- **Settlement reports where an edit landed.** The Session's terminal settlement
  carries an optional landing (accepted revision and snapshot token) for completed
  edits, Undo/Redo, gesture commits and opens. *Unchanged* resolutions settle as
  completed, landing at the current accepted snapshot, without a Core request.
  *Retire* settles as a rejection carrying the resolver's reason. There is no new
  terminal outcome variant, which avoids a 137-site migration.
- **Strict paths are unchanged.** Gesture commit, export-owned commits and
  electrical remap review keep their captured-revision checks.
- **The edit ticket lives in the web runtime crate** (no Dioxus, compiled natively),
  so native tests can compile it. Its interface is: begin a ticket from an owner and
  a resolver; read a settlement given the current read model and an owner-liveness
  answer. Settlement is one of *Pending*, *Landed { revision }*, *Failed {
  message }* or *Retired*. Retired means the owner is gone or the session moved
  on, and the caller shows nothing. The ticket owns observation, transaction IDs,
  standard failure wording and the rule that a failure returns the field to the
  accepted value. It reaches Runtime through a small submit/observe port with two
  adapters: Runtime itself, and a native driver over a real Session and
  `CoreEngine` used in tests.
- **The Runtime test adapter covers Core and persistence only.** Runtime executes
  Core requests and saves through two ports. Production adapters wrap the Core
  worker and browser storage. One test adapter runs `CoreEngine` in-process and
  keeps saves in memory, with gates to hold or fail a reply or save. This
  generalises the existing project-name test support. The layout-inspector and
  definition-name interception modes are deleted once their tests move to the
  adapter. CAD, export and preview executors are out of scope.
- **Migration is expand–contract.** First the new event, landing and ticket are
  added beside the old forms. Then each feature cluster's call sites move over, with
  the Layout Inspector as the tracer bullet. Contract happens last: `Pending*`
  settlement structs and interception modes are deleted, and the investigation and
  backlog entries are closed.
- **Whole-document replacements migrate by moving their clone-and-mutate step into
  the resolver.** That makes them correct against the accepted document without new
  Core operations. Typed Core edits (candidate 2) follow as a separate decision
  informed by what remains.
- **Glossary.** Use *accepted document*, *draft value*, *pending edit* and *landed
  edit* as defined in [CONTEXT.md](../../../CONTEXT.md).

## Testing Decisions

- Good tests here assert externally visible results through the highest seam: the
  accepted document, the settlement, Undo history and the field text the user sees.
  They don't assert command shapes or internal queue state.
- **Application seam (native):** Session plus the real `CoreEngine` driving queued
  intents, with completions and saves stepped by hand. Prior art is the
  `application` crate's durable session tests. Required cases:
  - X then Y both survive, and Undo removes Y then X;
  - a resolver sees the previous commit's result;
  - a vanished target retires;
  - an unchanged resolution completes with no new revision;
  - recovery blocks;
  - an epoch change drops queued intents;
  - each completed outcome carries its landing.
  The existing test asserting base-revision refresh for plain `Event::Edit` stays,
  because that path stays.
- **Edit ticket (native):** a table of outcomes mapped to settlements, plus owner
  liveness. It runs against the native Session driver adapter. Prior art is the
  web crate's operation-outcome tests.
- **Mounted forms (browser):** the Layout Inspector suite runs against the
  in-process Runtime adapter and asserts accepted X/Y and Undo after rapid entry,
  revert-on-failure, and the existing focus, Enter/blur and precision cases. Prior
  art is the project-name mounted tests. Browser tests stay for focus and event
  ordering only. Each new browser test is registered with the browser test
  owners/baseline tooling described in the README.
- Feature clusters keep their current behavioural tests. Assertions about captured
  events or `Pending*` internals become accepted-result assertions where the
  in-process adapter makes that possible.

## Out of Scope

- Typed, intent-level Core edits that replace whole-document replacements
  (architecture review candidate 2). They are tracked on the [map](map.md) as a
  later decision, not built here.
- CAD, export, preview and generation ports in Runtime (rest of candidate 3), and
  shared export capture leases (candidate 7).
- Changing gesture, export-commit or electrical-remap admission.
- Visual redesign of pending or error states beyond the inline message and value
  revert.
- Automatic retry.

## Further Notes

- The investigation's constraints still apply: keep selection lifetime separate
  from accepted snapshot identity, preserve raw geometry on untouched blur, keep
  first-selected anchoring for groups, and never make presentation drafts a second
  document store.
- Line references in tickets are orientation only. They were taken at `9548275`
  and will drift.
