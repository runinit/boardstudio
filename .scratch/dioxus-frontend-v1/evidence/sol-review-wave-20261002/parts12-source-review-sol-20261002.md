# Parts 12 independent source review — Sol, 2026-10-02

## Frozen packet and scope

Both-axis source review CLEAR for exact commit `476bed5e765a3f648aee5dd2565305c413ddd76a` in `/home/chris/.local/share/boardstudio/worktrees/parts-definition-name-20261002`. Reviewed cumulative production diff from `932798dd`, including initial implementation `1b4e2ace`, draft/disclosure fix `98787ca41b68c62092ff7166f44e501fc65755c1`, and mounted-scope correction `476bed5e765a3f648aee5dd2565305c413ddd76a`.

Approved planning inputs verified: contextual spec SHA-256 `f739b30e6e4fb5aa743ccfd0be3b069a81abd701d96297857a0251bfa48f9884`; issue 12 initial approved SHA-256 `63017b6b9b19c46b09603bc5140b0586dfcb8015a62b7ed25b52c9ace064687f`; current authorized status-only issue SHA-256 `779b1cef5c6705f1708a27775a8c67e00a3f9e00c125239496182d2b2302a90b`. No canonical graph changes are included in this clearance.

Frozen source SHA-256: `web/src/parts_definition_name.rs` `7a1b83d1e22a331cb79ac65354a4c50d828ea39e3464285797af3a6fdb2f3355`; `web/src/runtime.rs` `7dcdf5f2b90df42fac834253ea4596b0f25c3cb509a8ac74d8733accd3b701f6`; `web/assets/m1.css` `d47233271d85968b6b4bb2b64223a130b0cb2539527e306482bb6514e6f04760`. Dirty shared CSS hash `100a1b423a5af0d903510ba154354c152a3bea7708c11ece547c205bcaa14faa` is excluded and must not be integrated as part of this packet.

Evidence reviewed: browser/component report SHA-256 `c137380088de60cd42300aa5043939dc0ee9c5104d815002fdbd2bb19aa08a68`; status receipt SHA-256 `e77589f6c8730ebd0174b0f903915836b078071da935ca10881f58ec3e615e56`.

## Standards

Zero remaining actionable findings against supplied project instructions, `CONSTRAINTS.md`, architecture, ADR 0003, and the code-review smell baseline. The production adapter remains private and uses the existing Session/Core edit authority. No public operation, existing member visibility, schema, or domain authority is widened. Draft ownership and accepted snapshot admission remain separate.

## Spec

Zero remaining source findings. The mounted control keeps a dirty draft across unrelated accepted revisions while refreshing its admission capture. Scope/selection/definition/accepted-name changes reset draft ownership. Disclosure choices survive changes to the same definition and reset to pad-count defaults for a new owner. The adapter resolves the exact current accepted document, modifies only the selected definition name, and rejects stale scope/session/document/token/revision/selection, missing targets, generator targets, and unchanged values before submitting an edit. Raw-string behavior matches React.

One finding during review was corrected: the mounted fixture fabricated a nonempty board scope despite its accepting Session having an empty active-board identity. The replacement commit uses `session.scope()` and asserts that it exists. The test-only Runtime seam executes the production mounted blur callback and production adapter; its emitted event is accepted through Session/Core, preserving the unrelated accepted edit. It is component/Session evidence, not public app acceptance.

## Verification and remaining gates

Independently inspected immutable commit source, cumulative diff, exact source/input/evidence hashes, and `git diff --check`. Reused the author's fresh replacement results: native focused tests 3/3; mounted Chrome/WASM test; strict all-target WASM Clippy with `-D warnings`; WASM check; formatting and diff checks, all reported passing. The report records red/green for dirty-draft retention and definition-owner disclosure reset. No app sources were edited by this reviewer.

Paired public React/Dioxus interactions, unrelated-edit blur preservation, Enter/Escape, accepted-name synchronization, selection/stale-context behavior, Undo/Redo, and save/reopen acceptance remain OPEN. Source clearance does not close issue 12, F4.2, INT.2, or authorize React removal. Retain RF-001/RF-009 context; no new refactoring takeaway observed.
