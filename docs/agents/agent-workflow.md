# Agent workflow: roles, rules and handoff (Codex and Claude Code)

Project-wide, for every branch and both apps. `AGENTS.md` points here. Written 2026-10-10 when the orchestrator role moved from
Claude to Codex. Everything an orchestrator or lane agent needs is in the repository; nothing depends on chat history or one
app's private memory. Read this page, then
[verification-process.md](verification-process.md), [gitnexus-index.md](gitnexus-index.md) and
`docs/plans/typed-core-edits/parallel-lanes.md` (lanes, prompts: `docs/plans/typed-core-edits/lane-prompts.md`).

## Models and who does what (the same in Codex and Claude)

Set by the user on 2026-10-10. The roles are fixed; use these models for them in whichever app is running.

| Role | What it does | Model / effort |
| --- | --- | --- |
| Orchestrator | tickets, ADRs, prompts, ordering, lane setup, merging, batch check, GitNexus refresh, answers questions | **Sol 6.1, medium, fast** |
| Planning research (read-only) | design options, plans, audits of code | **Sol 6.1, max, fast**; **Astra, medium** only as a last resort |
| Implementer | one ticket in one lane worktree, test first | **Luna 6, high, fast** |
| Verifier / reviewer | reproduces, mutation-checks, fixes small defects, sets `resolved` | **Sol 6.1, xhigh, fast** |
| Browser or CAD runs | needs Chrome, `wasm-pack`, podman | the same sessions, sandbox lifted |

Implementer and verifier must be different sessions with different models (Luna implements, Sol verifies). Commit trailers name
the agent that made the commit (for example `Co-Authored-By: Codex <noreply@openai.com>` or the Claude trailer for work done in
Claude Code). Until 2026-10-10 Claude Haiku, Sonnet and Opus filled these roles; tickets resolved before then say so.

## Differences between the apps

