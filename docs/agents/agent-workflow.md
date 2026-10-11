# Agent workflow: roles, rules and handoff (Codex and Claude Code)

Project-wide, for every branch and both apps. `AGENTS.md` points here. Written 2026-10-10 when the orchestrator role moved from
Claude to Codex. Everything an orchestrator or lane agent needs is in the repository; nothing depends on chat history or one
app's private memory. Read this page, then
[verification-process.md](verification-process.md), [gitnexus-index.md](gitnexus-index.md) and
`docs/plans/typed-core-edits/parallel-lanes.md` (lanes, prompts: `docs/plans/typed-core-edits/lane-prompts.md`).

## Models and who does what (the same in Codex and Claude)

Set by the user on 2026-10-10 and revised on 2026-10-11 to reduce duplicate verification. Choose the lowest sufficient effort;
use one fresh independent review for code/behaviour changes, with escalation only when evidence warrants it.

| Role | What it does | Model / effort |
| --- | --- | --- |
| Orchestrator | tickets, ADRs, prompts, ordering, lane setup, merging, batch check, GitNexus refresh, answers questions | **Sol 6.1, medium** |
| Clerical support | inventories, extraction, status summaries, mechanical documentation | **Luna 6, low** |
| Bounded planning research (read-only) | focused source research, straightforward plans | **Luna 6, medium**; **high** for harder bounded investigation |
| Complex planning research (read-only) | material cross-domain, API or hardware/electrical risk | **Sol 6.1, max**; **Astra, medium** only as a last resort |
| Implementer | one ticket in one lane worktree, test first | **Luna 6, medium** for routine low-risk changes; **high** for substantive implementation |
| Verifier / reviewer | one fresh independent review after the completed implementation; reuses sufficient checks, targets evidence gaps | **Sol 6.1, medium**; **xhigh** only for justified complexity/escalation |
| Browser or CAD runs | needs Chrome, `wasm-pack`, podman | the same sessions, sandbox lifted |

For code/behaviour review, implementer and reviewer are different sessions/models (Luna implements, Sol reviews). Low-risk docs,
mechanical copy and tracking may use author checks plus root review without a separate verifier; see verification-process.md. Commit trailers name
the agent that made the commit (for example `Co-Authored-By: Codex <noreply@openai.com>` or the Claude trailer for work done in
Claude Code). Until 2026-10-10 Claude Haiku, Sonnet and Opus filled these roles; tickets resolved before then say so.

Active sessions keep their settings and fixed scope. Avoid delegation overhead for tiny tasks; pass fresh agents only
the necessary task, paths, references and evidence. Reuse sufficient findings. Luna preparation does not replace the
one fresh Sol acceptance review for code/behaviour changes. Reuse source-bound writer evidence; preserve recorded acceptance
requirements and counterexamples. Reassess ticket-specific counted gates explicitly rather than silently dropping them.

## Codex dispatch

Every BoardStudio lane has a persistent coordinator chat, as requested by the user on 2026-10-10. The lane coordinator
owns its planning, implementation dispatch and fresh independent review; root owns shared scheduling, leases, merges and
index refreshes. Before dispatch, editing, requesting checks or verification handoff, read the central
[lane registry](../plans/typed-core-edits/lane-registry.md) at its documented absolute integration path. Follow its
conflict/escalation procedure; blocked coordinators hold implementation while retaining their lane's planning context.

The role table is the authority; app defaults are not role selection. Codex model IDs are `gpt-6.1-sol`, `gpt-6-luna` and
`gpt-6-astra`. Select the effort in the table for each session, including the
orchestrator; an existing session keeps its selected settings when a config file changes.

Use Codex's `collaboration.spawn_agent` for work delegated within the current chat. For an implementer, set
`fork_turns: "none"`, `model: "gpt-6-luna"`, with `reasoning_effort` selected from the role table; for a fresh verifier, use
`fork_turns: "none"`, `model: "gpt-6.1-sol"`, `reasoning_effort: "medium"`; justify xhigh from actual complexity or escalation. Select planning research by the same table.
A full-history fork inherits the parent's model and effort, so it cannot select the different role model.
Select the lowest sufficient effort; do not infer or claim a runtime service tier from model selection.

