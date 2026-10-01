# Focused STEP export green check

This is a focused public-UI regression check for the CAD worker reply decoder fix. It is component evidence only; the served stage is not a maintained production release.

- Page: `http://127.0.0.1:46823/`
- Stage: `/home/chris/.local/share/boardstudio/worktrees/m1-cad-worker-array-fix-20261001/web/target/builds/m1-cad-worker-array-fix-dev/site-root`
- Flow: open the Sofle v2 copy, generate its case, then export STEP from the public CasePanel.
- Result: browser download completed at 10,334,256 bytes. The file starts with `ISO-10303-21;\nHEADER` and has SHA-256 `582f343c9a9eabfedab5a8f69c2dc078c5ef31c0ade008d9de989a8d53b15195`.
- Worker reply: `export-step` completed with result keys `revision` and `step`; no `bodies` field was present. The decoder accepted this valid STEP-only response.
- Object URL: one 10,334,256-byte Blob URL was created and that same URL was revoked about 1.02 seconds later. No download anchor remained. The page reported exact geometry ready and saved locally; the captured page error log is empty.
- Raw browser observations: [browser-events.json](browser-events.json), [page-errors.txt](page-errors.txt). Downloaded bytes: [sofle-v2.keyboard.step](sofle-v2.keyboard.step).

The page WASM was built in a focused stage with provider/core-worker/CAD/renderer/fixture assets copied from the earlier BC1 stage. This proves the corrected page decoder against the public UI path but does not establish final-release asset freshness, offline behavior, or complete release provenance. Repeat the export smoke on the next maintained release.
