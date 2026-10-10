# Running the remaining tickets in parallel lanes (Codex, Claude, others)

Written 2026-10-10. For an orchestrator who wants several apps or agents working at once. It reuses the
shape of the earlier [module deepening handoff](../module-deepening/handoff.md) (roles, waves, a
"touches" column) and adds what this branch taught us: a shared build directory serialises everything,
timing work must run alone, and a few files can only have one writer. Process for each ticket:
[verification-process.md](../../agents/verification-process.md). Tracker rules:
[issue-tracker.md](../../agents/issue-tracker.md).

Persistent coordinator chats, live worktree/file leases, command windows and cross-lane conflict escalation are in
[lane-registry.md](lane-registry.md). Read its central integration copy before every dispatch/edit/check handoff.

## Roles

- **Orchestrator (one person or session):** owns the integration branch `typed-core-edits/mechanical`, hands out
  lanes, merges them in order, runs the batch check, refreshes GitNexus, updates `map.md`.
- **Lane agent (one per lane at a time):** works only in its lane worktree on its lane branch, on the files its
  tickets name, and writes an `## Outcome` in its ticket files. It never edits `map.md`, never merges, never
  pushes, never refreshes GitNexus.
- **Lane coordinator (one persistent chat per lane):** owns planning and dispatches its implementer then a fresh verifier;
  follows the registry's leases and escalates cross-lane overlaps to root before contested edits.
- **Implementer and verifier are different sessions with different models** (Luna 6 implements, Sol 6.1 verifies; models and
  efforts are in [agent-workflow.md](../../agents/agent-workflow.md)). An independent model reviewing the work is the point of
  the verify step.
- **Human:** answers decision tickets and approves anything that leaves the machine.

## Lanes

| Lane | Tickets, in order | Touches (main files) | Heavy checks | Notes |
| --- | --- | --- | --- | --- |
| A web test speed | [36](issues/36-sleeps-case-pcb-parts.md), [37](issues/37-sleeps-layout-library-ui-page.md), [38](issues/38-merge-duplicate-browser-tests.md) | `web/crates/{case,pcb,parts,layout,library,ui-shared,keycaps,keymap}` test code, `web/src/presentation` tests | `browser` (Chrome) | no Core edits; confirm current ticket status before dispatch. |
| B mirror core | 31, 32, 39 integrated; [105](issues/105-mirrored-half-derivation-kernel.md) through the [19](issues/19-reconcile-mirrored-halves.md) child chain, then 40 and [20](issues/20-recipes-build-one-half.md) | `core/src/mirrored_half.rs`, `core/src/mechanical/halves.rs`, `core/src/model.rs`, `core/tests/mirrored_half.rs`, recipes | ticket-scoped checks; 20 adds `build` | Core children stay serial with exact file/API leases; isolated A/F edits may overlap after 105 verification. Activation/timing windows stay exclusive. |
| C check speed | [11](../check-speed/issues/11-browser-measure.md) (alone, quiet machine), then [12](../check-speed/issues/12-browser-quick-wins.md) | `scripts/check.py`, `scripts/run-wasm-tests.py`, `.github/workflows` | `browser` many times | 11 measures timings, so nothing else may run. 12 waits for lane A to finish 38. |
| D audit and cleanup | [35](issues/35-test-maintenance-cuts.md) | version literals in tests; goldens (`transcript.json`, the demo baseline) | full `check.py` | Golden edits last, after B merges, then re-bless. |
| E web adapter | [21](issues/21-mirrored-half-web-adapter.md), 41, then [22](issues/22-mirrored-half-integrated-verification.md) | `web/crates/catalogue`, `web/crates/layout`, physical setup | `typecheck browser` | waits for B's native-model dependencies; follow ticket headers. |
| F boards/components | [25](issues/25-shipped-boards-review.md), selected Sofle component follow-ups 118–127; [23](issues/23-redo-keyboard-04-on-native-halves.md) later | exact component assets/tests after audit; shared registration/recipes have one owner | affected checks in root windows | BOM selection delegated; existing catalogue first. Isolated qualification/onboarding may overlap B after reviewed BOM/path leases; native recipes retain 20+44 dependencies. VIK deferred. |

Approved scheduling: B Core work remains serial; A's isolated remaining37 tests and F's qualified component work may
receive disjoint edit leases after 105 verification and exact audits. Root separately grants every foreground command
window; no concurrent builds/browser runs. A/F audits and planning start now. 38 remains blocked by incomplete37;
C/D/E preparation may proceed, while their implementation dependencies remain. See the central registry for actual leases.

## Files with one writer only

Parallel edits to these produce conflicts that are noise. The owner is the only one who edits them; everyone else
leaves them alone and asks.

| File | Owner |
| --- | --- |
| `docs/plans/typed-core-edits/map.md`, `docs/plans/check-speed/map.md` | orchestrator, at merge time |
| `core/tests/fixtures/boundary/transcript.json`, `core/tests/fixtures/demo-projects-baseline.json` | orchestrator only; lanes propose regeneration in their Outcome. Generated, never hand-merged; root regenerates when needed at the final batch boundary. |
| `CONTEXT.md`, `docs/adr/*` | orchestrator; lanes propose text in their Outcome |
| `scripts/check.py` | lane C; root's temporary tooling-test-list conflict ownership ended at workflow merge `f2bccedc` |
| GitNexus index | orchestrator ([gitnexus-index.md](../../agents/gitnexus-index.md)) |

