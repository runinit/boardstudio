# 002: Establish the real App composition test seam

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Make the real application composition mountable with the existing test Runtime, so
workspace navigation, pending edits, board reconciliation and teardown can be checked
together before moving workspace ownership. Normal startup and user behavior remain
unchanged. This is the prefactoring and characterization gate for every extraction.

## Scope and constraints

- Separate Runtime construction from the existing private App composition without
  changing hook scope, startup policy, the one Runtime subscriber or provider ancestry.
- Reuse production scope transitions, selection reconciliation, header navigation,
  Export-return tracking and Library/Editor mount logic. Do not copy them into a fake
  host, add a second subscriber or install prototype controls in normal startup.
- Use real in-process Session/Core and memory persistence. A scope-correct fixture
  must drive effects without the prototype's cross-scope Signal warnings. Characterize
  any pre-existing diagnostics separately; do not suppress warnings to claim success.
- Establish baseline results against the current checkout. The experimental branch
  proves a possible shape, not permission to cherry-pick it wholesale.

## Acceptance criteria

- [ ] Production startup retains its Runtime constructor and existing behavior; tests inject the existing real test-support Runtime through one private composition seam.
- [ ] Mounted tests use the production subscriber and actual workspace/header controls; board navigation advances the real scope generation/transition and clears selection according to current policy, without manually incrementing generation.
- [ ] A minimal two-board fixture supports real edit submission, filtered reply/save gates, bounded state-based waits and saved/accepted-state assertions. Browser preferences and global DOM/listener state are isolated and restored.
- [ ] Workspace cycling preserves expected surfaces and causes no accepted-document edit. A held field edit can leave/return and settle once without stale feedback or draft writes.
- [ ] Export entry, Back, Escape and header toggle return to the actual last non-Export workspace; production return tracking is not replaced with an alias of the workspace signal.
- [ ] Showing Library hides rather than remounts Editor while permitted work settles. Actual App removal and an actual Editor-only removal exercise Dioxus teardown, not merely root-DOM removal or a Close request.
- [ ] Releasing held work after removal does not revive DOM, navigation or focus or access dropped Signals. Session-owned work follows existing drain/save rules rather than being canceled with its observer.
- [ ] A replacement project reusing document/board/part IDs establishes a new session identity; retained stale UI callbacks cannot act on it. Record where existing controlled-adapter tests reject late replies rather than pretending a serialized Open is post-replacement reply coverage.
- [ ] The fixture's mounted edit, workspace switch, board reconciliation, Export return, gated completion and real teardown run without the prototype's cross-scope warnings or dropped-Signal errors. If safe driving cannot be demonstrated, stop extraction and record the blocking failure.
- [ ] No test diagnostics, alternate persistence, prototype feature, fixture UI or HTML is shipped through normal startup.

## Verification

Run the documented quick checks for the changed scope, page-only WASM compilation,
the page-bin mounted browser suite and relevant Runtime tests. Preserve existing
mounted Editor tests. Record the baseline, exact commands, observed diagnostics and
receipt scope using the plan's verification contract. This ticket does not require
moving any workspace controllers.
