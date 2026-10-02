# Offline failure: QA profile filesystem condition

The initial application-regression classification is superseded. The actual failure was reproduced, but the unchanged f65 candidate installs and reloads offline when its isolated browser profile is on a filesystem with sufficient free space. No application, build, manifest, production server or global configuration change is warranted by these results. The original observations remain intact; see `DIAGNOSIS-PRELIMINARY.md` and the original red captures.

## Evidence and controlled result

Candidate: source `f65b0c833694d1980ac71caa21d2f932cf785d53`, immutable `web/target/builds/frontend-parts-context-aria-20261002`, original server34659. Baseline47/server34651 remains the original control. Browser: agent-browser0.38.1 / Chromium154.0.8037.92. Build, manifest, service-worker, page WASM and server-copy hashes are in `environment-controls/hashes.txt`.

The normal app calls registration itself. No manual registration, storage seeding, mocked provider, modified artifact or injected worker was used. The final probe opens the real app, listens to actual ServiceWorker lifecycle events, and remains online until the worker is `activated` or `redundant`. Only then does it switch offline and reload. A45-second timeout is reported as unresolved, not an install failure. CDP observes lifecycle; it does not change install logic.

| Condition | Captures | Observed result |
| --- | --- | --- |
| Unchanged candidate, original server, fresh disk-backed profiles | `disk-r1/r2/r3.json`, `disk-s1/s2/s3.json` | Root3/3 and subpath3/3 activated, controlled, offline landing rendered (plus final asserted pair below) |
| Final disk-backed cache checks | `disk-r3.json`, `disk-s3.json`; `final-root.json`, `final-subpath.json` | Each contains exactly51 entries; final script asserts controller,51 entries and real offline landing, exits0 for both |
| Final exact-script red control | `final-tmp-red.json` | Explicit `/tmp` profile becomes redundant before offline; same script exits1 |
| Fresh default `/tmp` profile | `tmp-r3.json`, `chunked-root.json`, `chunked-subpath.json` | Installation redundant while online; offline browser error |
| Explicit `/tmp` profile, same profile option as disk controls | `explicit-tmp.json` | Redundant, no controller/registration, zero cached entries, offline browser error |
| Identical candidate bytes, copied server with only Content-Length added, default `/tmp` | `length-root.json`, `length-subpath.json` | Both redundant; header hypothesis falsified |
| Baseline47, ordinary temporary profile | `fresh-b1.json` plus original baseline captures | Installed and offline landing rendered |

Each lifecycle capture retains start time and event timestamps. Disk-profile paths were `/var/tmp/frontend-offline-fix/profile-disk-r1`, `-r2`, `-r3`, `-s1`, `-s2`, `-s3`; the explicit temporary control was `/tmp/frontend-offline-fix-profile`. Final assertion profiles were `/var/tmp/frontend-offline-fix/profile-final-root`, `/var/tmp/frontend-offline-fix/profile-final-subpath` and `/tmp/frontend-offline-final-red-profile`. All are task-owned isolated profiles; no user project/profile or protected baseline was changed. One earlier unchanged candidate reload also succeeded in a temporary profile (`/var/tmp/frontend-offline-fix/f65-request-events.json`); this is retained, not discarded. The Chromium allocation ordering below explains why the low-space failure can be intermittent.

At the controlled runs, `df -B1` reported `/tmp` available3,479,863,296 bytes versus `/var/tmp` available249,762,443,264 bytes. `/tmp` was not literally full. Its available space was below Chromium's blob-paging reserve.

## What was observed directly

Worker-target capture `f65-worker-events.json` records normal registration, installation, `event.waitUntil` rejection, and redundant/deleted registration. The exact exception is `NetworkError: Failed to execute 'addAll' on 'Cache': Cache.addAll() encountered a network error`. `failed-wasm-trace.json` links the failure to the candidate page WASM: HTTP200 followed by body-consumption `net::ERR_FAILED`; other fetches are then canceled. It occurs about37ms after worker start, not after the offline toggle.

An online-only Chromium NetLog (`/var/tmp/frontend-offline-fix/attempt1.json`) records all10,533,402 WASM bytes delivered successfully over HTTP; installation still fails within34ms. Thus successful HEAD responses and transport delivery do not prove successful browser blob construction. The baseline WASM is9,696,729 bytes; candidate is10,533,402, just above10MiB=10,485,760.

## Version-matched source explanation and confidence boundary

Exact Chromium154.0.8037.92 sources, URLs and hashes are copied into `environment-controls/primary-sources.json` and adjacent source files:

1. `cache.cc`: Cache.addAll loads each response as a blob. A failed body load produces the exact NetworkError above. `fetch_data_loader.cc` registers the incoming data pipe as a streamed blob and propagates failure if no blob is returned.
2. `blob_storage_constants.h` and `blob_memory_controller.cc`: on64-bit desktop the memory allowance is2GiB, the minimum page-file size5MiB, and the external free-space reserve is `2 × (2GiB − 5MiB)` =4,284,481,536 bytes. The effective disk allowance initially uses the total filesystem size; creation of the first paging file subsequently measures free space and adjusts it. Below the reserve, effective allowance becomes the amount already used.
3. `blob_builder_from_stream.cc`: streamed blobs initially use5MiB of memory, then a5MiB file block. Once file-backed, they continue using files. A further block fails if available blob-file quota is insufficient. This distinguishes the baseline below10MiB from the candidate above10MiB under the measured low-space condition. If another concurrent blob updates the allowance before WASM chooses its storage path, it can remain memory-backed instead, consistent with occasional temporary-profile success.

The filesystem causal condition is demonstrated by repeated controlled browser runs. The exact internal allocation branch is inferred from version-matched source and the10MiB boundary, not claimed as a debugger-captured C++ branch. There is no evidence of a general10MiB CacheStorage limit. Agent-browser's10,000,000-byte inspector buffer is used only for HAR recording, which these probes did not enable; ordinary worker attachment does not install fetch interception here.

## Smallest correction and remaining scope

Use an explicit per-task `/var/tmp` profile for the remaining offline QA, as authorized. Keep global browser configuration unchanged and preserve existing profiles/artifacts. Reclaiming unrelated temporary data is unnecessary. No retry loop, reduced manifest, altered WASM, relaxed assertion or production-code change is justified.

Runnable retained procedure (use a fresh SESSION/PROFILE each time):

```sh
node .scratch/dioxus-frontend-tranche-1/evidence/parts-context-e510dd4c/packaging/offline/environment-controls/natural-install-probe.mjs http://127.0.0.1:34659/ SESSION OUTPUT.json /var/tmp/PROFILE
```

Repeat with `/boardstudio/`. Successful installation must precede offline reload; final controls require51 entries and the actual offline landing UI. These results cover the stated artifact/host and offline landing-shell reload, not exhaustive offline editor/export workflows or every release host. At the final explicit temporary-profile red, `/tmp` still had only3,633,512,448 bytes free, below the same reserve. Task-owned live diagnostic browsers and the isolated Content-Length server were closed after capture; profile evidence remains. No compiler was run. RF: QA resource-filesystem condition; no new software-architecture finding. Coordinator owns RF009 recording.
