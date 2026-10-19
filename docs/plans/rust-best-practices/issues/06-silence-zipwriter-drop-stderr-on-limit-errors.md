# 06: Prevent ZipWriter drop stderr output from corrupting parallel test receipts

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** When archive packing fails because `BoundedWriter` hits its byte limit, dropping the unfinished `ZipWriter` no longer prints `ZipWriter::drop: failed to finalize archive` directly to process stderr. Preserve the size-limit error and memory bound while removing a source of interleaved `libtest` output that can corrupt verification receipts.

## Research decision (2026-10-19)

Keep: the locked zip 9.0.1 implementation retries fallible finalization in Drop and writes failures directly to process stderr, including after consumed `finish` fails. Source confirms the path; this research did not produce a fresh failing receipt. See [research 06](../../../research/rust-best-practices-ticket-validation.md#06--zipwriter-drop-and-receipt-integrity).

Prefer a local bounded-sink teardown fix. A candidate is sticky exhaustion: return the original size error, then discard teardown writes without growing the backing buffer, maintaining coherent logical Write/Seek positions. Prove that no failure returns a partial archive as success. Do not remove the bound, leak the writer, redirect process-global stderr, sanitize test output, or upgrade zip solely to suppress the message.

## Acceptance criteria

- [ ] Over-limit archive packing still returns the expected size-limit error without writing `ZipWriter::drop: failed to finalize archive` to stderr when `ZipWriter` is dropped.
- [ ] Child-process stderr regression tests cover limit failures during header/start, compressed data/entry close and central-directory finish. Each returns the size-limit error, no archive and no Drop diagnostic; the backing buffer never grows past the configured cap, including teardown writes.
- [ ] Successful archive roundtrip and shared-asset controls stay green. `cargo test -p boardstudio_core --locked` produces zero ZipWriter Drop diagnostics.
- [ ] `python3 scripts/check.py test --receipt .evidence/rust-best-practices-06.json` records complete terminal observations for invoked tests, including Core, with no ZipWriter Drop diagnostics. Cargo alone does not create a BoardStudio receipt; do not weaken receipt completeness checks.

## Answer

Resolved on `t3/implement-rust-best-practices` (base `c071dedcf`).

- `archive::BoundedWriter` now has sticky exhaustion: the first over-limit write returns the
  original size error, later teardown writes are discarded while advancing a coherent logical
  position, and the backing buffer never grows past the cap. `pack` returns the size error if
  `finish` "succeeds" against the exhausted sink, so no partial archive is ever returned.
- Child-process tests (`archive::tests::limit_failures_do_not_write_zipwriter_drop_diagnostics`)
  cover a limit reached at the header, compressed data/entry close and central-directory
  finish. Each asserts the child succeeds only with the size error, returns no archive, and
  writes no `ZipWriter::drop` diagnostic. A unit test asserts sink buffer length and
  Write/Seek coherence after exhaustion. Successful archive roundtrip/shared-asset controls
  remain green.
- `cargo test -p boardstudio_core --locked` and
  `python3 scripts/check.py test --receipt .evidence/rust-best-practices-06.json` produced no
  ZipWriter Drop diagnostics; the final receipt records complete observations for every
  invoked scope, including Core (794 passed), with zero failures/incomplete records.

Evidence: `.evidence/rust-best-practices-06.json` and `.json.md`; `check.py test` exit 0.