Subagents share the initial working directory and filesystem. They do not get an isolated checkout automatically.
Every brief must give the **absolute** lane worktree, branch, ticket, allowed files, checks and exclusions; require an explicit
`exec_command` `workdir` on every shell call. Before editing, the agent reports `pwd`, `git rev-parse --show-toplevel`,
`git branch --show-current` and `git status --short` from that worktree. Only one implementer or verifier writes a lane at a time.
The orchestrator starts the one required code/behaviour review only after the completed writer handoff, and stays responsible for the file-owner table
in `docs/plans/typed-core-edits/parallel-lanes.md`. Model selection is evidenced by dispatch/runtime settings, not an agent's
self-description.

### Handoffs and command windows

Grant a complete bounded cycle: exact paths, acceptance evidence, relevant checks and any justified mutations; when a ticket claim is required, commit
it before `START`. The implementer hands off only after stopping at the completed handoff, with the action taken and pointers to evidence. For affected browser tests or configuration, compile the actual target; intermediate RED/fix iterations remain one writer unit. Fresh independent code/behaviour review begins only after the completed handoff. Grant the
fresh Sol medium reviewer its own command window and fixed scope, reusing sufficient writer checks. Rerun targeted checks only
for a concrete risk or evidence gap; mutations prove important guarantees when needed, not every ticket automatically. Name each foreground command owner, and record transitions and
release; escalate any scope change before doing it. Keep one current state snapshot. At meaningful transitions or exceptions,
send the next action and evidence pointers instead of repeating the narrative. Poll foreground work every 30–60 seconds without
per-poll commentary or background watchdogs. Choose the lowest sufficient effort from the role table; use the existing Luna
low clerical role when needed, and preserve fresh Luna/Sol role independence.

If native subagents are unavailable, run a separate Codex session with the lane as its working directory and the same explicit
model and effort. The installed CLI accepts `codex exec -C <absolute-lane> -m <model-id> -c model_reasoning_effort=<effort>
-`, with the complete brief on stdin. Do not create sidebar chats for delegation unless the user
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
3. Ticket process: implementer sets `implemented (awaiting verification)`; the required reviewer resolves, or root reviews/resolves
   low-risk docs, mechanical copy and tracking after author checks. Run only the check steps the
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
3. Hand each ticket to an implementer with the common rules plus the lane prompt; after its completed handoff, use one fresh
   Sol medium review for code/behaviour, or author checks/root review for low-risk docs, mechanical copy and tracking.
4. Merge a verified lane into `typed-core-edits/mechanical` with `git merge --no-ff`; if a lane branch is behind, merge the
   integration branch into the lane first. Regenerate goldens, never hand-merge them.
5. After a batch: run the full `python3 scripts/check.py` once on the integration branch, write the `map.md` progress lines, refresh
   GitNexus, delete finished lane worktrees and Chromium profiles older than 30 minutes.
6. Enforce the single-writer table in `parallel-lanes.md`: the orchestrator owns maps, `CONTEXT.md` and ADRs; designated lanes
   own `scripts/check.py` and generated goldens. Ownership is assigned before dispatch.

## Current state

This file records durable workflow policy, not live ticket or worker state. Read the integration lane registry at ../plans/typed-core-edits/lane-registry.md and delivery queue at ../plans/delivery-queue.md for current owners, claims, readiness, and blockers; verify the checkout's actual state before dispatch or integration.

## Open work

The delivery queue and authoritative ticket headers record current work and human decisions. Do not carry forward the historical snapshot below or infer fresh worker state from it.

## First steps

Before dispatch or a check, read the current lane registry and ticket headers, verify the worktree/branch/status, and obtain the foreground command window. Preserve the explicit single-writer, impact, verification, and root integration/refresh rules in this document.

## Kickoff prompt for a new orchestrator session

Open Codex (Sol 6.1, medium) with `/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix` as the working folder and paste:

> You are the orchestrator for the typed-core-edits effort in BoardStudio. Read, in this order: `AGENTS.md`,
> `docs/agents/agent-workflow.md` (roles, models, rules, the state snapshot at its end), `docs/agents/verification-process.md`,
> `docs/agents/gitnexus-index.md`, `docs/plans/typed-core-edits/parallel-lanes.md` and `lane-prompts.md`, then
> `docs/plans/typed-core-edits/map.md`. You plan and orchestrate: you write tickets, hand them to implementers (Luna 6 at the lowest sufficient effort)
> and one fresh reviewer (Sol 6.1, medium by default) after each completed code/behaviour unit; low-risk docs/copy/tracking use author checks and root review. Merge verified lanes with `git merge --no-ff`, run the batch check, and
> refresh GitNexus with `python3 scripts/gitnexus-refresh.py`. You do not verify tickets you hand out and you do not merge to `dev`
> or push. Start with "First steps for the orchestrator", report the ready work and continue within the user's authorized scope.
