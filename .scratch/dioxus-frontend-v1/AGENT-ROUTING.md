# Agent models and dispatch policy

**User decision, 2026-10-02:** use Luna for the frontend port, with low/medium/high effort according to the packet; reserve Astra high/extra high for independent reviews and bug fixing. Keep Fast/priority execution where available. This policy changes task routing, not the frontend scope, existing gates or host configuration.

The [62-task graph](tasks.json) remains the workflow/acceptance authority. Its rows are work packages, not 62 one-shot agent prompts. The specs are sufficient for much of the implementation once the coordinator supplies a small, source-checked dispatch packet. Known boundary questions still require their own proof. This is a starting allocation to calibrate on the first tranche, not a measured claim about model speed, cost or correctness.

## Model allocation

| Role or packet | Default | Use and limits |
| --- | --- | --- |
| Mechanical work | Luna Low (`gpt-6-luna`, `low`) | Fixed-oracle icon/label/style port, pure search filtering/rendering, manifest/docs updates and running an already specified check. No new async lifecycle, scope/selection policy, persistence or shared contract decisions. “Light” maps to the tool's `low` effort value. |
| Normal feature implementation | Luna Medium (`gpt-6-luna`, `medium`) | Bounded Dioxus forms, lists, inspectors, menus and state/rendering work using a concrete existing contract. Default for most feature packets. |
| Difficult implementation/integration | Luna High (`gpt-6-luna`, `high`) | Gesture/range selection, resize/focus state, linked reflow, worker/renderer adapters and scoped async orchestration after the boundary is specified. Also the recommended coordinator for future implementation sessions. |
| Evidence author/executor | Luna Medium; High for race/resource scenarios | Prepare meaningful public tests, run affected commands/browser scenarios, compare fixture results and gather exact-source evidence. Evidence collection does not approve the change. |
| Independent review | Astra High (`gpt-6-astra`, `high`) | Review the contract where required, then exact integrated source and evidence for both Standards and Spec, including visible UI parity. Reviewer is different from the author. |
| Deep review | Astra Extra High (`gpt-6-astra`, `xhigh`) | Use when High leaves a material uncertainty: export-owned commit lineage, multi-owner cancellation, scope/reflection, public boundary reachability, or a difficult cross-workflow regression. Not automatic for every large task. |
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

INT.1 first proves private module reachability in the actual page binary/library arrangement and single-session authority. A `pub(crate)` library method is not callable from the separate page binary. Astra reviews that concrete seam once, then the feature agents can use it without repeated architecture interviews.

## Team shape and review cadence

The resumed runtime exposes eleven total slots (coordinator plus up to ten agents), replacing the earlier four-slot host limit. Use only capacity with disjoint ownership and concrete work. The current visible frontier may run three Luna authors for F2.1/F2.3/F3.1, one independent Luna verifier, and Astra contract/bug review; reserve room for the two independent final review axes. Automatic next-ticket preparation may use another Luna slot. Shared integration and heavy builds remain serial. Do not fill all slots with new features while review or integration waits.

The coordinator serializes shared presentation/runtime/global CSS/build/ledger edits, preserves unrelated work and integrates a frozen candidate. Routine integration can use Luna High. Semantic merge conflicts or changed ownership/snapshot behavior go to Astra; mechanical merges do not justify an always-on Astra coordinator.

Batch several small low/normal-risk packets from the same workflow into one Astra review of a bounded exact candidate. The review must cover **every included packet's Standards and Spec obligations** and the integration diff. Batching reduces repeated context, not review coverage. High-risk shared-boundary/transaction/renderer changes get a dedicated contract review before implementation and a dedicated integrated-source review afterward. Use `review_mode` in the routing map. The seven flagged parent tasks require a current reviewed contract, not seven additional review sessions: F7.3 can inherit an unchanged reviewed F7.1 contract, and F8.2 an unchanged reviewed BND.2 contract. Only new/changed boundaries or insufficient proof need another design review; integrated code review still applies. Any new uncertainty upgrades the packet's review mode even when its parent was initially ordinary.

Review is not the first test run: Luna supplies source/diff, commands/results, paired captures and failure-path evidence before Astra starts, except a deliberately bounded diagnostic review. Astra can request targeted checks. If Astra fixes a bug, use a different reviewer for the resulting patch. F9 joins whole-workflow evidence and final integration; a review batch cannot waive its release requirements.

The reviewed candidate is identified by commit plus any exact diff/artifact hashes. Later edits invalidate only the affected review/evidence and require the corresponding checks again. Other authors may continue on nonoverlapping work while a frozen candidate is reviewed. CAD-heavy checks stay serial; isolate browser sessions/stores and coordinate shared build directories.

## First implementation tranche

