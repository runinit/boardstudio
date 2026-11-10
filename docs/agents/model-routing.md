# Model routing

The user's direct requests take precedence over these defaults. Select from the available catalog; do not switch providers without the user requesting it. If a requested selection is unavailable, report that rather than silently substituting another provider.

## Roles

A delegated task names one **role**. The role fixes what the delegate does and when it is done; the route is its default model, with the fallback in brackets. Provider instance IDs: `claudeAgent`, `codex`, `muse` (the Meta provider) and `pi`.

| Role | Does | Done when | Route (fallback) |
| --- | --- | --- | --- |
| Planner | Writes a ticket's proof map and [slice boundary](issue-tracker.md#slice-boundary), and splits work into slices. Reads only. | Every boundary row has an oracle and a tier. | Opus 5.5 high or Fable 5.2 via Claude, or Astra xhigh via Codex (Wonder max via the Meta provider) |
| Scout | Finds files, symbols, prior decisions and library facts with Gortex, memsearch and Context7. Reads only. | The question is answered with pointers, or the gap is named. | Spark 1.3 low via the Meta provider (Terra 6.1 low via Pi) |
| Builder | Implements one slice inside its boundary: RED, fix, affected checks, boundary mutations. | The boundary's rows are GREEN with logs, or the blocker is reported. | Spark 1.3 Contributor max via Pi (`providerInstanceId: pi`, model `meta/muse-spark-1.3-contributor`, `thinking=max`) |
| Checker | Runs the named check and mutation commands and reports results and relevant failures. Changes nothing. | Every named command has a result. | Terra 6.1 low via Pi (Spark 1.3 low via the Meta provider) |
| Diagnoser | Takes a bug after two unchanged attempts, with the evidence so far, and follows `diagnosing-bugs`. | A confirmed cause with a red-capable command, or one next experiment. | Sol 6.1 high via Codex (Opus 5.5 high via Claude) |
| Reviewer | Gives one fresh Spec+Standards pass over a fixed diff, per `code-review`. Reads only. | Both verdicts are given and every finding is marked blocks or follow-up. | Terra 6.1 high via Pi; for consequential, disputed or cross-cutting work, Opus 5.5 high via Claude (Wonder max via the Meta provider) |

The coordinator keeps the boundary, the checklist and the disposition of findings; it delegates the roles and reads their results.

- **Independence.** The Reviewer's model family differs from the Builder's.
- **One Checker.** One Checker at a time owns a `target` directory and the full gate; Builders run focused checks only.
- **Parallel work.** Scouts run in parallel freely. Builders run in parallel only on disjoint files, each in its own worktree.
- **Escalation.** Move a role to its fallback or a higher effort for a stated reason. A long command alone is not one; route by the complexity of the problem.

## Handoffs

Coordinate delegated work with pointers. Name the role and the ticket. Specify the exact owned files, expected regression, passing behavior, and concrete done condition or blocker in the implementation handoff. A handoff is complete with a verified change and checks, a reproducible failure and a concrete materially different next experiment, or the exact blocker. After two inconclusive [attempts](issue-tracker.md#slice-boundary), restate the user goal and blocker, then escalate the evidence and a materially different next experiment. Respect the user's provider choices when escalating.
