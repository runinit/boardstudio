# Operating-rule history before the 2026-10-03 retro fixes

The user confirmed the five fixes: Layout-ready qualification; one project operating
contract; a local checkout/branch/freeze commit guard; automated candidate/RF reporting;
and a compiler preflight before packaging. Installed global skills were left unchanged.

All superseded prose is preserved verbatim in Git commit
`fc999d45825a6483509005bf4984493210f04015`. Read a historical document with
`git show fc999d45825a6483509005bf4984493210f04015:<path>`:

- `CONSTRAINTS.md`: chronological authority overrides, old check catalogue and proposals.
- `docs/agents/issue-tracker.md`: P1/P2/M1 checkpoints and frontend record history.
- `.scratch/dioxus-frontend-v1/AGENT-ROUTING.md`: first-tranche allocations and routing rules.
- `.scratch/dioxus-frontend-v1/EXECUTION.md`: team/adapter maps and source checkpoints.
- `.scratch/dioxus-frontend-v1/agent-policy.json`: full earlier routing metadata.

These snapshots are history, not parallel operating instructions. The current contract
is [CONSTRAINTS.md](../../../CONSTRAINTS.md). Detailed quality requirements were moved to
[acceptance requirements](../ACCEPTANCE-REQUIREMENTS.md); parent criteria, dependencies,
blocker history and RF observations stay in their existing canonical records.

The dated claim that no Dioxus crate/CLI exists is obsolete. Current page sources,
CI and package provenance establish compiler/build capability. Neither this correction
nor compiler/package success establishes application behavior or parent acceptance.
