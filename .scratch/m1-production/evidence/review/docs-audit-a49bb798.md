# Documentation audit — integration `a49bb798`

Read-only audit of `.scratch/m1-production/spec.md`, `PLAN.md`, `ACCEPTANCE.md`,
`HANDOFF.md`, `BUILD.md`, tickets 02/03/06, `docs/migration/RUN.md`,
`docs/migration/m1-production-run.json`, `TODO.md`, and saved evidence available
at integration HEAD `a49bb798cf751d0f111c265c29d7276841e513b9`. This is a checkpoint
review, not an M1 acceptance decision. No canonical documentation or production
source was changed. The final maintained build and remaining browser/performance
gates were still underway at review time.

## Findings

1. **Current-run state is stale for the latest public result.**
   `docs/migration/m1-production-run.json:3,41` says active-project restoration
   and physical-instance UI are “undergoing checks” and that browser verification
   is pending. Host evidence now records focused public green checks at
   `.scratch/m1-production/evidence/integration/startup-restore-instance-focused-b50ddbdd-qa/record.json`:
   same-origin reload restore, stale-preference fallback, board-filtered
   instance options, canonical reset on board change, and keyboard selection.
   The built candidate is a focused root-only artifact, source `b50ddbdd`; it
   intentionally omits offline policy and does not test subpath, delayed IndexedDB
   reads, or slow-WASM/newer-open races. Refresh the state to “focused online
   root checks pass; final maintained-build/release checks remain pending”,
   preserving those explicit limits. Keep the release source/build fields pinned
   to their actual artifact rather than presenting the focused build as final.

2. **The RUN story-audit section stops at the red reproduction.**
   `docs/migration/RUN.md:970-981` records the `b9748745` red and says stories 4/5
   remain under implementation, but no following entry links the later
   implementation, final reviewed source, or focused green record. Retain the red
   as history, then append the exact source/build and green evidence, with the
   root-only/offline/race limitations above. RUN's opening summary at lines 6-11
   similarly says restoration/instance selection are being completed; distinguish
   the focused green from the final release gate.

3. **Acceptance and handoff summaries lack the story-completion overlay.**
   `ACCEPTANCE.md:36-43` says active restore/instance selection are implementation
   gaps “under repair”; this is now out of date for focused online root behavior.
   Its retained-evidence framing should make clear that these particular checks
   passed on the focused candidate while final-build, subpath/offline and race
   checks remain open. `HANDOFF.md:19-21` says the features are still being
   completed. That statement is accurate for the named `b9748745` artifact, which
   predates the implementation, but omits the newer focused candidate; add a
   separate candidate note rather than silently changing the older artifact's
   claim. `BUILD.md:3` has the same distinction to make: b974 remains the latest
   completed maintained release at this checkpoint, while focused story repairs
   are verified and final maintained rebuild/QA is pending.

4. **Public-session test count is understated after the new regression.**
   Ticket 02 line 7 and the acceptance/handoff summaries (`ACCEPTANCE.md:27`,
   `HANDOFF.md:42`, `m1-production-run.json:17`) state 13 public application
   session tests. `application/tests/durable_session.rs` now contains 14 test
   cases, including `instance_navigation_updates_scope_and_cancels_in_flight_case_work`;
   the last recorded application suite result was 14 passed. Update counts only
   when recording that exact source/check result, and keep the historical 13-test
   run attributed to its earlier revision.

5. **Story-level edge coverage is still incomplete and should remain explicit.**
   Spec startup criteria require absent/stale preferences, slow worker startup and
   a newer explicit open superseding a delayed saved read. Current focused green
   covers stale ID and normal reload, but explicitly marks delayed-IDB/slow-WASM
   races untested. Physical selector browser checks cover filtering, canonical
   reset and keyboard selection; the focused record does not itself establish
   worker/case disposal on instance change. The full session/resource evidence
   and later maintained-release public race checks must close those edges before
   ticket 06 or M1 can close.

## Link check

A targeted existence check of local Markdown destinations in the audited spec,
plan, acceptance ledger, handoff, build guide, RUN, current JSON and TODO found
no missing file targets. This check does not validate external URLs or heading
anchors.

## Evidence scope

The public green record reports two axe runs with zero violations, but one
incomplete SVG color-contrast check; this is not screen-reader evidence. The
same record says offline, subpath and delayed-startup races were not tested.
Performance and screen-reader status remain as recorded/pending; this review does
not broaden or close those gates.
