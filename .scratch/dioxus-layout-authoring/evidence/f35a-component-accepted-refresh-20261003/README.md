# F3.5a accepted-refresh draft preservation

This private Inspector repair separates selected-component lifetime from accepted snapshot freshness. It keeps each numeric draft synchronized to its own accepted value, preserves a dirty constraint editor and the Relations tab through unrelated accepted revisions, and refreshes the event-time owner used for admission and edit base revision.

## Evidence

- Expected red against the original revision-key behavior: `old-revision-key-red.log`. The mounted regression fails because `context_generation` advances from 1 to 2 on an unrelated accepted revision.
- Green complete Layout component Inspector mounted/projection suite: `mounted-suite-green.log` (6 passed, 0 failed). It includes the final regression after the accepted-Y sync assertion was added.
- Formatting and whitespace checks: `cargo fmt --manifest-path web/Cargo.toml` and `git diff --check` passed.

The focused regression dirties X/Y position, part edge margin, and constraint offset X; selects Relations; then accepts a new Y value plus an unrelated document edit. It verifies X, margin, constraint, disclosure, and tab state are retained while Y synchronizes to its accepted value, then commits margin through the mounted blur/Enter path. The resulting `ReplaceDocument` uses the latest accepted revision and preserves both accepted changes.
