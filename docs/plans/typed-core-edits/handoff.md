# Implementation handoff: outline phase completion, then remaining matrix work

## Start here

The typed Core edits effort has two phases resolved and one in final verification:

- **Mechanical phase — RESOLVED.** Ticket 02 and children 05-10 are resolved with
  recorded Outcomes (see [map](map.md) Progress). The integrated parent branch
  holds everything through the legacy shared-construction read-back fix and the
  review fixes; the full check passed on the integrated source; the boundary
  transcript was unchanged through that phase.
- **Outline phase — near completion.** Decision 03 and tickets 11-14 are resolved
  with recorded Outcomes. Ticket 15's verification artifacts are done (see below);
  two acceptance gaps found by that verification are being fixed on the 14 lane
  RIGHT NOW by a live background worker; the phase then needs integration, a full
  check, independent reviews and final Outcomes.

Repository: `/home/chris/01_Projects/ts-boardstudio2`. All work happens in the
parent worktree `.worktrees/tce-matrix` on branch `typed-core-edits/mechanical`
plus per-ticket lane worktrees. Do not use the main checkout (it holds unrelated
user edits: `AGENTS.md`, `DESIGN.md`, `PRODUCT.md`, `docs/README.md`, untracked
research docs — preserve untouched).

## Live state at pause (2026-10-09)

- Parent branch `typed-core-edits/mechanical` at `4fc15897f` (all docs current;
  contains tickets 11-14 fully merged). The last doc commits resolved 14 and set
  the frontier to 15.
- Lane branches (all based on the parent, all clean unless noted):
  - `typed-core-edits/outline-ownership` `562ede9db` — merged (11).
  - `typed-core-edits/outline-resolution` `8bba73280` — merged (12).
  - `typed-core-edits/outline-edits` `069bc5f0c` — merged via lane merge `ba694988a` (13 + CreateGenerated).
  - `typed-core-edits/outline-web-adapter` — **IN FLIGHT** (14 fix round). A GLM
    5.3 Flash implementer (async child, fresh context) is working in
    `.worktrees/tce-outline14` from clean `6f9b3e2e7`, which already carries
    `d3cfdd7cf` (linked-refinement creation control + Copy product-context
    coverage) and `6f9b3e2e7` (never reissue a retired outline version identity
    from the adapter). When its completion notification arrives, read its report
    (output target `/tmp/tce-outline-reports/14-outline-web-adapter.md`), verify
    its claims, then continue with the integration steps below. It holds the
    Chrome lease until it releases it.
  - `typed-core-edits/outline-verification` `b8bcc687a` — ticket 15's commits,
    NOT yet merged into the parent: `39c0a6c06` (transcript append +20607/−0,
    red-before-bless with semantic assertions, native + WASM parity green),
    `80147bf69` (application/tests/outline_queue_orders.rs: both queue orders,
    one Undo per action), `b8bcc687a` (test-only serde_json dev-dependency + lock).
- Worktrees: `.worktrees/tce-outline` (11, merged, idle), `tce-outline12` (12,
  merged, idle), `tce-outline13` (13, merged, idle), `tce-outline14` (in flight),
  `tce-outline15` (15 branch). Old mechanical writer worktrees were removed;
  branches retained.
- Non-nested validation worktree `/tmp/boardstudio-tce-verify` detached at
  `b8bcc687a` (used for the native `test` half of the full check; see environment
  constraints). Its untracked `cad/.cache` symlink is expected state.
- Ticket 15's report: `/tmp/tce-outline-reports/15-integrated-verification.md`
  (transcript evidence, full-check results, product gesture pass with honest
  gaps, per-case evidence map). Lane reports live in `/tmp/tce-outline-reports/`;
  the mechanical phase's reports are in `/tmp/tce-next-reports/` and
  `/tmp/tce-reviews/`.

## Next steps, in order

### 0. Pending user-requested cleanup review (run FIRST when resuming)

The user requested a fresh-context parallel cleanup review of the current work
before further integration. Run it exactly as specified:

- Launch TWO reviewer subagents in parallel with `context: "fresh"` (never fork).
  Reviewers must inspect the repository, relevant instructions (AGENTS.md,
  CONTEXT.md, ticket contracts) and the current diff directly from files and
  commands — NOT the main conversation history. Review-only: they must not edit
  files. Prefer `output: false` for each reviewer; do not write reviewer output
  files into the repository. Their response is review feedback, not a context
  summary: concise, evidence-backed findings with file/line references and the
  smallest safe fix.
