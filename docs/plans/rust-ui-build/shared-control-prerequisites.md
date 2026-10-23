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

## Closure review correction: dynamic attributes

The combined closure review of `34846a6cc37e11a092f89144d10cbbbc52a717d2` approved Standards with
zero findings. Spec requested two P2 corrections before landing:

1. Button's explicit id slot and raw fallback id slot could issue conflicting writes on an
   update from fallback id to owner id, removing the final DOM id.
2. Button and InputGroupInput completed spreads were not sorted. Dioxus 0.7.10's
   `VNode::diff_attributes` compares attribute **names** using a sorted-list merge; it can add a
   retained blur listener and then remove its old copy when a preceding focus listener disappears.
   That detaches the native nonbubbling blur listener. Attribute composition still matches both
   name and namespace; sorting must follow the framework's name comparator without discarding
   unrelated namespaced attributes.

`697204b4` adds mounted regressions before either production correction: native/link Button id
fallback/owner/default transitions with actual Tooltip focus descriptions; native/link Button
focus-listener removal retaining raw-before-owner blur; InputGroupInput focus-listener removal
retaining blur, unknown attributes, namespaced style and precise caller-owned value/input handling.
The tests use an explicit render barrier after native focus/blur events, with no sleep or missing
callback retry. Earlier review pins and receipts above remain unchanged.

The first transition batch at `697204b4320b6cabb03df8d20aea955359fab60f` failed during the listener
update with an uncaught `RawInterpreter.removeNonBubblingListener` error, `Cannot convert undefined
or null to object`. That poisoned the remaining mounted fixtures; their mount timeouts do not
establish independent REDs. `.evidence/shared-control-prereq/attribute-transitions-red.log` preserves
the complete trace. `90061acd` narrows the fixtures so only the focus/blur pair is out of name order;
unknown attribute removal and collisions remain covered.

Three fresh-browser runs at `90061acd`, with no production fix, each failed:

- `id-transition-red.log`: phase 1 expected `Identity native-owner-one`, observed **no DOM id**.
- `button-blur-transition-red.log`: removing focus from native/link Buttons reached the same
  nonbubbling-listener removal error before the update render barrier completed.
- `input-blur-transition-red.log`: the InputGroupInput update independently reached that same
  nonbubbling-listener removal error. These are executed runtime failures, not compile REDs; the
  callback-count assertions remain required after the update can complete.

These logs are under `.evidence/shared-control-prereq/`. Each isolated process ran its one named
regression and reported 0 passed, 1 failed, 78 filtered out. The sequence did not change production
source or retry a failing case until it passed.

Correction implementation/test pin: `15371a4acdc139d1292761059f1094ef1bf53031`, tree
`282039752f890d83a3b09d1031e08e428b7ade5d`. Button resolves owner-or-fallback id once, filters the
raw id, and renders that effective value through its single id slot in both branches. Button and
InputGroupInput sort their **completed** vectors by attribute name, exactly as Dioxus 0.7.10's diff
compares them; existing namespace-aware filtering/composition is preserved. No API, dependency or
shared helper framework was added for these corrections.

Correction GREEN checks and targeted Spec re-review remain pending. The coordinator reallocated
the 017 build slot for these checks; the target remains exclusive to this slice.

### Additional pinned-renderer blocker after sorting

The combined run at `15371a4a` is **not GREEN**:
`.evidence/shared-control-prereq/combined-wasm-ui-corrected.log` first fails removing focus listeners
with the same `removeNonBubblingListener` error; later mount failures are cascading, not independent
regressions. A fresh-browser run of only the sorted Button blur test also fails:
`.evidence/shared-control-prereq/button-blur-sorted-isolated-diagnostic.log`.

A native mutation regression now verifies that the sorted native/link Button update emits exactly
two focus removals (one per branch), and **no blur removal**. It passes:
`.evidence/shared-control-prereq/native-listener-mutations-two-branches.log`. Its first diagnostic
compile attempt incorrectly matched the owned `Mutation.name` String with a string-literal pattern;
that compile error is retained separately in `native-listener-mutations-diagnostic.log` and is not
behavior evidence. The corrected diagnostic passed before expanding to both branches.

The remaining failure is in pinned `dioxus-interpreter-js` 0.7.10:
`src/unified_bindings.rs:86-87` removes `data-dioxus-id` **before** calling `removeListener`.
`src/js/core.js` / `src/ts/core.ts` then reads that removed attribute to index the nonbubbling listener
table. Even the now-correct single focus-removal mutation therefore indexes the table with null and
throws. The native mutation guard and executed browser failure distinguish this from the original
unsorted-vector defect. No renderer dependency or private API has been changed.

Coordinator decision is required for a local workaround: retain previously used listener names as
`AttributeValue::None` slots when the caller removes the callback. That keeps native listener
bookkeeping stable while Dioxus dispatch stops calling the removed callback, retains the blur
callback, and keeps initially omitted/default events absent. It would require a small, scope-local
history of listener identities in Button and InputGroupInput, with the same public Dioxus path on
all targets and no new API/dependency. The alternative is a separately authorized pinned-renderer
correction. Neither additional change has been made.

## Approved renderer repair and combined correction

The coordinator rejected control-local history/None-slot workarounds and approved the bounded
repository-owned renderer repair in [the repair proposal](pinned-renderer-repair-proposal.md).
The completed correction receives **one combined Standards/Spec review**, explicitly covering the
renderer and both shared-contract P2 findings, before local feature landing.

