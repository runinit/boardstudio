# BoardStudio frontend migration operating contract

Confirmed by the user on 2026-10-03. This is the single execution contract for this
project; installed skills retain their defaults in other projects. Prior rules and
rationale remain in [Git history](docs/migration/history/OPERATING-RULES-before-20261003.md).

## Scope and authority

Port the complete React/TSX capabilities and UI-owned behavior to Dioxus: theming,
menus, contextual Objects/Inspector panes and interactions. The user reprioritized
delivery on 2026-10-03: working application workflows first; exact visual layout
parity comes last, with UI improvements allowed along the way because the design
will change. Control placement, typography and cosmetic matching must not block
functional integration. Missing actions, incorrect edits and unusable controls
remain functional work. Retain earlier visual findings as deferred design inputs.
The TypeScript checkout and running app are read-only behavior references for this
migration; implement fixes in the Rust/Dioxus worktree.
On 2026-10-03 the user removed unfinished VIK parts from the first release and
explicitly removed their dedicated parent and criteria from the active roadmap.
Hide the VIK demo/catalogue without breaking existing saved projects; generic
mounted-module, part, Case and viewer behavior remains in scope. The previous
requirements and decisions remain recoverable in Git history.
Use functioning Rust engines and existing providers. Backend/kernel replacement
and the major structural refactor remain separate phases. Deliberate necessary UX
changes get a reason and comparison evidence; placeholders do not establish parity.

Necessary migration API, visibility, wire-contract and design changes are authorized.
Update callers, generated contracts and compatibility/migration handling coherently;
record architectural reasons in the RF ledger. Preserve unrelated changes, saved
projects, exports and history. Reset/discard/history rewrite, external publication,
production cutover and React removal retain their separate authorization/gates.

Mobile/compact validation is deferred by user instruction on 2026-10-04. Remove its
active automated checks and omit mobile browser qualification for now; retain desktop
checks and the responsive application behavior. Existing mobile evidence is historical,
not a current passing gate or acceptance requirement during this deferral.

## Delivery loop

The user authorized an end-to-end delivery restructure on 2026-10-05. Optimize for
completed user journeys on a published candidate. Source implementation, functional
verification and formal parent acceptance are separate outcomes.

1. **Choose one delivery batch.** Use the six workbench queues and `progress.py frontier`
   to group related runnable journeys on one saved fixture per workbench. Record the
   batch in the existing `qualification.pending_source_batch`: originating controls,
   accepted edit or output file, history/reopen where applicable, scope-change behavior,
   exact owned files and the checks that finish it. Several related criteria share one
   journey and receipt. Preserve original criteria and acceptance joins; unassessed
   F8/F9 work waits for its required prerequisites. Visual parity is deferred behind
   functional work, and mobile/compact validation remains deferred.
2. **Keep work bounded.** Coordinator owns integration, builds, browser qualification
   and canonical records. Use at most two implementation authors with disjoint file
   leases for the active batch; retain one independent reviewer. Existing ready work
   can join before source freeze. After freeze, add only fixes for a demonstrated
   failure in the batch's finish condition. Optional coverage, tooling improvements
   and unrelated investigation are stopped by user instruction. Preserve unfinished
   work without scheduling or extending it. A stalled optional fixture cannot hold
   an independent product repair.
3. **Prove the change at its owning layer.** Reproduce a product defect against the
   reference before editing; obtain meaningful RED/GREEN at the layer that owns it.
   Native tests prove state transitions; mounted WASM tests prove DOM wiring and
   presented states; public browser journeys prove the real application pipeline.
   Missing markup does not require an artificial native failure. Enumerate fixture
   contexts, assets and async completion barriers before compiling. Use `focused-test`
   for Rust tests, so zero executed tests cannot pass. A helper or intercepted event
   alone does not prove persistence, worker execution or a downloaded file.
4. **Integrate the settled batch once.** Obtain one independent diff review, repair its
   concrete findings and confirm the changed parts with the same reviewer. Run one
   combined required native/wasm-page/reachability/affected-headless gate. Reuse passing
   checks only when the recorded source, command, configuration and tool identities
   match. Commit explicit paths; preserve unrelated staged work. Tooling changes run
   their affected tooling suites. Broaden checks for a failure or identified risk,
   rather than repeating successful checks for each author or criterion.
5. **Package the committed identity.** Use the immutable committed snapshot build path
   once validated; author work may continue in the coordinator checkout during that
   build. A package is identified by its commit, maintained-input manifest, providers
   and served assets. Reuse unchanged providers only when provenance checks pass.
   Keep the previous published candidate available during new source work. At most
   two heavy jobs and one package run concurrently. Publish when a required changed
   journey is ready; reuse the candidate for unchanged qualification.
6. **Qualify whole desktop journeys.** On the published candidate, execute the batch's
   saved-fixture journeys against the pinned reference and record accepted edits,
   history/reopen and output bytes as required. Reuse unchanged successful branches;
   refresh changed actions and consumed interfaces. Pin fixture paths/hashes and a
   complete consumed-source footprint before automatic reuse. Historical results are
   not silently rebound to a different fixture or candidate. A public route that
   cannot trigger a fault stays unqualified; use deterministic owner/mounted evidence
   with its actual limits instead of repeating failed injection setup. Inspect saved
   values and exact option IDs before diagnosing data loss from a visible label.
