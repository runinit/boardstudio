# Independent Spec review — pre-CAD Case PCB preview contract

Disposition: **changes required before exact-hash clearance**. Read-only review; no source or contract edits, tests, browser runs, or subagents. Worktree `/home/chris/.local/share/boardstudio/worktrees/f73-case-preview-capability-20261002`, branch `codex/f73-case-preview-capability-20261002`, HEAD `42b9fdefe1941e3441ed7d43d2757e93e3588bde`.

Verified packet SHA-256 values:

- Draft `10-pre-cad-case-pcb-preview-owner.md`: `c1e970d81e46243d6cdd203c0dfa36e12d51c0357b9c5843fa90986f2106f049`
- Issue07: `b8cca5ced2b46748b494bc27c83916ed74ffbc309a9695a7dcd1dc976e0957e7`
- Issue08: `53ede6037df7ad11c656efa234e06c25cd53c0adb3fb2565d3d3b76d1e4c5d0a`
- Source reconciliation: `85173af6b3c7c21f0ab4887be3528f35661bfa33e36ea173f4b191b118fc18a4`

## Findings

1. **P2 — distinguish placement rows from decoded model delivery.** Issue08 line 20 says “Publish the Core-accepted `PcbPreview` immediately with `models: []`”; draft line 31 and Issue07 repeat this ambiguity. `PcbPreview.models` contains the authoritative `PcbModel` placement/reference/path records, which must remain available for decoding, renderer joins and picking. React publishes `{ board: result, models: [] }`: the empty array is the separate `AssemblyPreviewSnapshot.models` of decoded meshes, while `board.models` remains populated. Specify the two fields/types explicitly throughout, retaining the accepted preview's placement rows from the first publication. Otherwise an implementation can satisfy this sentence by discarding precisely the rows the rest of the contract requires.

2. **P2 — incremental successful-model publication is an extension, not observed React behavior.** The source-reconciliation sentence “Successful, missing and failed model rows update later snapshots while the request is still current,” together with draft story 4 / row-arrival publication rules, suggests React publishes successful rows as each decode finishes. Pinned React `app/src/assemblyPreview.ts::prepareBoard` instead pushes successful rows into a local `loaded` array, waits for `Promise.all`, and only then publishes `{ models: loaded }`; missing/failure messages can publish earlier. Keep the desired incremental Dioxus contract if intended, but label it an explicit extension and test it independently. Settled board/model identities and error outcomes remain the paired parity oracle; do not require React to demonstrate progressive successful rows.

## Source-backed assessment

The central boundary repair is correct. Pinned React `5a472a9426e6e38993361da402cd4ec730feb369` selects enabled reference `PreviewBoard` versus native `ExportClient.preview`; `export.worker.ts` performs `PreparePreview` → ordered conversion → `FinishPreview`. Its accepted board publication precedes model decoding. Mechanical Case preparation is independent.

Current `cad_jobs.rs::captured_case_document` and `captured_case_scene` derive immutable matching physical inputs directly from accepted snapshots without running mechanical generation. `effective_case_inputs` validates board/instance membership, mirrors selected parts/contours and disables imported references when flipped. Thus these helpers support the proposed pre-CAD source without relaxing Case CAD readiness.

Current shared-viewer projection does require a `CadScene` and leaves PCB surfaces/holes/model rows empty; current `ModelOwnerIdentity` does use `Weak<CadScene>`. The requested private owner change is necessary. The packet preserves one native producer (08), verified byte/decode/cache provider (07), and renderer consumer (12), with root-owned shared ports. It separates optional mechanical overlays, demands scope/token/revision/generation/batch/worker freshness across asynchronous boundaries, suppresses stale errors, preserves healthy rows, and retains public/root/subpath/offline/durable-store gates. Those requirements are coherent. No extra readiness dependency or duplicate provider is needed.

Fixture asset count and no-reference statements were not independently re-opened from the archive in this review; this review establishes source contracts, not fixture or public acceptance. RF: no new distinct refactoring takeaway observed; retain RF-003 and RF-006, and RF-002 if implementation encounters page/library reachability. Current parity failures remain implementation work.

## Frozen corrected packet acknowledgment — 2026-10-02

**Spec clear for this exact corrected contract packet; both prior findings are resolved.** Re-read the amended clauses and checked all four full hashes directly from the frozen worktree:

- Draft `10-pre-cad-case-pcb-preview-owner.md`: `4aa4fdaa52c6cc60d846c5bbde8eb8b4790e33a544d67cab01aca73f51631b8c`
- Issue07: `216d3cc516801e80c7dffc7bc5e3b6f9d9644a427d31c5ea1ec56076138adb05`
- Issue08: `0527f627834d7373718bbf9b0cdf260054d04c725ac3f1785c261af0027196c1`
- Source reconciliation: `2300c138dde7e98ffce049e7d08dcbc22bdcb24729fd4922e28c7ef81cdd1eed`

Draft lines 9, 18, 31, 39, 41 and 57; Issue07 line 20; Issue08 lines 20 and 26; and reconciliation line 9 now consistently preserve `PcbPreview.models` as source placement/path/reference/transform rows while the separate decoded-mesh collection begins empty. Successful decoded rows publish together only after all decode tasks settle; per-row missing/decode errors may publish earlier. This matches the previously inspected pinned React `AssemblyPreview.prepareBoard` board-first publication, local `loaded` accumulation and final `Promise.all` success publication. The packet no longer requires progressive success delivery.

No new actionable Spec finding observed in these corrections. The earlier source-backed ownership/identity assessment and RF-003/RF-006 carry-forward remain valid; no new refactoring takeaway or RF ID. This acknowledgment clears the contract only: capability-level start conditions, implementation reviews, fresh build identity and all required public/paired/offline/durability acceptance evidence remain open.
