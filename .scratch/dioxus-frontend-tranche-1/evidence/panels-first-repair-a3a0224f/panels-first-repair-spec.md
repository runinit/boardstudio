# Independent Spec review: first panel repair

**Source pass for the narrow repair `edf59161...a3a0224f`; no new Spec finding. T1-07 remains open.** Reviewed the exact two-file diff, unchanged parent track computation, corrected panel contract, and retained public red-regression README. No source edits, builds or browser execution.

- **Visibility/focus exclusion:** The ticket requires “Hidden content is excluded from focus and accessibility exposure.” Replacing boolean `inert` values with `condition.then_some("")` correctly expresses HTML attribute presence for hidden content and removal for visible content. Installed Dioxus 0.7.10 `nodes.rs:1117` converts `None` to `AttributeValue::None`; `dioxus-web` mutations remove the attribute. The compact shell remains inert only when compact and closed; desktop content uses its own hidden state, leaving the reveal rail outside that inert region. `aria-hidden` and mounted-child behavior are preserved. This directly addresses the recorded `inert="false"`/DOM `inert=true` failure; browser green remains required.

- **Grid modes and widths:** Removing only the two later fixed-grid declarations restores the existing Editor-computed pinned/32px/absent-Inspector tracks at narrow and short desktop sizes. The base width clamps, ≤1150px 220/300 defaults, inline persisted-width overrides and reference pinned available-space limits remain. Other responsive rules—including compact flex layout at ≤760px, tab sizing and short-height canvas/content constraints—are unchanged. This addresses the recorded stored-width regression without changing the approved policy.

The original `!compact_open()` guard still exists in `panels.rs:429`; desktop idle hiding can remain suppressed after compact-open → desktop. Keep that finding open for an unmasked red reproduction and independent repair review. No 980/820px drawer parity or resize completion is claimed; those remain tickets 09/08.

Coordinator reports strict WASM Clippy passed and a fresh build in progress. Complete T1-07 still requires fresh public green plus its panel independence, reload/workspace/optional-storage, hover/focus/menu/cleanup, hidden-tab exclusion, camera/history neutrality and shared acceptance evidence. This source review closes neither the full ticket nor the previously retained T1-02 joins.

No new refactoring takeaway observed.
