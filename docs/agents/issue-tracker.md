# Issue tracker: committed Markdown tickets

Tickets are Markdown files in a plan's `issues/` directory, one per ticket, each named in that
plan's `map.md`. The active plan is [typed-core-edits](../plans/typed-core-edits/map.md), whose
tickets live in `docs/plans/typed-core-edits/issues/`.

## Conventions

- File `NNN-short-name.md`, where `NNN` is the next free number; a letter suffix marks a sibling
  of an existing ticket (`109b`). The title line is `# NNN: Title`.
- Header lines at the top (the first twelve lines): `Status:`, `Type:` (`build`, `research`,
  ...), `Blocked by:`, and `Parent:`, `Related:` or `Decision:` when they apply.
- `Status:` is one of the five triage roles in [triage-labels.md](triage-labels.md)
  (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`) or a workflow
  state: `claimed` while someone works it, `resolved` once it is done.
- Comments and conversation append at the bottom of the ticket under `## Comments`.
- Name every new ticket in the plan's `map.md` (`scripts/check-doc-links.py` in `python3 scripts/check.py repo` verifies every Markdown link).

## Building and verifying a ticket

The ticket's acceptance list names what to run. Authoring, implementing or reviewing a ticket
follows the protocol below; the tests it specifies follow
[test economy](../../CODING_STANDARDS.md#test-economy).

### Proof map

Before implementation, the ticket maps each requirement to proof rows, one per distinct
observable failure story:

`requirement — precondition (critical timing) → exact action → independent oracle · tier · status`

- A row is a claim to prove, not a test to write. Reuse sufficient existing evidence first
  (Gortex `change` operation `tests` lists the tests that already reach a symbol);
  parameterize equivalent inputs; keep one row per timing, ownership, admission or failure path
  that can fail independently, never the cross product.
- The oracle is observable outside the code under test (rendered or saved state, a spec literal,
  a reopened result), not a restatement of the implementation.
- Tier is the cheapest faithful to the boundary: `native`, `mounted` or `packaged`.
- Status is `planned`, `reused` (cite the evidence), `RED` or `GREEN` (cite the log), or
  `proof gap`: new proof that passes the existing product. Record a proof gap as GREEN; a RED is
  observed, never fabricated.

### Slice boundary

The proof map, the checks the acceptance list names and the mutations below form the slice's
**boundary**, fixed before implementation. The slice is done when the boundary is met. Changing
the boundary is the user's decision.

- **Mutations.** A `RED` row has shown that its test can fail. A `proof gap` row names the one
  mutation that must turn it red. Run those, plus any the acceptance list names. The boundary
  gives each mutation its test command, its tier and the assertion that must fail; a kill
  anywhere else is a survivor. Use the mounted tier unless only the packaged build shows the
  failure.
- **Order.** RED → fix → affected checks → boundary mutations → one fresh reviewer covering
  Spec and Standards → the ticket's integrated gate. Each step runs once on a stable candidate.
  When browser-gated or `wasm32`-only Rust changes, affected checks include
  `python3 scripts/check.py lint`: native Clippy compiles only those modules' non-WASM stubs.
- **Findings.** A review finding or surviving mutation **blocks** when it shows a boundary row
  or a mandatory criterion unmet; fix it in the slice. List every other finding as a follow-up
  in the completion record, open a `needs-triage` ticket when it is a real defect, and continue.
  Setting aside a finding against a mandatory criterion needs the user.
- **Re-review.** After the review, a change to production code, or one that weakens, replaces or
  retires an oracle, gets one review of that delta. Added assertions and new cases get RED or
  mutation evidence and the author's diff read, including when they close a blocking finding:
  the kill at the named assertion closes it.
- **Attempts.** An attempt is a finished fix or diagnosis that left its target result unchanged.
  Expected RED, build waits and infrastructure retries are not attempts. After two, stop and
  report the evidence, the blocker and one materially different next experiment.
- **Stops.** Fixes inside the boundary proceed and are reported. Stop and ask the user when the
  boundary must change, scope grows or the next step is a new phase. A met boundary closes the
  slice; ticket acceptance stays with the ticket's own gates and, for `ready-for-human`, the
  human.

### Review reports

- Standards and Spec each end with their own verdict: `PASS`, `PARTIAL` or `CHANGES REQUIRED`.
  A separate acceptance readiness line says `READY` or `BLOCKED` and lists every unmet mandatory
  criterion. Changing a mandatory criterion requires explicit user approval; reviewer wording or
  ticket ownership does not waive it.
- State the fixed diff endpoints (`base..head` commits) and list every changed file, with hunks
  where a file was read in parts, as read or unread. Changed source that is unread or truncated
  caps its axes at `PARTIAL`. Git inventories are mechanical; judging that what was read covers
  the criteria stays the reviewer's.
- A complete review of source unchanged since its endpoints stands; review only the delta.

### Completion record

Keep it small: current `HEAD`; checks newly executed versus reused (log pointers); unmet gates and
retained cleanup; follow-ups; pointers; next action. The ticket's `## Answer` and the logs hold the
detail; do not restate historical receipts. Native source counts and test-graph coverage show reach, not
behaviour: behaviour needs a proof row's oracle.

## When a skill says "publish to the issue tracker"

Create the next numbered ticket as the Conventions describe: `Status: ready-for-agent` when an
agent can start, `ready-for-human` when it waits on a decision.

## When a skill says "fetch the relevant ticket"

Read `issues/<NNN>-*.md`; its `Blocked by`, `Parent` and `Related` lines link the neighbours.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** ticket per question.

- **Map**: `docs/plans/<effort>/map.md`, with its notes, decisions so far and open fog.
- **Child ticket**: `docs/plans/<effort>/issues/NNN-slug.md`, with the question in the body, a
  `Type:` line (`research`, `prototype`, `grilling`, `task`) and a `Status:` line.
- **Blocking**: a `Blocked by:` line. A ticket is unblocked when every ticket it lists is
  `resolved`.
- **Frontier**: the tickets that are open, unblocked and unclaimed; the lowest number goes first.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then add a
  pointer (gist and link) to the map's decisions so far.
