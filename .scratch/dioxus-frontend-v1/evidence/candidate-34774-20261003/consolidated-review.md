# Candidate 34774 consolidated Standards and Spec review

Finalized 2026-10-03. Requested profile: Sol 6.1 High. This is the single candidate pass, including assessment of the repair found during that pass. The audit alongside this file pins the source and evidence. The 34771 review remains historical.

**Scoped verdict:** the five delivery controls pass source review after the full-build provenance repair. Candidate 34774's changed Layout integration needs the Properties shortcut repair below. Its two changed browser journeys remain unexecuted due to verifier/tool-session handling. No browser or product hang is established. No parent is accepted by this review.

## Scope and source pins

- Workflow baseline `fc999d45`; integrated workflow commit `067af1be647fe90b9d1eecc69389ba0d03c32dc3`, plus repair commit `0943ae9ece188c69d39c25a8a195b8b04e878a68` to `progress.py` and its regression test.
- Served UI source `5c8161cf80d2c93c6214cb7caed6dffc4463f64d`, build `frontend-source-batch-20261003-5`, root `http://127.0.0.1:34774/` and `/boardstudio/`. The reviewed Layout leaves and root composition are unchanged from that pin.
- React oracle `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5175/`.
- Standards and Spec were assessed together against `CONSTRAINTS.md`, the retained `ACCEPTANCE-REQUIREMENTS.md`, delivery-record rules and the relevant existing Layout specifications. No separate per-parent reviews or handoffs were created.

## Open actionable finding

**L1 — P2, Spec/behavior: Transform property shortcuts retain the Relations tab.** At `web/src/presentation.rs:7093–7109`, the shared `on_show_properties` handler opens the Inspector without selecting `LayoutInspectorTab::Properties`. After selecting a matrix context's Relations tab or using Align → Relationships, Transform → Position & rotation opens Relationships again; Splay & origin and Row offsets also retain Relations after changing context. `web/src/presentation/layout_workspace.rs:177–178,204` uses that shared tab to replace the numeric controls with relationship content. Pinned React `app/src/ui/Workbench.tsx:1310–1312` explicitly selects Properties for all three shortcuts, as required by `specs/F33-transform-align-toolbar.md`'s property navigation contract. Set the shared tab to Properties in the guarded Layout handler and verify the Relations → Transform shortcut transition. This is established by current source and the reference; the browser attempt did not independently confirm it.

## Resolved finding within this pass

**W1 — P1, Spec/behavior: a successful fresh full package could not be recorded.** At the frozen workflow commit, `progress.py:166–171` required inherited command lineage unconditionally, while `scripts/build-m1.py:1425–1428` initializes a full build without inherited fields. A fresh successful 23-command package therefore failed `record-candidate` before publication or qualification. A bounded isolated fixture reproduced `ValueError: Candidate provenance has invalid inherited command lineage`; no application/build suite was run by this reviewer.

The assessed repair accepts absent lineage only when both lineage fields and all reuse/base-provenance markers are absent and the proof reports integer zero inherited commands. Reuse-marked and partial-lineage records still fail; supplied lineage keeps exact type/count and successful-command checks. The author reports the new full-build regression failed for the expected reason before repair, then all 17 progress tests and `py_compile` passed. The repaired source and test hashes are frozen in the audit. W1 is resolved for those exact bytes; it does not change the served UI source.

## Five delivery controls

| Control | Assessment |
| --- | --- |
| Compiler preflight and provider proof | Full and reuse paths run the exact locked WASM page check before fixtures/providers/page packaging. New full lineage has 23 commands. Legacy 22-command admission retains the pinned helper Git identity, canonical command/log proof and provider/source/tool/asset checks. Coordinator reports actual retained legacy admission succeeded after the workflow commit: 56 eligible inputs and 22 inherited commands, with no output build. |
| Registered checkout, branch, role and freeze guard | The local common-directory registry validates the registered branch/role. The hook rejects direct coordinator commits; its delivery command retains a shared lease while committing, conflicting with the build's exclusive lease. Interrupted-build markers are reaped only after the kernel lease is free. Existing custom hooks are chained, and configured `core.hooksPath` is rejected instead of silently bypassed. This is the documented accidental-misuse guard, not protection against same-user file edits. |
| Verified atomic candidate and RF rendering | Proof/provenance identity, complete status, commands, source inventory, both local asset maps and served asset bytes/headers are checked before atomic replacement of the run JSON. Full-build shape is handled by W1's repair. RF rendering includes nested and unknown ledger fields, retaining observations rather than collapsing them into counts. |
| Layout qualification trigger | Verified package recording checks integrated Layout ancestry, no pending Layout packet and a matching page compiler check, without waiting for other workbenches. Already-started results retain their frozen candidate scope. A new candidate resets its journeys/review and preserves prior results in history. This starts qualification and never accepts parents. |
| Simplified operating rules | The short operating contract and linked acceptance requirements retain the behavior, compatibility, accessibility, performance and retirement floor. Canonical task criteria and joins were not edited in this workflow diff; the coordinator remains responsible for their actual decisions. No concrete weakening was found in this bounded change. |

Coordinator-supplied focused checks before W1 repair passed: build reuse 33, source ownership 3, progress 15, qualification 6 and delivery guard 9. Local guard installation and a guarded coordinator commit succeeded. These are reused evidence, not reruns or application qualification by this reviewer.

## Bounded Layout source and browser assessment

The changed shared Properties/Relations composition projects current accepted matrix/member relationships and routes Edit placement relationship through the existing scoped tree callback. It introduces no second accepted document or history owner. L1 is its concrete routing regression.

The outline scan covered mounted generated/fixed geometry controls, draft finish/cancel, saved feature/rectangle controls, linked connection attachment and coordinate changes, and gap focus. Action admission retains Layout workspace, accepted scope/token/revision/generation, board/context and active-version/feature checks; commits use the existing Core/Session edit/history/persistence routes. No additional concrete blocker was identified in this bounded source scan. Source inspection does not establish pointer, keyboard, Undo or durable reopen behavior.

The corrected `layout-journey.md` records both focused changed journeys as **unqualified/unexecuted**. React Sofle was opened and its baseline captured. Candidate start was clicked. Later screenshot and lightweight title/URL commands yielded after ten seconds; the verifier discarded their returned execution session IDs and did not retrieve eventual results. A command yield is not a timeout or failure. The independent browser agent did not establish candidate selection, edit, Undo or save/reopen. No browser or product nonresponse is established, and neither journey is qualified. Cleanup of all browser sessions also remains unverified.

The existing package proof records seven fresh and 22 inherited commands, 1,381 sources with no mismatches, 190 assets per route with no mismatches, root/subpath HTTP 200, isolation headers and zero release warnings. Its provenance digest is `ef5f71c5086563dfefb6b53ed8e2108485b30d0d7072214a67c47794b86321bd`. The earlier compiler result is pinned to `9f12d32a`; qualification's retained-source comparison is responsible for its validity. The package audit is reused without another asset sweep or build.

## Limits and required follow-through

Repair L1 and execute the changed Layout journeys with completed command results. This review covers the five workflow fixes and the named changed Layout integration, not all 62 parents or the whole larger UI source batch. It does not establish full accessibility, responsive, performance, corpus/export, retirement or cutover qualification. No implementation edit, suite, Cargo command, build, commit, parent transition or canonical progress/RF mutation was performed by this reviewer. The only saved review outputs are this file and `consolidated-audit.json`.
