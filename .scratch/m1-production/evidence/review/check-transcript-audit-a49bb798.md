# Check transcript audit — active-instance candidate

This records what is and is not retained for the focused candidate checks. It
does not rerun any check or turn transient results into durable raw logs.

## Retained exact command records

The latest pre-feature full source-check ledger is
`.scratch/m1-production/evidence/integration/final-source-checks/record.json`.
It records exact argv, cwd, timestamps, exit codes and logs `01.log`–`07.log`
for application/web tests, application/web fmt, application strict Clippy and
web page/CAD-worker WASM Clippy. Its tested source is `2040e23ba2cd97aef3757d24b81fe2463d6f86be`;
it predates active-project restoration/instance navigation and therefore cannot
substantiate checks on those changes. At that source, the application integration
test log records 13 durable-session tests; the web log records 12 tests.

`.scratch/m1-production/evidence/integration/focused-checks.json` and
`application-tests.log` provide an older exact command/check record at
`97a779a60f06f6ee109ecace17d06fa28e625eee`: `cargo test --manifest-path
application/Cargo.toml --locked`, exit 0, 12 durable-session tests. This also
predates the active-instance addition and is not evidence for its regression.

## Focused-candidate results visible in the task transcript only

The active-instance handoff reported these exit-zero checks for source
`b50ddbdd6b74a2489e80078e4a0d2923aeca6edb`, with working directory
`/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`:

- application tests: 14 passed;
- web tests: 12 passed;
- fmt checks for application and web;
- strict all-target application Clippy;
- strict WASM page Clippy;
- focused release Dioxus page build.

The exact tool calls/output are present in the originating task's transient
tool transcript, but no raw native/fmt/Clippy log, argv-and-time ledger, or
check-specific hash record for this focused candidate was found in the retained
worktrees. The precise timestamps are unavailable. This note preserves the
reported outcome without inventing stdout, timestamps, or command spellings.
The focused Dioxus argv and page asset hashes are retained in
`startup-restore-instance-focused-b50ddbdd.json`; they do not substitute for the
missing native/fmt/Clippy command ledger.

The browser QA record is retained separately at
`startup-restore-instance-focused-b50ddbdd-qa/record.json`. The new application
regression is present in `application/tests/durable_session.rs` as
`instance_navigation_updates_scope_and_cancels_in_flight_case_work`.
