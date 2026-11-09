# Full handoff: typed Core edits — finish the outline phase, then the remaining matrix work

> **Superseded 2026-10-09.** The outline phase is closed at `ecac36129`: both lane branches
> are merged, the full check passed at `77b9548b8`, the Standards and Spec reviews are
> dispositioned and ticket 15 is resolved (see its Outcome and `map.md`). Only §6 (environment
> constraints) and the follow-up efforts in §4 step 5 (matrix 04-11, keyboard 04) still apply.
> `CARGO_BUILD_JOBS=2` no longer applies; `.cargo/config.toml` sets 12 jobs.

Written 2026-10-09 for a fresh agent (Claude). Self-contained: everything below is
verifiable from the repository; do not rely on any prior conversation. Verify each
claimed state with the given commands before acting — states can drift.

Repository: `/home/chris/01_Projects/ts-boardstudio2`.

## 0. Where to start — checkouts, worktrees, branches

**Main checkout** (`/home/chris/01_Projects/ts-boardstudio2`, branch `dev`):
NOT used for this work. It holds unrelated user edits (`AGENTS.md`, `DESIGN.md`,
`PRODUCT.md`, `docs/README.md`, untracked `docs/design/` and `docs/research/`
files). Never stage, reset or clean them.

**Parent worktree** (the integration point): `.worktrees/tce-matrix`, branch
`typed-core-edits/mechanical`, tip `a8c77581c`. All merged work and docs live here.

**Lane worktrees** (each has its own branch; check each with
`git -C <path> status --short && git -C <path> log --oneline -3`):

| Worktree | Branch | State |
| --- | --- | --- |
| `.worktrees/tce-outline` | `typed-core-edits/outline-ownership` `562ede9db` | ticket 11 — MERGED into parent, idle |
| `.worktrees/tce-outline12` | `typed-core-edits/outline-resolution` `8bba73280` | ticket 12 — MERGED, idle |
| `.worktrees/tce-outline13` | `typed-core-edits/outline-edits` `069bc5f0c` | ticket 13 (+CreateGenerated) — MERGED, idle |
| `.worktrees/tce-outline14` | `typed-core-edits/outline-web-adapter` `6f9b3e2e7` | ticket 14 fix round — **NOT yet merged** (see §2) |
| `.worktrees/tce-outline15` | `typed-core-edits/outline-verification` `b8bcc687a` | ticket 15 verification — **NOT yet merged** (see §2) |

Old mechanical writer worktrees were removed; their branches
(`typed-core-edits/mechanical-{integration,actions,boundary}`) are retained with
all their commits (already merged into the parent).

**Non-nested validation worktree**: `/tmp/boardstudio-tce-verify`, detached at
`b8bcc687a`, untracked `cad/.cache` symlink (expected). Used ONLY for the native
`test` half of the full check (see §6). It exists because nested worktrees break
the native CAD test's cargo workspace discovery.

**No live subagents; Chrome is free.** The last worker (GLM 5.3 Flash, the 14
lane) completed and released the browser lease.

## 1. Effort state

Resolved, fully gated (behavior + full check + reviews, Outcomes recorded — see
`docs/plans/typed-core-edits/map.md` Progress):

- Matrix Core 01-03 (`../matrix-layout-consistency/…` — see that map).
- Mechanical: this effort's ticket 01, parent 02 and children 05-10.
- Outline: decision 03, tickets 11-14.

NOT yet done (this handoff's work):

1. The user-requested fresh-context parallel cleanup review (§4 step 0) — never run.
2. Merge the 14-lane fix commits + amend ticket 14's Outcome (§4 step 1).
3. Merge the 15 branch (§4 step 2).
4. Re-run the full check at the final integrated tip (§4 step 3).
5. Independent Standards + Spec reviews of the outline phase (§4 step 4).
6. Record ticket 15's Outcome and the final map update (§4 step 5).
7. Then the separate follow-ups: matrix tickets 04-11 (see
   `docs/plans/matrix-layout-consistency/map.md`) and this effort's
   keyboard ticket 04 (blocked by matrix 11 — NOT by mechanical 02, which is done).

## 2. Unmerged work — exact commits to merge

**14-lane fix round** (branch `typed-core-edits/outline-web-adapter`, tip
`6f9b3e2e7`, tree clean, web-only). Its base `31b9e2960` is already merged; the
two unmerged commits fix the two acceptance gaps that ticket 15's verification
found:

- `d3cfdd7cf` — adds the reachable **Refine outline** control (maps to the typed
  `OutlineChange::CreateLinkedRefinement`: created + selected, one Undo removes
  it) and Copy product-context mounted coverage. Before this, linked refinements
  were unreachable in the product (test-only) — the phase's headline ADR-0002
  behavior had no UI entry.
- `6f9b3e2e7` — the adapter never reissues a retired outline version identity.

The same round root-caused ticket 15's "Copy outline silently produced no
version": it was the **stale core-worker wasm provider asset** (see §6, the
dev-web staleness hazard), not a code defect; with providers rebuilt from source,
Copy lands live. Evidence: report
`/tmp/tce-outline-reports/14-outline-web-adapter.md` (second pass), 21
screenshots under `/tmp/tce-outline-reports/screenshots/` (20-31), the
save/reopen artifacts, contour capture files, and all-green checks (native
layout 65, mounted layout browser 145, workspace 1137 incl. boundary parity,
fmt/clippy/wasm-typecheck).

