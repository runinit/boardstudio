# Project19 repaired-source review — bf12905c

Disposition: **Standards CLEAR; Spec CLEAR** for exact frozen source `bf12905cb58e080cec3f3329757a970e15b054ee`, with evidence-only head `b1962ac15bfeedcff434a6e3ba4aa4bc244c86cb`, in `/home/chris/.local/share/boardstudio/worktrees/project-menu-name-19-20261002`.

This replaces the narrow source HOLD at `ff3b0848b4d36019eb73b8c075826e6284b11f74`; both that report and the initial `ffaaf838` HOLD remain historical evidence. Governing reviewed planning is `331f1af31e494d3352eaa36b53651b8fab25ed93`, spec SHA-256 `74cc557a53b29850403601c738e111804ee1aa77b88f1327f9b8f98a18b459d7`, ticket SHA-256 `f9883a1178917d55fe4cecb0f97bcc3fda8b481f63ef36d903034a655ed4f98d`.

## Standards

The follow-up makes the existing commit closure an explicit private `ProjectNameCommitAction` carrying the same Runtime, project/session owner and shared mounted guard. The production blur handler calls this action, and the test-only probe retains that exact value. No public API, persistence owner, history path or schema changes are added. Its commit still checks mount and accepted owner, rebases the clone on the latest accepted document and rechecks submission token/revision before the ordinary `Event::Edit`/`ReplaceDocument` dispatch. Runtime/CSS remain as independently reviewed at ff3. No actionable source finding remains.

## Spec

The main mounted Library journey now opens the exact document loaded from BrowserStore through the normal `Event::Open` path. It verifies both the durable renamed value and unrelated accepted data in the reopened accepted model, plus the mounted input. The new lifecycle regression removes Library through its actual Dioxus conditional mount, observes `use_drop` retire the shared guard, invokes the retained production blur action, and verifies no operation, name change or revision change. These close the two ff3 coverage findings without substituting a reconstructed helper call or manually accepted save.

The earlier ff3 evidence remains sufficient for accepted-refresh draft retention and latest-document blur, inline Current project markup, trim/no-op/Enter/Escape, Undo/Redo, epoch reset, real Persist failure and delayed completion after owner replacement. The follow-up is coverage of already guarded behavior, rather than a new bug fix requiring an invented red.

## Exact verification

Independently read the committed follow-up source, traced action construction/blur/cleanup/commit, checked the clean evidence head and source diff whitespace, and verified frozen source matches the working copy. `web/src/presentation/library.rs` SHA-256 is `965813e53ecfe5c12af2a791d6804a45e9c3f37845e01775d36c58f988217038`.

Reused exact retained results, independently hashing and reading logs:

- Mounted saved-reopen/failure run: 2 passed, 0 failed; `repair/mounted-reopen-saved.log` SHA-256 `3edf3b27f1709374592adf9eeafa5c56fdff264a4f916c092dd1bc410f380caf`.
- Mounted retained production action after VDOM unmount: 1 passed, 0 failed; `repair/mounted-unmount-retained-action.log` SHA-256 `5e29fa39b0e759701803086c68248df1c011b2ab140bcbf5f2bf09e61e33be10`.
- Exact-source supported page WASM all-target Clippy with `-D warnings`: exit 0; `repair/wasm-clippy-followup.log` SHA-256 `2eadd378c7b189b3a1f646c880d814a775f0194ef52c30a5e9cb5cdc21911f33`.

The author also records final fmt/diff checks. Prior native and delayed-owner results remain attached to ff3, not falsely presented as rerun on bf129. No heavy checks were duplicated.

## Boundary

This is bounded source clearance. Root must preserve the leaf/composition on integration and qualify the combined affected build. The packaged production Project-menu journey, compact/theme comparison, public history/save-reopen and parent acceptance remain open. No full Editor/browser or whole-feature acceptance is inferred from the mounted Library tests.
