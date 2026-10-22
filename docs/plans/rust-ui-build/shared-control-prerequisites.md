# Shared control prerequisites for 016, 017 and 020

This slice breaks the shared-control dependency cycle without importing any workspace migration.
The coordinator owns closure review and local feature landing; workspace claims and global acceptance
remain unchanged.

## Source pins

- Feature base: `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`, fast-forwarded from the clean bound
  `feat/shared-control-prereq` checkout at `b237903705244a266b07a64e5f23b8a3b5219c03`.
- Button: cherry-pick `01c631e8ba1033515d8716db8ffec32c3673add6`, tree
  `2bdf85a35f7bca06b2a59a4e494d608584061021`, parent equal to the feature base. Git source contains
  145 additions in `ui/button.rs`, including the forwarding test module. The contributor's earlier
  statement that those tests were uncommitted was contradicted by that source.
- Checkbox: cherry-pick `150670bad0c4add2e4f6b63bcc49b6fe68d0868b`: only the public upstream
  `CheckboxState` reexport in `ui/checkbox.rs`; `CheckboxProps` remains privately imported.
- Input Group: cherry-pick `df96cddad9a326a25bdfd0acff6692ef6eacfd3d`: 169 additions and one deletion
  in `ui/input_group.rs`, including focused tests. No Case migration ancestry is included.

The previously reported Button forwarding 1/1 pass used dirty workspace dependencies. Its source
blob matched the isolated contract, but that receipt is not clean combined verification. The
`href: Option<String>` fixture correction was compile evidence, and the 020 `TooltipTrigger.onclick`
E0599 was a fixture compilation failure, not a behavioral RED.

## Approved contracts

- Button adds optional keyboard, focus, blur, pointer-enter and description props. Existing owners
  remain in control; native and link branches forward click and pointer callbacks without adding
  navigation suppression or changing disabled-link behavior.
- Button additionally accepts an explicit `attributes: Vec<Attribute>`, without a blanket
  `GlobalAttributes` extension. Tooltip composition uses the existing public `TooltipTrigger.as`
  render callback and this vector on the same Button element. Raw listeners run before typed owner
  listeners, once each. The pinned Tooltip uses **mouse** enter/leave, focus, blur and Escape;
  pointer events remain separate.
- Button's class, style, `data-slot`, type, href, disabled and explicitly represented scalar
  attributes remain authoritative, including omitted scalar defaults. A raw id is a fallback when
  the owner omits id. Description id tokens are joined and deduplicated. Other raw attributes are
  retained. Composition does not introduce a separate style implementation or a new dependency.
- InputGroupInput adds max and native global attribute/event forwarding. Only exact collisions
  with explicit native attributes/listeners are filtered; explicit value, class and input listener
  remain authoritative. Other events, namespaced styles and validation ARIA pass through using
  public Dioxus APIs. The spread stays last, as Dioxus 0.7 requires.
- Checkbox retains upstream controlled, uncontrolled and disabled behavior; no visibility is widened
  beyond reexporting the already-public state enum.

## Verification and handoff

The first prerequisite link-callback run at `f42029438168e5296e15c7216593217762b23c33` compiled, but
timed out waiting for native pointer callbacks because the fixture's synthetic events did not bubble
to Dioxus's delegated listener. `.evidence/shared-control-prereq/button-link-red.log` preserves that
fixture failure; it is not the expected product RED. `bd311f9b` corrects the event initialization to
bubble (and makes the click cancelable so navigation suppression can be detected).

At `bd311f9b`, `.evidence/shared-control-prereq/button-link-red-bubbling.log` establishes the actual
behavioral RED: the log contains each native click/pointer callback once, and the assertion fails
because the link click callback count is zero instead of one. No production fix was present.

Clean combined implementation/test pin: `f26407663d21f51955cbda72c0b3a6232d40f641`, tree
`05365cd1ceb63437cb6a19cbb99a2ec035104e80`. The branch contains only the feature base, the three
isolated contract cherry-picks, the regression fixture commits and the shared implementation/tests.

- WASM: `wasm-pack test --headless --chrome --mode no-install web/crates/ui-shared --lib -- ui::`
  **18/18 passed**, 58 filtered out. The repository runner's `runner_environment` and
  `desktop_webdriver_config` supplied the locked wasm-bindgen runner, Chrome configuration and
  `CARGO_PROFILE_DEV_DEBUG=false`. Evidence: `.evidence/shared-control-prereq/combined-wasm-ui.log`.
  This includes the unchanged Button forwarding test, the actual RED test now GREEN, native/link
  Tooltip focus/blur/Escape/mouse behavior and raw-before-owner event composition, identical Button
  classes and canonical slot, default and disabled behavior, Checkbox controlled/locked/mixed/
  disabled/uncontrolled cases, and both Input Group constraints/precision/ARIA/event tests.
- `cargo fmt --all --check` passed; `.evidence/shared-control-prereq/format.log`.
- `python3 scripts/check-doc-links.py` passed across 568 Markdown files;
  `.evidence/shared-control-prereq/doc-links.log`.
- `git diff --check` passed.
- Native: `cargo test -p boardstudio-web-ui-shared --lib` **16/16 passed**, including the new
  Button scalar/style ownership, id fallback, description deduplication, unknown-attribute and
  empty-default policy test. `.evidence/shared-control-prereq/native-default.log`.
- Native with test support: `cargo test -p boardstudio-web-ui-shared --lib --features test-support`
  **16/16 passed**; `.evidence/shared-control-prereq/native-test-support.log`. Both runs used the
  same exclusive target and implementation/test source pin above. Only this note changed during
  native verification. The contributor's earlier native default-configuration import failure was
  not reproduced by these scoped commands; these are not workspace-wide native results.
- `cargo clippy -p boardstudio-web-ui-shared --lib --tests --features test-support -- -D warnings`
  passed; `.evidence/shared-control-prereq/native-clippy.log`.
- The repository native-name guard reused the passing native log with `full=False, update=False`;
  the selected binary loses no baselined tests and reports five additions (including additions from
  the feature base). No baseline was rewritten. Its aggregate baseline-size message is not a claim
  that 2,104 tests executed here. `.evidence/shared-control-prereq/native-name-guard.log`.
- `python3 scripts/check-wasm-tests.py` passed: no new WASM-only plain `#[test]` functions;
  `.evidence/shared-control-prereq/wasm-test-guard.log`.

Pending: one combined Standards and Spec review through the coordinator, then approved local
feature landing. No workspace ticket is closed by this note. No full workspace, physical/native
desktop interaction or global acceptance claim is made.

Builds use the exclusive target
`/home/chris/.cache/boardstudio-target/shared-control-prereq-t3-b5fe12f0`, outside `.evidence/`.
The coordinator allocated the 017 slot to this slice while 017 continues independent source work;
the build phase has ended with no active build process, so the slot is ready to return with the
review handoff. Target cleanup waits for approved
landing and confirmation that no active owner remains. Evidence is preserved.
