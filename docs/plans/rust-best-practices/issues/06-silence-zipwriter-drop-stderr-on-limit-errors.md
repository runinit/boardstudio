# 06: Prevent ZipWriter drop stderr output from corrupting parallel test receipts

Status: ready-for-agent
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
