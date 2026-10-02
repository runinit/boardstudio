# Release key / draft-owner audit

Read-only source audit of integration candidate `14d1bfeb` and current checkout `014febc1579edd52da12f0e2fe27c77c30a0e25b`; no source/build artifacts changed. Trigger: seven release unused-key-variable warnings in `web/target/builds/frontend-layout-align-wave-20261002/page-root.log`. Installed framework source: dioxus-rsx 0.7.10 under `/home/chris/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

## Actual macro semantics

`template_body.rs:107–109,189,279–280` computes the release VNode key exclusively from the first root of that rsx template. `component.rs:260–264` excludes key from ordinary component props; component token generation at 90–112 constructs a VComponent without an independent key. Thus a key on a component nested under a static section/div is not a separate keyed VNode boundary. `assign_dyn_ids.rs:62–79` nevertheless collects formatted key text in debug mode; the debug dynamic-literal pool at template_body.rs:183–186 evaluates it, masking the unused-variable warning. `hot_reload_mapping` also selects only the template's implicit root key. These nested keys are ineffective in debug as well as release; debug compilation does not establish identity behavior.

## P1 — Matrix fields can preserve a draft under a new owner

`web/src/presentation/objects/matrix_inspector.rs:112–160` puts all five owner keys on nested MatrixFieldEditor components inside one static section template. The parent call in `layout_workspace.rs:82` has no owner key. MatrixFieldEditor initializes draft/baseline/status with use_signal; its accepted-value effect at 230 depends on value/baseline/feedback, not owner identity. When matrix A and B have equal field baselines, switching A→B reuses those child scopes and leaves A's dirty draft intact. The new commit closure captures B's owner/token/revision and may submit that old draft as an edit for B. Different baselines can leave the new owner showing stale/error state instead of resetting. This is an actual ownership gap supported by the expansion and caller chain; the release warnings must not be silenced as cosmetic.

Required bounded repair: give each field a real owner-keyed template root (or use an explicit production owner-transition reset) while preserving drafts across unrelated accepted revisions. Add a mounted production-field owner-switch regression with equal values and a dirty draft; demonstrate red under the prior nested-key form and green under the corrected boundary. Release browser acceptance must switch between real matrices and confirm target/draft isolation. This audit does not claim that browser interaction has already been executed.

## Encoder warning disposition

`keymap/encoder_editor.rs:61–62` nested clockwise/counterclockwise BindingEditor keys are also ineffective. However `keymap_workspace.rs:138–151` places EncoderEditor itself at the root of a separate keyed rsx template containing scope/layer/projection generation/editor-instance identity, and `binding_editor.rs:535–568` places the actual stateful KeycodeField at the root of its own keyed template containing scope/editor/layer/field/target/accepted value. Those effective outer and draft boundaries protect the identified owner/value transitions. No encoder draft-retargeting defect established by this audit. Remove redundant keys only after preserving/proving those actual boundaries; do not merely rename key variables or suppress warnings.

SHA256: matrix_inspector.rs `1ecf91c95a121036aa564e29f995242108e2bea6abd47b01533318851337c130`; encoder_editor.rs `b32e33cdfedb2b8903323285e35d603a13eed999efb4a1b72001f9f3f62385d4`.

No new checks/config/allowances introduced. RF handoff: existing RF-001/RF-006/RF-009 scoped draft/lifecycle/evidence records apply; debug strict success did not cover release expansion or mounted identity. No new RF identity proposed. Root source freeze remains respected.
