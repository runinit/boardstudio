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
| Implementer | one coherent bounded unit in one lane worktree, test first | **Luna 6, medium** for routine low-risk changes; **high** for substantive implementation |
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

Grant a complete bounded cycle: exact paths, acceptance evidence, relevant checks and any justified mutations. Commit an initial claim before first `START` where required; the standing-authority rule below supersedes per-child START/claim checkpoints within an already approved chain. The implementer hands off only after stopping at the completed handoff, with the action taken and pointers to evidence. For affected browser tests or configuration, compile the actual target; intermediate RED/fix iterations remain one writer unit. Fresh independent code/behaviour review begins only after the completed handoff. Grant the
fresh Sol medium reviewer its fixed scope, reusing sufficient writer checks; grant a HEAVY slot only if its targeted verification needs heavy execution. Rerun targeted checks only
for a concrete risk or evidence gap; mutations prove important guarantees when needed, not every ticket automatically. Name the GLOBAL HEAVY owner, and record transitions and
release; escalate any scope change before doing it. Keep one current state snapshot. At meaningful transitions or exceptions,
send the next action and evidence pointers instead of repeating the narrative. Poll active heavy work every 30–60 seconds without
per-poll commentary or background watchdogs. Choose the lowest sufficient effort from the role table; use the existing Luna
low clerical role when needed, and preserve fresh Luna/Sol role independence.

If native subagents are unavailable, run a separate Codex session with the lane as its working directory and the same explicit
model and effort. The installed CLI accepts `codex exec -C <absolute-lane> -m <model-id> -c model_reasoning_effort=<effort>
-`, with the complete brief on stdin. Do not create sidebar chats for delegation unless the user
asks for them.

### Standing lane authority and continuity

A user/root-approved standing chain authorizes coordinator planning → Luna implementation → ONE fresh Sol 6.1 medium acceptance review → dependency-ready fixed-scope continuation. The coordinator may group tightly coupled private preparation slices into one coherent unit when ownership and each ticket’s acceptance remain explicit; do not create a review/session automatically for every ticket number. Within that approved chain, routine private helpers, tests, fixture ports and fixed-scope children do not need another root START or clerical claim checkpoint. Record child claims and satisfied acceptance in the existing tracker/handoff; root updates shared tracking at meaningful integration boundaries. Existing meaningful regressions, counterexamples, complete graph gates and final parent/activation acceptance remain binding.

Current explicit approvals and grants survive compaction; older branch docs, summaries or handoffs cannot revoke them. A handoff to root is notification, not an implicit STOP unless it identifies an actual decision/conflict requiring a stop. Writers still STOP at the completed handoff before fresh acceptance.

User-approved lighter scheduling (2026-10-11) supersedes every older global-every-shell/read rule, including historical checkpoints and branch copies. Read-only shell commands, searches, diffs and graph queries run freely without a global lease; approved disjoint source work in separate worktrees may run alongside another lane’s checks. Keep one writer per worktree, protect shared writable outputs and execute commands sequentially within each lane.

Only builds/compilation, test execution, browser runs, timing and index refresh use the single GLOBAL HEAVY slot. Root schedules its explicit owner and release; resource-heavy runs never overlap, and timing requires the machine alone. Root retains source-bound index refresh, integration/batch checks and generated goldens.

**L7 private UI development exception (user approval2026-10-11):** the UI redesign lane may keep its own dev server/watchers running and perform frontend rebuilds and visual browser inspection independently, using its own target, provider outputs, assets and port. It may bootstrap private providers once, then rebuild only frontend inputs for UI-only changes; refresh providers when their inputs change. The lane records its live processes/port/output ownership and uses the supported Dioxus/Tailwind pipeline after source-grounded research. This exception overrides the GLOBAL HEAVY requirement only for that private development loop. Shared output writes, full/affected acceptance suites, CAD validation, integration builds and GitNexus refresh remain scheduled. Timing requires an explicit pause of active UI compilation/browser activity. Private dev-server readiness is not acceptance evidence by itself.

Grant a complete bounded debugging + affected-check cycle. The coordinator carries approved RED → fix → GREEN in the same HEAVY window without per-command or single-probe handoffs; escalate actual scope/conflict decisions, not ordinary debugging iterations. Fresh independent review begins after the completed writer STOP; read-only review needs no HEAVY slot, and any necessary heavy verification obtains one explicit window. Finish/restore an already-running probe before switching writers.

Escalate actual product decisions or representability limits, cross-lane ownership/conflicts, genuinely unapproved API visibility/schema/configuration changes, destructive user-data actions, or a concrete failed-acceptance scope. Already approved ticket/ADR feature operations and schema contracts are authorized as specified, including MakeLocal/Omit/Rejoin/Unlink and companion placement; they do not need repeated human approval. Routine private implementation/test choices are not new gates. The narrow wrapper approval is recorded in ticket106; it is not blanket permission to widen other visibility.

