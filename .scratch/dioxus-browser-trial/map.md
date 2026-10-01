# Dioxus browser trial and Rust application migration

Labels: wayfinder:map
Tracker: local-markdown
Created: 2026-09-30

## Destination

A working browser-first Dioxus trial of layout editing and case generation,
with user-reviewed evidence and an agreed staged route to an application whose
maintained code is Rust, replacing TypeScript and React completely.

## Notes

- User-confirmed destination: a working trial as part of this effort, not only
  a specification. This overrides wayfinder's planning-only default: runnable
  prototypes belong in the map when needed to answer a decision. The charting
  session creates the map and resolves research; subsequent sessions work one
  design/prototype decision at a time.
- User-confirmed platform: browser first, preserving the current web application.
- User-confirmed scope: both layout editing and case generation in a focused
  workflow, then staged migration. Full workspace parity is not required for
  the first trial.
- User-confirmed final boundary: all maintained application code in Rust.
  Generated JavaScript browser bindings and development/test scripts may remain.
  Existing TypeScript CAD adapters and executable Ergogen generators are
  application logic; retaining them indefinitely would not meet the destination.
- Trial architecture, exact acceptance criteria, fixture selection and migration
  order are open decisions. Recommendations are not human resolutions. A layout
  trial cannot establish case/3D or complete migration feasibility.
- Consult `wayfinder`, `grilling` and `domain-modeling` for design decisions;
  `research` and version-matched `context7-mcp` for external facts; `prototype`
  for runnable investigations and `agent-browser` for live browser verification.
  User-confirmed Rust/browser/persistence requirements take precedence over a
  generic prototype's in-memory HTML defaults. Use `codebase-design` if a new
  module boundary needs design.
- Current execution-route session (2026-10-01) continues accepted P2 at
  migration revision `689f8962`; it investigates Luna Fast and seeks one bounded
  renewal decision. It does not reopen the architecture or resolve the still-open
  layout/case/staged-route tickets. Global config and provider files stay unchanged.
- Tracker conventions:
  `/home/chris/.agents/skills/setup-matt-pocock-skills/issue-tracker-local.md`.
  Children carry labels, type, status, assignee, parent and blocking metadata.
  Open, unblocked, unclaimed children sorted by number are the frontier. Answers
  live in their child tickets; this map contains only gists and context pointers.
- Existing Rust domain, CAD and renderer engines should be assessed for reuse.
  Preserve typed edits, preview/commit separation, one-step Undo, committed
  export snapshots, worker cancellation/stale-result protection and frozen
  performance references. Browser/API support in documentation is not execution
  evidence for this application.
- Main checkout started on `dev` at `96dd51d3` with active encoder/VIK work,
  uncommitted test cleanup and planning changes. Do not reset, clean, switch or
  discard that work. Research and trial artifacts use isolated branches/worktrees.
  Recheck live source before implementation because concurrent work can advance.
- Trial persistence uses a separate storage namespace and origin, with copies
  of fixtures. Reading or round-tripping a copied current project can establish
  compatibility; do not write trial data into existing saved keyboards.
- Root [PLAN.md](../../PLAN.md) records direction; [TODO.md](../../TODO.md) tracks
  this effort without replacing the encoder/VIK or performance backlog.

## Decisions so far

<!-- One gist/link per resolved child; detailed answers live only in the child. -->

- [Verify the Dioxus browser platform and toolchain](issues/01-dioxus-browser-platform.md): Dioxus 0.7.10 is a versioned browser candidate without a declared Rust-floor obstacle; actual build, browser adapters, worker packaging and offline behavior remain trial gates.
- [Establish Rust routes for engine integration and remaining application JavaScript](issues/02-rust-engine-browser-integration.md): Existing Rust engines are reusable, while browser orchestration and parameter-dependent generator logic need Rust implementations and compatibility evidence.

- [Verify Luna Fast execution for the existing prototype](issues/07-verify-luna-fast-execution.md): Installed Codex supports Luna and Fast requests; model/effort are observed, while served Fast is not exposed in the available runtime record.

- [Choose a bounded route to a runnable P2 prototype](issues/08-choose-p2-execution-policy.md): User approved the exact formatter prerequisite, one Luna Fast coding writer, unchanged P2 gates and separate finite repair budgets.

## Not yet specified

- Runtime or packaging obstacles found while compiling and running the chosen
  Dioxus/worker/CAD path may expose additional investigations.
- Differences in interaction behavior, reactive updates, load time and memory
  seen in the real trials may change which workspaces migrate next.
- Generator-specific semantic/asset compatibility work may need separate
  decisions once research establishes the possible Rust replacement routes.
- How to converge shared infrastructure with concurrent encoder/VIK changes
  depends on the actual trial implementation and the later cutover boundary.

## Out of scope

- Complete production migration or deleting the React application during the
  focused trial; the staged migration route is in scope, execution beyond the
  evaluated trial is a later effort.
- Desktop/mobile targets, a backend/service migration or an unrelated visual
  redesign; the user selected the browser and existing application workflows.
- Replacing the CAD geometry kernel, changing the case design, rewriting the
  existing Rust domain algorithms or expanding unrelated feature scope.
- Completing or changing the unrelated encoder/VIK and performance tasks.
