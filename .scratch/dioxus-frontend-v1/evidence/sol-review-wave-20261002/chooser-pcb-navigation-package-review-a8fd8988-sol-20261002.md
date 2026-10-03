# Independent full package review — chooser, PCB mode, Keycaps navigation

Standards: CLEAR for package integrity and delivery. Spec: CLEAR for the same bounded package qualification. No source defect or asset mismatch found. This does not accept interactive workflows, fixture-dependent gates, or parent features.

Frozen package: `frontend-chooser-pcb-navigation-integrated-20261002`, source `a8fd8988f649c70c11393066314095447535dce5`, provenance JSON SHA-256 `183c7cd7dd133c6cdaf9c423d49877e37b1bfbe0ed2469e85a26fb02dd048c52`. Site is actually served at `http://127.0.0.1:34739/` and `http://127.0.0.1:34739/boardstudio/`; the failed 34738 binding is not evidence of a served candidate.

Independent read-only checks verified:

- All 1,347 current source input hashes equal the recorded build inputs, with no additions, removals or drift. Of these, 1,340 also match frozen Git blobs. The seven remaining inputs are generated `core/pkg` files explicitly listed in the machine audit; their current bytes match the build snapshot. Current HEAD `ef4fa71a24287a5cdd7b9fd975e758d1614ed407` has advanced without changing these source bytes.
- All 22 commands match the full-build sequence, exact argv, cwd and recorded extra environment, each exit 0; retained logs are present and independently hashed. Timestamps are ordered and total 538.159979 seconds. Version probes agree with installed tools. This is a full build, with no reuse-derived lineage. Log completion records agree with page/provider output generation; ordinary provider/package warnings do not contradict completion.
- Both routes contain exactly the recorded 145 file paths and hashes. Core worker, CAD worker, renderer, fixtures and CAD packages match their generated/staged provider directories; all current `web/assets` copied bytes also match. Worker entry scripts match the builder's exact glue.
- Each offline manifest contains exactly its route's asset set and correct build/route version. Offline JavaScript matches its generated package, and service-worker embedded WASM decodes to that route's generated offline-worker WASM bytes.
- Twenty critical actual HTTP responses per route (40 total) returned 200 with exact local/provenance hashes: indexes, CSS, fixture provenance, page JS/WASM, all worker/CAD/renderer JS/WASM and generator JS. Index, CSS, JS and WASM MIME types are correct. Every checked response has COOP `same-origin` and COEP `require-corp`.

The independent checker also executed the builder's read-only full-baseline guard, then separately checked frozen Git blobs, current input hashes, provider copies, offline embedding and live HTTP bytes. No application files, build outputs or serving configuration were modified, and no rebuild/heavy test run was performed.

Machine audit: `/home/chris/.local/share/boardstudio/reviews/chooser-pcb-navigation-package-independent-audit-20261002.json`, SHA-256 `774429cfe3907fa423803ee328c045327d2aaded7a24121cd6e09e7afe92f66a`. It retains exact command/log hashes, tool versions, provider hashes, all local asset hashes and live response hashes/headers. Reproduction script: `/home/chris/.local/share/boardstudio/reviews/audit-chooser-pcb-navigation-package-20261002.py`.

Public chooser placement/history, PCB mode/Apply, real Keycaps camera/focus, marker child, missing original linked-Core fixture and full parent joins remain open as previously recorded. Package delivery is independently qualified; those acceptance gates are unchanged.