### Laptop multi-stream resource mode

User-approved2026-10-11; active while multiple workstreams/builds are in use. Each lane sets these variables in its command environment, including inherited dev-server/watcher processes:

```sh
export CARGO_BUILD_JOBS=4
export RUST_TEST_THREADS=4
export BOARDSTUDIO_CHECK_JOBS=1
export BINARYEN_CORES=4
```

This caps Cargo compilation at four jobs, each native Rust test executable at four test threads, and Binaryen WASM optimization at four workers ([upstream environment-variable documentation](https://github.com/WebAssembly/binaryen/wiki/Debugging)). Serial suite launches prevent the check runner from multiplying those pools. These are worker limits, not hard CPU affinity or a four-core aggregate guarantee. Containerized commands must receive the caps explicitly; host environment variables alone do not establish container limits. Other runners use their supported worker limit of at most four; keep one active browser instance per lane. A private UI watcher inherits the caps on its next safe restart. Preserve running command evidence; changing the cap alone does not require a rerun.

Root records the active mode in the lane registry and includes it in dispatch briefs. Existing GLOBAL HEAVY ownership, isolated outputs and L7 private-development exception still apply; caps do not authorize additional concurrent heavy lanes. Integration checks use the same caps while laptop mode is active. The checked-in12-job default is available outside this mode; do not edit shared/global configuration to switch modes.

During development run meaningful focused regressions and affected crate/target checks, including actual WASM compilation and affected mounted UI coverage when relevant. Full workspace/native/browser suites run once when integrating a coherent batch into the main app integration branch. A concrete cross-cutting risk or evidence gap may justify broader lane checks; record why. Reuse adequate source-bound writer evidence during independent review. See [verification-process.md](verification-process.md) for acceptance and integration requirements.

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
2. Use the laptop multi-stream resource mode below while multiple lanes are active; it overrides the checked-in Cargo default of12 jobs. Never run `python3 scripts/check.py release`.
3. Ticket process: implementer sets `implemented (awaiting verification)`; the required reviewer resolves, or root reviews/resolves
   low-risk docs, mechanical copy and tracking after author checks. Run only the check steps the
   change needs; the full `check.py` runs once per batch. Heavy runs use one GLOBAL HEAVY slot; commands remain sequential within each lane. A scope given to an agent is
   fixed when it starts; private helpers/tests needed for its accepted contract stay in the unit, while unrelated product scope goes to root.
4. GitNexus is never refreshed by lane agents; the orchestrator refreshes at batch boundaries and addresses the index by
   `--repo boardstudio-<worktree>`.
5. Preserve unrelated work: the main checkout holds the user's uncommitted `AGENTS.md`, `DESIGN.md`, `PRODUCT.md`,
   `docs/README.md` and untracked docs. Do not touch them.

## The orchestrator's loop

1. Read `docs/plans/typed-core-edits/map.md` and the ticket headers (`Status:`, `Blocked by:`); pick the ready tickets per lane.
2. Reuse/create the lane's persistent coordinator chat and set up its worktree when needed (commands in `parallel-lanes.md`:
   own branch, `target` directory on disk, real copies of `cad/.cache` and `cad/wasm/pkg`). Record owners and leases in
   the central registry. Schedule one GLOBAL HEAVY slot for builds/tests/browser/timing/index refresh; read-only commands and disjoint source work run freely. Timing needs the machine alone.
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

Before dispatch or a check, read the current lane registry and ticket headers and verify the worktree/branch/status; obtain the GLOBAL HEAVY slot only for heavy execution. Preserve the explicit single-writer, impact, verification, and root integration/refresh rules in this document.

## Kickoff prompt for a new orchestrator session

Open Codex (Sol 6.1, medium) with `/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix` as the working folder and paste:

> You are the orchestrator for the typed-core-edits effort in BoardStudio. Read, in this order: `AGENTS.md`,
> `docs/agents/agent-workflow.md` (roles, models, rules, the state snapshot at its end), `docs/agents/verification-process.md`,
> `docs/agents/gitnexus-index.md`, `docs/plans/typed-core-edits/parallel-lanes.md` and `lane-prompts.md`, then
> `docs/plans/typed-core-edits/map.md`. You plan and orchestrate: you write tickets, hand them to implementers (Luna 6 at the lowest sufficient effort)
> and one fresh reviewer (Sol 6.1, medium by default) after each completed code/behaviour unit; low-risk docs/copy/tracking use author checks and root review. Merge verified lanes with `git merge --no-ff`, run the batch check, and
> refresh GitNexus with `python3 scripts/gitnexus-refresh.py`. You do not verify tickets you hand out and you do not merge to `dev`
> or push. Start with "First steps for the orchestrator", report the ready work and continue within the user's authorized scope.
