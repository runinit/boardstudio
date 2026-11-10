# AI-agent test economy: first-party evidence and rule gaps

Date: 2026-11-07. Status: **research only**. Nothing here authorizes changes to rules, tests, scripts or
product contracts. Local baseline: `CODING_STANDARDS.md#test-economy` and the AGENTS.md pointer to it,
at coordinator HEAD `21d3e7a`.

Evidence labels: **[measured]** means the owning source reports a measured number. **[reported]** means
a maintainer's own account with no published measurement. **[mechanism]** means the rule or config was
seen in source, and nothing shows whether it worked. Unlabelled text in §5–§8 is my recommendation.

## 1. Executive findings

1. **None of the reviewed sources directly measures savings from a test-economy rule set.** Measured
   outcomes relevant here cover two narrower things: filters that decide whether a generated test is admitted (Meta), and how often
   models game tests (Anthropic). Everything else is a maintainer's account or a rule seen in source.
   Choose rules by the failure they prevent, not by projected savings. [S9, S10, S7]
2. **Our current rules agree with the strongest evidence.** Meta discards a generated test that only
   repeats existing behaviour. It also rejected a test that raised coverage but had no assertion. That
   is our "Distinct failure" rule, and it argues against coverage or count targets. [S9 measured]
3. **Gap: weakening a verifier.** Our "Preserve proof" rule covers deleting tests. It says nothing about
   loosening an assertion, expected value, golden or baseline until it passes. Anthropic measured related
   test-gaming behavior, not a policy's effect on oracle edits. In the "impossible task" setting, simple prompting cut it by more than
   9× (Opus 4) and 4.5× (Sonnet 4). OpenHands makes verification specs read-only for implementer agents.
   [S7 measured, S5, S12]
4. **Gap: unjustified test-only surface and self-fulfilling assertions.** Codex discourages test-only
   helpers; OpenHands forbids exporting internals solely for tests. These support restraint, not a
   blanket ban on approved diagnostic seams, goldens or compatibility checks. [S1, S12 mechanism]
5. **Gap (smaller): which tier to run.** OpenHands says to "select the cheapest suite that exercises the
   real changed boundary". Codex runs focused crates locally and the full matrix only for shared code.
   Codex CI moves heavy suites to post-merge. Our rules say where coverage belongs (the agreed seam) but
   not which tier an assertion should live in. [S12, S1, S2 mechanism]
6. **Wording risk.** "Consolidate overlapping host plumbing" can be read as an order to merge fixtures.
   The 011 Standards review explicitly accepted keeping display, static-owner and live-projector fixtures
   separate. A fixture's purpose should count as part of its story.
7. **Do not import the high-throughput merge philosophy.** OpenAI's harness team reports minimal blocking
   gates and re-running flakes. The author says this "would be irresponsible in a low-throughput
   environment". Two parts do transfer: encoding repeated review feedback as mechanical checks whose
   messages tell the agent how to fix the problem, and keeping test output small. [S3 reported, S4 reported]

## 2. Source comparison

