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

## Delivery loop

1. Keep six workbench queues: Layout, PCB, Keymap, Keycaps, Case/shared 3D, and
   Parts/Project. Prioritize missing or broken end-to-end functionality across all
   six queues, rather than Layout visual closure. Each stream has at most one
   unresolved author packet at a time. Once its owned source has landed and its
   consumed interface is stable, the stream may start its next functional packet;
   it need not wait for another stream, a combined compile, package, or review.
2. Dispatch from the criterion queue: name a missing criterion, current source,
   owned files, consumed capabilities and finish condition. Use bounded investigation
   for unassessed criteria, qualification for implemented criteria, and closure only
   when every criterion and final join is satisfied. Reuse the relevant settled
   TypeScript source/journey evidence, inspect current Rust code and refine the
   existing spec/ticket. Open a fresh reference browser journey only when behavior
   is uncertain or at the integrated candidate boundary. Start when the capabilities
   actually consumed are proven.
   Before calling a functional criterion implemented, locate its mounted originating
   control, destination edit/service port, and current-scope admission; a source
   pointer to a helper alone does not establish the user action.
   Preserve parent criteria and original dependency rationale. Create a new ticket only
   for distinct runnable work. A no-gap audit returns source pointers by message.
   Do not dispatch an implementation author against an `implemented` criterion without
   a reproduced missing user action. Qualify several related implemented criteria in
   one candidate journey; turn only an observed failure into a repair packet.
   For an interactive slice, its finish condition names the originating control,
   destination workspace/editor, observable edit result and scope change behavior;
   rendering or selecting an object alone does not complete that journey.
   When the functional implementation queue is empty, probe one unverified user
   action per workbench against the pinned reference. Turn a reproduced gap into a
   missing criterion and repair it directly; do not repeat a broad source audit or
   create an issue for behavior already mounted.
3. Implement mounted behavior with explicit file ownership. Authors normally use their
   assigned isolates; the coordinator may grant disjoint edit-only leases in its checkout
   for a single batch, retaining sole commit/build ownership. Inspect overlapping edits.
   Return a commit or owned diff plus existing evidence pointers; no extra handoff form.
4. Integrate ready disjoint work as it lands. Serialize only overlapping edits and
   shared-file commits; do not hold an independent stream for another stream's
   compile or qualification. Reuse a relevant settled base across unrelated changes;
   refresh only overlapping source/consumed interfaces. Publish shared helper
   signatures early. The author continues the next real gap once its packet lands.
5. Integrate several runnable functional packets before freezing a candidate. Run
   one combined format/compiler check for the batch, not one full check per author
   packet. Pin the candidate's changed-action qualification scope and source-file
   footprint in the run record before packaging; later disjoint author edits do not
   change the published package's identity or its reusable evidence. Package
   early only for a blocking interaction that cannot be checked from source or focused
   tests. The package helper runs the existing locked Dioxus page compiler check once
   per batch. Repair its diagnostic list in place; reuse verified unchanged providers
   and warm caches. Publish through the provenance-derived proof and asset validation.
   Coordinator assigns heavy slots; at most two heavy jobs, including one package,
   run at once. A newly connected cross-workbench action needs its destination and
   edit result named in the packet, with one focused smoke when first packaged.
6. Qualify and review at coherent functional milestones, not after every packet or
   intermediate package. At a milestone, check changed user journeys once against the
   pinned TypeScript reference and reuse unchanged evidence. Run those focused
   candidate journeys before the consolidated Sol 6.1 High review: repair a
   reproduced defect in the same batch and review the resulting candidate once.
   A journey with an unavailable public trigger stays explicitly unqualified and
   does not require repeated setup before review. If a retained fixture cannot
   exercise the action, record that limit once and move on while a suitable fixture
   is prepared. Keep broader lifecycle, visual parity and release qualification
   queued until their owning milestone. Other streams continue during qualification.
   Use one paired saved fixture/session per workbench milestone to cover several
   related controls and edits, then point every satisfied criterion to that one
   receipt. Do not repeat successful setup or unchanged branches for each criterion.
   An unavailable fault-injection route stays explicitly unqualified; repeated
   attempts with the same mechanism add no evidence. Earlier Layout-ready
   qualification remains historical evidence.
   A criterion does not need its own browser session: one paired journey may prove
   several visible actions, and unchanged successful branches retain their evidence.
   Before a browser automation result is called data loss, inspect the accepted
   saved value and the full option list; duplicate visible labels require exact
   option values, and a displayed default may be a select-binding defect.
   For an error or stale-result branch that has no public trigger, use one focused,
   deterministic owner-level regression together with evidence that the mounted UI
   presents the resulting state and retry action. State the public-observation limit
   accurately; do not repeat browser fault-injection attempts that cannot reach the
   provider. Evidence-only criterion reconciliation needs no new package or review.
