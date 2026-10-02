# Macro source Spec review

**Source clear for private integration; full F6K.3 acceptance remains open.** Reviewed new untracked files against worker HEAD `1378ab2a8de1535a8c9775e9d4315b390b6913af`, approved contract `fd8261c6b04ff610ea652b2d71cb1478ee547f16e8d10b09bf92813f7d8a3a7d`, published issue04 and pinned React `5a472a9426e6e38993361da402cd4ec730feb369`.

Exact final SHA-256:

- `macro_editor.rs`: `15c47dc1a483462c15c56a410b020b65c9e3575d39026cda49034d542c484302`
- `macro_controller.rs`: `059d3ee36c2435c26ed881f9f3bf0e53fdeda37ce1065f087357671dde8d4e95`

No remaining material Spec finding. The bounded correction requested during this review is present: Add admission now validates and derives `Macro N` from the current accepted macro count, satisfying the contract’s “derives `Macro N` from the current macro count.” Entity IDs remain distinct from operation IDs and collision checked.

The reference defaults, persisted macro order, blur-only raw-name/numeric/trimmed-keycode edits, kind defaults, append/remove limits and Core-owned validation are retained. No reorder feature, persisted step IDs, new schema or public API appears.

Full Scope/editor/accepted-field draft identity and shared exact ordered-sequence stamps prevent shifted duplicate rows retaining another row’s draft. Admission verifies the live sequence pointer and contents before indexing; field reconstruction preserves the current step kind, and acknowledgement checks the expected resulting sequence. The final narrow correction also requires the original cached Rc stamp for actual step-failure relevance: structural removal/restoration cannot revive an old rejection merely because contents match again. Unrelated token/name/timing changes preserve the stamp and relevant failure. The read source retains an Arc handle; per-macro sequence copies are bounded and shared.

Ready/Saved, instance-policy, full Scope/generation/token/revision, unique target and single-flight checks guard submission. Operation observation precedes submission; exact admitted ownership governs Pending and terminal results. Collection Add/Remove feedback survives membership changes. Persistence failure leaves accepted state unchanged in Session, so original-field relevance preserves the error and draft. Runtime notifications no longer rebuild unchanged immutable projection.

Required joins: unconditional root hook/mount lifetime, native/WASM compilation, focused current Core boundary test, real public validation/recovery, duplicate-row removal, Undo/Redo, save/reopen, scope/workspace changes, stable macro references and actual firmware output. No Cargo, browser execution or source edits performed in this review. RF009 remains applicable; no new structural takeaway.
