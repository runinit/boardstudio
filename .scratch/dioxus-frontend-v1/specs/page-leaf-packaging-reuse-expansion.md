# Audited page leaf packaging reuse expansion

Status: implementation isolated; independent source review and a helper-matched full baseline remain required. Child of the existing packaging reuse workflow. All canonical parents and acceptance joins remain unchanged.

## Problem Statement

Small frontend parity corrections still wait for unchanged CAD, renderer and worker optimization. The first guarded reuse implementation proves only the shared panels and stylesheet inputs, so most contextual control fixes still require full builds.

## Solution

Extend the existing explicit eligibility proof to a small, enumerated set of already-mounted page leaves. Each candidate still rebuilds the page and route packages, preserves exact provider bytes and inventories, and publishes honest inherited versus executed command provenance. Shared registration, provider, contract or dependency changes continue to require full builds.

## User Stories

1. As a frontend author, I want an audited command-pill label correction to become a comparison candidate without rebuilding unchanged CAD.
2. As a PCB author, I want an audited part-input control correction to use the same guarded packaging workflow.
3. As a Parts author, I want an audited definition-name editor correction to reach the browser promptly.
4. As a coordinator, I want each eligible leaf to have a concrete worker-exclusion proof.
5. As a reviewer, I want shared registration and dependency inputs held unchanged while a leaf changes.
6. As a reviewer, I want new source files or new consumed configuration to reject reuse.
7. As a user, I want the comparison build to contain the current page controls and the same verified geometry providers.
8. As a user, I want root and subpath routes to work with the same package assets.
9. As a verifier, I want an ineligible change to fail before creating a candidate directory or running build commands.
10. As a verifier, I want extra, missing or modified provider assets to reject the candidate.
11. As a maintainer, I want honest full-build lineage and freshly executed command records.
12. As a coordinator, I want this optimization to leave paired edit, history and persistence acceptance gates intact.

## Implementation Decisions

- Reuse the existing build-helper eligibility and provider validation seam; no second packager or general build cache.
- Audit only these already-mounted leaves: Layout command-pill composition (`web/src/presentation/layout_workspace.rs`, `web/src/presentation/objects/layout_toolbar.rs`, `web/src/presentation/objects/layout_transform_toolbar.rs`, `web/src/presentation/workspace_composition.rs`); PCB part-input Inspector body (`web/src/presentation/pcb_wiring/part_input_settings.rs`); and Parts definition-name editor (`web/src/parts_definition_name.rs`). Preserve the earlier `panels.rs`, scroll-test, and CSS allowlist. Do not admit adjacent owners, parent workspaces, or presentation-directory wildcards.
- Lex and compare each changed leaf's module declarations, all attributes, imports, macro registrations, and `include!`/`include_str!`/`include_bytes!` token groups against the helper-matched full source commit. The scanner must handle comments, nested scopes, whitespace and token groups; ordinary `rsx!` leaf behavior edits remain eligible. Keep the root module chain, Cargo/build inputs, helper and generators frozen. `lib.rs`'s core-worker test alias for `presentation/objects/layout_align_geometry.rs` remains excluded and protected.
- Verify the page-only module chain and all alternate module/include references, including the native test aliases. The existing core-worker test alias for Layout alignment geometry must remain excluded.
- Freeze the root page/worker registrations, library entry point, manifest/build configuration, helper, generators and tool identities against the helper-matched full baseline.
- Any changed registration, newly added source path, worker alias, contract, provider or dependency input is ineligible and requires the full path.
- The allowlist expansion itself requires independent source review and a new full-build baseline; an old helper baseline cannot authorize the new helper.
- Preserve the current exact provider path-and-hash maps and fresh provider-only staging, including configuration addition/change/deletion guards under optimized Python.
- Keep source integration and browser acceptance separate; faster packaging does not close a feature or canonical parent.

## Testing Decisions

- Exercise production eligibility and staging through the existing disposable full-build/reuse harness.
- Each added leaf needs a positive permitted-change case and a worker/shared-input negative control. Test source additions and alternate-reference changes as ineligible, not merely different helper constants.
- Run normal and optimized Python checks; retain original guard regressions.
- Bootstrap a real helper-matched full build, make a representative reviewed leaf change, run the real eight-command reuse path, and independently compare all inherited provider inventories.
- Click the actual changed control in paired React and Dioxus, then qualify root/subpath and offline behavior. Report measured timings as observations rather than guarantees.

## Out of Scope

General caching; reusing changed providers; wildcard page eligibility; weakening feature checks, paired browser journeys or parent joins; modifying public APIs or saved formats.

## Further Notes

RF-009 records repeated packaging cost and provenance hazards. RF-001 records shared mount coupling. Their broader restructuring remains deferred. User authorization already covers automatic reviewed spec/ticket refinement and capability-specific starts; this draft needs independent technical review, not another user interview.