- **Reviewer 1 — deslop pass** (agent: `standards-reviewer` or equivalent
  read-only reviewer): AI-slop patterns in the changed scope: comments that
  restate code, placeholder text, stale rationale, debug leftovers; defensive
  checks that hide useful errors, vague defaults, validating trusted internal
  data after a real boundary; type escapes, broad casts, duplicated type
  definitions, object-bag typing where a local source-of-truth type exists;
  style drift from nearby non-slop code and project instructions;
  generated-sounding docs/changelog/UI copy/status text/test names;
  pass-through wrappers, dead helpers, duplicate helper signatures, duplicated
  test harness setup, abstractions that enforce no invariant; noisy/vague/
  brittle UI or CLI copy. Tool output and slop-scan findings are leads, not
  verdicts — flag only concrete issues with evidence, severity, file/line and
  the smallest safe fix.
- **Reviewer 2 — verbosity pass**: needless verbosity in code, tests, docs,
  status text, receipts, changelog wording: single-use helpers that paraphrase an
  expression; temporary variables naming obvious expressions; nested returns/
  branches that can become direct returns without hiding intent; multi-line
  cleanup scaffolding replaceable by a local direct pattern; repeated boilerplate
  that could use an existing fixture or small local helper; tests restating
  formatter details covered at a cheaper layer; regression tests where one
  focused assertion would cover the bug but wrapper tests repeat the same claim;
  prose that says the same thing twice or buries the important rule. Shorter is
  only better when clearer AND it preserves behavior, error signals, cleanup
  semantics, useful invariants and local style.
- After both return: synthesize into fixes worth doing now / optional
  improvements / feedback to ignore or defer (with a short reason each). Do not
  blindly apply. **Without an explicit `autofix` word from the user, ASK before
  applying any fixes**, ending with a compact numbered menu:
  `[1] Apply only the fixes worth doing now. [2] Apply fixes plus optional improvements.`
  (`autofix` in an invocation is workflow control, not scope — strip it first.)

### 1. Finish ticket 14's fix round