- Published vendor baseline and committed Node regression:
  `9ee203f69eaf1ace39f5f5b5e1fa88133074251a`. All 31 published archive files were unchanged;
  upstream license texts, provenance and the regression are additions. No global Cargo cache was
  modified. The unchanged Node regression failed **1 passed, 4 failed** against published source;
  `.evidence/shared-control-prereq/vendor-listener-red.log`.
- Renderer repair: `d29dec86553a8cd532c4eb764c07a80e39ec0e2d`, tree
  `964d4115c141c529ea690b968875d7bdf1f9222b`. The hand delta is limited to
  `unified_bindings.rs` and `core.ts`; the existing bundler regenerated `core.js` and its input
  hash. Three unrelated Bun formatting-only outputs retain published bytes. The same Node
  regression now passes **5/5**; `.evidence/shared-control-prereq/vendor-listener-green.log`.
- The locked, offline production WASM check passed;
  `.evidence/shared-control-prereq/vendor-bundle-compile.log`.
- Combined mounted WASM `ui::` checks passed **21/21**, 58 filtered out, on the corrected shared
  source plus the renderer repair; `.evidence/shared-control-prereq/combined-wasm-ui-renderer-fixed.log`.
  This includes all three dynamic-attribute regressions and all earlier shared-control checks.
  The new id test covers eight fallback/owner/default/id-change phases on both native and link
  branches, actual Tooltip focus descriptions and exactly one owner focus callback per phase.
  Button and Input Group listener tests remove focus while retaining nonbubbling blur, verify
  callback counts/order and unknown-attribute removal/collisions, and preserve caller value/input.
- Native default `cargo test --locked --offline -p boardstudio-web-ui-shared --lib` passed **17/17**;
  `.evidence/shared-control-prereq/native-default-renderer-fixed.log`. The native mutation guard
  verifies exactly one focus-removal mutation per Button branch and no retained blur removal.

Native test-support also passed **17/17**, and native Clippy with warnings denied passed;
`.evidence/shared-control-prereq/native-test-support-renderer-fixed.log` and
`native-clippy-renderer-fixed.log`. The first WASM Clippy run rejected consecutive initial `push`
calls in the new Input Group fixture (`vec_init_then_push`); `2410f5eb` replaces them with the same
entries in the same order in `vec![]`, without an allow or assertion change. The failure remains in
`wasm-clippy-renderer-fixed.log` and is a lint failure, not a behavioral regression.

Final shared implementation/test pin: `2410f5ebaa4d9c3925d1b9f488950651a4701409`, tree
`ea6bdfde40cce0712a1849f6958d670476c30cc1`. Only documentation changes follow this source pin.

Final checks, all executed using the exclusive prerequisite target and the unchanged repository
browser environment:

| Check | Result | Evidence under `.evidence/shared-control-prereq/` |
| --- | --- | --- |
| Unchanged published-source Node regression after repair | 5/5 pass | `vendor-listener-green.log` |
| Shared `ui::` mounted WASM tests | 21/21 pass; 58 filtered | `combined-wasm-ui-final.log` |
| Native UI shared lib, default | 17/17 pass | `native-default-final.log` |
| Native UI shared lib, test-support | 17/17 pass | `native-test-support-final.log` |
| Native Clippy, lib/tests/test-support, warnings denied | pass | `native-clippy-final.log` |
| WASM Clippy, lib/tests, warnings denied | pass | `wasm-clippy-final.log` |
| Workspace rustfmt | pass | `format-final.log` |
| Authored documentation links | pass | `doc-links-final.log` |
| WASM-only plain-test guard | pass | `wasm-test-guard-final.log` |
| Native-name guard, reused native log, partial mode, no baseline rewrite | pass | `native-name-guard-final.log` |
| Root lock and bounded vendor delta | pass | `final-static-source-audit.log` |
| Locked/offline resolved dependency and compiled-feature audit | pass | `final-dependency-feature-audit.log` |

Cargo test and Clippy commands used `--locked --offline`; both targets were checked sequentially.
The authored-doc and WASM guards are source checks; they do not add runtime acceptance. The native
guard covers the selected binary, not every test in the repository's aggregate baseline.

The resolved interpreter is the excluded workspace-local vendor path at version 0.7.10, with only
`dioxus-web` 0.7.10 as a reverse edge. Its resolved and all original-registry/vendored WASM compiled
feature sets agree: `default`, `minimal_bindings`, `sledgehammer`, `webonly`. No binary-protocol or
serialize feature was added. Root lock contents equal the feature-base lock except removal of the
interpreter's registry source/checksum. `cargo metadata --locked --offline --format-version 1
--filter-platform wasm32-unknown-unknown` supplies the standard metadata receipt; no custom source
manifest was created. The vendor differs from its published baseline in exactly the two approved
hand-authored source files, generated core JS and existing input hash; the Node regression is
unchanged between its RED and GREEN.

The build phase is complete; the allocated slot is returned with the correction-review handoff.
One completed-correction Standards/Spec review and approved local feature landing remain pending.
Global acceptance and workspace ticket claims remain unchanged. Target cleanup still waits for
approved landing and no active owner; evidence is preserved.

Builds use the exclusive target
`/home/chris/.cache/boardstudio-target/shared-control-prereq-t3-b5fe12f0`, outside `.evidence/`.
The coordinator allocated the 017 slot to this slice while 017 continues independent source work;
the build phase has ended with no active build process, so the slot is ready to return with the
review handoff. Target cleanup waits for approved
landing and confirmation that no active owner remains. Evidence is preserved.
