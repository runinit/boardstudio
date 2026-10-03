# Migration issue tracker

The active scope is Dioxus frontend parity. Follow the project
[operating contract](../../CONSTRAINTS.md); older P1/P2/M1 checkpoints are
[history](../migration/history/OPERATING-RULES-before-20261003.md).
Specs and issues remain Markdown under `.scratch/<feature>/`.

## Frontend delivery records

| Fact | Authoritative record |
| --- | --- |
| Current candidate, six queues, pending joins, phase, journey progress | [run JSON](../migration/dioxus-frontend-v1-run.json), `current_progress` |
| 62 parents, criteria, status, dependencies and final joins | [tasks.json](../../.scratch/dioxus-frontend-v1/tasks.json) |
| Workflow requirements and source coverage | [spec](../../.scratch/dioxus-frontend-v1/spec.md), existing child issues and [coverage](../../.scratch/dioxus-frontend-v1/coverage.json) |
| Architecture/design/theory/quality findings | [RF ledger](../../.scratch/dioxus-frontend-v1/refactor-findings.json) |
| Candidate review | One finalized candidate review/audit, referenced by parent decisions using path/hash |

The coordinator writes live/parent/RF records. Authors send changed source, existing
spec/evidence pointers and new findings; a no-gap result needs only source pointers.
PLAN, RUN and stream summaries link here instead of copying live counts or evidence.
Historical source observations stay historical; inspect current code before rewriting
an allegedly missing capability.

Use `python3 .scratch/dioxus-frontend-v1/progress.py --help` for commands:

- `show` / `show --json`: current operations and counts derived from the graph.
- `record-candidate`: validate an existing package proof/provenance and live assets,
  then publish the served candidate atomically. No additional receipt is created.
- `sync`: derive counts and the complete readable RF report from their source records.
- `check`: check record consistency; it runs no application tests.
- `set-status`: preserve status history; acceptance requires a decision, final joins
  and pinned consolidated review/audit. Never copy that review per parent.

Qualification starts at the operating contract's Layout-ready boundary. Read phase and
unmet start conditions from the progress command instead of inventing another approval.
The generated [refactoring report](../migration/POST-PORT-REFACTOR.md) renders the full
ledger; edit JSON once and sync. Preserve stable IDs, uncertainty, failed attempts and
prior evidence. Necessary parity repairs remain current work; structural proposals
feed the later refactoring phase.
