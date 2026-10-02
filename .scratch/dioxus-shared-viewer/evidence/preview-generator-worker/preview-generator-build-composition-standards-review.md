# Preview-generator build composition — Standards review

Reviewed integration `.scratch/dioxus-frontend-v1/evidence/build-frontend.py` SHA-256 `33d3efddfa2be35bf74e6b27803e10a51931bc67949ec9599a435974e78ba6e4`, current dirty delta.

**Clear; no material Standards finding.** The private preview-generator graph is built once outside the route loop, then copied into each actual route's assets directory before manifest enumeration and offline-worker compilation. Enumeration includes worker.mjs, kicad.mjs, ergogen.mjs, catalogue.mjs and graph provenance, and final route asset hashing covers the copied outputs. Relative module imports therefore have the same packaged graph under root and /boardstudio/; actual serving/offline resolution remains a runtime gate.

Source provenance adds the new builder, worker and Kicad converter while retaining existing Ergogen source/catalogue hashes. The preceding layout-generator build runs the existing generated-catalogue freshness check; the new path does not bypass it. The final source recheck covers these inputs. The baseline provider-source allowlist, provider asset-hash verification and reused-provider boundaries are unchanged. No static bundled-model bytes, storage provider, React/Core initialization or unrelated build configuration is introduced.

Reported Node6/6 and Python syntax checks are supporting evidence only. Fresh complete build, manifest/output identity, both routes and offline worker execution remain open; this source review does not claim them. No source edits, Cargo or build run. No new refactoring takeaway observed.
