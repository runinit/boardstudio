# F9.6 service-worker deployment handoff repair

The run08 public RED reached the old React shell after the same-origin server switched to candidate f956. Its controlling registration kept fetching `/sw.js` (404); `/service-worker.js` was never requested. The application and durable-store code had not run. See `rehearsal/run-f956-20261005-08/report.json` and `proxy-requests.jsonl`.

## Repair and scope

The real embedder now emits an import-free, classic-compatible `sw.js` beside the unchanged Rust module bootstrap. Both full-build and provider-reuse root/subpath manifests include it. On installation the bridge requests activation. Activation claims its scope, enumerates only controlled window clients, and navigates those within the exact origin/path scope to their current URL. It has no fetch handler, so that navigation loads the deployed shell. It contains no cache, IndexedDB, local-storage, unregister, timer, or message-loop operation. The destination page then registers its ordinary worker. Activation causes one intentional reload; carry out deployment after saving/exporting work. Unsaved local UI state is not a preserved-data claim.

Reverse deployment uses `node scripts/web/stage-rollback.mjs <pinned-react-site> <new-output-site>`. The output parent must already exist; every existing target (including empty directories), a target inside the reference tree, or any overlay collision is rejected. Every original regular file is copied byte-for-byte and independently hashed; links/non-file entries and concurrent reference drift reject staging. The only added serving files are the module-compatible bridge at `service-worker.js` and `rollback-provenance.json`, recording all reference hashes, overlay hash, source location, and **staged-not-rehearsed** status. The original reference artifact is never edited. Serve the copy at its original deployment prefix. It is an attributed overlay, not an identical reference artifact claim.

The scope/type behavior follows the [Service Workers specification](https://www.w3.org/TR/service-workers/), [controlled-client enumeration](https://developer.mozilla.org/en-US/docs/Web/API/Clients/matchAll), and [WindowClient navigation](https://developer.mozilla.org/en-US/docs/Web/API/WindowClient/navigate). The same bridge parses as both classic and module code; the existing classic registration cannot execute a module-import bootstrap alias.

## Provider and source provenance

No Core, CAD, renderer, application, or Rust worker code changed. The embedder and bridge are explicit **fresh offline packaging** inputs, executed afresh for both routes after provider reuse; their committed bytes stay in the complete source inventory and final drift guard. The standalone rollback tool is a recorded control input; its test is a recorded verification input. All six edited/added files are present with exact hashes in the actual source inventory.

The existing verified-helper compatibility mechanism now also pins full donor `113d76fd43d2c2a25660af4e3ff51032c7082f08`, SHA-256 `2597f8a90a464e96b8c5442fb2f85454408e231b80152e47b2c1d344a5e1c5c7`, Git blob `ab802ca94231b51679345b06aa5a7c6eb1c9a4ca`. Its helper bytes equal HEAD before this patch. The source diff changes only offline manifest staging plus explicit input classification/pin; inherited provider commands are unchanged. Exact donor identity, successful 23-command lineage (including page check), tool identities, assets, provider ownership, current committed input bytes, and final drift checks remain required. Modified provider inputs still reject reuse. A unit test rejects changed donor commit, SHA, or blob. Real committed preflight remains the coordinator's next package step; no package was built for this task.

## Executed evidence

- `handoff-packaging-red.log`: real embedder succeeded but both root/subpath tests failed on missing `sw.js` (0/2), before the bridge was added.
- `handoff-reuse-red.log`: new packaging input identities and changed builder helper rejected by the original guards (0/2); expected cause retained.
- `handoff-packaging-green.log`: **3/3 passed**. Exercises the real embedder at both prefixes, classic/module parsing, activation ordering, scoped navigation, absent fetch/storage behavior, and real rollback staging/hash/no-overwrite behavior.
- `handoff-reuse-green.log`: **41/41 passed**. Complete existing reuse suite plus exact helper/input regressions; mocked builds inspect root/subpath offline manifests and fresh handoff assets. These are packaging unit checks, not real provider builds.
- `git diff --check` and Python byte-compilation passed. `handoff-source-manifest.json` records the frozen six-file hashes. `handoff-repair.patch` contains the complete source diff, including added files.

## Required remaining proof

The coordinator must package/publish the frozen candidate and run the actual same-origin forward → staged rollback → forward rehearsal, wait for each destination's final ordinary worker control, and prove durable saved-project readback and archive preservation in both stores without deleting registrations, caches, or IndexedDB. Unit lifecycle mocks do not establish browser navigation timing or successful cutover. No public GREEN is claimed here.
