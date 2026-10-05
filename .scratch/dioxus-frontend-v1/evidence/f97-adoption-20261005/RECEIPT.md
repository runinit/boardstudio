# Approved Dioxus adoption

User authorization (2026-10-05): “approved, cutover to new frontend and decommsion react”. No push is authorized; this is the local default-entrypoint and checked-in CI/Pages cutover. Hosted deployment is not performed.

Default start/dev/build and Pages select the complete Dioxus package. Default checks select Rust frontend/page/mounted-browser and shared provider tests. React-only launch/typecheck/unit/E2E/performance commands are explicit :react fallbacks; the production React entrypoints are retired. Shared app provider helpers, reference sources/tests, pinned reference artifact and byte-preserving rollback overlay remain recoverable. No source deletion or project-store reset is involved.

The first review identified stale React Vite E2E/performance commands after the new build. They were moved to explicit fallback commands, with React performance invoking build:react. The second identified fresh-host wasm-bindgen runner cache prerequisites. test:browser now runs the existing normal-install locked wasm-pack list command before the strict runner; that exact bootstrap executed successfully. This is compilation/listing evidence, not a fresh whole-suite pass. JSON/YAML parse and touched-path diff checks pass.

Repository-check unit suite: 7/7 pass. Full repository check fails on an unused generator export and broken links in archived POST-PORT-REFACTOR documentation. All diagnostics concern unchanged files (ergogen/src/index.ts and docs/migration/history/POST-PORT-REFACTOR-before-20261003-streamline.md), verified unchanged against HEAD. Those pre-existing broader findings are retained; no repository-check pass is claimed.

Source review, committed full default launch/package and adopted-path public browser results are recorded below when complete. Existing accepted application evidence retains its original source/fixture attribution; this batch changes entrypoints only.

Final four-file source review: CLEAR, consolidated case-profile-handle-review-20261005.md SHA73b70aa16adf57fd58dc39b8cedf7ff5a30217110472c2ad70dee2bbc8d8be5e. Integration prepare-gates terminal0, source keyc629e790fef1217f; no Rust source changed, so no new native/headless suite claim.

The actual root `pnpm start` command built committed source f41076301c592cf7145fbd2125bf7de00e4e7b72, then started its own server at http://127.0.0.1:4173/. Full build start-f4107630-20261005T181117Z-3535061 completed 23 fresh commands in 501.925 seconds; no provider command reuse. Package proof verifies 1,412 source inputs and 191 served assets at each root/subpath route, zero mismatches or release warnings, HTTP200 and required isolation headers. Actual default-start session40201 remains alive as the server. Public adopted-path qualification is separate below.

`application-source-freshness.json` confirms application/provider/test roots are unchanged from the accepted 82207f75 candidate. Required prior native/headless/workflow evidence retains its original identities; this batch does not claim rerunning those suites.

Final adopted public journey: PASS. See browser/RECEIPT.md and raw report; rename revision23→24 survived save/reload, all six matching workbench panels opened, portable copy preserves every other field and all six asset hashes, ordinary worker controls final page with no waiting/installing successor, and errors/console observations are empty. Sole independent final C01/C02/F9.7 review CLEAR, consolidated report SHA5babc21e3c4380ee161892b57b2161da3bf86cd809d44a3b39e1e616f0cb86e0.
