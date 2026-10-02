# Parts issue 14 start-capability receipt — 2026-10-02

This receipt proves only the bounded issue 14 start edge. It does not assert F4.1/F4.5 completion, async adapter availability for issues 15/16, or public browser acceptance.

## Reviewed contract

- Contextual spec: `.scratch/dioxus-parts-catalogue/drafts/contextual-workflows-spec.md`, SHA-256 `f739b30e6e4fb5aa743ccfd0be3b069a81abd701d96297857a0251bfa48f9884`.
- Issue 14: `.scratch/dioxus-parts-catalogue/drafts/14-edit-manual-fit-profile.md`, SHA-256 `ac1a6bad76207504be0a90f32da894f25b84acf229ca24bc09d9f0ad605a5c41`.
- Independent BOTH-axis planning review: `/home/chris/.local/share/boardstudio/reviews/parts14-16-planning-review-sol-20261002.md`, SHA-256 `01f126076a45dfc5bc5dd228360b93fc0ff90c5395a2046c4e38924e0e2b9c64`.
- Source base: integrated commit `2394b9961a5f7d2dd814f1637cf611185d4449d6`.

## Callable current capabilities

- The shared canvas router mounts the Parts-specific center leaf (`web/src/presentation/workspace_composition.rs:93–100`). The Parts leaf forwards accepted snapshot, selection scope, query and selected definition into `PartsPreviewWorkspace` (`web/src/presentation/parts_workspace.rs:56–64`).
- `PartsPreviewWorkspace` resolves the selected catalogue/project entry from those inputs (`web/src/presentation/parts.rs:257–284`) and mounts the source-backed Parts center preview. Issue 14 can own the replacement of that center content while retaining its accepted selection source; the right Inspector is not its control location.
- `Runtime::operation()` and `Runtime::submit(Event::Edit)` are available to the private web presentation. `Runtime::submit` submits to the existing Session, updates the read model and drives Core effects (`web/src/runtime.rs:631–656`). Normal Session/Core history and durability remain the authority.
- Frozen source hashes at this base: workspace composition `e1889ecbebfc0d1271ad3b908061bb47ecc8acdb78b2d14b1e3eaa33eb70166d`; Parts workspace `b86ef24250d069eb912e3bfb2e910b1a9d3bea101a59d059cb949559ed37f87d`; Parts projection `7c017d0db047344c7178abc86212b24bfb42b2fb26adb99be24139c9f8285831`; Runtime `908b77738b1f4e4613adfdde2fe8daa3b1f1b86309fd871b4d2200f708c5c120`.

No issue 12 dependency or Core profile adapter is needed to edit the manual draft. Issue 14 consumes selected definitions supplied by the current Parts projection, including bundled-only definitions; exact current-snapshot replacement/materialization is implemented inside its private Parts save path. Issues 15/16 remain blocked on their separate, coordinator-owned callable Core and Artifact adapters.
