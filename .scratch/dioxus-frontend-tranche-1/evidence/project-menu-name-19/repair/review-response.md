# Child 19 source-review correction

Reviewed source before correction: `ffaaf8382f1addb0923524f9ee14ab40c308fe8a`.

Independent Sol 6.1 High report: [project-menu-name19-source-review-ffaaf838-sol-20261002.md](/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/sol-review-wave-20261002/project-menu-name19-source-review-ffaaf838-sol-20261002.md), SHA-256 `3e5f768a4b6633e1a091faec399a058750a3dc1e4fc5723e4112f16187b749db`.

The report raised two bounded issues:

1. The current-name control had a separate heading and visible `Project name` caption, unlike React's inline `Current project` label and input. Source commit `ff3b0848b4d36019eb73b8c075826e6284b11f74` changes the markup and desktop/compact row styles to the pinned React structure. The production-mounted test asserts the inline label text and class.
2. The mounted regression intercepted before Runtime/Session durability and manually declared every save committed, leaving failure, durable reload and delayed completion ownership untested. The new test-only fixture injects a `CoreEngine`, runs real Session effects through `Runtime::run`, uses the browser store for successful saves, and controls only the save result/timing when testing failure and owner replacement. The mounted tests prove an accepted unrelated revision survives trim/blur, stored data reloads with both rename and unrelated data, Undo/Redo and Enter/Escape behavior, same-ID/same-name epoch reset, failed persistence leaves the accepted name unchanged and reports the failure, and a delayed completion after Session replacement cannot settle the old observed operation or alter the new project's accepted name.

The first harness attempt exposed that Runtime's normal startup restoration can race an injected Session when a browser test database already contains an active project. The test-only `new_runtime` constructor skips only that startup restoration; production continues to call `Runtime::new()` with restoration enabled. This is fixture isolation, not a second persistence owner or a newly found product defect. Keep existing RF-001/RF-006/RF-009/RF-014 records and their status.

Current source commit: `ff3b0848b4d36019eb73b8c075826e6284b11f74`. The source needs fresh independent re-review; public package and parent acceptance remain open.