7. Accept a parent only from its actual criterion verdict and final joins. The
   [acceptance requirements](docs/migration/ACCEPTANCE-REQUIREMENTS.md) retain behavior,
   compatibility, accessibility, performance and retirement gates. Report parents,
   completed journeys, unmet criteria and candidate age separately.

During source integration, ordinary reversible UI needs no new per-packet application
suite/browser/review or full compiler run. A reproduced bug gets its focused regression
and affected checks. When disjoint authors share one checkout, defer compilation of an
in-progress shared source graph until their edits settle; the coordinator runs one
combined check.
Run focused Rust tests through `scripts/migration-deliver.py focused-test -- ...` so
a successful command with zero executed tests cannot count as verification.
Focused tooling checks validate delivery controls only when they change. Broaden only
for a failure or identified unresolved risk. Source, compiler and package success
never imply acceptance; keep unqualified criteria open until their milestone.

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

Run the existing `retro` skill briefly after every two published functional candidates,
or immediately when the same avoidable failure repeats. Use the run record and RF ledger
as sources; apply the highest-impact process fix in place without opening another
review chain or document. This is a project cadence, not a gate before the next author
packet. Record the last checkpoint in the run record so the cadence survives handoffs.

Process rules (2026-10-04 retro; each is checkable):
- Status is generated, not hand-written: counts, served candidate and handoff come from
  `progress.py`; do not paste them into prose. Commit records with the source change that
  they describe, or at a milestone; no record-only commit per candidate.
- A state-changing control is `implemented` only with a native test of its edit's state
  transition (include the mirrored/linked case when it exists). The browser proves mounting
  and the user journey, not the state logic.
- Presentation code is wasm-only: run the wasm `page` check before committing it. Tests
  must execute (`check-wasm-tests.py`, `focused-test`); a plain `#[test]` in wasm-only code
  does not.
- Run one independent diff review per integrated batch before packaging; its findings become
  failing tests before the fixes. The consolidated review is for accepting a parent.
- At most two published candidates per UTC day; `gc-builds.py` keeps the newest three plus
  served, baseline and accepted builds.

Token discipline (a long session re-reads its whole context on every call):
- Restart the session at about 150k context from a generated `handoff.md`
  (`progress.py handoff` plus a short hand-written tail); do not wait for auto-compact.
- Agent reports are 150 words or less: verdict, files changed, failing checks, and a path
  to detail in a file. Do not spawn an agent for an edit under about three files.
- No screenshots or `innerText` dumps unless a visual difference is under test; have scripts
  return small JSON. Pipe build and test output through `tail` or `grep`.
- Run Opus only for one diff review per batch and for bugs Sonnet could not solve.

Delegate by default: dispatch bounded, disjoint-file packets to author agents in parallel
and keep build, publish, ledger and commit ownership with the coordinator. Use the route
for the running harness (Codex: Luna low/medium/high authors, Sol 6.1 High review, Astra
high/xhigh for difficult evidenced bugs; Claude: Sonnet 5.5 authors, Opus 5.5 review and
coordinator, Opus 5.5 for difficult bugs). Worktree isolation does not work here (no
`origin/main`; it would branch from `main`), so give parallel authors disjoint files in one
checkout and commit their exact files yourself. Runtime slots bound the user ceiling
of 30. [Agent routing](.scratch/dioxus-frontend-v1/AGENT-ROUTING.md) holds launch details.
