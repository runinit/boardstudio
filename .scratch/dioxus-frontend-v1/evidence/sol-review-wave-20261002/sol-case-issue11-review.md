# Frozen Case Issue 11 independent source review

Requested reviewer: Sol 6.1 High. Observable runtime model/effort metadata is not exposed to this reviewer; no provider configuration was changed.

Candidate: `6485475a2fa9af5b067e6566943d28a232d6718f` in `/home/chris/.local/share/boardstudio/worktrees/case-static-model-paths-impl-20261002`.
Base: `71a144ec3a58c8e7fe8169fae5fbe8429f995f6a`.
Approved planning source: `6ffe6be935d4ef0db857ea3d46f3733cedc59a7e`, Issue 11 SHA256 `7ec0c74f55e1c46cedea35b4f3e9a6ea162e8813f2bf99778a54575d40b6d35c`.
Author receipt SHA256 verified: `8c6a81830762d951acc7f5eaf7d477803c6e24c414a10fdc0725e3bc7bfa38dc`.
Review scope: two-file frozen source diff, existing generated catalogue/build owner, accepted request/publication path, approved Issue 11 and diagnosis, and current integration-root CONSTRAINTS. No application source edits or subagents.

## Spec

**CLEAR for bounded source integration; no actionable source findings.**

`web/src/bundled_models.rs:16` exposes existing descriptor `id`/`url_path` rows without another parser, ID grammar, registry or byte provider. `web/src/case_preview.rs:457` checks all accepted document asset IDs before adding catalogue paths, preserving both valid document-path precedence and missing/error behavior for document-owned unusable assets. The resulting table is copied into the existing request at line 487 and retained through accepted preview publication at line 519. Unknown catalogue IDs receive no generated fallback. Existing accepted physical capture and owner identities remain unchanged.

The new assertions cover the exact Cherry MX descriptor in capture and actual request, a valid overriding document asset, and an unusable overriding document asset. Existing scope/epoch/lease/supersession tests still pass.

**Acceptance remains open.** Source CLEAR does not complete Issue 11: fresh paired public Case runs using the exact retained project, root and `/boardstudio/` route resolution, the retained-fixture Core/worker/FinishPreview proof including diode rows and transforms, and the remaining specified unknown/unsafe-path evidence must be retained or freshly executed as applicable. The earlier diagnostic harness/results are explicitly unavailable in the author receipt, so their prose/hashes are historical evidence, not a fresh raw execution artifact. Issue 10 owns the subsequent `${KIPRJMOD}/` emitted-path-to-ID join and decoded bytes; the parallel reviewer/coordinator must verify that integrated join before public claims. No 90/90 delivery or parent closure follows.

## Standards

**CLEAR for bounded source integration; no documented-standard violations or actionable smell findings.**

The diff is limited to the private catalogue iterator and existing private Case producer. It preserves Core/shared contracts, saved data, document ownership, existing path validation, and source-lifetime guards. It adds regression assertions without removing assertions, raising thresholds, adding suppressions, widening an external API, or implementing another static provider. Borrowed catalogue rows and document IDs are used until the existing owned request table must be constructed. The author receipt records the expected pre-fix regression failure and correctly limits its claims.

No new refactoring takeaway observed within this diff. Existing RF-003 covers producer/path representation, and RF-006 covers accepted physical source ownership; preserve those records rather than creating duplicate RF IDs.

## Verification

Independently rerun at exact candidate, clean worktree:

- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page case_preview::tests`: 12 passed, 0 failed; native-target existing dead-code warnings remain visible.
- `cargo fmt --manifest-path web/Cargo.toml -- --check`: passed.
- `git diff --check 71a144ec3a58c8e7fe8169fae5fbe8429f995f6a...6485475a2fa9af5b067e6566943d28a232d6718f`: passed.
- Exact spec and receipt SHA256 verified; `git status --porcelain=v1` empty before and after checks.

Author-reported WASM page check reused; not independently rerun. Strict Clippy, complete affected-crate tests, production build/browser/route acceptance were not executed by this bounded source reviewer and remain explicit coordinator gates where applicable.

Source findings: Spec 0; Standards 0. Source CLEAR is not public acceptance.
