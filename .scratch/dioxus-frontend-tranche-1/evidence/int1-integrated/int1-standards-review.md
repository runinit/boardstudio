## Standards

No documented-standard breaches or newly introduced baseline smells found in the four-file diff `257bad888a2e6c8a5b1343701d87ccf4d7d7c855...598b2c026941130f9b95ff4fee57353f8eefcb0d` (commits `ad483859`, `598b2c02`).

Reviewed the supplied AGENTS.md defaults, `CONSTRAINTS.md`, `docs/agents/issue-tracker.md`, `docs/agents/domain.md`, `CONTEXT.md`, and presentation standards. The worktree contains no root AGENTS.md.

`CONSTRAINTS.md` Architecture requires one authoritative state owner and preservation of public/API contracts; `SPEC-dioxus-presentation.md` places presentation drafts in web presentation modules. The extraction complies: `mod inspector; mod library; mod objects;` introduces private modules; their `pub(super) fn` entry points remain confined to the same presentation-parent scope that previously owned the functions. This does not expand effective public or crate API exposure. Inspector draft state and its helpers become more tightly scoped.

All extracted component bodies and inspector definitions/helpers match the baseline verbatim after normalizing only entry-point visibility and `super::close_project_menu` qualification. Root Runtime creation, root subscription, session ownership, existing event handlers and component call sites are preserved. No new middleware, generalized framework, duplicated authority, suppression or weakened check appears.

Refactoring takeaways: **No new refactoring takeaway observed** in this extraction. It partially mitigates existing RF-001 (shared presentation integration hotspot) while preserving the RF-007 single-root-subscriber constraint. Existing duplicated inspector input handling is unchanged baseline debt, not an introduced finding.

This is source review only; no builds or browser checks were run. Executable and parity acceptance evidence remains the coordinator’s responsibility under `CONSTRAINTS.md` Enforcement.

Standards: 0 findings; no worst issue.