**15 branch** (`typed-core-edits/outline-verification`, tip `b8bcc687a`, clean):

- `39c0a6c06` — the intentional boundary-transcript golden update: 28
  `apply-outline-edit` entries appended (+20607/−0, prior matrix/mechanical
  entries byte-identical), red-before-bless with semantic assertions green
  first, native + WASM host parity green. Transcript now has 60 core entries.
- `80147bf69` — `application/tests/outline_queue_orders.rs`: matrix, mechanical
  and outline typed actions in BOTH queue orders through real Session/Core,
  three Undos walking back one action at a time.
- `b8bcc687a` — test-only `serde_json` dev-dependency for the application crate
  (+ the lock edge).

Ticket 15's full report: `/tmp/tce-outline-reports/15-integrated-verification.md`
(transcript evidence, full-check results, product gesture pass, per-case
evidence map). `/tmp` artifacts may not survive a reboot; the durable record is
the ticket Outcomes in the repo plus these files while they exist.

## 3. Issues and findings from the recent work

**Blocking the phase close (handled by §4):** the pending cleanup review; the
two unmerged branches; the full check at the final tip; the phase reviews; the
15 Outcome + map.

**Ticket 14's Outcome needs amending** after the merge: as recorded it omits the
Refine-outline creation control and the Copy root cause/fix — add them, and
note the mounted product-context coverage.

**Deferred/accepted items (already recorded in the ticket Outcomes — do not
"fix" without a product decision):**
- Outline 12: last-valid display after a failed resolution is the last
  persisted projection; inactive alternatives are recomputed at source level only
  (compose-level failures of unselected alternatives surface when selected);
  per-version recompute perf unmeasured; pre-existing duplicate-findings pattern.
- Outline 13: queued edits on a FIXED version retire on active-version/entity
  changes only (authored shape has no independent source); `RemoveFeature`
  refuses the base generated envelope (no product path submits it); `reviewed`
  carries full contours on the wire (a digest is an optional optimization);
  no matrix-transform fixture for version-target retirement (matrix moves settle
  as component moves — same rule); protected-gap equality is exact.
- Outline 14: the gap check/uncheck UI has no mounted test (no web fixture scene
  produces repair candidates — native adapter coverage only); a live
  Generated-active Copy click was not re-driven (coverage is mounted + one live
  linked-context click); the material-bridge-with-authored-connection gesture is
  not drivable in the bundled demos (no authored connection exists; `Connect
  points` disabled — Core-level bridge coverage stands).
- Mechanical 02 (recorded in its Outcome): paired-only physical removal;
  single-half legacy conversion; `UnlinkMirrorPair` Core-tested but unsurfaced;
  static outline classification; host-crate storage tests; Legacy ownership
  stamped only by migration.
- `SetProtectedGap` no longer clears stale alias ids on the check direction
  (removal clears all aliases); unobserved by web tests — confirm before any
  UI work on gaps.

