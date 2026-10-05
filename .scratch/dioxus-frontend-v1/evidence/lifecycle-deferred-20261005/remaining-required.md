# F7.3-C07 desktop lifecycle qualification

Prior receipts cover Parts form preservation, shared-viewer owner rejection, and renderer cleanup/context-loss. The new resize/DPR test passed in the renderer lifecycle module (6/6); the native mirror guard passed 1/1. Mobile qualification remains deferred.

The mounted PCB Save replacement test passed 1/1 in `pcb-save-diagnostic-run6.log`. Initial incompletion came from fixture ordering: Save submits inside `spawn_local`, but the test drained queued effects immediately after `.click()`. It now yields until the mounted saving state before draining. While Persist is pending, `ReadModel.accepted` correctly remains the last durable document (X=1.0); after release, real IndexedDB readback proves the old project reached the next revision with X=7.25. The colliding-ID replacement stays at X=4.0 without stale feedback. No product defect was observed.

Frozen source `web/src/presentation/pcb_module_inspector.rs`: SHA-256 `06b6afff9288f8124b51c9fb945a64b1c160890f0076b93faac3ae90cfa6ec52`. Review patch `draft/pcb-save-green.patch`: SHA-256 `8bc43be26dc642742798b3270904ac3ab0c4fc177a72197827aaf842a92b4715`. Renderer hashes: page base `b7131cfa9f51d27e52aea8a38730eaf9508b92d7b49775f2eb2a1ec9db1692f4`; library mirror `5d4454ead1b6a375b00e11aedb82ce5ed18c324e5caf1c6cbbaee5e7b0489576`.
