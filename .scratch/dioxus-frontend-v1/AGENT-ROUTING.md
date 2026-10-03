# Agent models and dispatch policy

## Necessary API and design changes — user authorization 2026-10-02

The user explicitly directed: “if you need to change things (APIs, design) change them.” Necessary Rust/public API, visibility, wire-contract and design changes within this migration are authorized without another permission round. This supersedes earlier separate-decision restrictions for those necessary changes; it does not authorize unrelated work, discarding work, history rewrites or external publication/cutover.

Implement the smallest coherent solution rather than preserving an unsuitable interface. Update affected callers, generated contracts, tests and documentation together. Record the design reason, changed behavior and compatibility strategy in the migration/refactor ledger. Existing saved projects and exports remain acceptance inputs; if a format must change, supply and verify the required migration/recovery behavior. Review API/design changes in the integrated candidate, alongside frontend behavior. TypeScript user-visible parity remains the default goal, with deliberate necessary deviations documented and tested.


## Candidate-first execution — user update 2026-10-02

The user directed: “structure this for speed, less review handoffs” and proposed reviewing when a candidate is complete. This supersedes earlier mandatory per-child planning, source and root-composition approval sequences for routine frontend work.

Authors pin the React journey, refine the existing spec/ticket, implement the mounted workflow and run focused affected checks. They do not wait for separate independent planning or source approvals. The coordinator serially joins ready work and builds one frozen integrated candidate. Sol6.1 High reviewers divide that candidate in parallel, covering Standards and Spec alongside paired browser verification. Return consolidated findings; recheck affected repairs rather than repeat unchanged evidence or the entire approval sequence.

Known correctness/parity defects still need repair. A completed candidate is a reviewable build, not a claim that its parents are accepted. Preserve all62 parent criteria, blocker/history records, paired edit/Undo/save-reopen gates and RF ledger. Necessary API/design changes are authorized by the user; external publication/cutover remains separate. Private composition, callback ownership and internal adapters can be implemented within existing authority and reviewed in the candidate.


**Latest user decision, 2026-10-02:** use Sol 6.1 High (`gpt-6.1-sol`, `high`) for independent contract, Standards and Spec reviews. Luna low/medium/high remains the implementation allocation; the earlier Astra bug-fixing allocation remains available. Keep Fast/priority execution where available. This decision supersedes Astra review defaults in older execution/portfolio prose. Retained Astra review profiles and completed reviews are historical compatibility evidence; new reviews use Sol 6.1 High. Existing running agents retain their launched model until an explicitly configured replacement is dispatched. This policy changes routing, not frontend scope, acceptance gates or host configuration.

The [62-task graph](tasks.json) remains the workflow/acceptance authority. Its rows are work packages, not 62 one-shot agent prompts. The specs are sufficient for much of the implementation once the coordinator supplies a small, source-checked dispatch packet. Known boundary questions still require their own proof. This is a starting allocation to calibrate on the first tranche, not a measured claim about model speed, cost or correctness.

## Model allocation

| Role or packet | Default | Use and limits |
| --- | --- | --- |
| Mechanical work | Luna Low (`gpt-6-luna`, `low`) | Fixed-oracle icon/label/style port, pure search filtering/rendering, manifest/docs updates and running an already specified check. No new async lifecycle, scope/selection policy, persistence or shared contract decisions. “Light” maps to the tool's `low` effort value. |
| Normal feature implementation | Luna Medium (`gpt-6-luna`, `medium`) | Bounded Dioxus forms, lists, inspectors, menus and state/rendering work using a concrete existing contract. Default for most feature packets. |
| Difficult implementation/integration | Luna High (`gpt-6-luna`, `high`) | Gesture/range selection, resize/focus state, linked reflow, worker/renderer adapters and scoped async orchestration after the boundary is specified. Also the recommended coordinator for future implementation sessions. |
| Evidence author/executor | Luna Medium; High for race/resource scenarios | Prepare meaningful public tests, run affected commands/browser scenarios, compare fixture results and gather exact-source evidence. Evidence collection does not approve the change. |
| Independent review | Sol 6.1 High (`gpt-6.1-sol`, `high`) | Review the contract where required, then exact integrated source and evidence for both Standards and Spec, including visible UI parity. Reviewer is different from the author. |
| Focused unresolved review | Sol 6.1 High (`gpt-6.1-sol`, `high`) | Retain prior evidence and run a discriminating check when a material uncertainty remains: export-owned commit lineage, multi-owner cancellation, scope/reflection, public boundary reachability, or a difficult cross-workflow regression. Not automatic for every large task. |
| Behavioral bug fixing | Astra High; Extra High when needed | Start from a failing public reproduction, diagnose the cause, implement the smallest coherent repair and run affected checks. A different reviewer then reviews the repair; the fixer does not self-approve. |

