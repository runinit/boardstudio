# 06: Prevent ZipWriter drop stderr output from corrupting parallel test receipts

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** When archive packing fails because `BoundedWriter` hits its byte limit, dropping the unfinished `ZipWriter` no longer triggers a second failing central-directory write that prints `ZipWriter::drop: failed to finalize archive` directly to process stderr—preventing interleaved stderr output from corrupting parallel `libtest` result lines and causing verification receipts to report `boardstudio_core` as `INCOMPLETE`.

## Acceptance criteria

- [ ] Over-limit archive packing still returns the expected size-limit error without writing `ZipWriter::drop: failed to finalize archive` to stderr when `ZipWriter` is dropped.
- [ ] Running `cargo test -p boardstudio_core --locked` produces zero `ZipWriter::drop` messages on stderr and a complete verification receipt with all invoked tests reaching a terminal outcome.
