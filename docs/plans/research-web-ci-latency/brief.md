# Brief: web CI latency

Repository: the working directory. Commit `46d758d3fe8fc049084c87f077083dbe3eae137b` (`46d758d`), branch `audit/layout-engine-pcb-layout`.

## Questions

1. `how does X work` — How do build, binding generation, browser startup and test execution contribute to the elapsed time of the `web` job's `check.py build browser` step?
2. `where does X live` — Where do scheduling, concurrency limits and package or build locks introduce waiting in that step?
3. `how does X work` — How do output buffering and progress reporting determine what the job log shows while the step runs?
4. `why is X this way` — How do caching, timeout and shutdown handling work in that step, and how did they come to be?
5. `what touches X` — Which tests cover these mechanisms?

## Limits

- File budget: 15 files per question. History budget: 10 commits followed. Validation: `normal`.
- Left out of every search: `docs/plans/`, `_bmad/`, `_bmad-output/`, `.agents/`, `.evidence/`, and any other plan, spec, story or ticket file.
- Never opened: `necessist.db`; `docs/plans/initiative-layout-test-strength/tickets/epic-repair-weak-tests/epic-repair-weak-tests-retrospective.md`; `docs/plans/backlog/story-preserve-integration-audit-pilot-reconstruction-evidence-plan-quick.md`.
- Read-only: no builds, tests, reruns, edits outside this folder, commits or pushes.

## Granted evidence beyond the code

- Hosted job logs `/tmp/pr19-web-failure.log` (attempt 1) and `/tmp/pr19-web-success.log` (attempt 2).
- GitHub Actions run `44018165934`, attempts 1 and 2, read through `gh api`.

## Statements to verify, not assume

- Attempt 2 step total 2146.1 s: build 417 s, browser 1729.1 s.
- The keymap launch took about 290 s although its tests took about 8 s.
- Attempt 1 started with cold caches and its log timestamps are buffered.
- Attempt 1 ended with exit 143 on a shutdown signal, and the job finalized late.

## Claim classes

Each claim in the document is one of: **measurement** (from the logs or the run API), **mechanic** (from code at the commit), **history** (from a commit), **hypothesis** (consistent with evidence, not shown) or **unknown**.