Luna may correct its own obvious syntax, type or formatting errors during normal implementation. A failing behavioral regression, unexplained stale result, lost edit, incorrect geometry/export, resource-lifecycle defect or recurring failure goes to Astra High with the evidence. Do not spend a sequence of speculative Luna rewrites trying to avoid the review model.

## Dispatch packet, not another specification phase

Fill these fields immediately before assigning a packet; do not pause the whole project to rewrite every spec:

1. Parent task and packet ID, exact source/spec baseline, active workspace, author profile and separate verifier/reviewer.
2. One observable result, existing React source/fixture/action trace, expected output/visual states and non-goals.
3. Actual callable types/methods/callbacks, accepted scope/revision identity, draft/commit/cancel behavior, and relevant existing regression oracle.
4. Exact private files the author may edit; coordinator-owned integration files and a bounded handoff for any shared edit. Confirm no overlapping owner.
5. Named affected checks, browser fixture/session/evidence destination, red reproduction for a bug, and all required integration joins. Fixtures may unblock work but cannot satisfy a real-provider acceptance join.
6. Stop/escalation conditions, prior attempts/evidence, and RF register entry or explicit “No new refactoring takeaway observed.”

Split a packet if its public outcome, owned files or state/async contract cannot be described concretely. A large task may have several independently useful packets; its original acceptance still requires all of them. Do not split solely by file count, create permanent mock paths, or assign a whole workspace to Luna with “port everything.”

INT.1 first proves private module reachability in the actual page binary/library arrangement and single-session authority. A `pub(crate)` library method is not callable from the separate page binary. Sol reviews that concrete seam once, then the feature agents can use it without repeated architecture interviews.

## Team shape and review cadence

Keep six persistent workbench author lanes: Layout, PCB, Keymap, Keycaps, Case/shared 3D, and Parts with coordinator-owned Project support. The user/configured ceiling is 30 concurrent agents; actual runtime capacity may be lower (this tool currently exposes eleven total slots). Schedule within the lesser available capacity, reserving independent verification, both final review axes and serial root integration. Automatic next-ticket preparation uses spare capacity. Reduce simultaneous authoring when review or public verification queues, while keeping every stream supplied with a runnable private packet. A rejected spawn is an actual capacity limit; retain queued work and use suitable existing agents without changing runtime settings.

The coordinator serializes shared presentation/runtime/global CSS/build/ledger edits, preserves unrelated work and integrates a frozen candidate. Routine integration can use Luna High. Semantic merge conflicts or changed ownership/snapshot behavior go to Sol review; mechanical merges remain routine coordinator work.

Batch several small low/normal-risk packets from the same workflow into one Sol review of a bounded exact candidate. The review must cover **every included packet's Standards and Spec obligations** and the integration diff. Batching reduces repeated context, not review coverage. High-risk shared-boundary/transaction/renderer changes get a dedicated contract review before implementation and a dedicated integrated-source review afterward. Use `review_mode` in the routing map. The seven flagged parent tasks require a current reviewed contract, not seven additional review sessions: F7.3 can inherit an unchanged reviewed F7.1 contract, and F8.2 an unchanged reviewed BND.2 contract. Only new/changed boundaries or insufficient proof need another design review; integrated code review still applies. Any new uncertainty upgrades the packet's review mode even when its parent was initially ordinary.

