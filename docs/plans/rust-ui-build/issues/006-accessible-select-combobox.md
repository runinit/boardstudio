# 006: Accessible Select and Combobox

Status: resolved
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [accessibility floor](../../rust-ui-migration/accessibility-floor.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

Select and Combobox pass every row of the accessibility floor, so the 105 selects in the app can migrate to them.

## Acceptance criteria

- [x] Select returns focus to its trigger after Escape and after choosing an option, and its open listbox has an accessible name taken from the Field label
- [x] Select opens with Enter, Space and ArrowDown; arrows, Home, End and typeahead move; Enter selects; its trigger shows the option's label, not its raw value
- [x] Combobox keeps focus on its input, moves `aria-activedescendant`, filters on typing, selects with Enter and closes with Escape
- [x] Repository browser tests using the role-and-name helper cover every floor row for both controls and pass
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches


## Outcome

### Implementation and ownership

- T3 binding and actual checkout both verified as `/home/chris/.t3/worktrees/ts-boardstudio2/lane-006-accessible-select-combobox`, branch `lane/006-accessible-select-combobox`, created from local migration base `cb58198cfb3fcb032b452e016a95559593c09caf`. Claim saved in `16e343f` before implementation.
- Applied the existing `new-lane.sh` setup steps in the bound checkout without creating another worktree: isolated `target` symlink to `/home/chris/.cache/boardstudio-target/006-accessible-select-combobox`, ignored settings and provider caches copied only where absent. Checked active builds; all lane builds ran serially against that target with project concurrency settings. No Gortex tools or gates used.
- Changed only this ticket, `web/crates/ui-shared/src/ui/select.rs` and `web/crates/ui-shared/src/ui/primitives_mount_tests.rs`. Combobox already met its floor through the pinned primitive, so no behavioral rewrite or helper change was necessary. Preserved 004's icon implementation/imports and 002's classes/tokens on clean merges (`74340a5e4` and `4f5117b52`, respectively).
- Select routes caller `id` and ARIA attributes to the interactive trigger; its `aria-label` / `aria-labelledby` also name the listbox. The test composes `FieldLabel` with a labelled span ID referenced by `aria-labelledby`. Focus returns on actual closes, including controlled closes. Enter and Space open on keydown with native duplicate activation prevented; primitive arrows, Home/End, typeahead, option selection and disabled-item handling remain intact.
- The existing primitive `text_value` contract supplies the visible/search label when it differs from the raw value. Tests select Banana/Cherry while controlled callers receive `b`/`c`, check selected state, and retain input focus / active descendant behavior for Combobox. Icon markup contributes no text to the tested names. No new dependency or public API, no existing test rename/removal, and no baseline update.

### RED and GREEN evidence

All logs below are gitignored under this lane's `.evidence/006/`; results are executed here unless explicitly described as earlier evidence.

- `54de29128`: committed the Select/Combobox floor tests. `red-mounted.log` first real failure: no button named `Fruit` from the Field label. Later failures in that batch were panic-hook fallout, not independent regressions. After forwarding the label, `red-focus.log` failed exactly at `selection closes and restores the trigger`; the preceding naming, roles, navigation, disabled skipping and typeahead assertions passed.
- `df262f32a`: implemented naming/focus fixes and controlled outcome checks. `green-mounted.log`: focused 3/3 passed. `green-ui-shared-browser.log`: complete ui-shared subset 42 + 2 passed, before the 004 merge. These are earlier, narrower results, not final merged acceptance.
- `merged-build-lint-repo.log`: `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build lint repo` passed all three steps (435.0s) at `74340a5e4`, including 004 but before 002 and the review correction.

### Closure review response

The real initial review of `cb58198c...74340a5e4`, restricted to the three owned paths, returned Standards: zero findings; Spec: one P2 hypothesis about duplicate close callbacks. It was not treated as approval.

- Runtime confirmed the P2 in `298dba67`, before changing production forwarding. `red-close-callbacks.log` runs the actual copied Select with a real paused Web Animation. Condition waits prove focus returned while the original option and list remained connected, then explicitly finishing the animation removes the popup. All five cases execute before the assertion: controlled and uncontrolled Escape/Enter each emitted `[false, false]`; an externally controlled close emitted `[false]`, where no request was due. No sleeps, retries, or weakened assertions.
- `bdaeffebaa7409f921734e76d1e7a2fffb9054e6`: filter primitive open requests against the actual wrapper open state before forwarding. Focus-induced blur of a retained option can no longer send a duplicate close request or echo an external close; value ownership and keyboard behavior remain with the primitive/caller.
- `green-close-callbacks-and-floor.log`: `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib -- ui::primitives_mount_tests::` passed 4/4 after the correction and 002/004 integration, including the unchanged five-case callback assertions and existing floor tests.

### Final verification and landing

Executed on corrected source `bdaeffebaa7409f921734e76d1e7a2fffb9054e6`, including latest local feature `ba8b881c392cb2dde8fc7931f80348b234ee6e2c` (002/003/004):

- `cargo test --package boardstudio-web-ui-shared --locked --lib`: **14/14 passed**, including native copy guards; `final-native.log`.
- `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build lint repo`: **all three steps passed**, 97.7s; `final-build-lint-repo.log`.
- `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only ui-shared`: **44 + 2 mounted tests passed**, including Select/Combobox floor, retained-animation callback regression, and 004 icon names; `final-ui-shared-browser.log`. The runner explicitly reports a partial browser subset, not full browser acceptance.
- `git diff --check`: passed. No test names were renamed or removed.

Closure approved: the coordinator delivered the real targeted Spec review of `c663c0524c11a64e3f4738af097eee7e6989db91`, with zero actionable findings. The P2 is resolved: the actual-open guard prevents duplicate/echo close callbacks while preserving genuine controlled requests awaiting acceptance and leaving selection/value ownership unchanged. The reviewer reused the executed runtime logs above rather than rerunning them. The earlier Standards review returned zero findings; neither review is global-gate approval.

Before resolution/landing, the local feature tip was rechecked and remained `ba8b881c392cb2dde8fc7931f80348b234ee6e2c`, already merged into this lane. The lane was clean and only the three owned paths differed from that feature tip. This final change only records review resolution/status, so the sufficient executed checks above are reused without duplicate test runs. Local landing follows the lane protocol's fast-forward-only `git push . HEAD:refs/heads/feat/rust-ui-migration`; the final commit/ref verification is reported to the coordinator.

Full browser/quick/CAD acceptance, the unresolved CAD browser `worker.onerror` root cause, first-paint timing, user workspace review and the final merge to `dev` remain outside this ticket's evidence. No push to origin or merge to dev.
