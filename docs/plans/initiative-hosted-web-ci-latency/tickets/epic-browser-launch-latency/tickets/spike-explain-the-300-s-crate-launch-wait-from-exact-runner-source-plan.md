---
title: 'Explain the ~300 s crate-launch wait from exact runner source'
type: feature
ticket: 1
created: '2026-11-17'
status: built
baseline_revision: 'e66cfcff21073dec7760c030bf246956865dcbbe'
route: oneshot
route_source: auto
risk: medium
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
followup_review_recommended: true
context: []
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** Nine of fourteen crate launches in each of two hosted attempts carried 285–322 s of overhead beyond their tests, the slow set changed between attempts, four crates were fast in both, and the research's only explanation is an unproven hypothesis tied to a 300 s navigation allowance named in a comment.

**Approach:** Read only the exact external and local source to name, with file/function/line anchors and quoted constants, which wait consumes the time, whether `pageLoadStrategy: eager` reaches it, whether the `tiny_http` patched pair is relevant, and what distinguishes the fast four; if source cannot settle it, specify one discriminating instrumented experiment instead. Write the result to `docs/plans/initiative-hosted-web-ci-latency/tickets/epic-browser-launch-latency/finding-launch-wait.md` with every claim labelled demonstrated-from-source, hypothesis, or unknown.

</intent-contract>

## Implementation Notes

Oneshot: no code change; one finding document produced directly by this delegated agent from source reading, so no coding subagent or multi-file coordination is needed.

Binding overrides recorded at planning time:

- Source-reading only. Where this workflow would otherwise run builds/tests/benchmarks, none are run: no builds, tests, CI reruns, browser runs, or experiments.
- External source fetching is approved: exact `wasm-bindgen-test-runner` 0.2.129 (crates.io / GitHub tag), ChromeDriver/WebDriver `pageLoadStrategy` semantics for Chrome for Testing 156.0.8078.73, `tiny_http` 0.12.0 source for the condition patched in `scripts/build-patched-wasm-bindgen.py` plus commit `9ba41e4a6` history, and the fast-four (`host`, `runtime`, `ui-model`, `catalogue`) vs slow crates' test-page setup at HEAD.
- `done_checkpoint = true`: stop after the spike; no fix, no adopt/delete of the patched pair, no source/config change outside this plan and the finding.
- Rendered step-01 forbids `tickets.py pull` ("never run pull"), so no leaf is pulled despite the dispatch parenthetical; this plan joins the ticket via `ticket: 1`.
- Worktree at activation was dirty only with the three protected untracked files the dispatch names; they are never opened or touched. Initiative/epic frontmatter left as found.
- Research claims re-checked before planning: `git diff --stat 46d758d..HEAD` over `scripts/run-wasm-tests.py`, `scripts/check.py`, `scripts/build-patched-wasm-bindgen.py`, `scripts/wasm_bindgen_tools.py`, `.github/workflows/check.yaml`, `.cargo/config.toml` is empty; the two intervening commits are docs only, so the research anchors (eager comment at `scripts/run-wasm-tests.py:512-514`, patch constants at `scripts/build-patched-wasm-bindgen.py:33-34`) still hold verbatim at HEAD.

Acceptance, in the ticket's terms: the finding names the wait with exact-source anchors, or holds a complete experiment spec (hypothesis, observations that settle it, stopping condition, approval needed); states whether the patched pair is relevant; states what distinguishes the fast four; does not promote the research hypothesis to a cause without source proof; and records what remains unknown and the user's next decision.

Verification is source verification only: re-read the finding against each cited external/local anchor (file, function, line, quoted constant) and confirm no other file changed (`git status`). No build or test command is run, per the binding constraint above.

Implementation outcomes (appended during implementation):