Review is not the first test run: Luna supplies source/diff, commands/results, paired captures and failure-path evidence before Sol starts, except a deliberately bounded diagnostic review. Sol can request targeted checks. If a reviewer or debugger fixes a bug, use a different reviewer for the resulting patch. F9 joins whole-workflow evidence and final integration; a review batch cannot waive its release requirements.

The reviewed candidate is identified by commit plus any exact diff/artifact hashes. Later edits invalidate only the affected review/evidence and require the corresponding checks again. Other authors may continue on nonoverlapping work while a frozen candidate is reviewed. CAD-heavy checks stay serial; isolate browser sessions/stores and coordinate shared build directories.

## Shared-file handoff and build window

The root coordinator serializes review, merge and conflict resolution for `web/src/presentation.rs`, `web/src/main.rs`, `web/src/runtime.rs`, shared composition/registration, global CSS, packaging/manifest scripts and canonical run/RF ledgers. Authors implement required private Runtime adapters, registration, composition and feature-scoped CSS in their isolated worktrees, with an identified join diff and affected production tests. This is the approved shared-file integration policy in CONSTRAINTS.md; coordinator ownership does not require the coordinator to author every join. Six authors own named private feature leaves, their tests and stream evidence in isolated worktrees. Private extraction consumes the reviewed composition contract; changed lifecycle or callback ownership receives independent review before consumers use it.

Use the [integration handoff template](integration-handoff-template.md) for every ready packet. Root applies the bounded shared mount and feature-scoped style fragment serially, records source/dirty-diff identity and freezes integration inputs through packaging. Authors continue on separate private worktree packets. Release the build window after before/after source hashes agree; retain a changed-input candidate as invalid rather than accepting mixed provenance. Root marks the packet wired only after public mounting, then obtains independent integrated Standards and Spec verdicts and required paired browser evidence.

Provider reuse must prove complete consumed source/tool/feature identity and every reused asset hash against a retained full build. Rebuild both page base paths and their offline manifests/workers for each changed frontend candidate; preserve unique outputs and isolated browser profiles/stores. Use the full build when dependency coverage is uncertain. The [page-only reuse contract](specs/page-only-packaging-reuse.md) has a reviewed, implemented fail-closed path in scripts/build-m1.py. The retained frontend-command-icons-reuse-20261002 comparison executed eight fresh commands in 104.66 seconds versus its 471.61-second full baseline; inherited 22-command lineage is separate evidence. Use it only when current inputs pass its exact allowlist, registration/dependency proof and full-baseline artifact guards. Shared module changes, added/deleted inputs and provider changes still require a fresh full build. The current 34735 batch used the full 22-command path; do not infer reuse eligibility for subsequent slices.

These process improvements mitigate RF-001 shared-file contention and RF-009 evidence reliability. They do not establish a new private API, automated cache checker or completed parity gate.

## First implementation tranche

| Packet | Result | Author | Before dispatch / acceptance |
| --- | --- | --- | --- |
| INT.1a | Concrete private read-model/callback and same-crate reachability proof for library, panels and tree | Luna High | Existing session/types; Sol 6.1 High reviews the proposed seam before consumers implement against it |
| INT.1b | Mount the private slots and preserve F1/F3a behavior | Luna High/coordinator | Reviewed INT.1a; affected compilation and public shell/layer regression evidence |
| F2.1a | Saved/demo cards, summaries/previews and loading/empty/damaged-preview states | Luna Medium | INT.1 accepted; existing store/fixture inputs, immutable list projection |
| F2.1b | Search/clear/no-match and list retry states | Luna Medium | Pure filtering/render-only portion may use Low after its oracle is fixed; asynchronous loading/retry stays Medium |
| F2.1c | Open saved/demo copy, pending/supersession feedback | Luna High | Existing scoped open callbacks proven; stale-open and active-identity evidence; no create/delete/archive scope creep |
| F2.3a | Panel modes/width defaults, preferences and unavailable-storage fallback | Luna Medium | INT.1; exact reference bounds/preferences and shell geometry |
| F2.3b | Pointer/keyboard resize, capture/cancel and width constraints | Luna High | F2.3a; zero document/camera side effects, one pointer owner, cancellation/keyboard evidence |
| F2.3c | Compact drawers, scrim/Escape and focus restoration | Luna High | Panel host/breakpoint fixed; hidden content excluded from focus; paired viewport/keyboard checks |
| F3.1a | Canonical-board object hierarchy and independent disclosure | Luna Medium | INT.1; exact accepted matrix membership and outline/version labels |
| F3.1b | Tree/canvas selection, toggle and rectangular range semantics | Luna High | Confirm existing stable IDs/membership/anchor contract; Sol reviews an unresolved mapping before implementation |
| F3.1c | Scope transitions, invalid-selection cleanup and compact keyboard behavior | Luna High | Existing navigate/open lifecycle; cancel active gesture and reject stale scope without duplicating session state |

