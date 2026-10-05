# Required aggregate gate ownership repair

Actual combined gate2801352 exited1 before commit. The retained aggregate-profile-and-owner-red.log records zero matching WASM modules for selection.rs and renderer_host.rs, in addition to two grouped profile fixture failures. Neither unmatched file has tests under its own binary module name.

The selected-source owner map now binds this exact selection.rs base/current revision to the three existing mounted selection adapter tests: same-matrix additive selection, mixed component selection, and Key-mode toggle removal with accepted-command targeting. It binds the exact renderer_host.rs test-only mirror revision to all six existing renderer_host_page lifecycle tests. The unchanged native source-sync guard enforces exact normalized library/page equality. Source/base hash mismatch disables either mapping and fails closed. No test is removed or excluded; selected direct modules and owner tests deduplicate through the existing runner.

Executed `python3 scripts/test-run-wasm-tests.py`:37tests, OK (terminal2026-10-05). This validates existing owner-map, hash-guard, inventory and result rules; the next actual aggregate run must additionally prove these source mappings resolve to executed listed tests. Independent reviewer is checking the precise mapping delta. No package/public result is implied.
