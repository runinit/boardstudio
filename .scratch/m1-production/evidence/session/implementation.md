# Ticket 02 headless session implementation evidence

Candidate branch: `codex/m1-session-worker-20261001`, created from integration `e934c013`. Implementation is isolated to `application/` and this evidence directory. It does not update root RUN/TODO or coordinator status.

## TDD evidence

- Initial public-session test compile failed as expected because `boardstudio_application::{Session, Event, Completion, Effect, ...}` did not exist yet (`error[E0432]: unresolved imports`). This established the missing seam before implementation.
- The session test `part_drag_uses_gap_geometry_snap_at_the_two_millimeter_tolerance` then failed on the expected observable mismatch: the submitted Core preview did not contain the React reference's snapped `19.0 mm` result from a 19.6 mm candidate with 18 mm envelopes and 1 mm gap. After porting the existing envelope landmark and rectangular-gap policy to `application::interactions`, the same test passed.
- Each completed behavior is exercised through public `Session::submit` / `Session::complete`; core open/edit/preview/undo behavior uses the real `CoreEngine`, with persistence and asynchronous ordering controlled at the effect boundary.

## Implemented headless behavior

- Ordered open/edit/Undo/Redo, save-before-publication, exact pending reply retention and save-only retries with new `SaveAttemptId`.
- Engine request/reply IDs and executor epochs; expected variant checks; uncertain core failure blocks queued work, emits worker restart, and only resumes after explicit durable-document reopen.
- Session epochs and accepted-snapshot tokens fence equal-ID/equal-revision reopen, generation completion and artifact delivery. Board/instance navigation cancels scoped work. Generation/export intents wait behind ordered document work.
- Captured part gestures with pointer ownership, frame coalescing, one final commit, cancellation, grid/Alt behavior and the existing 2 mm physical envelope landmark / rectangular gap snap policy. One-undo verification passes.
- Logical part selection (replace/add/toggle/range), board/instance navigation and 2D center/zoom camera updates are session-only.
- Asset ID/hash/media references accompany persistence effects; original bytes and atomic IndexedDB semantics remain the host's responsibility.

## Checks run

- `cargo test --manifest-path application/Cargo.toml --locked --test durable_session`: 8 passed.
- `cargo clippy --manifest-path application/Cargo.toml --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --manifest-path application/Cargo.toml --check`: passed after merging integration tip `57594008`.
- `git diff --check 57594008..HEAD`: passed after the merge.
- Integration tip `57594008` was merged into the worker branch; no conflicts.

## Limits

Headless tests do not prove Chromium workers, actual IndexedDB transaction abort/retry or asset-byte atomicity, archive import/export, offline behavior, Dioxus controls/a11y/responsive behavior, root/subpath deployment, exact CAD/case parity, renderer lifecycle, STEP packaging/delivery or React comparison. Those remain integration and browser acceptance gates. Selection-range UI mapping, multi-board visible-part filtering for geometric snapping, screen coordinate conversion, pan interaction, job executor epochs and export-owned accepted-edit adoption need downstream host/domain integration or further headless slices. Do not close ticket 02 or M1 on this evidence alone.
