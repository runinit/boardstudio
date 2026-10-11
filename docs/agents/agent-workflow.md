# Agent workflow: roles, rules and handoff (Codex and Claude Code)

Project-wide, for every branch and both apps. `AGENTS.md` points here. Written 2026-10-10 when the orchestrator role moved from
Claude to Codex. Everything an orchestrator or lane agent needs is in the repository; nothing depends on chat history or one
app's private memory. Read this page, then
[verification-process.md](verification-process.md), [gitnexus-index.md](gitnexus-index.md) and
`docs/plans/typed-core-edits/parallel-lanes.md` (lanes, prompts: `docs/plans/typed-core-edits/lane-prompts.md`).

## Models and who does what (the same in Codex and Claude)

Set by the user on 2026-10-10, with a later Luna-first effort policy to reduce token usage. Keep role independence;
choose the lowest sufficient effort for each new bounded task and escalate when evidence warrants it.

| Role | What it does | Model / effort |
| --- | --- | --- |
| Orchestrator | tickets, ADRs, prompts, ordering, lane setup, merging, batch check, GitNexus refresh, answers questions | **Sol 6.1, medium, fast** |
| Clerical support | inventories, extraction, status summaries, mechanical documentation | **Luna 6, low, fast** |
| Bounded planning research (read-only) | focused source research, straightforward plans | **Luna 6, medium, fast**; **high** for harder bounded investigation |
| Complex planning research (read-only) | material cross-domain, API or hardware/electrical risk | **Sol 6.1, max, fast**; **Astra, medium** only as a last resort |
| Implementer | one ticket in one lane worktree, test first | **Luna 6, medium, fast** for routine low-risk changes; **high** for substantive implementation |
| Verifier / reviewer | reproduces, mutation-checks, fixes small defects, sets `resolved` | **Sol 6.1, xhigh, fast** |
| Browser or CAD runs | needs Chrome, `wasm-pack`, podman | the same sessions, sandbox lifted |

Implementer and verifier must be different sessions with different models (Luna implements, Sol verifies). Commit trailers name
the agent that made the commit (for example `Co-Authored-By: Codex <noreply@openai.com>` or the Claude trailer for work done in
Claude Code). Until 2026-10-10 Claude Haiku, Sonnet and Opus filled these roles; tickets resolved before then say so.

Active sessions keep their settings and fixed scope. Avoid delegation overhead for tiny tasks; pass fresh agents only
the necessary task, paths, references and evidence. Reuse sufficient findings. Luna preparation does not replace the
fresh Sol acceptance verifier/reviewer. Ticket-specific higher-effort gates remain binding until explicitly revised.

## Codex dispatch

Every BoardStudio lane has a persistent coordinator chat, as requested by the user on 2026-10-10. The lane coordinator
owns its planning, implementation dispatch and fresh independent review; root owns shared scheduling, leases, merges and
index refreshes. Before dispatch, editing, requesting checks or verification handoff, read the central
[lane registry](../plans/typed-core-edits/lane-registry.md) at its documented absolute integration path. Follow its
conflict/escalation procedure; blocked coordinators hold implementation while retaining their lane's planning context.

The role table is the authority; app defaults are not role selection. Codex model IDs are `gpt-6.1-sol`, `gpt-6-luna` and
`gpt-6-astra`. Fast mode uses `service_tier = "priority"`. Select the effort in the table for each session, including the
orchestrator; an existing session keeps its selected settings when a config file changes.

Use Codex's `collaboration.spawn_agent` for work delegated within the current chat. For an implementer, set
`fork_turns: "none"`, `model: "gpt-6-luna"`, with `reasoning_effort` selected from the role table; for a fresh verifier, use
`fork_turns: "none"`, `model: "gpt-6.1-sol"`, `reasoning_effort: "xhigh"`. Select planning research by the same table.
A full-history fork inherits the parent's model and effort, so it cannot select the different role model.
The spawn tool has no service-tier option; check the session's tier instead of claiming the tool selected fast mode.

