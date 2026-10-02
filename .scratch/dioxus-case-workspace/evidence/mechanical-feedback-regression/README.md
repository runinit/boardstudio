# Mechanical feedback regression

The production feedback policy is extracted into a private pure module and used by the mounted Case settings. Applying the retained old-policy patch produced three expected assertion failures: terminal failures disappeared after snapshot-token advancement and unrelated edits. Reversing that patch restores all four policy regressions; the full native suite passes 34 tests.

Native and WASM page Clippy pass with warnings denied, and formatting passes. This proves the bounded policy and compiler checks; public UI interaction and parent acceptance remain open. The patch is retained as evidence and is not applied. Exact production file hashes are recorded in source-hashes.json. The mounted source is committed as `0cad7577`; independent Spec and Standards reports are retained alongside the logs.