**Environment/tooling issues (see §6 for the disciplines):** the
verify-worktree cadrum build failure (symlink-in-container, no outbound
network); the dev-web.py provider staleness hazard (manual rebuild discipline —
a hash/mtime staleness check in `dev-web.py` is a worthwhile product-tooling
follow-up, outside the outline lane's scope); the transcript golden is now
~1.0 MB (a future trim needs its own bless); GitNexus incremental-index quirk;
shared-target artifact leak; boundary_parity parallel-run flake; 7 pre-existing
wasm typecheck warnings in `web-catalogue`/`web-case` (unrelated to this work).

**Schema note:** `CURRENT_VERSION` is 3; the outline phase added only
serde-default additive fields (absence-equals-fixed ownership, retired ids) so
legacy documents stay byte-identical. Any future format change appends the next
ordered migration under ADR-0004 and needs an explicit decision.

## 4. Next steps, in order

### Step 0 — the user-requested cleanup review (run FIRST)

Launch TWO reviewer subagents in parallel with `context: "fresh"` (never fork).
Reviewers must inspect the repository, relevant instructions (AGENTS.md,
GLOSSARY.md, ticket contracts) and the current diff directly from files and
commands — never the main conversation history. Review-only: they must not edit
files. Prefer `output: false`; do not write reviewer output into the repo. They
return concise, evidence-backed findings with file/line references and the
smallest safe fix.

- **Reviewer 1 — deslop pass**: AI-slop patterns in the changed scope: comments
  restating code, placeholder text, stale rationale, debug leftovers; defensive
  checks hiding useful errors, vague defaults, validating trusted internal data
  after a real boundary; type escapes, broad casts, duplicated type definitions,
  object-bag typing where a local source-of-truth type exists; style drift from
  nearby non-slop code and project instructions; generated-sounding docs,
  changelog/UI copy/status text/test names; pass-through wrappers, dead helpers,
  duplicate helper signatures, duplicated test harness setup, abstractions
  enforcing no invariant; noisy/vague/brittle UI copy. Tool output and
  slop-scan-style findings are leads, not verdicts — flag only concrete issues
  with evidence, severity, file/line and the smallest safe fix.
- **Reviewer 2 — verbosity pass**: needless verbosity in code, tests, docs,
  status text, receipts, changelog wording: single-use helpers paraphrasing an
  expression; temporary variables naming obvious expressions; nested
  returns/branches replaceable by direct returns without hiding intent;
  multi-line cleanup scaffolding replaceable by a local direct pattern; repeated
  boilerplate that could use an existing fixture or small local helper; tests
  restating formatter details covered at a cheaper layer; regression tests
  where one focused assertion would cover the bug but wrapper tests repeat the
  claim; prose saying the same thing twice or burying the important rule.
  Shorter is only better when clearer AND it preserves behavior, error signals,
  cleanup semantics, useful invariants and local style.

Suggested review range: the outline phase `71e159d91..` the then-current tip
(include the unmerged §2 branches by reviewing each branch separately or after
merging). After both return, synthesize into: fixes worth doing now / optional
improvements / ignore-with-reason. Do not blindly apply. **Without an explicit
`autofix` word from the user, ASK before applying any fixes**, ending with a
compact numbered menu (`[1] Apply only the fixes worth doing now. [2] Apply
fixes plus optional improvements.`). Treat the literal word `autofix` in an
invocation as workflow control, not scope — strip it first.

### Step 1 — merge the 14-lane fixes and amend ticket 14

1. Verify `.worktrees/tce-outline14` is clean at `6f9b3e2e7`.
2. From `.worktrees/tce-matrix`: `git merge --no-ff typed-core-edits/outline-web-adapter`
   (inspect any conflict; stop for scoped resolution if one appears).
3. Re-run affected gates (post-`touch core/src/lib.rs`): fmt/clippy workspace,
   `cargo test --workspace --locked --exclude boardstudio-footprints`, wasm page
   typecheck, mounted layout browser suite
   (`wasm-pack test --headless --chrome web/crates/layout --locked --lib`).
4. Amend `docs/plans/typed-core-edits/issues/14-outline-web-adapter.md`'s
   Outcome with the refine control, the Copy root cause/fix and the gesture
   evidence; run `python3 scripts/check.py repo`; commit.

### Step 2 — merge the 15 branch

Same discipline: merge `typed-core-edits/outline-verification` (`b8bcc687a`),
then verify the boundary transcript reproduces unchanged at the merge
(`cargo test -p boardstudio_core --test integration boundary_parity` — 2 tests;
WASM host parity `boundary_parity::` — 1 test). If any later commit changed wire
behavior, that is an intentional re-bless (never accidental) — investigate
before blessing.

### Step 3 — full check at the final integrated tip

Split-worktree discipline (§6): parent checkout runs
`CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target python3 scripts/check.py repo tooling lint build typecheck browser`;
`/tmp/boardstudio-tce-verify` (checkout the exact final tip there first) runs
the same with `test`. All steps must pass. The `test` step includes native CAD
(51 passed/4 ignored at baseline).

### Step 4 — independent Standards + Spec reviews of the outline phase

Two fresh-context read-only reviewers over `71e159d91..<final tip>`, following
the mechanical phase's two-axis pattern (`standards-reviewer` + `spec-reviewer`
or equivalents). Spec axis: tickets 11-15 contracts, the 03 Answer,
ADR-0001/0002 (+0004/0005), the map. Standards axis: AGENTS.md, GLOSSARY.md,
DESIGN.md's outline sections, plus the smell baseline (Mysterious Name,
Duplicated Code, Feature Envy, Data Clumps, Primitive Obsession, Repeated
Switches, Shotgun Surgery, Divergent Change, Speculative Generality, Message
Chains, Middle Man, Refused Bequest — judgement calls; repo standards override;
skip what tooling enforces). Fix blockers with lane workers (never
cross-worktree edits), rerun gates.

### Step 5 — record outcomes, then the follow-up efforts

