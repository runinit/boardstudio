---
status: accepted
---

# Resolve queued edits against the accepted document when they run

Presentation built every edit's payload from the accepted document it saw when
the user committed, and Session only refreshed the queued command's base revision
before execution. A second edit queued behind the first therefore restored the
values the first had changed (the X/Y overwrite reproduced in the
[layout part-editing investigation](../investigations/layout-part-editing.md)).
On 2026-10-06 the user chose to resolve each pending edit's intent against the
accepted document immediately before it runs, so rapid edits each apply on top of
the edits accepted before them.

A pending edit whose target no longer exists is retired with an explanation rather
than applied. While an edit is pending its field keeps the draft value; when the
edit fails or is retired, the field shows the accepted value again with an inline
message, and nothing retries automatically. Gesture commits and export-owned
commits keep their strict captured-revision checks.

## Considered Options

- **Reject stale edits** (strict base revision): simple, but rapid entry across
  fields loses every edit after the first and asks the user to re-enter it.
- **Serialize at each form** (no new commit while one is pending): prevents the
  overwrite within one form only; different panels still race.
