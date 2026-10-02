# PCB-07 generator-source eligibility repair

The independent review found that standalone plated-pad eligibility had been inferred from the serialized definition ID prefix. TypeScript calls the packaged Ergogen `isErgogen(definition.generator?.source)` helper, so project-owned IDs can still reference packaged generators and an `ergogen:`-looking ID alone does not establish generator membership.

The repair uses the packaged helper for the Inspector's direct-pad visibility and again during proposal admission. Proposal admission captures the accepted generator source, waits for the package classification, then revalidates the exact selected part, accepted token/revision/document, board/scope, and generator source before creating/submitting the existing `ReplaceDocument` operation. Named terminal mappings remain available independently of the standalone-pad filter.

The regression covers both classification directions: a project-owned definition ID with the packaged `ceoloide/trrs_pj320a` source is denied direct-pad assignment, while an `ergogen:`-prefixed definition without a generator remains eligible. A separate browser WASM test calls the packaged module's `isErgogen` function and verifies a known source and an unknown source.

## Verification

- Expected red: temporarily removing the source-based proposal guard made `standalone_pad_eligibility_uses_generator_source_not_definition_id` fail on the arbitrary-ID packaged generator case.
- Green: the regression passed after restoring the guard.
- Packaged classifier browser test passed with the generated `layout-generators` module served over HTTP with CORS enabled.
- `cargo fmt --manifest-path web/Cargo.toml --check` passed.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web --tests -- -D warnings` passed.
- `git diff --check` passed.

The mounted paired TypeScript/Dioxus PCB journey, owner/history/Undo/reopen evidence, and full ticket/parent joins remain open; these source checks do not close them. RF-001/006/009 remain the handoff references; this repair records no new architectural finding.
