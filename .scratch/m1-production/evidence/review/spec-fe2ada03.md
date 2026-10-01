# Spec review: `fe2ada03` (fresh Dioxus page output)

**Result: no blocking Spec findings.** The production-code delta is limited to `scripts/build-m1.py` (six changed lines). It moves the prior `web/target/dx/boardstudio-web/release/web/public` directory into the current unique build output before each root or subpath page build. The resulting fresh `public` directory is copied into that route's staged site, after which the existing asset-copy, offline-manifest, provenance-hash, and source-equality steps continue unchanged.

The M1 specification requires reproducible provider assets and exact-candidate reviews (`.scratch/m1-production/spec.md`, maintainer story 30), and the acceptance ledger requires exact command/source/asset provenance (`.scratch/m1-production/ACCEPTANCE.md`). Ticket 06 requires preserving that provenance while running the full integration checks (`.scratch/m1-production/issues/06-acceptance.md`). Excluding the previous Dioxus output from each freshly staged route is consistent with those requirements: the preserved tree is outside `site-root` and `site-subpath`, and the source, route contents, and asset hashes are still recorded by the existing builder logic.

The change does not alter runtime behavior, user-facing functionality, public APIs, persisted data, or frozen thresholds. It does not claim overall M1 acceptance. This review covers the builder-script fix only; release reproducibility still depends on the exact full build and final inventory checks completing successfully.

**Review basis:** compared `scripts/build-m1.py` at `2040e23b` and `fe2ada03`; inspected the M1 spec, acceptance ledger, and ticket 06 acceptance. `git diff --check` for the script delta passed. No build was run as part of this independent Spec review.
