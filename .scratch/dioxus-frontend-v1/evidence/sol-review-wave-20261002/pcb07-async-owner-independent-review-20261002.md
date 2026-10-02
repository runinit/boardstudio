# PCB07 async owner correction independent review

Reviewed exact frozen source `c0a0b1c6d2bf79347dda26e60342e0b385afe7ff` in clean isolated worktree `pcb07-contextual-part-inspector-20261002`. Bounded correction diff: `0a1d08ad7c72592186e11c00c06c9d1c6b684699..c0a0b1c6d2bf79347dda26e60342e0b385afe7ff`; surrounding source-classification and proposal-admission paths from the prior PCB07 packet were checked as context. Requested independent Sol6.1 High review. Review performed on both axes without additional subagents, per coordinator instruction.

## Standards

**Clear for bounded source integration; zero new findings.** The hook uses an Editor lifetime `Rc<Cell<bool>>` with `use_drop`. The short-circuit lifetime check precedes live workspace/generation reads and the signal-backed selection predicate in the detached post-classification continuation. No further await separates this check from those reads. It preserves the prior captured accepted-document/source revalidation, registers the exact outcome slot before submit, and leaves settlement ownership unchanged. No new member/API widening, dependencies, suppressed checks or weakened assertions were found. Root CONSTRAINTS lifetime, stale-result and regression requirements are satisfied for these two source defects.

## Spec

**Both prior asynchronous source holds corrected; zero new findings in this bounded repair.** The callback retains the workspace Signal rather than the pre-await string; changing PCB to Layout while classification is pending rejects admission. Owner teardown rejects before any dropped Signal access. The existing packaged `isErgogen(generator.source)` remains the classifier for both standalone Inspector rows and proposal eligibility; project definition ID prefixes do not establish generator membership. Exact selected part, board/session/scope, accepted token/revision/document and generator source checks remain after classification.

## Verification and limits

Independently executed on the frozen source:

- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --test pcb_part_net_admission`: **2 passed**, no failed/ignored tests. The native page-only binary prints existing dead-code warnings; this command is not strict native Clippy.
- `cargo fmt --manifest-path web/Cargo.toml --check`: passed.
- `git diff --check 0a1d08ad..c0a0b1c6`: passed.
- `git status --short`: clean; HEAD equals the frozen SHA.

Reused author expected-red/green evidence from `.scratch/dioxus-pcb-view/evidence/pcb07-source-repair-20261002/async-owner-fix.md`: moving the signal read before lifetime rejection caused ValueDroppedError, and omitting live-workspace comparison admitted the stale request; restored guard gives two green tests. These mounted tests exercise the exact production hook/guard, not the complete browser classifier/admission callback. Author strict WASM all-target Clippy is reported in that evidence; no new broad rebuild was needed for this narrow review.

This clears the original classification and two async **source** holds recorded in `pcb07-source-correction-independent-review-20261002.md`. It does not close PCB07, F5.2/F5.3, paired public mapping/create-net/history/Undo/Redo/save-reopen, accessibility/responsiveness, or full parent gates. Root's frozen current package excludes PCB07; integration belongs to the next wave to preserve its source identity.

No new refactoring takeaway observed. Retain RF-001, RF-006 and RF-009; no new RF ID or deferred correctness waiver.
