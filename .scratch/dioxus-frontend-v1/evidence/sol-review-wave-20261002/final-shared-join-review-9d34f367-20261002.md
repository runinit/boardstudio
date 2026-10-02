# Final bounded shared-source join review — Sol 6.1 High

**Standards CLEAR. Spec CLEAR for the bounded source composition** ending at `9d34f3672f4f05cb3771bc5cd358508b13e9c31f`, compared with the previously cleared shared join `6189fd8b88cf9892c1ce1a3004861ab6f5d2c60b`. Integration/source freeze and package qualification may proceed. This receipt does not accept the public build, individual tickets or their parent joins.

Personally reviewed immutable Git source along both axes, applicable AGENTS/current CONSTRAINTS and the existing independent leaf receipts. No app/root source edits, frozen worktree mutations, delegation or heavy builds. Root tracked source paths were clean at final inspection; unrelated scratch/evidence work remains untouched.

## Exact preservation

Independently compared full Git blobs with their cleared author packets:

| Packet | Files checked | Result |
| --- | --- | --- |
| Parts13 `f19dbffd7aab879eac853f18a2422acf6f4d36fe` | `parts_new_component.rs`, `parts_view_generation.rs` | Both byte-identical |
| Parts14 `bca0955ae3a2f61d8e679c252f229827e5d35cb8` | `parts/mechanical_profile.rs`, `parts/mechanical_profile_ui.rs` | Both byte-identical |
| Keycaps `4f05ca9267439f86dfc6217a8df6dd4de2f6da80` | `objects/keycap_resize.rs`, `keycap_size.rs`, `keycap_size_controller.rs`, `keycap_size_tests.rs`, application `durable_session.rs` | All five byte-identical |
| Canvas arbiter `8b75118dbff35fd55a9685b4179e906c9775a042` | `canvas_interaction.rs`, `part_placement.rs`, `objects/mirrored_pair_controller.rs` | All three byte-identical |
| Case16 `9a0aa99209a8c535bea639dd40bac725ef9ea724` | `shared_viewer.rs` | Byte-identical |
| Packaging `87bf9a02578c13d1555c336186f50a20b2110d49` | `scripts/build-m1.py`, `scripts/test-build-m1-reuse.py` | Both byte-identical |

Packaging helper SHA-256 is `d382c2fef137fad05f71d1d45864849fa4e3fb5c3cb4c096c9d32df6a7594e46`; reuse tests `487e4e23bec61764885eff78f161bd205290918a8ba82835514f634233475ca9`. These exactly match the independent final packaging receipt. They preserve root config/source inventory guards, safe CLI handling, provider-prefix equality, guarded page-leaf registration/token parsing and the full/reuse command paths. A baseline built with an older helper is not qualified for reuse under these new helper bytes.

Final Case layer leaf equals author `9a0aa992` after replacing precisely the production unavailable string and its assertion from `No generated geometry` to `Not generated`. Final SHA-256 is `c2eb6b0f92d38647efd3a1d6f3f53247ef0b350f2efe9dabdcf0e5ba0c1bb70f`. The `1acf81eb888da04f52ace765e268225461f7b43b`→final source diff contains only those two replacements.

## Shared composition

The Parts library mounts exactly one production `PartsViewGenerationOwner` outside catalogue success/loading/error branches and supplies the actual workspace and SelectionAdapter scope-generation signals to exactly one create action. Runtime subscription still increments generation synchronously whenever observed Scope changes. The accepted project-definition fallback remains available during unrelated catalogue loading/error, while their status/alert stays visible. Inspector Name and controller placement target/action/busy/error fields are retained together. Parts14 center editor and its exact operation/source/view correlation leaves remain unchanged; no second editor, outcome observer or callback is introduced by this join.

Keycaps retains one Editor `use_key_size` hook, passed into the Layout Inspector and mounted between matrix and transform sections only when its projection exists. Accepted selected-ID authority, delayed-owner validation, document-order drafts and warning semantics remain in the exact reviewed private files. Native-only test registration is separated from production WASM presentation registration. The new CSS remains confined to the reviewed Key size and Case classes, plus the toolbar close-focus correction to existing `--wb-accent`.

