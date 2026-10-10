# Running the remaining tickets in parallel lanes (Codex, Claude, others)

Written 2026-10-10. For an orchestrator who wants several apps or agents working at once. It reuses the
shape of the earlier [module deepening handoff](../module-deepening/handoff.md) (roles, waves, a
"touches" column) and adds what this branch taught us: a shared build directory serialises everything,
timing work must run alone, and a few files can only have one writer. Process for each ticket:
[verification-process.md](../../agents/verification-process.md). Tracker rules:
[issue-tracker.md](../../agents/issue-tracker.md).

## Roles

- **Orchestrator (one person or session):** owns the integration branch `typed-core-edits/mechanical`, hands out
  lanes, merges them in order, runs the batch check, refreshes GitNexus, updates `map.md`.
- **Lane agent (one per lane at a time):** works only in its lane worktree on its lane branch, on the files its
  tickets name, and writes an `## Outcome` in its ticket files. It never edits `map.md`, never merges, never
  pushes, never refreshes GitNexus.
- **Implementer and verifier are different sessions with different models** (Luna 6 implements, Sol 6.1 verifies; models and
  efforts are in [agent-workflow.md](../../agents/agent-workflow.md)). An independent model reviewing the work is the point of
  the verify step.
- **Human:** answers decision tickets and approves anything that leaves the machine.

## Lanes

| Lane | Tickets, in order | Touches (main files) | Heavy checks | Notes |
| --- | --- | --- | --- | --- |
| A web test speed | [36](issues/36-sleeps-case-pcb-parts.md), [37](issues/37-sleeps-layout-library-ui-page.md), [38](issues/38-merge-duplicate-browser-tests.md) | `web/crates/{case,pcb,parts,layout,library,ui-shared,keycaps,keymap}` test code, `web/src/presentation` tests | `browser` (Chrome) | no Core edits; confirm current ticket status before dispatch. |
| B mirror core | [31](issues/31-companion-placement-modes.md), [32](issues/32-mirrored-half-labels.md), then [19](issues/19-reconcile-mirrored-halves.md), then [20](issues/20-recipes-build-one-half.md) | `core/src/mirrored_half.rs`, `core/src/mechanical/halves.rs`, `core/src/model.rs`, `core/tests/mirrored_half.rs`, recipes | `lint test typecheck`; 20 adds `build` | 19 is the riskiest ticket: run it alone in this lane. |
| C check speed | [11](../check-speed/issues/11-browser-measure.md) (alone, quiet machine), then [12](../check-speed/issues/12-browser-quick-wins.md) | `scripts/check.py`, `scripts/run-wasm-tests.py`, `.github/workflows` | `browser` many times | 11 measures timings, so nothing else may run. 12 waits for lane A to finish 38. |
| D audit and cleanup | [35](issues/35-test-maintenance-cuts.md) | version literals in tests; goldens (`transcript.json`, the demo baseline) | full `check.py` | Golden edits last, after B merges, then re-bless. |
| E web adapter | [21](issues/21-mirrored-half-web-adapter.md), then [22](issues/22-mirrored-half-integrated-verification.md) | `web/crates/catalogue`, `web/crates/layout`, physical setup | `typecheck browser` | waits for B to merge 19 and 32. |
| F planning (no builds) | [25](issues/25-shipped-boards-review.md) research, [23](issues/23-redo-keyboard-04-on-native-halves.md) later | docs, read-only fixtures | none | safe to run any time; fits a different app well. |

Independent lanes: A, B and F can start now. C and D wait as noted. E waits for B.

## Files with one writer only

Parallel edits to these produce conflicts that are noise. The owner is the only one who edits them; everyone else
leaves them alone and asks.

| File | Owner |
| --- | --- |
| `docs/plans/typed-core-edits/map.md`, `docs/plans/check-speed/map.md` | orchestrator, at merge time |
| `core/tests/fixtures/boundary/transcript.json`, `core/tests/fixtures/demo-projects-baseline.json` | lane B, then lane D; generated, never hand-merged: on a conflict take the integration branch's file and re-run `BOUNDARY_BLESS=1` |
| `CONTEXT.md`, `docs/adr/*` | orchestrator; lanes propose text in their Outcome |
| `scripts/check.py` | lane C |
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
- The machine has 16 CPUs and 30 GB RAM. **At most two lanes run heavy checks at once**, and ticket 11 and
  any other timing measurement run alone. A noisy neighbour made earlier timings unreliable.
- Native `test` works from a nested worktree now (check-speed ticket 10).

## The brief a lane agent gets

Paste this, then name the ticket(s). Everything else is in the ticket.

> Work only in `.worktrees/lane-<lane>` on branch `lane/<lane>`. Read `AGENTS.md`,
> `docs/agents/verification-process.md` and your ticket(s). Claim a ticket by setting `Status: claimed` and
> committing (implementers only; verifiers keep `implemented (awaiting verification)` until acceptance holds).
> Use your absolute lane path as `exec_command`'s `workdir` on every shell call; report the path, branch and status first.
> Implement test-first, run only the check steps the process table names, one command at a time in the
> foreground. Stage explicit paths; no `git stash`, no `git add -A`, no push, no merge. Do not edit `map.md`,
> `CONTEXT.md`, ADRs or the generated goldens unless ownership is assigned to you. Run GitNexus impact before edits and
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

Use the handoff snapshot in [agent-workflow.md](../../agents/agent-workflow.md), then confirm live worktree status and ticket
headers. In particular, ticket 37 is awaiting verification and ticket 39 already has an implementer in lane B; a status on
the integration branch can lag a lane's claimed or resolved status. Set up C, D and E only when their predecessors are resolved.