- Exact sources fetched to a temp dir and checksum-verified: `wasm-bindgen-cli` 0.2.129 (`5fd044ed…`), `tiny_http` 0.12.0 (`389915df…`), `rouille` 3.6.2 (`3716fbf5…`, per the cli crate's `Cargo.lock`); Chromium tag `156.0.8078.73` source files; Dioxus 0.7.10 registry sources.
- Verdict is **experiment specified**: source demonstrates the wait's location (Navigate, pre-`DOMContentLoaded`), eager's application and limits, the `tiny_http` lost-queue mechanism the patched pair addresses, and the fast-four dependency difference, but does not settle which request stalls or which event releases it — the timing arithmetic contradicts both clean 300 s expiries as the usual release. The finding therefore holds one fully specified instrumented experiment and does not promote the research hypothesis to a cause.
- Surprise: the runner's Chrome session is legacy-mode, and ChromeDriver legacy responses are HTTP 200 even for `kTimeout`, which the runner's empty-struct `goto` parsing silently accepts as success — a demonstrated masking hole, unknown whether it fired in A1/A2.
- Files touched: this plan and `finding-launch-wait.md` in the epic folder only.

## Plan Change Log

## Review Triage Log

### 2026-11-17 — Review pass
- verdicts: 7 findings — high 0, medium 5, low 2, false 0, maybe-false 0
- findings:
  - `[medium]` `[patch]` Dioxus provenance in §1 named the published registry source, but the launches build the vendored patch (`Cargo.toml:31-33`, `web/vendor/dioxus-interpreter-js-0.7.10/PROVENANCE.md:10-12`) — verified by reading both; patched §1 to name the vendored tree as the interpreter anchor (its `lib.rs` module externs are byte-identical to published; the deltas are `unified_bindings.rs`/`core.ts`/`core.js`). The plan's implementation-outcomes line saying "Dioxus 0.7.10 registry sources" is superseded by this row.
  - `[medium]` `[patch]` §2/Q1 labelled the Navigate location demonstrated-from-source, but phase bounds do not exclude stacked waits, the unbounded bindgen phase, or pre-runner time inside the measured launch wall — verified against the phase table itself; patched to label location an elimination-grade hypothesis (bounds remain demonstrated) in the verdict table, §2, and §5's closing paragraph.
  - `[low]` `[patch]` §3 cited `chrome_launcher.cc:459`, which is the remote-session path — verified (`LaunchRemoteChromeSession:417`, `LaunchDesktopChrome:464`); patched to the desktop path at `chrome_launcher.cc:775`.
  - `[medium]` `[patch]` §6 quoted `document`-gated `eval.js` and `hydrate`-gated `inline_js` externs without tracing resolved features, and underplayed test-floor feature unification — verified the feature gates (`dioxus-web/src/lib.rs:21-22,96`, facade feature tables, `test-floor` manifest); patched §6 to rest the snippet claim on the interpreter's `minimal_bindings`+`webonly` externs (enabled unconditionally by `dioxus-web/Cargo.toml:116-121`), to state `document`/`hydrate` are not enabled by any traced chain, and to leave reachability in compiled binaries explicitly unknown pending §7's net-log.
  - `[medium]` `[patch]` §7 H2 contradicted §§3/5 (under eager, `load`-only resources cannot yield the `kTimeout`, and the ChromeDriver body normally arrives after the runner's ureq deadline) — verified against §§3/5 as written; patched H2 to make the ChromeDriver-side log the settling record and interpret settlement as expiry of a `DOMContentLoaded`-blocking stall, not a strategy fix surface.
  - `[medium]` `[patch]` §7's net-log capture was not producible as specified: one shared config path across up to four simultaneous Chromes — verified against `check.py:1231-1237,279-284` and `run-wasm-tests.py:512-518`; patched the design so the instrumented runner appends a unique `--log-net-log` path per session where it already mutates Chrome args.
  - `[low]` `[patch]` §7/§8 asked approval for one batch while the stopping condition allowed three — verified as written; patched the approval wording to cover up to three batches, stopping at the first conclusive one.

## Auto Run Result

- Summary: spike 1.1 finding written and quick-reviewed; verdict is **experiment specified** — source demonstrates the phase bounds, eager's application and its limits against a module-graph stall, the `tiny_http` lost-queue mechanism the patched pair addresses, and the fast-four `dioxus-web` dependency difference, but does not settle which request stalls or which event releases it, so no cause is claimed and one instrumented experiment is specified for separate approval.
- Files changed:
  - `docs/plans/initiative-hosted-web-ci-latency/tickets/epic-browser-launch-latency/finding-launch-wait.md` — the spike finding (sources, phase anatomy, eager trace, patched-pair analysis, three 300 s constants, fast four, experiment, next decision).
  - `docs/plans/initiative-hosted-web-ci-latency/tickets/epic-browser-launch-latency/tickets/spike-explain-the-300-s-crate-launch-wait-from-exact-runner-source-plan.md` — this plan with triage and result.
- Review findings breakdown: 7 findings, all verified real and patched (0 high, 5 medium, 2 low — see triage log); 0 deferred; 0 rejected. Patched counts by verdict: high 0, medium 5, low 2. `followup_review_recommended: true` because five medium entries were patched on this first pass; the named unverified risk is that the patched passages themselves (§2 label downgrade consistency, §6 feature trace, §7 H2/design) have not had an independent re-review.
- Verification performed: source verification only, per the ticket's binding constraint — every cited anchor re-read against checksum-verified exact sources (`wasm-bindgen-cli`/`tiny_http`/`rouille` crates, Chromium tag 156.0.8078.73, vendored `dioxus-interpreter-js`, published `dioxus-web` 0.7.10) and local files at HEAD; anchor corrections from review re-verified the same way. **No builds, tests, browser runs, CI reruns, or experiments were run** — where the default workflow would run them, the ticket forbids it.
- Residual risks: the §4 stranding remains a hypothesis for the hosted trigger and the release event is unknown (by design, §7 settles both); snippet reachability in compiled binaries is unverified without a build; patched passages await the recommended follow-up review pass.
- Finalization deviation: after the scoped commit, the working tree still shows the three protected untracked files that predated this run (`necessist.db`, the two protected docs) — never opened or touched, per dispatch. All reviewed-diff files are committed; the tree is otherwise clean.
