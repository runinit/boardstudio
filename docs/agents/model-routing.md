# Model routing

The user's direct requests take precedence over these defaults. Select from the available catalog; do not switch providers without the user requesting it. If a requested selection is unavailable, report that rather than silently substituting another provider.

## Roles

A delegated task names one **role**. The role fixes what the delegate does and when it is done; the route is its default model, with the fallback in brackets. Provider instance IDs: `claudeAgent`, `codex` and `pi`. Meta models route through Pi, because the Meta provider's own harness (`muse`) has no Gortex tools.

| Role | Does | Done when | Route (fallback) |
| --- | --- | --- | --- |
| Planner | Writes a ticket's proof map and [slice boundary](issue-tracker.md#slice-boundary), and splits work into slices. Reads only. | Every boundary row has an oracle and a tier, and every mutation has its command, tier and expected failing assertion. | Opus 5.5 high or Fable 5.2 via Claude, or Astra xhigh via Codex (Wonder 1.0 max via Pi, model `meta/muse-wonder-1.0`) |
| Scout | Finds files, symbols, prior decisions and library facts with Gortex, memsearch and Context7. Reads only. | The question is answered with pointers, or the gap is named. | Terra 6.1 low via Pi |
| Builder | Implements one slice inside its boundary: RED, fix, focused checks. Hands the boundary mutations and the full suites to the Checker. | The boundary's rows are GREEN with logs, or the blocker is reported. | Temporary (cost): Luna high via Codex (`providerInstanceId: codex`, model `gpt-6-luna`, `reasoningEffort=high`). Usual: Spark 1.3 Contributor max via Pi (`providerInstanceId: pi`, model `meta/muse-spark-1.3-contributor`, `thinking=max`) |
| Checker | Runs the named checks, the boundary mutations as the boundary names them, and the gate, and reports results and relevant failures. Changes nothing. | Every named command has a result, and every mutation is killed at its named assertion or reported as a survivor. | Temporary (cost): Luna high via Codex (`providerInstanceId: codex`, model `gpt-6-luna`, `reasoningEffort=high`). Usual: Terra 6.1 low via Pi (Spark 1.3 low via Pi, model `meta/muse-spark-1.3`) |
| Diagnoser | Takes a bug after two unchanged attempts, with the evidence so far, and diagnoses it. | A confirmed cause with a red-capable command, or one next experiment. | Sol 6.1 high via Codex (Opus 5.5 high via Claude) |
| Reviewer | Gives one fresh Spec+Standards pass over a fixed diff. Reads only. | Both verdicts are given and every finding is marked blocks or follow-up. | Temporary (cost): Luna high via Codex (`providerInstanceId: codex`, model `gpt-6-luna`, `reasoningEffort=high`). Usual: Sol 6.1 xhigh via Codex or Opus 5.5 high via Claude |

The coordinator keeps the boundary, the checklist and the disposition of findings; it delegates the roles and reads their results. Its own writes are the checkpoint file and the commits; every other edit, check and read-through goes to a role.

- **Independence.** The Reviewer's model family differs from the Builder's, except while the temporary cost routes above put both on Luna; then the Reviewer is a fresh session.
- **Review transfer.** Give the Reviewer the fixed endpoints, the per-file list from `git diff --stat base..head` and the ticket's acceptance criteria as a checklist. It reads one file's diff at a time, appends each file's findings to the report file as it goes, and states which files it read in full, in part or not at all. A single `git diff` for the whole range truncates and yields a `PARTIAL` review.
- **Mutation transfer.** The Checker runs only the rows the builder recorded as `proof gap` (see the [slice boundary](issue-tracker.md#slice-boundary)), one spec per row, with `scripts/mutation-check.py --locked` and that row's test filter in `--command`.
- **One Checker.** One Checker at a time owns a `target` directory and the full gate; Builders run focused checks only.
- **Parallel work.** Scouts run in parallel freely. Builders run in parallel only on disjoint files, each in its own worktree.
- **Escalation.** Move a role to its fallback or a higher effort for a stated reason. A long command alone is not one; route by the complexity of the problem.

## Transfers

Coordinate delegated work with pointers. Name the role and the ticket. Keep the checkpoint a delegate must read in a file under `.evidence/<slice>/` and give its path; one agent's notes are closed to the others. Specify the exact owned files, expected regression, passing behavior, and concrete done condition or blocker in the implementation transfer. A transfer is complete with a verified change and checks, a reproducible failure and a concrete materially different next experiment, or the exact blocker. After two inconclusive [attempts](issue-tracker.md#slice-boundary), restate the user goal and blocker, then escalate the evidence and a materially different next experiment. Respect the user's provider choices when escalating.

Every implementation or test transfer includes these lines verbatim, because delegates otherwise stop on them:

- Gortex for reads; on a stale, `view_building` or read-only write rejection retry once after 30 seconds, then use native Edit/Write and report which edits (see [AGENTS.md](../../AGENTS.md#editing)). Do not stop for a stale Gortex view.
- Do not end the turn until every named row is GREEN or has two recorded attempts; keep a checkpoint file under `.evidence/<slice>/`.
- No commits, no `git stash`.

The coordinator commits only between delegations, never while a delegate is working: a commit changes `HEAD`, and Gortex's checkout mutation view then reports stale until it rebuilds.
