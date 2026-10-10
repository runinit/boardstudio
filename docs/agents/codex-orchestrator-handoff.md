# Handing the planner and orchestrator role to Codex

Written 2026-10-10 by Claude, which has run this effort so far. Everything the orchestrator needs is in the repository;
nothing depends on Claude's chat history or memory. Read this page, then
[verification-process.md](verification-process.md), [gitnexus-index.md](gitnexus-index.md) and
[parallel-lanes.md](../plans/typed-core-edits/parallel-lanes.md) (lanes, prompts: [lane-prompts.md](../plans/typed-core-edits/lane-prompts.md)).

## Models and who does what

Fill the blanks for your Codex setup; the roles are fixed, the model names are not. Claude's choices are shown so you can
match capability.

| Role | What it does | Claude used | Codex model / effort |
| --- | --- | --- | --- |
| Planner / orchestrator | tickets, ADRs, prompts, ordering, lane setup, merging, batch check, GitNexus refresh, answers questions | Sonnet 5.5 (this session) | _choose_ |
| Planning research (read-only) | design options, plans, audits of code | Opus, medium effort | _choose_ |
| Implementer | one ticket in one lane worktree, test first | Haiku 5.5 | _choose a fast, cheap model_ |
| Verifier | reproduces, mutation-checks, fixes small defects, sets `resolved` | Sonnet 5.5, high | **Sol 6.1, xhigh, fast** (user choice) |
| Browser or CAD runs | needs Chrome, `wasm-pack`, podman | the same sessions, sandbox lifted | _confirm Codex can lift its sandbox for these_ |

Implementer and verifier must be different models (ideally different apps). Commit trailers name the agent that made the
commit (for example `Co-Authored-By: Codex <noreply@openai.com>`).

## Differences to settle when porting

- **Spawning agents.** Claude started implementers and verifiers as background subagents. In Codex use one session per lane
  worktree (or Codex's own subagent feature, if it has one; confirm it can use a different worktree and model). The briefs are in
  `lane-prompts.md`; they carry the rules because a lane agent does not remember between tasks.
- **No memory directory.** Claude kept five notes in its memory (below). They are restated here so Codex has them.
- **Hooks.** The GitNexus Stop hook is a Claude settings entry (`docs/agents/gitnexus-index.md`). Codex has its own hook or
  automation mechanism; either wire `python3 scripts/gitnexus-refresh.py --background` to its end-of-turn event, or run
  `python3 scripts/gitnexus-refresh.py` by hand at batch boundaries. The script is single-flight and idempotent either way.
- **Sandbox.** `wasm-pack`, Chrome and the CAD container (podman) need an unsandboxed run; cargo needs the `target` link
  writable. Note any step Codex cannot run and let the user or another session run it.
- **Settings files.** The harness protects `.claude/settings*.json` from agent edits; the user adds hook entries.

## Rules the user set (Claude's memory, restated)

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

## State snapshot (2026-10-10)

Integration branch `typed-core-edits/mechanical` at `a1672ca91` or later; nothing is merged to `dev` and nothing is pushed.

| Area | Done (resolved) | In flight or ready |
| --- | --- | --- |
| generator parameters (map: `docs/plans/generator-parameters/map.md`) | 01-06 | none |
| mirrored halves (map: `docs/plans/typed-core-edits/map.md`) | 16, 17, 18, 26, 27, 28, 29, 30 | **31, 32 resolved on `lane/mirror-core` (not merged)**; 39 ready (Codex); 19 (alone, riskiest), 40, 41, 20, 21, 22, 23 later |
| test speed / audit | 33, 34, 36 | **37 in progress in `.worktrees/tce-matrix` (Haiku, uncommitted edits)**; 38, 35, 24 later |
| check speed | 10 | 11 (needs the machine alone), 12, 13 (decision) |
| boards review | ticket 25 phase 1 done and verified on `lane/boards-review` | the user has since recorded hardware decisions there (commit `c47a27df5`); read ticket 25 and the audit doc, then write the per-board fix tickets, Sofle first |

Worktrees: `.worktrees/tce-matrix` (integration), `.worktrees/lane-mirror-core` (`lane/mirror-core`, merged with integration at
`21e4ea385`), `.worktrees/lane-boards-review` (`lane/boards-review`, docs only). Older worktrees from earlier efforts remain; leave them.

## Open items that need the user

- Add the GitNexus Stop hook to the local settings (snippet in `gitnexus-index.md`), or decide to run the refresh by hand.
- Hardware decisions for the shipped boards (ticket 25) if not all answered in `c47a27df5`.
- Ticket 13: whether to pursue a structural browser-step change after tickets 11 and 12 measure.
- Before anything leaves the machine or merges to `dev`: the main checkout's uncommitted docs conflict with a merge; the user decides.

## First steps for the new orchestrator

1. `git -C .worktrees/tce-matrix log --oneline -15` and read the ticket statuses above to confirm the snapshot.
2. Wait for (or ask about) ticket 37, then verify it with Codex Sol 6.1.
3. Merge `lane/mirror-core` (31, 32) and `lane/boards-review` into the integration branch once 37 has landed; run the batch check once; refresh GitNexus.
4. Hand ticket 39 to an implementer, then 19 alone.