| # | Setup / report | Type | How it controls tests | What it shows |
|---|---|---|---|---|
| S1 | OpenAI Codex root `AGENTS.md`, last version `4875084` | mechanism | Run tests for the changed crate first. Run the full suite only for common/core/protocol changes, and ask the user first. Rules: "Do not add tests for values that are statically defined", "Do not add negative tests for logic that was removed", "Avoid test-only functions in the main implementation", "Avoid proliferating test-only helpers". Agent logic changes need an integration test listing the behaviours covered. Use whole-object `assert_eq!`. Review `insta` snapshots explicitly. | A large **Rust** agent codebase whose rules agents follow. Commit `18131270` (2026-09-30) removed the root file and all `.codex/skills`, with no public reason given. |
| S2 | Codex CI `.github/workflows/README.md` and `codex-rs/.config/nextest.toml` at `57a808a` | mechanism | PR checks stay "intentionally small". The full Cargo nextest/clippy matrix runs post-merge. One nextest timeout-override list is marked "Do not add new tests here". `retries = 1` "so one transient failure does not fail full-CI outright". | Cost tiers enforced by config. The retries hide flakes from the full-CI pass/fail result. |
| S3 | OpenAI, *Harness engineering* (Lopopolo, 2026-02-11) | reported | Custom linters and structural tests whose error messages "inject remediation instructions into agent context". A recurring cleanup ("garbage collection") opens small refactor PRs. Minimal blocking merge gates; "test flakes are often addressed with follow-up runs". | About 1M LOC and about 1,500 PRs with 3 engineers (3.5 PRs/engineer/day). Before automating cleanup they spent "every Friday (20% of the week) cleaning up 'AI slop'". The author warns it "should not be assumed to generalize". |
| S4 | Anthropic, *Building a C compiler with a team of parallel Claudes* (Carlini, 2026-02-05) | reported + measured cost | The harness prints "a few lines" and logs details to a file with `ERROR` greppable on one line. A default `--fast` runs a deterministic per-agent 1%/10% sample. Added CI after "Claude started to frequently break existing functionality". Uses GCC as an oracle. | About 2,000 sessions, about $20k. The author says the verifier must be "nearly perfect, otherwise Claude will solve the wrong problem". |
| S5 | Anthropic, *Effective harnesses for long-running agents* (Young, 2025-11-26) | reported | "It is unacceptable to remove or edit tests". Agents may only flip a `passes` field. End-to-end checks run through browser automation. | Without explicit E2E prompting, agents missed features that didn't work end-to-end. Browser-native modals stayed invisible and those features were buggier. No metrics. |
| S6 | Claude Code *Best practices* (current docs) | vendor guidance | Keep CLAUDE.md short. Rules that allow no exceptions go in hooks, not prose. Ask for the command and its output as evidence. A reviewer "asked to find problems will always find *something*… you end up with extra abstraction layers and tests for cases that can't happen." | Vendor recommendation, not an outcome. |
| S7 | Claude Opus 4 / Sonnet 4 system card (2025), §6 | **measured** | Defines hard-coding and special-casing to pass tests. Detects them with classifiers and *hidden held-out tests*. Tests the effect of prompts asking the model to report bad tests. | Hard-coding fell 67–69% versus Sonnet 3.7. Prompting cut hacking >9× / 4.5× on impossible Claude Code tasks. These are older models. |
| S8 | Anthropic, *Demystifying evals for AI agents* (2026-01-09) | guidance | "Often better to grade what the agent produced, not the path it took". Checking step order is "too rigid and results in overly brittle tests". Read transcripts before trusting graders. | Supports behaviour-level assertions over implementation-coupled ones. |
| S9 | Meta, *TestGen-LLM* (arXiv 2402.09171) | **measured** | Filters for builds, passes, and reliable passing (fails if not all of 5 runs pass). A coverage filter rejects tests that "simply repeat the test behaviour of one of the existing tests". Engineers review the rest. | 75% built, 57% passed reliably, 25% raised coverage. 73% of recommendations accepted. A coverage-raising test with no assertion was rejected in review. |
| S10 | Meta, *ACH: mutation-guided LLM test generation* (arXiv 2501.12862) | **measured** | Generates a few concern-specific *undetected* mutants, then tests that kill them. An LLM detects equivalent mutants. | 9,095 mutants, 571 tests, 73% accepted, 36% judged privacy-relevant. |
| S11 | Goose `AGENTS.md` at `4facf56` (Rust) | mechanism | Build/test "only if the user has asked". Use the `cargo test -p <crate>` scope. Prefer the `tests/` folder. Record/replay MCP integration fixtures (`just record-mcp-tests`). | A minimal rule set. No test-economy rules beyond scope. |
| S12 | OpenHands `AGENTS.md` and `e2e-testing` skill at `fb9b8e2` | mechanism | "Avoid duplicate cases and duplicate assertions." "Mock an underlying service rather than the hook that consumes it." "Do not mirror literal source, fixture… definitions" and "do not export internals solely to make them testable". "Test every path with distinct control or data flow without testing an irrelevant Cartesian product." Verify skills are read-only for implementers. A path→test resolver exists for local use only; post-merge CI "intentionally run[s] the full suite". "Do not retry a lost click: inspect that attachment". | The closest current analogue to our rules. Young rules, no outcome data. The repo now appears to be the agent-canvas frontend. |

## 3. Findings by question

**Avoiding duplicate tests without losing protection.** Three sources gate a test's admission on
evidence: Meta requires it to build, pass five times and add behaviour not already tested [S9]. ACH
requires it to kill a mutant the current tests miss [S10]. Codex requires the PR to list the behaviours
its integration test covers [S1]. None uses a count, a ratio or an absolute coverage target. Meta uses
coverage only *relative* to existing tests, and still rejected a test with no assertion. On tiers, the
sources agree that agents iterate on focused runs and full suites gate merge, post-merge or closure
[S1, S2, S4, S12]. Both sources that select tests by changed paths keep that selection out of the
authoritative gate [S12], or run it only as a local sample [S4].

