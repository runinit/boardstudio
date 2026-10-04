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
   unintegrated packet at a time; once its source is integrated and the combined
   compile passes, the stream may start its next functional packet while public
   qualification waits for the batched milestone.
2. Dispatch from the criterion queue: name a missing criterion, current source,
   owned files, consumed capabilities and finish condition. Use bounded investigation
   for unassessed criteria, qualification for implemented criteria, and closure only
   when every criterion and final join is satisfied. Pin the relevant settled TypeScript journey, inspect current Rust code and refine
   the existing spec/ticket. Start when the capabilities actually consumed are proven.
   Before calling a functional criterion implemented, locate its mounted originating
   control, destination edit/service port, and current-scope admission; a source
   pointer to a helper alone does not establish the user action.
   Preserve parent criteria and original dependency rationale. Create a new ticket only
   for distinct runnable work. A no-gap audit returns source pointers by message.
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
4. Integrate ready work serially. Reuse a relevant settled base across unrelated changes;
   refresh only overlapping source/consumed interfaces. Publish shared helper signatures
   early. The author continues the next real gap once its waiting packet is resolved.
5. Integrate several runnable functional packets before freezing a candidate. Package
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
   does not require repeated setup before review. Keep broader lifecycle, visual parity and release qualification
   queued until their owning milestone. Other streams continue during qualification.
   Use one paired saved fixture/session per workbench milestone to cover several
   related controls and edits, then point every satisfied criterion to that one
   receipt. Do not repeat successful setup or unchanged branches for each criterion.
   An unavailable fault-injection route stays explicitly unqualified; repeated
   attempts with the same mechanism add no evidence. Earlier Layout-ready
   qualification remains historical evidence.
7. Accept a parent only from its actual criterion verdict and final joins. The
   [acceptance requirements](docs/migration/ACCEPTANCE-REQUIREMENTS.md) retain behavior,
   compatibility, accessibility, performance and retirement gates. Report parents,
   completed journeys, unmet criteria and candidate age separately.

During source integration, ordinary reversible UI needs no new per-packet application
suite/browser/review. A reproduced bug gets its focused regression and affected checks.
When disjoint authors share one checkout, defer compilation of an in-progress shared
source graph until their edits settle; the coordinator runs one combined check.
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

Use Luna low/medium/high for bounded implementation, Sol 6.1 High for candidate review,
and Astra high/xhigh for difficult evidenced bugs. Runtime slots bound the user ceiling
of 30. [Agent routing](.scratch/dioxus-frontend-v1/AGENT-ROUTING.md) holds launch details.
