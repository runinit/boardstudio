# PCB wiring mode resolver transition follow-up

This packet adds executable production-path evidence requested after commit `86619b21c44aee6fe97fdffbd3b02848c1412197`. The Editor resolver controller now calls the shared `begin_resolution` transition before replacing its state with Pending; the `Runtime::resolve_electrical_preview` method calls the shared `electrical_preview_request` builder. Both helpers live in the target-independent PCB wiring operation module and are invoked by production code as well as native tests.

The transition test begins with a production `Current` Matrix plan identity at token/revision 1/0, supplies the accepted post-mode-edit identity 2/1, and asserts the actual shared transition becomes Pending for 2/1. It verifies duplicate current requests are deduplicated and that Idle/Failed states can begin a changed request. The request test uses the accepted revision-1 document with Direct mode and a saved board lock, calls the exact builder used by Runtime, and verifies the emitted Core request reads Direct mode, current revision, board ID and saved locks. The previous mode-owner success test still independently verifies exact SaveCommitted settlement and Saved feedback retention after the accepted identity advances; no test manually promotes old resolution state as evidence of resolver correctness.

## Verification

- `production-resolution-green.log`: 6 PCB operation tests passed, including transition and request builder. SHA-256 `ed1deded178a9173aaa2d09cdd952e32cb371654e21dd85d61c7462fc007994e`
- `mode-owner-green.log`: 5 mounted owner tests passed. SHA-256 `a837d9543c3a550824158537305c62bc8ae4599821e4675e965213480d585a65`
- `expected-saved-red-with-plan-guard.log`: expected failure when Saved feedback was temporarily subjected to old plan identity filtering; exit 101. SHA-256 `df213119540664a541de8b9e9860e21a95465562b485e9a398de9d19838408af`
- `fmt.log`: `cargo fmt --manifest-path web/Cargo.toml -- --check`, passed. SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `diff-check.log`: `git diff --check`, passed. SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

No new RF item is proposed; preserve RF-001/RF-006/RF-009. Packaged paired browser acceptance, Undo/Redo/save-reopen and all F5.1/F5.2/F5.3 joins remain open.
