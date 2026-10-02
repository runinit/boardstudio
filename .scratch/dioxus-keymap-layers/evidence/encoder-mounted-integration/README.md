# Encoder mount source and checks

The bounded encoder editor is mounted in final source `868edfcb`, after its original mount `e82c039b` and the independently reviewed initial-selection correction. It reuses the ordinary binding editor and existing Core keymap operations. The source hashes and exact executed checks are in `source-checks.json`; review reports and compiler logs are retained alongside it.

Native page tests passed 34 tests. Encoder presentation is WASM-only: its source tests were typechecked by strict all-targets WASM Clippy but were not executed by the native suite. Strict Clippy and formatting passed. The oversized private helper signature failures are retained in the initial Clippy log. An earlier missing-borrow compiler correction is recorded in the source cleanup; its raw failure output was not retained. The final correction groups related read inputs with explicit admission/settlement identity policy.

Input rows use the existing Core peripheral-description bridge and the source-supported attached EC11 visibility rule. Push IDs are reported by Core; the frontend synthesizes none. Full F5 accepted plan/fingerprint, actual firmware output, attached-module public rejection proof and complete parent acceptance remain open.

The original build at 34685 exposed a real initial dropdown mismatch: accepted None bindings displayed Key press without document mutation. That expected-red evidence is retained. Final build `frontend-encoder-select-fixed-20261002` at 34687 passed all eight commands and 973 frozen source hashes; its actual default controls and bounded genuine archive workflow are green. Full F5/F8, module, scope-switch/focus/AT and parent acceptance remain open.

No new refactoring takeaway observed. Existing RF-006 covers the WASM-only test boundary and binding draft/admission identity; RF-009 covers source-to-parity evidence reconciliation.

Correction `868edfcb` passed strict WASM checks and a fresh eight-command/973-hash build at 34687. Root repeated the exact read-only DOM oracle and observed green; both public verifier packets retain original red plus final fixture/flow evidence. See `../encoder-select-regression/README.md` for the diagnosis, reviews and red/green logs. The genuine archive packet is released at `../encoder-public-workflow/RESULTS.md`; actual defaults, edits/history and nonempty archive import pass. Broader scope switching, full keyboard/AT, module rejection and F5/F8 joins remain open.