| Packet | Result | Author | Before dispatch / acceptance |
| --- | --- | --- | --- |
| INT.1a | Concrete private read-model/callback and same-crate reachability proof for library, panels and tree | Luna High | Existing session/types; Astra High reviews the proposed seam before consumers implement against it |
| INT.1b | Mount the private slots and preserve F1/F3a behavior | Luna High/coordinator | Reviewed INT.1a; affected compilation and public shell/layer regression evidence |
| F2.1a | Saved/demo cards, summaries/previews and loading/empty/damaged-preview states | Luna Medium | INT.1 accepted; existing store/fixture inputs, immutable list projection |
| F2.1b | Search/clear/no-match and list retry states | Luna Medium | Pure filtering/render-only portion may use Low after its oracle is fixed; asynchronous loading/retry stays Medium |
| F2.1c | Open saved/demo copy, pending/supersession feedback | Luna High | Existing scoped open callbacks proven; stale-open and active-identity evidence; no create/delete/archive scope creep |
| F2.3a | Panel modes/width defaults, preferences and unavailable-storage fallback | Luna Medium | INT.1; exact reference bounds/preferences and shell geometry |
| F2.3b | Pointer/keyboard resize, capture/cancel and width constraints | Luna High | F2.3a; zero document/camera side effects, one pointer owner, cancellation/keyboard evidence |
| F2.3c | Compact drawers, scrim/Escape and focus restoration | Luna High | Panel host/breakpoint fixed; hidden content excluded from focus; paired viewport/keyboard checks |
| F3.1a | Canonical-board object hierarchy and independent disclosure | Luna Medium | INT.1; exact accepted matrix membership and outline/version labels |
| F3.1b | Tree/canvas selection, toggle and rectangular range semantics | Luna High | Confirm existing stable IDs/membership/anchor contract; Astra reviews an unresolved mapping before implementation |
| F3.1c | Scope transitions, invalid-selection cleanup and compact keyboard behavior | Luna High | Existing navigate/open lifecycle; cancel active gesture and reject stale scope without duplicating session state |

These are decomposition proposals; exact filenames and fixtures are finalized from current source at dispatch. They do not claim INT.1 or any UI task is already implemented. Keep initial packets serial within their shared module; only independent owners run in parallel.

Prepare F7.1, BND.1 and BND.2 with Luna High source mapping, then use Astra High to review the concrete boundary proposal before proof/implementation. Deep export-lineage or crate/scope issues may warrant Extra High. Investigations run early when a slot is free, alongside visible feature work. They do not block unrelated project/panel/2D controls.

## Escalation and feedback

- **Unclear contract before editing:** stop only the affected packet, preserve the source evidence and propose the narrow question/change. Astra High reviews it. Use existing authorization for ordinary private composition; actual public API/schema/cutover decisions retain their existing human approval rules.
- **Behavioral failure:** preserve an expected-failure reproduction, actual result and exact candidate. Send Astra High the packet, diff, fixture/actions, logs and attempted explanation. Other independent work continues.
- **Still uncertain after High:** Astra Extra High gets the same retained evidence plus what High ruled out. Prefer a discriminating experiment over another broad rewrite. Avoid resetting context by reopening the entire milestone.
- **After repair:** rerun affected checks, obtain independent review of the new candidate, and update RF findings. Do not change a test/budget/contract just to make the repair pass.
- **Calibration:** after the first reviewed public tranche, record first-pass acceptance, types of escaped defects, retry/review time and observed usage where available. Keep Medium for routine success, route recurring state/lifecycle mistakes to High or smaller packets, and reserve Astra for review/repair. Do not manufacture numerical throughput or cost savings.

## Runtime controls and truthful provenance

The available agent tool accepts exact `model` and `reasoning_effort` values. Explicitly set both when spawning: otherwise a review/root agent could pass an expensive model to routine work. Use a concise `fork_turns="none"` packet (or the smallest necessary partial context); full-history `all` forks do not accept model overrides. Reuse an existing worker only when its model/effort and role are suitable; `followup_task` has no model override. Create a new subagent when the required profile differs. Do not create separate user-owned chats for internal packets.

On this host the inspected config sets `service_tier = "priority"`, default subagent model `gpt-6-luna`, and default subagent effort `high`. The current collaboration metadata lists priority service for Luna and Astra. The exposed spawn call has **no per-agent service-tier/Fast argument**. Therefore record Fast/priority as the execution preference, retain the host's existing setting, and record actual effective tier only when runtime metadata proves it. Do not invent a tool field or claim that editing this plan changed a running model or enabled Fast. No global config/provider changes are part of this policy update.

For each dispatched packet log requested model/effort/tier preference, agent ID, observed runtime model/effort/tier if exposed, source/artifact identity and any fallback. A missing observation is `unverified`, not inferred from the model's own statement or a config default. The machine policy records the supported launch profiles and assignments for every parent task.

[Workflow execution](EXECUTION.md) · [Machine policy](agent-policy.json) · [Refactoring takeaways](../../docs/migration/POST-PORT-REFACTOR.md)
