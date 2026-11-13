# AI-generated documentation and retrieval

Use this guidance when producing agent notes, research, transfers or generated documentation,
or changing Gortex exclusions. It supplements the current instructions; it does not replace the
[committed issue tracker](issue-tracker.md#building-and-verifying-a-ticket).

## Place the output

- Keep disposable exploration and draft prose in `.scratch/`, and session memory in the agent's
  memory store. These are working material, not project authority.
- Keep verification logs in `.evidence/`, following the root instructions. Retain committed
  transfer records and ticket evidence links; a retrieval exclusion is not permission to remove
  files or stop recording required proof.
- Keep ticket proof maps, review verdicts and completion records in their existing ticket
  workflow. Keep approved specifications, ADRs, `GLOSSARY.md`, human-reviewed research, code and
  tests in their established locations. AI authorship alone is not an exclusion criterion.

## Promote a claim, not a transcript

Before proposing durable documentation, compare it with current code and the governing ticket,
ADR or specification. Identify sources and distinguish observations, proposals and user-approved
decisions. Summarize reusable findings rather than copying session transcripts or repeated status
receipts. Human review or the existing approval protocol decides authority; a generated document,
search hit or repeated agent assertion does not establish it.

If a draft contradicts an approved decision, report the conflict for a decision instead of
rewriting the authority. Preserve the ticket's mandatory gates and completion record.

## Change retrieval safely

`.gortex.yaml` contains repository-local indexing/watch exclusions. `.gitignore` controls Git's
untracked-file handling; it neither removes tracked documents nor proves that this installed
Gortex version excludes the same paths. Check the effective Gortex rules and intake, not just
Git ignore status. Exclusion also does not guarantee immediate removal of old indexed content.

Before adding a pattern, inventory its tracked matches and protect required documents. Prefer a
rooted disposable directory or a log-specific pattern over a topic-wide exclusion. In particular,
keep `docs/plans/`, `docs/adr/`, `docs/research/`, the glossary and committed `.evidence/` transfer
records searchable. Stop for a user decision if a proposed rule would block a necessary tracked
document or change the authorized ticket workflow.

For an audit, record CLI version, effective exclusions (`gortex config exclude list`), admission
counts and bytes (`gortex init --dry-run-intake --json`), and worktree routing
(`gortex repos families`). The intake is a candidate-file report, not proof of successful parsing,
current graph contents or correctness. Compare before/after in the same checkout and account for
concurrent edits. Use read-only inspection; ordinary audits do not authorize tracking a linked
worktree, forced reindexing, corpus repair or deletion.

Native Gortex MCP reads and edits must resolve to the intended checkout. A linked worktree uses
its automatic family overlay, not a separately tracked graph. Report pending routing or missing
integration rather than silently editing a fallback branch. Inspect representative retained
specifications and source/test searches after a change; report unsupported checks honestly.
