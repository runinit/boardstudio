# First frontend tranche: shared acceptance and dispatch rules

This proposal refines INT.1, F2.1, F2.3 and F3.1 from the existing frontend plan.
It does not replace, close or modify their parent specifications or the 62-task
graph. The first twelve tickets and six follow-on children are published under AUTHORITY.md. Routine next-ticket publication is automatically authorized; retained draft snapshots are historical.

## Acceptance for every ticket

- Deliver the stated behavior through the integrated public Dioxus application,
  existing session and real provider where applicable. A standalone component,
  fixture-only implementation or screenshot alone cannot complete a ticket.
- Characterize the same fixture/actions against the pinned React reference.
  Compare relevant visible results, document/history/selection side effects and
  storage outcomes. For a defect correction, first reproduce the failure for
  its expected reason. Do not treat a newly discovered reference defect as an
  automatically approved behavior change.
- Include the normal, empty/invalid/error and scope/lifecycle states relevant
  to that action in its own ticket. Do not defer stale-result rejection or
  cross-project correctness to a later hardening ticket.
- Preserve the existing shell, themes, default keycap outlines, five Layout
  layers and shared Footprints toggle. Reuse unchanged evidence where sufficient;
  rerun checks affected by extraction, composition or behavior changes.
- Match the reference's labels, tokens, layout and control behavior. Capture
  affected desktop/compact and light/dark states; check keyboard operation,
  focus and axe. Actual assistive-technology evidence remains separately
  required where applicable; an unavailable host does not count as a pass.
- Run the applicable established formatting, strict lint, native/WASM, build,
  browser and repository checks. Preserve relevant performance/resource gates
  and existing non-green evidence. This proposal installs no new gate or budget.
- Retain exact source/diff/build/fixture identities and commands/results.
  Independent Astra review covers Standards and Spec for every included ticket.
  A batch review must cover every ticket, and a bug fixer cannot self-approve.
- Update the existing refactoring register with evidence-backed observations,
  or explicitly record “No new refactoring takeaway observed.” Required parity
  fixes stay in the current ticket; broader redesign belongs to the later phase.

The current planning activity only checks documents, source mapping and the
dependency graph. It does not claim any new application test or UI acceptance.

## Dispatch under current authority

Use the saved [agent policy](../dioxus-frontend-v1/AGENT-ROUTING.md). Normally run
two Luna authors and one Luna verifier alongside the coordinator. Rotate a slot
to Astra High when an exact candidate and its evidence are ready. Reserve Extra
High for an unresolved material review question or difficult evidenced bug.

The coordinator owns shared shell/runtime composition, module registration,
global styles, build wiring and ledgers. Authors receive disjoint private module
ownership; shared edits are integrated serially. A file-ownership queue is a
scheduling constraint, not a reason to invent a product dependency between
otherwise independent tickets.

Immediately before dispatch, attach current exact files, callable inputs/actions,
scope identity, fixtures, checks, browser session and evidence destination in a
separate packet. Ticket prose stays about behavior and durable design decisions.
Work only the frontier whose actual blockers are complete. Every ticket includes
its own integration and verification; do not postpone these to a final merge task.

Retain one authoritative session/document/history path. Keep private UI adapters
in the page crate or use already sufficient public contracts. Do not widen API
or member visibility, change saved formats, rewrite providers or switch production
entrypoints under this proposal. If a concrete boundary cannot be implemented
within existing authority, record the smallest affected decision and continue
unrelated work.

## Automatic publication under current authority

Publish one approved ticket per file in this tranche's `issues` directory,
numbered in dependency order and marked `ready-for-agent`. Draft files and the
proposal manifest retain pending status until that approval. Parent task and
milestone statuses remain unchanged by ticket publication. Parent completion
still requires all mapped ticket evidence and the parent's existing acceptance
conditions; any uncovered responsibility remains open.

The user is approving ticket granularity and blocking edges, not waiving public
API, actual assistive-technology or production-cutover decisions.
