# Failed-export cleanup evidence

This evidence covers review finding P2 from `evidence/review/spec-97a779a6.md`.

The regression test exercises public `Session` events/completions and verifies both that `export_is_current` becomes false after `Completion::ExportFailed` and that a later reopen does not emit `CancelExport` for the settled operation. The test was added in commit `9b865587` together with the matching export-registration cleanup. Before that source change, the test failed at the first `export_is_current` assertion because the failed operation remained registered.

The original red test output was observed in the tool history, not retained as a raw log file. `red-observation.md` preserves the command and the exact assertion/failure excerpt that was observed; the original tool wrapper did not retain the numeric shell exit code, so it is not inferred here.

Fresh checks were run after merging integration commit `4cd72b223bc60a3e326dc695f654f243ede5d433`, whose history includes the fix. Their complete test output is in `application-tests.log`; formatter and Clippy are silent on success, with captured exit status recorded in `check-results.json`.

The application session suite now has 13 integration tests, up from 12 before this regression was added.
