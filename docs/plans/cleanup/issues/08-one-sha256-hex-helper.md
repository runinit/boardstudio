# 08: Use one SHA-256 hex helper for asset identity

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Every stored, archived or delivered asset is identified by the lowercase hex
SHA-256 of its bytes. Those strings are compared across browser storage, portable archives, model
import, routed-board references, CAD delivery and mechanical extraction. So every place that
computes one must produce the same text. Today about nine private copies of the same "digest, then
format each byte as `{:02x}`" helper exist in Core and the web crates, beside the host's public
`sha256_bytes`. One comparison already differs: model-byte verification ignores case, while
storage and archive import compare exactly.

Add one public helper in Core (every affected crate already depends on Core), make the host's
`sha256_bytes` call it or replace it, and delete the private copies. Make model-byte verification
compare exactly, like storage and archive import.

While touching model file import: it currently hashes the file, then calls model-byte verification
on a full clone of the bytes only to compare that hash against itself. The size check it repeats
has already run. Remove that redundant call, so a large import costs one hash pass and no extra copy.

Leave alone the call sites that need the raw digest bytes rather than hex text (for example, a
digest used as an array key).

## Acceptance criteria

- [x] One public lowercase-hex SHA-256 helper exists in Core. A whole-tree search finds no other `{:02x}` SHA-256 hex formatter in non-test production code; raw-digest uses are left as they are.
- [x] Model-byte verification compares hashes exactly; a test shows an uppercase expected hash is rejected, the same way storage and archive import reject it.
- [x] Model file import hashes the bytes once and does not clone them for a self-check.
- [x] A Core unit test pins the helper to a known vector (for example, the empty input's SHA-256).
- [x] `python3 scripts/check.py quick` passes; it falls back to every Rust check if a shared manifest changed.

## Answer

Commits `a35ff28da`, `a26264d4c` and `bd9b097f7` put lowercase SHA-256 text generation in `core::hash::sha256_hex`, remove the private production copies, retain raw-digest uses and make model-byte checks exact. Model file import no longer rehashes or clones for a self-check; BrowserStore remains the persistence verification seam. The Core known-vector test, uppercase-rejection test and WASM browser model-file tests were added, with the native count baseline updated. Post-review `python3 scripts/check.py quick` passed all seven steps (519.9 s), including native and browser tests. Two-axis review found the remaining mechanical-extraction formatter, case-insensitive loader check and import-store self-check; all three were corrected and reverified. A cold wasm-only build may now compile Core for the host because Runtime's build script uses the shared helper; no comparative build-time measurement was made. A subsequent `repo` check in the dirty `dev` checkout failed on seven unrelated links to its already-deleted `CONTEXT.md`; no Cleanup 07/08 link was reported.
