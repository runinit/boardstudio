# Parts initial-default preview repair

Only `web/src/presentation/parts/generator_settings.rs` is owned by this repair. The prior author’s mounted provider fixtures are preserved and completed. No Runtime, Case, catalogue, Parts root, mechanical profile, source selection, generator engine, canonical records, staging or package changes were made.

## Actual defect and owner

Published source38641dd1 initially displays Back in bundled MX settings while drawing the Front geometry. The same public input becomes the reference-matching seven-pad Back preview after explicit Front→Back without Apply. Exact public evidence is `../parts-desktop-visual-20261005/original-context-explicit-back.json` and its paired screenshots; source chain is in that folder’s `source-diagnosis.md`. The retained engine intentionally falls back to F when explicit side is absent; the reference initializes a transient draft with schema defaults plus saved values. The Dioxus editor previously only requested a preview after input.

## Executed RED and correction

The initial mounted test ran against handed-off generator SHA `a1e76148e239d7c00487543dd5d28c7c1adece785be726913dc0124150d88a5a`. It mounted the real editor, loaded the real packaged schema and observed Back, then failed solely because no initial side-B provider request arrived: **0 passed / 1 failed**, strict complete inventory, no setup failure. `generator-initial-red-full.log` retains full stdout/stderr; `generator-initial-red.json` retains source identity and exact outcome (35,397 ms filter). The prior interrupted62996 run is not RED evidence.

The editor now initializes its transient request from actual schema defaults overlaid with saved values. Missing defaults stay absent (undefined net bindings must not become fabricated JSON nulls). Initial and user-edited requests share the same Pending/Ready/Failed provider path, sequence and current-owner checks. Owner changes reset the draft and seed their own request; component retirement invalidates pending responses before they read mounted signals. No automatic accepted edit is introduced.

The failure and stale-result fixtures target the actual Apply button, consume the new initial request and await provider-return barriers. They prove an older edit and the delayed initial reply cannot replace the newer Ready draft; a retired selection cannot replace the new selection’s Pending draft, which then becomes its own Ready draft. Existing accepted-edit metadata/rebase/deleted-owner assertions remain.

## Fixture inputs and limits

The actual GeneratorSettingsEditor receives Runtime, GeneratorDraftStore, SelectionAdapter/scope generation, PartsSelectionGeneration, WorkspaceState and version contexts. Runtime accepted snapshot/scope are fixture seeded; this is not a real Core/Persist execution proof. The schema and the initial candidate normalizer use the official runner’s real packaged generator asset URL. Only completion ordering at the existing candidate-provider boundary is controlled. The initial response is normalized by the real retained provider and proves seven pads, pad1 left/pad2 right, Ready publication of that exact candidate, sideB, and full accepted-document equality without an input event or Apply. The test does not mount the parent Parts preview canvas, claim a rendered canvas, or claim persistence; the required packaged public replay remains with the coordinator.

The retained geometry adapter does not populate optional Pad.side for the quoted layer tokens here. An attempted assertion on that optional metadata was incorrect; the final test asserts the directly observed geometry positions instead. Read-only real-provider inspection returned pad1 x=-7.109999999999999 and pad2 x=5.842. The shared provider was not changed.

## Affected checks

Actual executions use the official `runner_environment`, `desktop_webdriver_config`, `packaged_generator_harness` and strict inventory/outcome accounting, with Rust invocation through `migration-deliver.py focused-test -- wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- presentation::parts::generator_settings::tests::`. The full module contains exactly four tests. Public reproducible entry point is `python3 scripts/run-wasm-tests.py --files web/src/presentation/parts/generator_settings.rs --result-json <output>`. No unrelated modules were rerun.

Intermediate full-module attempts are retained: attempt1 1/4 exposed the synthetic null defaults and ambiguous button selector; attempt2 3/4 exposed only the unsupported Pad.side assertion. These are not passing evidence. Final result is appended below only after terminal verification.

## Integration coverage

The existing four owner-map names cover this file’s entire WASM module. Its coverage description must identify the initial-default production request and owner lifecycle repair rather than claim a test-only diff. Preserve real generator assets, compiler/native/reachability checks and independent changed-file owners. Root owns the mapping, combined gate, commit, necessary package and public initial-selection Back replay.

## Final terminal settlement

Session54431 exited0. **4 expected / 4 passed / 0 failed / 0 incomplete**, strict complete inventory, no duplicate outcomes, no exclusions or problems. Full-module filter duration **34709 ms**. Raw output: `generator-module-green-full.log`; structured result: `generator-module-green.json`. `git diff --check` and scoped rustfmt pass.

Frozen source SHA-256: `7a5b565ba1a56476a13c94ad788144b437660a9e1f33c85c342d84cb531cf676`. Compiler released to the other author at terminal; no further changes or tests are scheduled by this author. Final independent review and packaged public replay are coordinator-owned and remain required.