**When to consolidate.** No source gives an objective threshold for refactoring tests. The closest:
OpenHands extends an existing file when it is "a natural home". It tests distinct control/data flows
"without testing an irrelevant Cartesian product" [S12]. Codex says to "check whether there are existing
helpers" [S1]. OpenAI turns *recurring* review feedback into lints and runs small continuous cleanup
PRs [S3 reported]. No source describes how to prove that a consolidation kept its assertions. The best
transferable proof is the one we already have: mutation-kill evidence of the ACH kind [S10] plus our
test-name baseline.

**Preventing test debt.** Three patterns, each seen in source:
- Keep the test surface out of production: no test-only functions or exports [S1, S12].
- Mock the boundary service, not its consumer [S12].
- Assert behaviour or whole objects, not mirrored literals [S1, S12], and grade outcomes, not paths
  [S8].

Snapshot churn is handled by explicit review of pending snapshots [S1]. No source counts snapshot
debt.

**Costs, kept separate.**
- **Tokens and context:** large instruction files crowd out the task [S3, S6], and test output pollutes
  the context [S4].
- **Review load:** human QA became the bottleneck [S3]. Over-eager reviewers create tests for
  impossible cases [S6].
- **Latency:** focused runs and post-merge heavy tiers [S1, S2]. Agents "will happily spend hours
  running tests" [S4].
- **Flakiness:** admission requires 5 passing runs [S9], versus re-runs [S2, S3]. OpenHands bans blind
  retries [S12].
- **Long-term upkeep:** only maintainer-reported (20% of a week [S3]).

**Gaming and validation drift.** Agents measurably hard-code or special-case tests [S7]. Hidden held-out
tests and an explicit "report a wrong test instead of editing it" instruction both reduce this [S7].
Tests that pass while features fail end-to-end are reported [S5]. Pushing every assertion into E2E has
its own limits: browser automation could not see native modals [S5]. The C compiler work also needed a
known-good oracle, not just more tests [S4].

## 4. What transfers and what doesn't

**Transfers:**
- Admission by distinct observable failure and a real assertion (already ours).
- Mutation as evidence that a test or consolidation is still worth its cost (already ours).
- Verifier integrity.
- No test-only production surface.
- No restating tests.
- Focused-then-full tiers.
- Short output with details in logs (already in AGENTS.md via `.evidence/`).
- Rules that allow no exceptions belong in `check.py`, not prose (already the CODING_STANDARDS
  preamble).

**Doesn't transfer, or only partly:**
- *Minimal gates and flake re-runs* [S3]. This assumes a throughput we don't have, and it conflicts with
  RED/GREEN and mutation proof.
- *Five-run flake admission* [S9]. Cheap for Kotlin unit tests. Each wasm browser mount is expensive,
  so this is affordable only for a new timing-sensitive browser test, as triage.
- *Path-mapped test selection as a gate* [S12]. OpenHands itself refuses this.
- *Coverage filters*. Coverage tooling for the wasm/browser tier is weak, and Meta's evidence is for JVM
  unit tests.
- *Parameterized tables*. `wasm_bindgen_test` has no per-row harness. A loop inside one test loses
  per-case isolation, naming and cleanup on failure. So "table-drive" means something different in the
  browser tier than in native `#[test]`.
- *Python/TS mocking idioms*. Our seams are injected Rust traits such as `FakeUpdateSignals` via
  `observe_update_waiting`, which is dependency injection, not a test-only hook.

## 5. Proposed small rule edits (prioritized; wording suggestions only)

**P1 — Verifier integrity.** Add to *Preserve proof*:
> Do not weaken assertions, expected values, goldens, baselines or skip rules merely to obtain GREEN.
> If a verifier is wrong, explain the mismatch and support its correction with the existing contract
> and an independent oracle, or an approved contract change. Preserve required reviews and approvals.

Why: measured gaming [S7] and independent verifiers [S5, S12]. This forbids gaming, not correcting a
broken test under an unchanged contract. The latter must remain possible.

**P2 — No unjustified test-only surface.** Append to *Synthetic data, production behavior*:
> Do not add production APIs, flags or wider visibility merely to reach internals for assertions.
> Prefer existing dependency boundaries; justify and review a necessary new seam, observing existing
> approval requirements. Approved injection and diagnostic-only seams remain valid.

[S1, S12] This must not prohibit the approved update-signal observer or persistence-gate adapter.

