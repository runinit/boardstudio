# Layout Escape and multi-selection repair

RED: temporarily removing the Perimeter Escape handler made focused `mounted_outline_perimeter_escape_returns_to_board_inspector` fail because selection remained Outline. The handler was restored.

GREEN (prior packet): the scoped module runner reported `executed 25, failed 0`, but this was not 25/25 passing: it silently excluded the known RF-033 test `mounted_component_drafts_survive_unrelated_acceptance_and_blur_uses_latest_owner`. No per-test result JSON or full terminal log was retained, so the earlier run cannot establish that module's zero-exclusion result. The separate direct `additive_` matrix-selection tests passed 2/2; native Session group MoveParts + Undo passed 1/1; wasm `page` check and `git diff --check` passed. Root will rerun the combined gate with the exclusion accounted for.

Deselection review regression: `toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys` exercises toggle-add A/B/C and toggle-remove C/B through `submit_matrix_cell_selection`, then edits Local X and verifies the command updates remaining A while deselected B stays unchanged. RED was observed: 0 passed/1 failed at the assertion that removing C should move the Key Inspector to a remaining selected member; the stale C hit context remained active. The original terminal stream was not retained; the command and exact failure excerpt transcribed from the tool output are in `layout-escape-repair-20261005/toggle-removal-red-excerpt.md`. GREEN: focused `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys` passed 1/1; full terminal output is `layout-escape-repair-20261005/toggle-removal-green.log`. `rustfmt` and `git diff --check` passed.

SHA-256 of final packet files:

- `outline_lifecycle.rs` `24e80e1475d9e9e44b73d0b0fdcfc629de249d4e2246a69db2337dea7ba1d1ec`
- `outline_lifecycle_browser_tests.rs` `59acbc4788759fd949afb71a57eb5b934a16649aa8e46f25c1b7146666420942`
- `selection.rs` `2f567500f058f690043815bc765cc61cea45297eabbe6a5e05ac722b71ab8378`
- `matrix_transform_inspector_tests.rs` `e06d6630eb76ef05b4bf39b6c394b8d0477306e0107d915e59d2ce2cd742c189`
- `layout_component_inspector.rs` `6a9c3988589438ad868c1d616c556a8b3ea47582cb83947a83baf0e309ad107f`
- `layout_component_inspector_tests.rs` `7ee263b0db8a48a2b87a8c938819715779d391bebacb447c5679fc52f250acd9`
- `layout_workspace.rs` `ae8fcd59fb7a9ac78c95fcca936a13614d74103c82553a08a361f93fb95f02fd`
- `presentation.rs` `fae911b0f35c6d935c6ce512319b2ef8b83b70e8e5a3a34c3a63f7defacc7b5c`
- `durable_session.rs` `79c1dc8226cf4b652fbfda820a10c4da6dbe23641526bc95c9903afa8f7c37b3`

## Public reference comparison — intentional correction

On pinned TypeScript reference 5a472a9 at5175, using the manufacturing input fixture and Right PCB, Ctrl-select SW1/SW2/SW3 then toggle off SW3/SW2. The reference retains Key3 then Key2 Inspector headings even though only SW1 remains selected. Actual Local X=1 followed by Fit selection blur moved deselected SW2 from X231.77 to232.77 while selected SW1 stayed212.77; one Undo restored the edit. See `layout-escape-repair-20261005/reference-toggle-removal.json`. The repair deliberately targets remaining selected keys, correcting an unsafe inherited reference behavior under the operating contract's authorized functional/UX corrections. Candidate public replay remains pending; no exact reference parity claim is made for this branch.
