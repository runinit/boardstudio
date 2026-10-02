# Independent agent-policy review

Reviewed 2026-10-02 at clean worktree HEAD `7b7b004e717ce9976d60cc8c0326627b64caa04b`, `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`. This is a planning review, requested as Astra high; no implementation, builds, browser checks, provider changes, or new agents were performed. Inspected PLAN.md, EXECUTION.md, the complete task inventory/dependency summary and selected acceptance details, F2/F3 issue details, planning reconciliation, boundary follow-up, and refactor review. Existing planning/executable/reference hashes retain their separate meanings.

## Recommendation

Adopt the proposed policy with the safeguards below. Luna medium is the default feature and public-test author. Luna low is suitable for mechanical assets, styles, already-established component patterns, documentation, and known verification commands with a fixed oracle. Luna high implements bounded complex state, gestures, and adapters after their contracts are precise. Astra high performs independent design/boundary review and integrated Standards/Spec acceptance review; Astra xhigh is reserved for difficult reviews or bugs such as export-owned lineage, stale-result races, cross-scope CAD, and demonstrated public-contract problems. Astra should not become the ordinary feature writer.

Do not translate the 62 tasks into 62 monolithic Luna assignments, or 62 separate Astra sessions. Preserve the task graph and acceptance contracts; add dispatch packets when work becomes runnable. Batch low-risk packets within one workflow against one exact integrated candidate. Review high-risk shared boundaries individually. This saves review setup cost without sampling away required review or qualification.

## Material findings

1. **Task sizes need a subordinate dispatch unit.** The graph contains 31 L, 29 M, and only 2 S tasks. F2.3 combines panel modes, bounded pointer/keyboard resize, persistence, compact breakpoint changes, focus, and revision/camera invariants. F3.1 includes anchored rectangular selection, board membership and gesture cancellation. Calling either a small generic UI task creates expensive failed iterations. A packet should be one coherent user action/state machine in named private files, with one integration seam and explicit behavioral oracle. Split at behavior boundaries, not arbitrary file or line counts. Parent task acceptance still requires all its packets and integration joins.

2. **INT.1 needs an executable placement decision.** EXECUTION.md:82 already records that the page binary and `boardstudio_web` library are different crates. INT.1's generic assertion that private composition suffices is not the actual call-path proof. Before feature dispatch, name where the private read model/callback types and adapters live, who calls them, and what existing public provider operations they consume. Require an affected compile check of the real consumer and Astra high review. F7.1, INT.2, BND.1 and BND.2 need the same concrete boundary discipline where applicable. Neither a reviewer nor an author may approve new public visibility on the user's behalf.

3. **A review batch cannot erase independence or task-level evidence.** EXECUTION.md:31 and :62 require independent Standards/Spec review of integrated source and evidence. Every accepted packet must appear in the batch review's scope-to-verdict mapping, including its required visual and public behavior evidence. Reviewing a sample of low-risk packets is insufficient. An Astra reviewer who fixes a bug becomes an author of that delta; a different agent must review that delta before acceptance. A preimplementation design review is also not acceptance of the later code.

4. **The first-wave wording should match available feedback capacity.** PLAN.md:26 currently prescribes three feature authors. Default to coordinator + two Luna builders + one Luna verifier; rotate a slot to Astra when an integrated candidate or boundary design is ready. Three builders are a short initial exception only when no verification, integration, or review queue exists and their packet ownership is disjoint. Do not keep launching implementation while acceptance backlog accumulates. Preserve the first visible tranche F2.1/F2.3/F3.1, but stagger its packets as necessary.

5. **Reasoning effort is not a latency switch.** Available agent interfaces expose model and reasoning choices but no Fast control. The parent reports priority configuration, and tool metadata advertises priority support; neither establishes that a particular child actually ran on a particular service tier. Record requested model/effort, returned agent identity, and actual runtime/tier only when observably reported; otherwise record unknown. Do not invent a Fast flag or change account/provider/configuration settings.

## Packet readiness at launch

A packet is ready when the coordinator has recorded:

- Parent task ID; current baseline and overlap inspection; exact owned files; shared-edit owner; allowed public providers and required private call path. Shared Runtime, presentation, global CSS, manifests/build and ledgers remain coordinator-owned.
- A complete bounded outcome, relevant pinned React fixture/action trace, explicit invariants and negative states, and the acceptance obligations assigned to this packet. Identify the integration joins still required by the unchanged graph.
- Concrete input/read-model/callback types and scope/draft lifetime rules. Async packets additionally specify request/session/document/revision/board/instance identity as applicable, invalidation, errors, cancellation, late replies, and cleanup.
- The initial regression failure for a bug fix, affected checks, public output/history/storage assertions, paired visual/keyboard/axe obligations, and who independently verifies and reviews them.
- Model/effort and escalation route; a bounded handoff including source/diff, executed check results, pending evidence, and the refactoring observation or explicit “none observed.”

Prepare this only for the next runnable work. Do not stop the whole migration to elaborate all 62 tasks. An unresolved boundary pauses dependent edits, while read-only investigation and unrelated fixture-backed work continue under the existing graph.

## Exact failure and escalation loop

