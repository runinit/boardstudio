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
   six queues, rather than Layout visual closure. Each stream has at most one implemented
   packet waiting for integration or validation; help finish/repair that packet.
2. Dispatch from the criterion queue: name a missing criterion, current source,
   owned files, consumed capabilities and finish condition. Use bounded investigation
   for unassessed criteria, qualification for implemented criteria, and closure only
   when every criterion and final join is satisfied. Pin the relevant settled TypeScript journey, inspect current Rust code and refine
   the existing spec/ticket. Start when the capabilities actually consumed are proven.
   Preserve parent criteria and original dependency rationale. Create a new ticket only
   for distinct runnable work. A no-gap audit returns source pointers by message.
3. Implement mounted behavior with explicit file ownership. Authors normally use their
   assigned isolates; the coordinator may grant disjoint edit-only leases in its checkout
   for a single batch, retaining sole commit/build ownership. Inspect overlapping edits.
   Return a commit or owned diff plus existing evidence pointers; no extra handoff form.
4. Integrate ready work serially. Reuse a relevant settled base across unrelated changes;
   refresh only overlapping source/consumed interfaces. Publish shared helper signatures
   early. The author continues the next real gap once its waiting packet is resolved.
5. Freeze one combined candidate. The package helper runs the existing locked Dioxus
   page compiler check before fixtures/builds. Repair one diagnostic list in place,
   then retry; preserve existing fixes, specs and failed logs. Reuse verified unchanged
   providers and warm caches. Publish through the delivery command's provenance-derived
   proof and existing asset validation. Coordinator assigns heavy slots; at most two heavy jobs,
   including one package, run at once.
6. When a coherent functional batch is integrated and its candidate compile/package
   passes, qualify its affected working journeys and use one consolidated
   Sol 6.1 High candidate review. Other streams continue. Reuse unchanged paired evidence;
   check changed behavior and affected repairs. Do not wait for visual Layout closure
   or all six workbenches. Earlier Layout-ready qualification remains historical evidence.
7. Accept a parent only from its actual criterion verdict and final joins. The
   [acceptance requirements](docs/migration/ACCEPTANCE-REQUIREMENTS.md) retain behavior,
   compatibility, accessibility, performance and retirement gates. Report parents,
   completed journeys, unmet criteria and candidate age separately.

During source integration, ordinary reversible UI needs no new per-packet application
suite/browser/review. A reproduced bug gets its focused regression and affected checks.
Focused tooling checks validate these delivery controls. Broaden only for a failure or
identified unresolved risk. Source, compiler and package success never imply acceptance.

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
