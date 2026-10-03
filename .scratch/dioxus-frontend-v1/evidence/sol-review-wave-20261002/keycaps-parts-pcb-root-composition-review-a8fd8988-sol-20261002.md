# Keycaps + Parts/PCB root composition review

Independent Sol6.1 High, both axes,2026-10-02. Exact root a8fd8988f649c70c11393066314095447535dce5 versus prior Parts/PCB root3d1fe8d7e1901606d076ec631c32687f9fd46142 and final author Keycaps9f192d7ccaaeffbc5c85165292c3646cafb144ec. No reviewer source edits or repeated heavy tests.

Standards: CLEAR. Spec: CLEAR for bounded integrated composition.

Final keycaps_fit.rs/keycaps_navigation.rs are byte-identical to cleared9f. Root CaseViewer differs from author only in earlier reviewed layout_preview:None wiring at its two CaseSharedViewer callsites; clear_layer_for_scope repair is retained exactly. Keycaps workspace adds finding/mechanical-layer ports and preserves earlier private Mirror/placement/chooser props. Its prior settings test shell receives only the new required empty/default ports. Root callback slots, accepted current Case layer projection, guarded finding admission/route, pending-fit hook and lifetime/RAF focus are mounted in Editor. Accepted snapshot epoch is explicitly preserved, and body route clears only a same-scope layer.

Parts/Objects/placement/Layout/shared viewer, PCB mode/controller/operation, Runtime and CSS files are unchanged from cleared3d1. In particular the old5fd pin helper conflict was resolved by retaining the existing reviewed Parts pin_inspector_on_desktop, is_compact_viewport and browse_parts_workspace behavior; Keycaps uses that desktop Inspector pin helper. Earlier Case13 settings admission and Layout viewer/Mirror composition remain intact. No public API/schema/task graph change or new geometry authority.

Independent frozen-source whitespace and byte comparisons pass. Current root combined page/core-worker all-target WASM Clippy completed successfully16.66s; retained log read/hash verified. Existing16-filter Chrome report and exact new shared-hook destination red/restored1/1 evidence retain their author9f/9f7 documentation bounds: three VirtualDom production-hook/lifetime cases invoke EventHandler directly with controlled ports, rather than full Editor/actual DOM clicks/real SVG geometry.

The independent final9f source report remains keycaps05-production-owner-review-9f192d7c-sol-20261002.md SHA59095b9c66d9c4dd0761385151d81cb788befed6f8507c86dd46adaee2eb2c02. Source clearance is limited to this integrated navigation owner. Original5fd/646 holds remain preserved historical reports, with newer source explicitly closing the required orchestration/active-owner regression.

Package/public gates stay open: source-stamped pointer/keyboard findings navigation, camera/Inspector/panel behavior, focused marker07, original missing fixture identity, save/reopen/history and F6C.4/INT.2/Case/all parent joins. Served34737 remains earlierb9e74 until the new package is built; this report does not qualify that server as currenta8fd. The intentional Dioxus body/layer accessibility correction remains disclosed rather than called literal React parity.

Retained strict log: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/parts-pcb-keycaps-root-join-clippy.log`, SHA-256 `8f85b883f97adb899b0e18607e810b031000604ad6e51d407d90c994a6b5d4cb`.

| Exact root leaf | SHA-256 |
| --- | --- |
| `web/src/presentation.rs` | `0178b1bfe5e21d596c8560f022ce9952863b93e18133d9c42d349eb415a18f41` |
| `web/src/presentation/keycaps_fit.rs` | `cc41fc7bc9bf79a731721ef59d857058d174fb92e2b986f3cc68bd568e268389` |
| `web/src/presentation/keycaps_navigation.rs` | `640ef013f23a78ca6cb1605c140b376f94fbeb8a6f25c2ee600e7c27e362a605` |
| `web/src/presentation/case_viewer.rs` | `0211c34075fb55ccd2726b9c3a36d6cf0f5d42d2adce086b1326021f389fd7c1` |