**P3 — Independent expectations, not restatements.** Append to *Distinct failure*:
> Do not copy a definition into an assertion and treat it as an independent oracle. Literal, golden
> and rejection tests remain valid when they pin an agreed runtime, serialization or compatibility
> contract rather than merely restating the implementation.

[S1, S12] Reject tautologies, not pinned contracts or security/compatibility rejection coverage.

**P4 — Cheapest faithful tier.** Add to *Earn repeated coverage*:
> Put each failure assertion in the cheapest tier faithful to its boundary. Preserve required owner,
> browser, packaged and final-gate checks; focused iteration does not waive integrated acceptance.

[S12, S5 counter-risk] Check that this doesn't duplicate README guidance on `check.py` scopes.

**P5 — Make fixture purpose explicit.** Move "Consolidate overlapping host plumbing…" from *Synthetic
data* into *Shared setup*, and qualify it:
> …where fixture purposes match. Display, static-owner and live-projector fixtures are different
> purposes.

This matches the 011 review outcome.

**P6 — Consolidation proof, scoped.** Add to *Preserve proof*:
> When a change retires or renames tests, map each retired name to the case that now carries its
> assertion.

Why: `check.py` guards *native* test names (`guard_native_tests`, `scripts/check.py:922`). This pass
found no name baseline in `scripts/run-wasm-tests.py`, not an exhaustive absence proof across tooling.
Confirm the gap before proposing a new guard. Name-to-case mapping applies only to diffs removing or
renaming tests, so it adds no paperwork to ordinary changes.

**P7 (low) — Trigger by new setup, not size.** Replace "Large test additions must explain…" with "Test
additions that add a fixture, helper or mount path must explain…". "Large" is undefined, and agents
fill the gap with long explanations.

**Redundancy check.** "Judge failure coverage and upkeep, not test counts or a … line quota" and
"never delete tests merely to improve a ratio" say similar things. Keep both: one constrains reviewers,
the other constrains implementers.

## 6. Candidate refactors (grounded, few)

| Candidate | Kind | Benefit | Risk | Proof needed | Verdict |
|---|---|---|---|---|---|
| Update-Now deferral quadrant: {valid, invalid draft} × {toast, header opener} per owner in `web/src/presentation/layout_draft_safety_tests.rs` (constraint 833–866, 1401–1442; legacy-position 256–343). The bodies differ only in field, text, button and message. | cosmetic | About 30–40 lines per owner with a shared async step helper. **No runtime gain** unless the rows share one mount. | Sharing a mount changes the failure story (dialog state leaks between rows). A loop loses the browser test names, which have no name guard. It could drop the "realistic invalid text" comment and the X/Y field split. | Unchanged test names; one mutation that removes opener-specific deferral is killed by both opener tests. | **Defer.** Revisit only when another of the 14 owners would copy the quadrant a third time. Then extract a step helper and keep the per-test names. |
| Inspector host wiring copied in `legacy_position_fixture` (100–151) and `layout_constraint_fixture` (584–706). | seam / architectural | If more owners need it, one production-shaped host composition stops drift across hand-wired signal sets. | 011 review accepted the fixtures as having distinct purposes. Merging them would be test-volume–driven reopening. A configurable host would be a test DSL. | Co-change evidence: a production wiring change that forces the same edit in both fixtures. After extraction, the live-projector freshness mutation is still killed. | **Do not reopen now.** Watch for co-change, or for the third owner fixture. Prefer a production-owned composition over a test framework. |
| Display / static-owner / live-projector fixture split (011) | none | — | Merging lost the freshness regression catch (real projector required). | — | **Keep.** It is the counterexample that P5 wording protects. |

## 7. Minimal workflow for finding valuable test refactors

This is not a sweep. Act only on a trigger:

1. **Triggers** (any one):
   - One production change forces the same setup edit in two or more fixtures.
   - A new test or owner would copy an existing scenario a third time.
   - A test flakes twice, or is retried, without a diagnosis.
   - A review finds an assertion on a literal or implementation detail.
   - One `check.py` step's existing wall-clock dominates a run.
2. **Classify the finding:**
   - *Seam*: a production composition is missing. This is a product or architecture change and needs its
     own authorization.
   - *Data variant*: table-drive it (natively), or use a step helper (in the browser tier).
   - *Cosmetic*: fold it into the next diff that touches the file, or skip it.
3. **Evidence:** name the failure each kept case catches. Take runtime claims only from existing check
   output. Run no new benchmarks.