Subagents share the initial working directory and filesystem. They do not get an isolated checkout automatically.
Every brief must give the **absolute** lane worktree, branch, ticket, allowed files, checks and exclusions; require an explicit
`exec_command` `workdir` on every shell call. Before editing, the agent reports `pwd`, `git rev-parse --show-toplevel`,
`git branch --show-current` and `git status --short` from that worktree. Only one implementer or verifier writes a lane at a time.
The orchestrator waits for the implementer to finish before starting its verifier, and stays responsible for the file-owner table
in `docs/plans/typed-core-edits/parallel-lanes.md`. Model selection is evidenced by dispatch/runtime settings, not an agent's
self-description.

### Handoffs and command windows

Grant a complete bounded cycle: exact paths, acceptance evidence, mutations and checks; when a ticket claim is required, commit
it before `START`. The implementer hands off only after stopping, with the action taken and pointers to evidence. Then grant the
fresh Sol verifier its own command window and fixed scope. Name each foreground command owner, and record transitions and
release; escalate any scope change before doing it. Keep one current state snapshot. At meaningful transitions or exceptions,
send the next action and evidence pointers instead of repeating the narrative. Poll foreground work every 30–60 seconds without
per-poll commentary or background watchdogs. Choose the lowest sufficient effort from the role table; use the existing Luna
low clerical role when needed, and preserve fresh Luna/Sol role independence.

If native subagents are unavailable, run a separate Codex session with the lane as its working directory and the same explicit
model and effort. The installed CLI accepts `codex exec -C <absolute-lane> -m <model-id> -c model_reasoning_effort=<effort>
-c service_tier=\"priority\" -`, with the complete brief on stdin. Do not create sidebar chats for delegation unless the user
asks for them.

## Differences between the apps

- **Spawning agents.** Claude used background subagents. Codex uses the dispatch procedure above. The briefs are in
  `lane-prompts.md`; pass the complete brief because a fresh lane agent has no task history.
- **Memory.** Cross-session memory is `memsearch`, shared by both apps (use the `memsearch:memory-recall` skill for a specific prior decision, rationale or failure that current context does not answer). Search once for 3–5 hits, deduplicate, and expand one relevant section. Return dated facts with source pointers; do not load journals or maintained summaries wholesale. Current human instructions, repository authorities and verified state govern over recall.
- **Hooks.** Codex uses `~/.codex/hooks.json`; Claude uses its settings file. Register only the orchestrator session for
  automatic index refresh as described in [gitnexus-index.md](gitnexus-index.md). Explicit batch-boundary refreshes remain
  required; a hook may skip while checks run. Keep the existing memsearch and design hooks.
- **Sandbox.** `wasm-pack`, Chrome and the CAD container (podman) need an unsandboxed run; cargo needs the `target` link
  writable. Note any step Codex cannot run and let the user or another session run it.
- **Local setup.** `.agents/`, `.claude/` and `.codex/` are gitignored and do not arrive with a branch merge. When setting up a
  lane, make the project's `.agents/skills` available (create a real `.agents` directory and link its `skills` to the main
  checkout's `.agents/skills`; a top-level symlink would not match the `.agents/` ignore rule). Do not copy
  orchestrator hooks into lane settings. If the harness rejects a settings edit, report that restriction and the exact required edit.

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
2. Reuse/create the lane's persistent coordinator chat and set up its worktree when needed (commands in `parallel-lanes.md`:
   own branch, `target` directory on disk, real copies of `cad/.cache` and `cad/wasm/pkg`). Record owners and leases in
   the central registry. Run one foreground command globally at a time; timing work needs the machine alone.
3. Hand each ticket to an implementer with the common rules plus the lane prompt; when it reports, hand it to the verifier with
   the verifier prompt.
4. Merge a verified lane into `typed-core-edits/mechanical` with `git merge --no-ff`; if a lane branch is behind, merge the
   integration branch into the lane first. Regenerate goldens, never hand-merge them.
5. After a batch: run the full `python3 scripts/check.py` once on the integration branch, write the `map.md` progress lines, refresh
   GitNexus, delete finished lane worktrees and Chromium profiles older than 30 minutes.
6. Enforce the single-writer table in `parallel-lanes.md`: the orchestrator owns maps, `CONTEXT.md` and ADRs; designated lanes
   own `scripts/check.py` and generated goldens. Ownership is assigned before dispatch.