When the GLM worker's report arrives: verify its claims at the seams (mounted
tests, product screenshots/artifact in `/tmp/tce-outline-reports/`), merge its
tip into the parent with a normal `--no-ff` merge, rerun affected gates
(fmt/clippy, workspace native tests post-touch, WASM page typecheck, mounted
layout browser suites — take the Chrome lease only after the worker released
it), and AMEND ticket 14's Outcome if the fixes changed its recorded picture
(the Outcome currently omits the linked-refinement creation control and the Copy
fix — both must appear). If the worker found a Core-side defect, route it to a
Core lane worker (13's lane) rather than editing `core/src/**` from the web
worktree; a Core behavior change to the copy path requires an intentional
transcript re-bless (see transcript discipline below).

### 2. Merge ticket 15 and re-run the full gate

- Merge `typed-core-edits/outline-verification` (`b8bcc687a`) into the parent.
- Re-run the FULL check at the final integrated tip with the split-worktree
  discipline (below). The transcript golden from `39c0a6c06` must reproduce
  unchanged at the merge; if any later commit changed wire behavior, that is an
  intentional re-bless, never accidental.

### 3. Independent Standards + Spec reviews of the outline phase

Two fresh-context reviewers over the phase range `71e159d91..<final tip>`
(71e159d91 = post-mechanical docs commit; the outline phase base), following the
same two-axis pattern the mechanical phase used (`standards-reviewer` +
`spec-reviewer`; use the user's preferred fast models — see models below —
different model per axis). Spec axis: tickets 11-15 contracts + the 03 Answer +
ADR-0001/0002 + map; Standards axis: AGENTS.md, CONTEXT.md, DESIGN.md outline
sections, the smell baseline (Mysterious Name, Duplicated Code, Feature Envy,
Data Clumps, Primitive Obsession, Repeated Switches, Shotgun Surgery, Divergent
Change, Speculative Generality, Message Chains, Middle Man, Refused Bequest —
judgement calls, repo standards override, skip what tooling enforces). Fix any
blockers with lane workers, rerun gates, then record outcomes.

### 4. Record final Outcomes and resolve the phase

Ticket 15 Outcome (transcript stat, full-check evidence, product gesture pass,
review dispositions), map update (phase complete; frontier moves to the matrix
effort), then the remaining planned work: **matrix tickets 04-11**
(Inspector/toolbar/offset/conversion/Relations/highlighting/integrated product
verification — see `docs/plans/matrix-layout-consistency/map.md`) and
**keyboard refresh 04** (this effort's ticket 04, blocked by matrix 11). Matrix
11 requires a real product gesture pass at `http://127.0.0.1:4173/` (root path —
`/boardstudio/` 404s in this build), save/reopen and screenshots.

## Environment constraints (verified, do not re-derive)

- **Full check split discipline**: the parent checkout runs
  `repo tooling lint build typecheck browser` (real `cad/.cache`); the non-nested
  `/tmp/boardstudio-tce-verify` runs `test` (native CAD needs non-nested cwd).
  The verify worktree CANNOT run `build`: its `cad/.cache` symlink dangles inside
  the podman container and cadrum falls through to a blocked OCCT download.
  Parent covers it. A permanent fix (real cache dir in the verify worktree) is
  optional and not yet user-approved.
- **Shared target**: `CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target`
  for ALL cargo/wasm commands; `touch core/src/lib.rs` before each run to defeat
  the sibling-artifact leak; `CARGO_BUILD_JOBS=2` for full-check runs; no
  `cargo clean` (shared cache). Parallel-workspace runs can flake
  `boundary_parity` — always re-run isolated before believing a failure.
- **Transcript discipline**: golden updates only via behavioral RED (golden
  mismatch with semantic assertions green first), then `BOUNDARY_BLESS=1`, then
  native + WASM host parity; prior entries must stay byte-identical (append-only).
  One transcript blessing owner at a time.
- **GitNexus**: each worktree needs its own index
  (`node /home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix/.gitnexus/run.cjs analyze --index-only`,
  run from that worktree; add `--force --no-parse-cache` when `outline_edit`
  symbols go unresolvable — known incremental-index quirk). Impact before
  symbol edits, detect-changes before commits, `--repo <that cwd>`. HIGH/CRITICAL
  must be reported; UNKNOWN requires targeted source confirmation, never treated
  as safe.
- **Stale web worker assets** (product gesture passes): `scripts/dev-web.py`
  rebuilds providers only when an asset is MISSING, never when stale — re-seed
  freshly built workers (parent checkout, same commit) before serving, or
  outline controls fail with `unknown variant 'apply-outline-edit'`.
- **Models**: the user wants fast/cheap subagents — `opencode-go/deepseek-v4.1-flash`
  and `opencode-go/glm-5.3-flash`. The current 14-lane worker was explicitly
  switched to glm-5.3-flash. Switch models only on explicit user request
  (stop the run at a clean commit boundary, relaunch fresh with the new model
  and a task that inherits the landed commits).
- **Git safety**: stage explicit paths only; never `git add -A`/`git add .`,
  `stash`, `reset`, force cleanup, push, dev merge or PR without authorization.
- **Chrome lease**: one owner at a time (currently the GLM worker); wasm-pack
  headless suites also count as browser use.

## Verification conventions

TDD at public seams (CoreEngine Open/Edit/Scene/evaluator, archive, browser
persistence, real Session/Core, mounted controls, native/WASM exports). One
behavioral red → minimal green; compilation errors are not reds. Independent
expectations for geometry (never values computed by the code under test).
Ticket status changes and Outcomes are recorded by the parent after merge +
gates, with honest accepted deviations. Lane workers never resolve their own
tickets.

## Last verified state — 2026-10-09

Resolved: matrix 01-03, mechanical 01-02 + children 05-10, outline decision 03,
outline 11-14. In flight: 14 fix round (GLM worker, `tce-outline14`).
Pending merge: 15 branch `b8bcc687a`. Full check last passed at `39c0a6c06`
(parent: repo/tooling/lint/build/typecheck/browser; verify: native test half).
Boundary transcript: 60 core entries (32 matrix/mechanical + 28 outline), native
+ WASM parity green. Recheck all of this rather than assuming it is unchanged.
