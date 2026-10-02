# Outline root integration bounded reack — 2026-10-02

Bounded Spec + Standards join review clear at c701233ea7bab6a4c794cba38cabf13be3a0b498, read-only in migration-m1-continuation-20261001. This reviews integration composition only; authored lifecycle/activation source relies on the separate independent Standards reviews, not self-approval here.

Compared the integration changes from adb8a442 through c701 with reviewed isolated sources. Root retains the MatrixInspector, Transform, Align and MatrixSetup hooks and adds Outline unconditionally in the same Editor scope, before projection-dependent rendering. Guide panel preference and existing shared owners remain mounted. Layout Inspector receives the optional Outline projection without losing the existing Matrix/Transform mounts. Duplicate context summary is suppressed only when the actual Outline Inspector is present.

The c701 compiler corrections preserve semantic boundaries: native Case part-picks carry no Outline action; Align context_matrix/make_edit explicitly return None for Outline/OutlineVersion/Bridge, which are not alignable matrix/part targets. No new fallback coercion, document authority, public visibility or suppression appears. Root immutable rendered activation payload and pre-selection currentness check match independently reviewed e1c27fc0.

No new code or tests authored for this small join. Reused root native/strict verification and independent17 mounted activation tests. Exact packaged paired behavior remains required; currently served34726 predates this Outline join.

SHA256:
- `web/src/presentation.rs`: `08b12411edb1d95713b708123a5acc40bd27b573191d206a864e12b436d87d07`
- `web/src/presentation/layout_workspace.rs`: `fc844aeb62f22357f3a265cb6edafd0f4cb6f0f57de001b92c38cc717adb4486`
- `web/src/presentation/case_viewer.rs`: `4cb2f09c48da19fc9b3deaf262a03b9781e8ce574f5b8e5b64cb8515a8643b45`
- `web/src/presentation/objects/layout_align_controller.rs`: `885205a0a892994fd4a7a705b0cf77da16ef6dfa306f54e5f8e31ef14eb25955`