## Setting up a lane

From the integration branch's worktree (replace `<lane>`). Resolve paths before changing directory so lanes are siblings
under the main checkout, including when this command starts in `.worktrees/tce-matrix`:

```bash
integration=$(pwd -P)
project=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")
lane="$project/.worktrees/lane-<lane>"
git worktree add "$lane" -b lane/<lane> typed-core-edits/mechanical
cd "$lane"
mkdir -p ~/.cache/boardstudio-target/<lane>
ln -s ~/.cache/boardstudio-target/<lane> target     # the tooling tests look for target/ in the checkout
mkdir -p .agents
ln -s "$project/.agents/skills" .agents/skills
cp -a "$integration/cad/.cache" cad/.cache         # real copies, only needed for `build`
cp -a "$integration/cad/wasm/pkg" cad/wasm/pkg
```

- **A separate `target` per lane.** The old advice to share one `target/` serialises every cargo run on one
  build lock and lets one lane's rebuild invalidate another's. Disk is the cost: the first build in a fresh target
  is cold (about 5 minutes for `build`), and each target is several GB. Keep targets on disk, never under `/tmp`.
- The machine has 16 CPUs and 30 GB RAM. The user's current rule is **one foreground command at a time globally**;
  never overlap builds or browser runs. Root grants the check window. Ticket 11 and other timing work require
  the machine alone. Documentation research may proceed while a check runs, without shell/build overlap.
- Native `test` works from a nested worktree now (check-speed ticket 10).

## The brief a lane agent gets

Paste this, then name the ticket(s). Everything else is in the ticket.

> Work only in `.worktrees/lane-<lane>` on branch `lane/<lane>`. Read `AGENTS.md`,
> `docs/agents/verification-process.md` and your ticket(s). Claim a ticket by setting `Status: claimed` and
> committing (implementers only; verifiers keep `implemented (awaiting verification)` until acceptance holds).
> Use your absolute lane path as `exec_command`'s `workdir` on every shell call; report the path, branch and status first.
> Implement test-first, run only the check steps the process table names, one command at a time in the
> foreground. Stage explicit paths; no `git stash`, no `git add -A`, no push, no merge. Do not edit `map.md`,
> `CONTEXT.md`, ADRs or the generated goldens. Run GitNexus impact before edits and
> detect-changes before commits, bound to your checkout. Do not refresh GitNexus; if its answer is stale, empty or UNKNOWN,
> confirm callers with `rg` and report the limitation. Follow the role time boxes in `verification-process.md`.
> Implementers finish with the ticket set to `implemented (awaiting verification)` and an honest
> Outcome. Commit trailers name the agent that made the commit. Report what you ran and what you did not.

Verifier brief: the same, plus "do not trust the Outcome; reproduce it; fix only small clearly correct defects; set
`resolved` only if the ticket's own acceptance holds".

## Merging

1. The verifier marks a ticket `resolved` on the lane branch.
2. The orchestrator merges lanes into `typed-core-edits/mechanical` in dependency order (B before E; A before C's
   12; goldens last). Prefer `git merge --no-ff` of the lane branch; resolve conflicts only in files the lane owns.
3. Re-bless generated goldens if needed after merging. After the batch's lanes have landed, run the **batch check**
   (full `check.py`, never `release`) once on the integration branch, then update `map.md` and refresh GitNexus.
4. Remove a finished lane worktree and branch only after the merge is recorded, the tree is clean and no agent owns it.
   Preserve active and unrelated worktrees.

Nothing merges to `dev` without the user.

## Cross-app notes

- Codex and Claude both read `AGENTS.md`, so the repo rules already apply. Neither should rely on chat history: the
  ticket file and this page are the whole briefing.
- Commit trailers should name the real author (the earlier tickets used a fixed trailer; from here use the agent that
  wrote the commit).
- Sandboxes differ. `wasm-pack`, Chrome and the CAD container need network-free but unsandboxed execution; say so in
  the lane brief for the app being used.
- A lane that cannot run a heavy check (no Chrome, no podman) reports "not run" and the orchestrator runs it at merge.

## Lane status

2026-10-10 batch at `12a6784d` passed all default checks (813.3 s). Lane B 31/32/39 and lane F's audit/planning
are integrated. Ticket 37's delivered slice is independently verified at `fb2fe7bb7`, but its remaining acceptance
keeps 37 open and 38 blocked. The mirror sub-orchestrator coordinates 105 first, Luna 6 high implementation then
a fresh Sol 6.1 xhigh verifier; root owns sync, graph refresh, batch tests and merges. The boards sub-orchestrator
owns Sofle BOM selection, the hardware report and bounded component plans in the managed `codex/sofle-adaptations`
worktree. The user delegated best-fit hardware selection and finding/adding missing footprints; IDs 118–127 are
reserved for these follow-ups. Component qualification, native-model prerequisites and root command-window scheduling
remain gates. Confirm live status before dispatch.