These are decomposition proposals; exact filenames and fixtures are finalized from current source at dispatch. They do not claim INT.1 or any UI task is already implemented. Keep initial packets serial within their shared module; only independent owners run in parallel.

Prepare F7.1, BND.1 and BND.2 with Luna High source mapping, then use Sol 6.1 High to review the concrete boundary proposal before proof/implementation. Deep export-lineage or crate/scope issues require a focused Sol High review with retained proof and targeted experiments. Investigations run early when a slot is free, alongside visible feature work. They do not block unrelated project/panel/2D controls.

## Escalation and feedback

- **Unclear contract before editing:** stop only the affected packet, preserve the source evidence and propose the narrow question/change. Sol 6.1 High reviews it. Use existing authorization for ordinary private composition; actual public API/schema/cutover decisions retain their existing human approval rules.
- **Behavioral failure:** preserve an expected-failure reproduction, actual result and exact candidate. Send Astra High the packet, diff, fixture/actions, logs and attempted explanation. Other independent work continues.
- **Still uncertain after High:** A focused Sol 6.1 High review gets the retained evidence plus what the prior review ruled out; difficult repair work may use the existing Astra debug allocation. Prefer a discriminating experiment over another broad rewrite. Avoid resetting context by reopening the entire milestone.
- **After repair:** rerun affected checks, obtain independent review of the new candidate, and update RF findings. Do not change a test/budget/contract just to make the repair pass.
- **Calibration:** after the first reviewed public tranche, record first-pass acceptance, types of escaped defects, retry/review time and observed usage where available. Keep Medium for routine success, route recurring state/lifecycle mistakes to High or smaller packets, and reserve Sol for independent review and Astra for evidence-led repair. Do not manufacture numerical throughput or cost savings.

## Runtime controls and truthful provenance

The available agent tool accepts exact `model` and `reasoning_effort` values. Explicitly set both when spawning: otherwise a review/root agent could pass an expensive model to routine work. Use a concise `fork_turns="none"` packet (or the smallest necessary partial context); full-history `all` forks do not accept model overrides. Reuse an existing worker only when its model/effort and role are suitable; `followup_task` has no model override. Create a new subagent when the required profile differs. Do not create separate user-owned chats for internal packets.

On this host the inspected config sets `service_tier = "priority"`, default subagent model `gpt-6-luna`, and default subagent effort `high`. The current collaboration metadata lists priority service for Luna, Sol and Astra. The exposed spawn call has **no per-agent service-tier/Fast argument**. Therefore record Fast/priority as the execution preference, retain the host's existing setting, and record actual effective tier only when runtime metadata proves it. Do not invent a tool field or claim that editing this plan changed a running model or enabled Fast. No global config/provider changes are part of this policy update.

For each dispatched packet log requested model/effort/tier preference, agent ID, observed runtime model/effort/tier if exposed, source/artifact identity and any fallback. A missing observation is `unverified`, not inferred from the model's own statement or a config default. The machine policy records the supported launch profiles and assignments for every parent task.

[Workflow execution](EXECUTION.md) · [Machine policy](agent-policy.json) · [Refactoring takeaways](../../docs/migration/POST-PORT-REFACTOR.md)