Record ticket 15's Outcome (transcript stat, full-check evidence, product
gesture pass, review dispositions), update the map (phase complete), then the
remaining work is the separate efforts: **matrix tickets 04-11** (their own map
and owners; matrix 11's integrated verification needs a real product gesture
pass at `http://127.0.0.1:4173/` — the root path; `/boardstudio/` 404s in this
build — with save/reopen and screenshots) and **keyboard ticket 04** (this
effort, blocked only by matrix 11 now).

## 5. Verification conventions

- TDD at public seams only (CoreEngine Open/Edit/Scene/evaluator, archive,
  browser persistence, real Session/Core, mounted controls, native/WASM
  exports). One behavioral red → minimal green; compilation errors are not
  reds. Geometry expectations are hand-worked, never computed by the code under
  test.
- Lane workers never resolve their own tickets: the parent (integrator) merges,
  re-runs gates, then records Outcomes with honest accepted deviations.
- The boundary transcript: golden updates only via behavioral RED (golden
  mismatch with semantic assertions already green), then `BOUNDARY_BLESS=1`,
  then native + WASM host parity; prior entries byte-identical (append-only);
  one transcript blessing owner at a time.
- Browser/WASM presentation is only proven by the actual browser suites
  (wasm-pack headless chrome) — native cargo tests never prove WASM UI.

## 6. Environment constraints (verified — do not re-derive)

- **Full-check split**: the parent checkout has a real `cad/.cache` and runs
  `build`/`browser`; `/tmp/boardstudio-tce-verify` runs the native `test` half.
  The verify worktree CANNOT run `build`: its `cad/.cache` symlink dangles inside
  the podman container (only its `cad/` dir is mounted) and cadrum falls through
  to a GitHub OCCT download that this machine's network blocks. Parent covers
  it. Materializing a real cache there is optional and needs user approval.
- **Shared target leak**: `CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target`
  for ALL cargo/wasm commands; `touch core/src/lib.rs` before each run; no
  `cargo clean`. Parallel-workspace runs can flake `boundary_parity` — rerun
  isolated before believing a failure.
- **dev-web staleness hazard**: `scripts/dev-web.py` rebuilds provider assets
  only when a REQUIRED file is MISSING, never when stale. Before any product
  gesture pass, rebuild providers from the exact source
  (`python3 scripts/build-web.py --providers-only` from the checkout being
  served), or outline controls fail with `unknown variant 'apply-outline-edit'`.
- **Chrome lease**: one browser owner at a time; currently free.
- **No outbound network** (GitHub downloads fail); podman is available; the CAD
  wasm build runs through podman.

## 7. Operational rules

- Follow AGENTS.md and the tracker conventions
  (`docs/agents/issue-tracker.md`). Stage explicit paths only; never
  `git add -A`/`git add .`, `stash`, `reset`, force cleanup, `push`, dev merge
  or PRs without authorization. Never touch the main checkout's unrelated edits.
- Subagent models: the user wants fast/cheap workers — `opencode-go/deepseek-v4.1-flash`
  and `opencode-go/glm-5.3-flash`. Switch a live worker's model only on an
  explicit user request (stop at a clean commit boundary, relaunch fresh with
  the new model and a task that inherits the landed commits).
- Send concise progress updates: active ticket, evidence from the current TDD
  cycle, checks completed, any real blocker. Do not report worker "done"
  claims as acceptance — verify at the seams.

## 8. Commit inventory (trace)

- Mechanical phase: parent branch history `8054e4a82` → … → `71e159d91`
  (02 + 05-10 resolved there; the map lists the key lane commits per ticket).
- Outline phase on the parent: `5d29d63dd` (03 approval + tickets 11-15
  charted) → `99b03179` (Q14) → `4ab266e64` (11 merge) → `04e794bcf` (11
  outcome) → `08bbc3975` (12 merge) → `ab60df7a0` (12 outcome) → `dede916a0`
  (13 merge) → `e26bdbc32` (13 outcome) → `9fac9a14c` (14 merge) →
  `4fc15897f` (14 outcome) → `a8c77581c` (this handoff's predecessor).
- Unmerged: `typed-core-edits/outline-web-ownership`-family branches listed in
  §0/§2 (`d3cfdd7cf`, `6f9b3e2e7`; `39c0a6c06`, `80147bf69`, `b8bcc687a`).

## Last verified state — 2026-10-09

All worktrees clean; 14 lane at `6f9b3e2e7` and 15 branch at `b8bcc687a` both
verified complete by their reports; no live subagents; Chrome free; parent at
`a8c77581c`. The full check last passed at `39c0a6c06` (parent steps) and
`b8bcc687a` (verify native test); the unmerged 14 fixes have per-suite green
(native 65 / mounted 145 / workspace 1137) but no full-check run yet — that is
§4 step 3. Recheck everything before acting.
