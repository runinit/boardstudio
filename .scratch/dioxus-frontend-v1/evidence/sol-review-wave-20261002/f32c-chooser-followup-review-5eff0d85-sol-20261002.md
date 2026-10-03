# F3.2c chooser follow-up independent review

Reviewer: Sol 6.1 High, Standards and Spec, 2026-10-02. Frozen source 5eff0d852e037f44227a1dcf3c6683a7984af482 in f32c-placement-menu-followup-20261002; fixed point 5bb696dfac11c293b063cfafc3b15ff4043d9217, initial source 7ae5566ad0a7ebcb631d9a2be9445fbd275dd28e. Pinned React 5a472a9426e6e38993361da402cd4ec730feb369 Workbench addContent/availableLibrary/Browse route is the behavior oracle. Review is bounded to the two prior chooser findings and their minimal composition; independent initial source/context review remains separate.

Standards: CLEAR. Spec: CLEAR for this bounded source follow-up. Joined strict all-target WASM and packaged production browser qualification remain required and explicitly unexecuted here.

Default groups and nonempty search both now reuse existing catalogue_choices eligibility. Project overrides/catalogue merge and existing retirement/assembly-snapshot rules remain single owned behavior. React's Components/Controllers/Displays/Encoders ordering and matching predicates stay unchanged. No helper eligibility widening, Runtime change, public API/schema change or task graph change occurs.

The chooser receives the existing Editor PartsQuery signal through Layout Objects; editing advances the existing PartsSelectionGeneration, preserving async intent guards. Browse calls the Editor's private browse_parts_workspace through a typed callback, retaining the shared query. The actual compact panel flag opens; desktop PanelSettings switches to Pinned while retaining width. The existing <=760 compact policy is reused, and extracting is_compact_viewport preserves prior Inspector pin behavior. Other workspace Objects callsites explicitly supply no chooser ports. Mirror overlay test registration is updated for the private props only; no Mirror behavior changes.

Evidence accounting: the four Chrome tests are two actual DOM-mounted production chooser input/Browse tests (desktop and compact), plus two pure catalogue/group/search tests in the Chrome harness. They do not mount the full Editor, compute panel CSS geometry, or qualify public ownership/history. The mounted host uses the same production browse helper as Editor, and checks actual input changes shared query/generation plus real Browse button dispatch. All three retained mutants fail at the intended eligibility/query/panel assertions; restored four-test run passes. These are sufficient bounded regression evidence. Retained logs were whitespace-normalized; their stated hashes refer to those committed derived logs.

Independent frozen-range whitespace check passes; clean worktree and exact artifact hashes verified. Author reports fmt, page WASM check, WASM test compilation and four Chrome tests green. Final strict all-target Clippy is deliberately deferred to the joined root and must not be called already passed for this follow-up. No redundant heavy run or reviewer source edit was performed. Initial accepted-component context/placement behavior is unchanged by this delta; public menu/context/history/save-reopen and full F3.2/F3.1/F3.5/F3.7 joins remain open. Existing-half mirroring, Board outline launch and Geometry scripts remain explicit missing menu capabilities; source clearance here does not close the full reference menu.

Exact artifacts:

| File | SHA-256 |
| --- | --- |
| `web/src/presentation.rs` | `c2c8b153dcdee9b4dce01d8bfd86d42082e6f6d8a9ba5867b5cf828c88c94ae8` |
| `web/src/presentation/parts.rs` | `bee6b9b3edc91c1690d28f84b0b4d88fdfde452594bc198ab22c3b0a617fc95a` |
| `web/src/presentation/objects.rs` | `dad8dd33d51dc4e682cd0480558a0d1c64418a4c1039d29fff77c420ef3d314f` |
| `web/src/presentation/layout_workspace.rs` | `34d4bd9af36d3c17fbac3b3cf02b43798fcf448ee5e0d01f55edf8db9b723f48` |
| `.scratch/dioxus-frontend-v1/evidence/f32c-placement-menu-followup-20261002/source-handoff.md` | `ae9d25511a2aaee18babd7f3702e45bc9bf12beecf01e00969c9c7ab94604f1b` |
| `.scratch/dioxus-frontend-v1/evidence/f32c-placement-menu-followup-20261002/catalogue-filter-expected-red.log` | `fe397e70354793c07e2b79f969a5912e2d2121868646c8fcd2e5183438aaa8a3` |
| `.scratch/dioxus-frontend-v1/evidence/f32c-placement-menu-followup-20261002/browse-query-expected-red.log` | `5e149b7eed850bd630d6cebea4568451905ccd8b8638e1039b6d958e51e8170e` |
| `.scratch/dioxus-frontend-v1/evidence/f32c-placement-menu-followup-20261002/panel-reveal-expected-red.log` | `a1a6f42837fda77a5ef5c85cb4b45920906574adcea09417a8a2eabe3e6b45ae` |
| `.scratch/dioxus-frontend-v1/evidence/f32c-placement-menu-followup-20261002/green.log` | `b4fabdcc476e108ebe5beff532b144acdbb8d4c11e7799d25697a13fb0f86288` |
