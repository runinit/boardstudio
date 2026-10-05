# Approved Dioxus adoption

User authorization (2026-10-05): “approved, cutover to new frontend and decommsion react”. No push is authorized; this is the local default-entrypoint and checked-in CI/Pages cutover. Hosted deployment is not performed.

Default start/dev/build and Pages select the complete Dioxus package. Default checks select Rust frontend/page/mounted-browser and shared provider tests. React-only launch/typecheck/unit/E2E/performance commands are explicit :react fallbacks; the production React entrypoints are retired. Shared app provider helpers, reference sources/tests, pinned reference artifact and byte-preserving rollback overlay remain recoverable. No source deletion or project-store reset is involved.

The first review identified stale React Vite E2E/performance commands after the new build. They were moved to explicit fallback commands, with React performance invoking build:react. The second identified fresh-host wasm-bindgen runner cache prerequisites. test:browser now runs the existing normal-install locked wasm-pack list command before the strict runner; that exact bootstrap executed successfully. This is compilation/listing evidence, not a fresh whole-suite pass. JSON/YAML parse and touched-path diff checks pass.

Repository-check unit suite: 7/7 pass. Full repository check fails on an unused generator export and broken links in archived POST-PORT-REFACTOR documentation. All diagnostics concern unchanged files (ergogen/src/index.ts and docs/migration/history/POST-PORT-REFACTOR-before-20261003-streamline.md), verified unchanged against HEAD. Those pre-existing broader findings are retained; no repository-check pass is claimed.

Source review, committed full default launch/package and adopted-path public browser results are recorded below when complete. Existing accepted application evidence retains its original source/fixture attribution; this batch changes entrypoints only.

Final four-file source review: CLEAR, consolidated case-profile-handle-review-20261005.md SHA73b70aa16adf57fd58dc39b8cedf7ff5a30217110472c2ad70dee2bbc8d8be5e. Integration prepare-gates terminal0, source keyc629e790fef1217f; no Rust source changed, so no new native/headless suite claim.