7. **Finish and move on.** Record the result once in its authoritative record and
   commit records with the source milestone. Parent acceptance requires every actual
   criterion and final join plus the batch review; compiler/package success is not
   acceptance. Report delivered journeys, open functional failures, formal joins and
   candidate age separately. The next batch comes from a reproduced user-action gap
   or a grouped qualification-ready journey, not another broad source audit. If no
   gaps are known, probe one unverified action per workbench to choose that batch.

For the acceleration changes, completion additionally requires operational evidence:
repeat integration gates must reuse exact-input successes; relevant changes must
invalidate them; a snapshot package must remain attributable while the live checkout
changes; provider savings must be measured on an eligible real build. Label projected
savings and untested paths explicitly. Use the next necessary product build for these
measurements; do not create extra builds solely to benchmark tooling. Every safeguard
must have a named failure it prevents and a deterministic pass/fail result. Reuse
identical passing evidence; remove duplicate checks only when equivalent coverage is
identified. Optional work is stopped; prioritize the next required product journey.

## Ownership and enforcement

Register the canonical checkout/expected branch and role using
`scripts/migration-deliver.py`. The cheap local hook rejects mismatched registered
checkouts, direct coordinator commits and commits during that checkout's build freeze.
Coordinator commits use its delivery command. Registration/hook state is local to the
migration Git common directory. Unrelated repositories retain their own hooks/settings.
This guards accidental misuse by same-user agents; source hash checks still detect edits.

The coordinator owns shared integration, builds and canonical records. Disjoint authors
continue during its freeze. Retain existing custom hooks; do not activate the expensive
checked-in precommit pipeline as a side effect. Inspect command `--help` for exact usage.

## Records and skills

[Issue tracker](docs/agents/issue-tracker.md#frontend-delivery-records) maps each fact to
its one authoritative record. Update that record once; generated views and existing
specs link to it. Preserve blocker history and every RF observation. The original
62-parent portfolio remains in Git history; the active portfolio excludes F4.7.

Local overrides for to-spec, to-tickets and implement-spec: refine existing specs and
real gaps without another approval quiz; use assigned relevant bases; directly integrate
ready source; omit mandatory TDD for ordinary UI, separate merger/review chains, worktree
reset/cleanup and duplicate receipts. Explicitly invoked decision interviews still honor
the user's requested design checkpoint. These overrides apply only to this project.

When the same avoidable failure blocks delivery twice, correct its cause within the
active batch and record the decision in the existing run record. Optional retrospectives,
process reports and scheduled review cycles are stopped. This contract is the source of
truth; operational evidence belongs in the existing batch receipt.

Process rules (2026-10-04 retro; each is checkable):
- Status is generated, not hand-written: counts, served candidate and handoff come from
  `progress.py`; do not paste them into prose. Commit records with the source change that
  they describe, or at a milestone; no record-only commit per candidate.
- A state-changing control is `implemented` only with a native test of its edit's state
  transition (include the mirrored/linked case when it exists). A missing wasm-only DOM
  control uses a meaningful mounted WASM RED/GREEN with the existing native owner checks;
  it does not require an artificial native failure for absent markup. The browser proves
  mounting and the user journey, while native tests prove the state logic. This follows
  the user's 2026-10-05 instruction to implement all acceleration findings.
- Presentation code is wasm-only: run the wasm `page` check before committing it. Tests
  must execute (`check-wasm-tests.py`, `focused-test`); a plain `#[test]` in wasm-only code
  does not.
- Run one independent diff review per integrated batch before packaging; its findings become
  failing tests before the fixes. The consolidated review is for accepting a parent.
- A daily publication quota must not block a required repair. Reuse an unchanged
  candidate; `gc-builds.py` keeps the newest three plus served, baseline and accepted builds.

Token discipline (a long session re-reads its whole context on every call):
- Restart the session at about 150k context from a generated `handoff.md`
  (`progress.py handoff` plus a short hand-written tail); do not wait for auto-compact.
- Agent reports are 150 words or less: verdict, files changed, failing checks, and a path
  to detail in a file. Do not spawn an agent for an edit under about three files.
- No screenshots or `innerText` dumps unless a visual difference is under test; have scripts
  return small JSON. Pipe build and test output through `tail` or `grep`.
- Use agent completion notifications and one settled-source hash handshake. Before
  compiling a mounted fixture, enumerate its actual subtree's required contexts,
  assets and async completion barriers together; fixture setup errors are not product RED.
- Run Opus only for one diff review per batch and for bugs Sonnet could not solve.

Delegate by default: dispatch bounded, disjoint-file packets to author agents in parallel
and keep build, publish, ledger and commit ownership with the coordinator. Use the route
for the running harness (Codex: Luna low/medium/high authors, Sol 6.1 High review, Astra
high/xhigh for difficult evidenced bugs; Claude: Sonnet 5.5 authors, Opus 5.5 review and
coordinator, Opus 5.5 for difficult bugs). Give parallel authors disjoint files in the
canonical checkout and commit their exact files. Do not create authors from an inferred
remote default branch. The package helper's owned detached snapshot uses an explicit
committed revision and is independent of author checkout routing. Runtime slots bound the user ceiling
of 30. [Agent routing](.scratch/dioxus-frontend-v1/AGENT-ROUTING.md) holds launch details.