- **Spawning agents.** Claude started implementers and verifiers as background subagents. In Codex use one session per lane
  worktree (or Codex's own subagent feature, if it has one; confirm it can use a different worktree and model). The briefs are in
  `lane-prompts.md`; they carry the rules because a lane agent does not remember between tasks.
- **Memory.** Cross-session memory is `memsearch`, shared by both apps (use the `memsearch:memory-recall` skill for "why did we do X"). It is a recall aid, not a rulebook: the rules below and the docs are what bind, and where memory disagrees with the docs, the docs win.
- **Hooks.** The GitNexus Stop hook is a Claude settings entry (`docs/agents/gitnexus-index.md`). Codex has its own hook or
  automation mechanism; either wire `python3 scripts/gitnexus-refresh.py --background` to its end-of-turn event, or run
  `python3 scripts/gitnexus-refresh.py` by hand at batch boundaries. The script is single-flight and idempotent either way.
- **Sandbox.** `wasm-pack`, Chrome and the CAD container (podman) need an unsandboxed run; cargo needs the `target` link
  writable. Note any step Codex cannot run and let the user or another session run it.
- **Settings files.** The harness protects `.claude/settings*.json` from agent edits; the user adds hook entries.

## Rules the user set

1. Never `git stash`; stage explicit paths only; never `git add -A` or `git add .`; no push, merge to `dev` or PR without the
   user. Use a temporary commit to set work aside.
2. Cargo uses 12 jobs (`.cargo/config.toml`); never pass `CARGO_BUILD_JOBS=2`. Never run `python3 scripts/check.py release`.
3. Ticket process: implementer sets `implemented (awaiting verification)`; the verifier resolves. Run only the check steps the
   change needs; the full `check.py` runs once per batch. Sequential, foreground, one run at a time. A scope given to an agent is
   fixed when it starts; new asks become tickets.
4. GitNexus is never refreshed by lane agents; the orchestrator refreshes at batch boundaries and addresses the index by
   `--repo boardstudio-<worktree>`.
5. Preserve unrelated work: the main checkout holds the user's uncommitted `AGENTS.md`, `DESIGN.md`, `PRODUCT.md`,
   `docs/README.md` and untracked docs. Do not touch them.

## The orchestrator's loop

1. Read `docs/plans/typed-core-edits/map.md` and the ticket headers (`Status:`, `Blocked by:`); pick the ready tickets per lane.
2. Set up a lane when needed (commands in `parallel-lanes.md`: own worktree, branch, `target` directory on disk, real copies of
   `cad/.cache` and `cad/wasm/pkg`). Keep at most two lanes running heavy checks; run timing work alone.
3. Hand each ticket to an implementer with the common rules plus the lane prompt; when it reports, hand it to the verifier with
   the verifier prompt.
4. Merge a verified lane into `typed-core-edits/mechanical` with `git merge --no-ff`; if a lane branch is behind, merge the
   integration branch into the lane first. Regenerate goldens, never hand-merge them.
5. After a batch: run the full `python3 scripts/check.py` once on the integration branch, write the `map.md` progress lines, refresh
   GitNexus, delete finished lane worktrees and Chromium profiles older than 30 minutes.
6. Keep the single-writer files (`map.md`, `CONTEXT.md`, ADRs, goldens, `scripts/check.py`) to yourself.

## State snapshot (2026-10-10, late)

Integration branch `typed-core-edits/mechanical` at `205a783dc` or later; nothing is merged to `dev` and nothing is pushed.

| Area | Resolved | In flight or ready |
| --- | --- | --- |
| generator parameters (`docs/plans/generator-parameters/map.md`) | 01-06 | none |
| mirrored halves (`docs/plans/typed-core-edits/map.md`) | 16, 17, 18, 26, 27, 28, 29, 30 on the integration branch; **31, 32 resolved only on `lane/mirror-core`** (verified by Claude Sonnet) | **39 claimed by Codex in `lane-mirror-core`, uncommitted work in progress**; then 19 (alone, riskiest), 40, 41, 20, 21, 22, 23 |
| test speed / audit | 33, 34, 36 | **37 `implemented (awaiting verification)` (partial: library, layout outline and two page files converted; the rest is listed in the ticket); needs a Sol 6.1 verifier**; then 38, 35, 24 |
| check speed | 10 | 11 (needs the machine alone), 12, 13 (decision) |
| boards review | ticket 25 phase 1 done and verified (`lane/boards-review`) | the user recorded hardware decisions (`c47a27df5`) and the ticket is `ready-for-agent` again: phase 2 = per-board fix tickets, Sofle first |

Worktrees: `.worktrees/tce-matrix` (integration), `.worktrees/lane-mirror-core` (`lane/mirror-core`, merged with integration at `21e4ea385`,
then the 39 claim), `.worktrees/lane-boards-review` (`lane/boards-review`, docs only). Older worktrees from earlier efforts remain; leave them.

`.worktrees/tce-matrix` is free again (the ticket 37 agent finished and the tree is clean), and `dev` now carries these workflow docs (commit `4e5a6fda3`, merged into this branch). Run browser checks there one at a time.

## Open items that need the user

- Add the GitNexus Stop hook to the local settings (snippet in `gitnexus-index.md`), or decide to run the refresh by hand.
- Hardware decisions for the shipped boards (ticket 25) if not all answered in `c47a27df5`.
- Ticket 13: whether to pursue a structural browser-step change after tickets 11 and 12 measure.
- Before anything leaves the machine or merges to `dev`: the main checkout's uncommitted docs conflict with a merge; the user decides.

## First steps for the orchestrator

1. `git -C .worktrees/tce-matrix log --oneline -15` and `git worktree list`; read the ticket statuses above to confirm the snapshot.
2. Wait for (or ask about) ticket 37, then verify it with Codex Sol 6.1.
3. Merge `lane/mirror-core` (31, 32) and `lane/boards-review` into the integration branch once 37 has landed; run the batch check once; refresh GitNexus.
4. Hand ticket 39 to an implementer, then 19 alone.

## Kickoff prompt for a new orchestrator session

Open Codex (Sol 6.1, medium, fast) with `/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix` as the working folder and paste:

> You are the orchestrator for the typed-core-edits effort in BoardStudio. Read, in this order: `AGENTS.md`,
> `docs/agents/agent-workflow.md` (roles, models, rules, the state snapshot at its end), `docs/agents/verification-process.md`,
> `docs/agents/gitnexus-index.md`, `docs/plans/typed-core-edits/parallel-lanes.md` and `lane-prompts.md`, then
> `docs/plans/typed-core-edits/map.md`. You plan and orchestrate: you write tickets, hand them to implementers (Luna 6, high, fast)
> and verifiers (Sol 6.1, xhigh, fast) as separate sessions, merge verified lanes with `git merge --no-ff`, run the batch check, and
> refresh GitNexus with `python3 scripts/gitnexus-refresh.py`. You do not verify tickets you hand out and you do not merge to `dev`
> or push. Start with "First steps for the orchestrator", then report the plan and wait for the user.
