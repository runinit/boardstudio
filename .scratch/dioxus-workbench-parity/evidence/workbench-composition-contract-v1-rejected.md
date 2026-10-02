# Workbench composition contract proposal

Status: design proposal only; implementation awaits independent Astra clearance.

## Baseline and authority

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/frontend-workbench-composition-20261002`
- Branch: `codex/frontend-workbench-composition-20261002`
- Exact base: `89b1de8a28fdf02db91d972c90a69235bfbbbffb`
- Baseline working tree was clean. No repository files have been changed and no build/test was run.
- Scope authority: user's approved six-stream migration and composition task packet; frontend constraints in `CONSTRAINTS.md`; accepted ownership in `docs/adr/0003-rust-application-ownership.md`; current frontend roadmap and first-tranche acceptance rules.

## Problem

`web/src/presentation.rs` has private modules for existing features (including Library, Objects, panels, Keymap, Parts, Case and Inspector), but the `Editor` component still directly decides which stream appears and mounts its Objects, toolbar/canvas and Inspector content through one large conditional tree. This leaves the shared presentation file as a recurring integration point as the six streams advance.

The current owner is intentional for shared lifecycle and composition. The task is a private presentation extraction that gives each stream a local composition boundary while keeping Session/Core and current actions authoritative.

## Contract

Add one private workspace-composition module in the page crate and register it privately from `presentation.rs`. It owns only the rendering composition/routing for the six streams Layout, PCB, Keymap, Keycaps, Case and Parts, including each stream's current Objects, toolbar, canvas/content and Inspector choice. Reuse existing feature components and current routes. The existing global shell, top-level workspace navigation, footer, and shared panel implementation remain with their existing shared owners.

`Editor` remains the sole owner of `Runtime` access and subscription, the root repaint signal, scope reconciliation, SelectionAdapter and selection cleanup, shared feature state, hook lifetimes, action construction, and submission of session events. It prepares and passes the already-authoritative immutable snapshot/scope and current private action/display values to composition. Composition renders them; it does not create another writable document, selection, history, service/session coordinator, lifecycle owner, or copy of a domain algorithm.

Keep the boundary in the same page crate and private. Do not make a library API or external member public. A narrowly scoped `pub(super)` item is acceptable only where Rust parent/child module visibility requires it and remains inside the existing page module. Avoid a generic plugin/registry/template framework or a six-crate/module hierarchy. Do not move hook initialization or cleanup merely to make the extracted component appear self-contained.

Preserve the current route and behavior of all six stream names. Existing placeholder content remains placeholder content until its vertical feature ticket. Keep the current Objects content selection, Layout-only toolbar/canvas details, Keymap controls/canvas, Case viewer and scoped Case state, Parts library/inspector, Inspector presence rules, panel preferences, compact reveal/focus behavior, and existing panel accessibility relationships. This is composition only: no labels, geometry, layout, styles, panel behavior, or user-visible feature expansion is intended.

Case-specific object-tree entries and moving mechanical settings controls to another menu are later feature slices. They are not prerequisites or implicit work in this extraction.

## Exact proposed ownership

Owned production files:

- Add `web/src/presentation/workspace_composition.rs` (private module): extracted six-stream body and per-stream panel-content routing, with a narrow private input contract.
- Edit `web/src/presentation.rs`: private module declaration/import and replace only the corresponding inline route/mount sites. Keep `Editor` state/hook/action ownership and shared shell/panel geometry there.

No ownership of `Runtime`, `Core`, CAD, global CSS, saved formats, generated/public contracts, or other context worktrees. Do not edit feature implementation files unless review proves an existing component cannot be composed within the approved private boundary; surface that as a new exact contract question first.

## Public behavior seam and characterization

Use the approved paired public browser journey seam against the current TypeScript reference at `http://127.0.0.1:5175` and Dioxus candidate at `http://127.0.0.1:34687`, with the same fixture, viewport/theme and user actions. Capture baseline evidence before extraction and rerun the same journeys afterward. Do not add source-shape or private-component tests. Assertions must come from visible/user-observable state and resulting document/history/selection behavior, not a value computed from the component routing implementation.

Characterize at least:

1. Opening the same saved/demo document and moving through all six workspace tabs preserves the current visible stream content and shell state; where a stream is currently placeholder-only, assert that existing behavior rather than inventing parity.
2. Layout object selection updates the visible selected-context/Inspector controls and shared canvas selection as it does today; existing panel open/close and keyboard/focus behavior remains intact.
3. Layout layer/Footprints controls, Keymap selection/panel, Parts query/selection and Case physical-instance selection/viewer retain their current public behavior and scope.
4. Compact Objects/Inspect toggles and focused Case workspace controls retain current visibility and focus behavior.

Treat this as characterization of existing candidate invariants for a behavior-preserving refactor. Do not manufacture a failing source test merely to force a red phase. For each unexpected baseline failure, record its exact public reproduction and stop before converting it into an assumed behavior change; preserve the existing acceptance rule that reference defects need an explicitly approved corrected oracle.

The accepted primary seam is the paired public browser journeys. Any additional verification is supporting evidence only: affected root-orchestrated native tests, strict WASM Clippy/build, and the established browser/build gates applicable to this diff. Do not run Cargo/build in this author worktree; the coordinator owns root-only integration verification. Report commands/results and unavailable gates without promoting a partial check to acceptance.

## Completion evidence

- Independent Astra clearance of the concrete private module/input contract before extraction; independent Standards and Spec review of the resulting diff.
- Baseline and post-extraction paired public journeys, same fixtures/actions, visible state plus document/history/selection outcomes, with source/build/fixture identities and browser/viewport/theme.
- Required affected native/WASM/build checks run by the coordinator, with existing failures and blocked gates retained.
- Diff review confirms only the two owned production files, private visibility, unchanged presentation behavior, no new authority/state/algorithm, and no weakened checks.
- Update the post-port refactoring register/handoff with source-backed evidence. Current expected takeaway: RF-001 is already supported by this composition hotspot; this extraction is a local mitigation, not proof that broad coupling is resolved. Add no duplicate RF unless the implementation provides new evidence.

## Risks and limits

- The inline editor currently combines substantial state/action preparation with RSX. A component boundary may demand an oversized prop bundle. If so, stop at the narrowest render region that can be moved without migrating hooks/lifecycles or inventing a broad framework; revise the exact contract for review rather than moving ownership to solve a code-shape problem.
- Dioxus component/macro ownership and closure types may constrain moving RSX across modules. Keep wrappers private and preserve captured values; do not widen visibility or alter action semantics to satisfy the compiler.
- Conditional mount changes can affect focus, viewer lifetime, listener cleanup and panel state even when markup appears identical. The public browser journey must cover those effects, particularly compact Case focus and scope changes.
- The extraction neither completes any of the six feature streams nor authorizes public API changes, broad refactoring, behavior changes, integration merges, production cutover, or edits to other worktrees.

## Refactoring takeaway

Evidence-backed RF takeaway: `web/src/presentation.rs` remains the shared composition hotspot identified by RF-001; existing private modules do not yet own the top-level six-stream mounting decision. The source supports the known integration-risk observation. No new RF is warranted from this contract alone, and the proposed extraction must not be reported as resolving the broader hotspot.
