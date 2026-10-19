# 08: Replace check-then-unwrap and defensive unreachable calls in fallible production paths

Status: ready-for-agent
Type: build
Blocked by: [01: Reject duplicate document identities and overflowing revisions on open and commit](01-reject-duplicate-identities-and-revision-overflow.md), [02: Preserve monotonic mechanical feature identities across removal, undo, and overflow](02-preserve-monotonic-mechanical-identities.md), [07: Fix WASM Clippy warnings across workspace crates and widen the lint gate](07-fix-wasm-clippy-warnings-and-widen-lint-gate.md)

**What to build:** Fallible production paths across core, application, CAD worker/jobs, keycaps view rendering, and STEP circular accuracy checking replace check-then-`unwrap()`, distant-guard `unwrap()`, and wildcard `unreachable!` arms with local `if let Some(...)` bindings, `Option` combinators, explicit enum variant patterns, or `.ok_or(...)?` error propagation.

## Acceptance criteria

- [ ] CAD worker dispatch and CAD job reply handling return structured errors when `prepared` input is absent or the initial request variant does not match, instead of calling `expect` or `unreachable!`.
- [ ] `OutlineChange` dispatch matches settings variants explicitly (or returns `Err`) instead of a wildcard `_ => unreachable!(...)` arm.
- [ ] `is_some()`/`is_some_and()` followed by `unwrap()` in `keycaps_fit`, `step_accuracy::circular`, `kicad::output`, and `application::session` are replaced with `if let Some` or `Option` combinators.
- [ ] Repeated post-validation `.unwrap()`/`.expect()` lookups in `electrical`, `archive`, and `mechanical::edit` use local bindings or `.ok_or(...)?`.
- [ ] Native and WASM lint and test gates pass cleanly.