1. Establish the oracle and smallest reproducible failure. Distinguish product regression, wrong fixture/test expectation, environmental failure, and missing contract. For a bug, observe the expected failing regression before changing the implementation.
2. Let the assigned Luna author make one targeted correction and rerun the affected check. A clearly local mechanical compiler/style/test-command correction may stay at its current effort.
3. If the same failure remains, do not repeat the same patch or broadly rewrite the feature. The coordinator records the hypothesis, evidence, attempted delta, and current failure; narrows the packet if necessary and moves low/medium work to Luna high for a changed, explicit hypothesis. Limit this to one additional targeted correction cycle before independent Astra diagnosis.
4. Escalate immediately to Astra high review when evidence contradicts the contract, identity/ordering/cancellation guarantees are unclear, the proposed solution crosses the stated ownership boundary, or the regression affects previously accepted behavior. Do not burn two Luna attempts first on an architectural contradiction. A reproducible stale export/race/CAD scope bug or unresolved high review receives Astra xhigh.
5. Astra identifies the defect and recommends a bounded repair; ordinarily Luna implements that repair. Astra may implement difficult bug fixes within the user's reserved bug-fix scope. If Astra edits, assign another agent to independently review the delta. A genuine public API/visibility need becomes a concrete proposal and dependent blocker, not an implicit permission request for the entire migration.
6. Reproduce the original failure on the pre-fix state where required, pass it with the fix, and run affected checks and integration evidence. Return to normal Luna effort after diagnosis. Never weaken the oracle, change thresholds, omit a required state, or mark a task accepted to clear the queue.

This is a limit on unproductive repeated attempts, not a mandatory promotion for every distinct compiler error. New evidence can justify an additional targeted check; unchanged failure and unchanged hypothesis cannot.

## Review coverage and sampling

Require Astra high design review before implementation of a new shared boundary: INT.1/INT.2, F7.1/shared viewer inputs, BND.1 and BND.2, and subsequent material changes to those contracts. Use xhigh when the difficult reasoning above warrants it. Existing approved contracts need not be re-reviewed for each faithful consumer.

Require independent Standards and Spec verdicts for every accepted implementation packet, with Astra high covering integrated source/evidence in the chosen batches. The verifier may be Luna medium; it derives public scenarios from the reference/spec, not merely from the author's implementation. On an exact candidate, every acceptance assertion and mandatory capture/check must have evidence. Carry old evidence only with a reason it remains applicable.

Sampling applies only to additional reviewer reruns: inspect all evidence and source changes, independently replay one representative public action per low-risk batch plus each new async/gesture/scope boundary and any disputed result. Choose the representative action after implementation rather than letting the author select it. Missing, inconsistent, stale or failed evidence expands reruns to the affected packet and shared consumers. Unchanged evidence may be reused; no percentage sample replaces required visuals, output assertions, Standards/Spec review, or final F9 qualification.

## Suggested first wave

1. Coordinator prepares INT.1's concrete private placement/read-model/callback packet; Astra high reviews the boundary proposal. Luna high can produce bounded private implementation, while coordinator applies shared composition edits. Verify the real call path and preserve F1/F3a. A preimplementation review alone cannot close INT.1.
2. Start Luna medium on the bounded F2.1 library projection/search/open packet and Luna medium on F3.1 tree disclosure/basic selection under the proven selection contract. Split advanced range/scope cancellation into a Luna high packet if the implementation entails consequential state logic. Luna medium verifier prepares paired public scenarios for the first tranche, including F2.3.
3. Rotate a freed builder into F2.3. Presentation/styles can be medium or mechanical low; panel resize, breakpoint/focus restoration and persistence must have their own precise state contract and high effort where needed. Integrate all three tranche outcomes on one candidate; run the verifier and then Astra acceptance review without waiting for unrelated full workflows.
4. Use the next boundary slot for F7.1/BND.1 and early BND.2 design/feasibility review. BND.2 is a reasonable xhigh review candidate because owner-commit adoption must distinguish its own accepted transactions from unrelated changes. These reviews should not consume all slots or block independent 2D/catalogue work. Keep F9 evidence preparation continuous and CAD-heavy functional checks serial.

## Preserved limits

No task status, dependency, acceptance join or external gate changes as a result of choosing cheaper authors. Actual screen-reader evidence remains host-blocked; keyboard/axe/AX evidence cannot replace it. Carried performance failures, ineligible CAD comparisons and unperformed resource/material checks remain explicit. The provider source/hash guard, accurate source/build/demo provenance, root and `/boardstudio/` deployment behavior, no permanent placeholders/React UI islands, and explicit approval of the final production cutover/retirement patch all remain required. Update the existing refactoring register at every handoff or state that no new takeaway was observed; required correctness stays in the current slice.

The main refactoring takeaway relevant to the resource policy is already recorded: shared Runtime/presentation and the library/binary boundary are coordination hotspots. More simultaneous authors will not solve those constraints. Small private packets, serialized shared edits, and early proof of call paths address them during the port without expanding the project into an architecture rewrite.

## Reconciliation with Luna readiness audit

Read `/tmp/frontend-agent-policy/luna-readiness.md` after completing the independent assessment. Its packet decomposition is useful; approve the coordinator's tier corrections: INT.1 high; F2.1b low only for pure search/rendering, medium for list retry/request state; F2.1c high for new open/supersession behavior, or medium when only wiring already-proven callbacks. F2.3b pointer/cancel/persistence and F2.3c breakpoint/focus restoration warrant high when implementing a new state machine; medium is appropriate for faithful composition of a proven controller. Apply the same distinction to F3.1c scope/gesture cancellation. Calling these concerns presentation-only does not make their state semantics mechanical.

Local configuration's Luna-high default does not conflict with explicitly requesting Luna medium for bounded packets. Record requested arguments and observed runtime metadata separately, with no claim that priority or Fast was enforced per call. No configuration changes are required.