## State snapshot (2026-10-10, late)

Workflow docs are on `dev` at `4e5a6fda3`; product changes remain on `typed-core-edits/mechanical` and lane branches.
`dev` was merged into integration at `4719a0bcf`. Nothing has been pushed. This is a handoff snapshot, not live state:
read each worktree's status, commits and ticket headers before dispatch or merge.

| Area | Resolved | In flight or ready |
| --- | --- | --- |
| generator parameters (`docs/plans/generator-parameters/map.md`) | 01-06 | none |
| mirrored halves (`docs/plans/typed-core-edits/map.md`) | 16, 17, 18, 26, 27, 28, 29, 30 on the integration branch; **31, 32 resolved only on `lane/mirror-core`** (verified by Claude Sonnet) | **39 claimed by Codex in `lane-mirror-core`, uncommitted work in progress**; then 19 (alone, riskiest), 40, 41, 20, 21, 22, 23 |
| test speed / audit | 33, 34, 36 | **37 `implemented (awaiting verification)` at `dc144edde`; partial scope and remaining sites are listed in its Outcome; needs an independent Sol verifier**; then 38, 35, 24 |
| check speed | 10 | 11 (needs the machine alone), 12, 13 (decision) |
| boards review | ticket 25 phase 1 done and verified (`lane/boards-review`) | the user recorded hardware decisions (`c47a27df5`) and the ticket is `ready-for-agent` again: phase 2 = per-board fix tickets, Sofle first |

Worktrees: `.worktrees/tce-matrix` (integration), `.worktrees/lane-mirror-core` (`lane/mirror-core`, merged with integration at `21e4ea385`,
then the 39 claim), `.worktrees/lane-boards-review` (`lane/boards-review`, docs only). Older worktrees from earlier efforts remain; leave them.

The ticket 37 implementer has finished; `.worktrees/tce-matrix` is available for verification once its live status is clean.
Ticket 39 is still being implemented in `lane-mirror-core`; preserve that work and coordinate before changing its branch.

## Open items that need the user

- Hardware decisions for the shipped boards (ticket 25) if not all answered in `c47a27df5`.
- Ticket 13: whether to pursue a structural browser-step change after tickets 11 and 12 measure.
- Before anything leaves the machine or merges to `dev`: the main checkout's uncommitted docs conflict with a merge; the user decides.

## First steps for the orchestrator

1. `git -C .worktrees/tce-matrix log --oneline -15` and `git worktree list`; read the ticket statuses above to confirm the snapshot.
2. Register this orchestrator for the integration worktree's refresh hook (see `gitnexus-index.md`). Hand ticket 37 to a fresh
   Sol 6.1 verifier; the orchestrator does not verify its own dispatch.
3. Merge `lane/mirror-core` (31, 32) and `lane/boards-review` into the integration branch once 37 has landed; run the batch check once; refresh GitNexus.
4. Check the existing ticket 39 session and its lane status; do not dispatch a second implementer. After its independent
   verification, run 19 alone.

## Kickoff prompt for a new orchestrator session

Open Codex (Sol 6.1, medium, fast) with `/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix` as the working folder and paste:

> You are the orchestrator for the typed-core-edits effort in BoardStudio. Read, in this order: `AGENTS.md`,
> `docs/agents/agent-workflow.md` (roles, models, rules, the state snapshot at its end), `docs/agents/verification-process.md`,
> `docs/agents/gitnexus-index.md`, `docs/plans/typed-core-edits/parallel-lanes.md` and `lane-prompts.md`, then
> `docs/plans/typed-core-edits/map.md`. You plan and orchestrate: you write tickets, hand them to implementers (Luna 6, high, fast)
> and verifiers (Sol 6.1, xhigh, fast) as separate sessions, merge verified lanes with `git merge --no-ff`, run the batch check, and
> refresh GitNexus with `python3 scripts/gitnexus-refresh.py`. You do not verify tickets you hand out and you do not merge to `dev`
> or push. Start with "First steps for the orchestrator", report the ready work and continue within the user's authorized scope.
