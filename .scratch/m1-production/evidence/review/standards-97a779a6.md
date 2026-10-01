# Standards review — `97a779a6`

Pinned comparison: `cd1efb34b2e9b316ab1f54fd66c5410d8763d559...97a779a60f06f6ee109ecace17d06fa28e625eee`.

## Actionable hard breaches

None found in the requested authored production scope. The new application crate follows the accepted headless session/interaction ownership; the web package composes host adapters and UI; the two permitted contract payload boxings preserve serialized form. I found no additional documented API/visibility expansion to core or contracts in the reviewed slice. No `.scratch/**/CONSTRAINTS.md` exists; root `CONSTRAINTS.md`, `docs/agents/domain.md`, `CONTEXT.md`, `docs/architecture.md`, ADR 0003, and the M1 plan/spec/host inventory were used.

## Judgment-call smells

- **Broad public module surface:** [web/src/lib.rs](/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/src/lib.rs:9) publicly exports host, CAD, offline, renderer and case-settings implementation modules. The production binary consumes several through the crate boundary, and the example consumes case settings, so the visibility has a concrete reason; still, this makes implementation details part of the new `boardstudio-web` library API. Keep this surface deliberate or narrow it if the binary/example can be organized without exporting more than intended. This is not an inherited API change and the package is `publish = false`.
- **Long coordinator module:** [web/src/runtime.rs](/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/src/runtime.rs:50) is a 948-line owner spanning session effects, CAD, persistence, fixture opening, import/recovery and export. The ownership is consistent with the accepted single root runtime, but this is the likeliest place for unrelated changes to accumulate; extract only when a second clear lifecycle owner exists.

No source changes or builds/tests were performed. Vendor/provider content matching pinned `dev5a472a...` was treated as inherited, and the untracked integration evidence was left untouched.