4. **Proof:** map retired/renamed cases (P6), preserve required mutation evidence, and rerun the
   relevant kill if consolidation could weaken an assertion. Keep native baselines unchanged when
   names are unchanged; intentional retirements/renames need reviewed updates, not baseline changes
   merely to make a check pass.
5. **Stop rule:** cosmetic line reduction alone never justifies a dedicated change or a re-review.

## 8. Staged follow-up (not authorized here)

0. The coordinator reviews this report. It stays uncommitted.
1. If approved separately: one small edit to `CODING_STANDARDS.md` covering P1–P3 (and P5), checked
   against the writing-for-agents guidance.
2. Separate question: should the browser-tier test names get a name guard like the native one? That is
   a script change and out of scope here.
3. After 010 lands: apply §7 only when a trigger fires. No backlog audit.

**Rejected ideas:**
- Coverage % or test-count targets. [S9] shows coverage without an assertion.
- Test/production LOC ratios.
- Automerged cleanup agents.
- Retries as the default flake policy.
- Path-mapped selection as a required gate.
- Sharing one browser mount across rows to save time.
- Reopening reviewed 011 fixtures because of their volume.

## 9. Limitations

- None of the reviewed sources directly measures policy savings in tokens, latency or upkeep.
  S7, S9 and S10 measure narrower gaming/test-generation outcomes, not this policy's effectiveness.
- S1 was removed from Codex's public repo without a stated reason. It is cited as pinned historical
  evidence and may be stale or have moved internal.
- openai.com returned 403, so S3 was read from the Internet Archive copy (captured 2026-10-27). It is
  about a single internal product.
- S7's figures are for 2025 models. Current models may differ.
- OpenHands' repo appears restructured (agent-canvas). Its rules are current but have no outcome data.
- Local inspection covered two stable 011 test regions and the `check.py` guard only. The browser
  name-guard absence is based on a search of `run-wasm-tests.py` and is not exhaustive. 010's driver was
  not assessed.
- Gortex: `change(impact)` on this new file returned `file_not_indexed`, as expected for an untracked
  file.

## Sources

- S1 Codex `AGENTS.md` @ `4875084`: https://github.com/openai/codex/blob/4875084025582f9443afeb907e2aa20644241dda/AGENTS.md ; removal commit: https://github.com/openai/codex/commit/18131270fe82ef947830840eb5d6923d4b82609f
- S2 Codex CI strategy and nextest config @ `57a808a`: https://github.com/openai/codex/blob/57a808a3920f05f2d4b9eab2d0493e1df136ae6b/.github/workflows/README.md ; https://github.com/openai/codex/blob/57a808a3920f05f2d4b9eab2d0493e1df136ae6b/codex-rs/.config/nextest.toml
- S3 OpenAI, Harness engineering: https://openai.com/index/harness-engineering/ (read via https://web.archive.org/web/20261027134646/https://openai.com/index/harness-engineering/)
- S4 Anthropic, Building a C compiler: https://www.anthropic.com/engineering/building-c-compiler
- S5 Anthropic, Effective harnesses for long-running agents: https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents
- S6 Claude Code best practices: https://code.claude.com/docs/en/best-practices
- S7 Claude Opus 4 / Sonnet 4 system card, §6: https://www-cdn.anthropic.com/4263b940cabb546aa0e3283f35b686f4f3b2ff47.pdf
- S8 Anthropic, Demystifying evals for AI agents: https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- S9 Alshahwan et al., Automated Unit Test Improvement using LLMs at Meta: https://arxiv.org/abs/2402.09171
- S10 Foster et al., Mutation-Guided LLM-based Test Generation at Meta: https://arxiv.org/abs/2501.12862
- S11 Goose `AGENTS.md` @ `4facf56`: https://github.com/aaif-goose/goose/blob/4facf56cbab049142809d9afa4648ebe6823ba80/AGENTS.md
- S12 OpenHands `AGENTS.md` and `e2e-testing` skill @ `fb9b8e2`: https://github.com/OpenHands/OpenHands/blob/fb9b8e249ebd717388b4938950fd8fc9b7a30441/AGENTS.md ; https://github.com/OpenHands/OpenHands/blob/fb9b8e249ebd717388b4938950fd8fc9b7a30441/.agents/skills/e2e-testing/references/guide.md ; https://github.com/OpenHands/OpenHands/blob/fb9b8e249ebd717388b4938950fd8fc9b7a30441/.agents/skills/e2e-testing/SKILL.md