The arbiter's Editor added/removed source lines are exactly equal to the reviewed `8b75118d` patch. Root conflict resolution preserves existing Case module registration while adding the private canvas module; it introduces no additional lease or owner mount. Mirror and Part hooks receive the same private synchronous arbiter. Keyboard, pointer move/up/cancel/start and hit callbacks retain action-time lease checks and block ordinary selection/pan fallback while form, preparation, ghost or commit owns the canvas. Mirror and Part admission/settlement/lifetime interfaces remain the exact reviewed owner implementations.

The accepted Mirror-created callback still validates current Layout, scope generation, full Scope, accepted token/revision, lifecycle, matching Saved durability and result entity IDs before selection and Inspector publication. It retains `pin_inspector_on_desktop(inspector_panel_settings)` inside that guarded branch. Desktop pinning still checks the action-time compact media query. Toolbar close/header/transform and matrix Inspector leaves are unchanged from `3a2d6697`; existing guarded navigation and Properties pinning routes are not replaced by the join.

Case receives one layer-menu mount inside the existing canvas shell using the same guarded display callback; the original pointer/wheel/host routes remain mounted once. Exact shared-viewer source preserves source Scope/token qualification for physical preview/model rows, configured stack union, native/CAD source separation and ViewerOwner/current-source/current-Runtime checks. Layer changes remain transient display preferences. PCB07/08 private wiring, input settings/owner and connections source are unchanged since `94480901`; PCB workspace's only later delta supplies absent Mirror fields to the shared tree. The unique contextual heading, Press→details/connections→Ergogen order and existing lifetime/live-workspace guards are preserved.

## Verification and limits

Executed full immutable blob comparisons, inspected the shared presentation/main/Parts/workspace/Case/CSS/config diffs, compared the arbiter Editor patch, checked final source status and ran `git diff --check 6189fd8b..9d34f367 -- web application scripts web/assets`: passed. Reused sufficient fresh independent leaf verification instead of rerunning identical tests. In particular the arbiter receipt independently ran actual Chrome PartPlacement 23 and owner 1; Mirror lease ownership was held directly in those tests, not through a mounted Mirror hook. The Parts13 receipt independently ran the actual production generation-owner VirtualDom test and five focused tests. Parts14, Keycaps, Case and final packaging retain their precise independent evidence and limitations in their linked receipts.

Coordinator supported mixed page/Core-worker WASM all-target checks were running when this review completed; this report does not assert their eventual result. Exact-candidate coordinator checks and the next package remain required. The author-reported new Part admission mutant red is not independently qualified here because its retained path had not arrived; preserve that provenance distinction.

The prior nonblocking Case P3 status text is corrected to React's `Not generated`. Its unavailable button title still uses reason-only text rather than React's `<label> — <reason>`; the existing paired visible-copy qualification remains open. This is not a newly introduced join defect and is not silently counted as complete parity.

Actual full Editor Mirror/Controller arbitration, accepted placement, stale browser callbacks, rendered Case models/visibility, Parts create/profile persistence, Keycaps resize/selection/warnings, Undo/Redo, same-archive React/Dioxus root/subpath/offline journeys, save/reopen, themes/compact/focus, and ticket/parent acceptance remain open. Missing original linked-Core fixture evidence is not recovered by this join. Fresh full baseline/provider reuse under the exact final helper remains a separate packaging qualification. Retain existing RF records; no new refactoring finding is introduced.

## Receipts reused

- `canvas-arbiter-source-review-8b75118d-20261002.md`
- `parts13-effect-correction-review-f19dbffd-20261002.md`
- `parts14-correction-source-review-bca0955a-20261002.md`
- `/home/chris/.local/share/boardstudio/reviews/keycaps-root-composition-review-sol-20261002.md`
- `/home/chris/.local/share/boardstudio/reviews/case16-final-stack-label-source-review-sol-20261002.md`
- `packaging-final-source-review-87bf9a02-20261002.md`
- `controller-parts14-shared-join-review-6189fd8b-20261002.md`
- `pcb07-08-integrated-correction-review-94480901-20261002.md`
- `layout-toolbar-desktop-pin-review-3a2d6697-20261002.md`
