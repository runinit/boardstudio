> Historical measurements below retain their original revisions and paths.
> Current command: `pnpm test:perf`. Timing scenarios are excluded from
> `pnpm check` and run serially through `app/playwright.performance.config.ts`.
> The five-session baseline, host requirement, and thresholds are unchanged.
> Actual Chromium versions are recorded as provenance; no browser build is pinned.

# Workbench baseline, 2026-09-23

Captured before matrix, library, and editor feature edits. Checkout HEAD was
`c69a129`; `v2/` was untracked. Host: Ryzen 9 8945HS, Node 26.10.0,
Chromium 153.0.8010.47, Rust 1.98.0. Browser ran at Playwright Desktop Chrome's
default viewport and device pixel ratio. Runs were serial.

The mounted-workbench fixture has 100 or 200 keys with a switch, diode, and RGB
LED per key (300 or 600 rendered parts), pads, row/column/key/LED nets, and a
switch-envelope outline. Each run has 10 warmups and 100 preview edits. The
worker column ends at the core reply; painted waits for a React layout commit
and two animation frames. It does not include physical pointer dispatch.

| Keys | Move | Median worker p95 | Median painted p95 | Painted p95, five runs |
| ---: | --- | ---: | ---: | --- |
| 100 | one switch | 4.7 ms | 33.6 ms | 34.0, 33.6, 33.6, 33.6, 33.6 |
| 100 | first row, all parts | 4.0 ms | 34.4 ms | 34.2, 34.4, 34.4, 34.4, 34.2 |
| 200 | one switch | 6.9 ms | 42.7 ms | 42.6, 45.4, 42.7, 43.2, 42.3 |
| 200 | first row, all parts | 7.0 ms | 48.7 ms | 47.9, 48.7, 47.7, 48.8, 49.2 |

The existing simplified outline benchmark passed three runs. Its 100-key p95
was 34.8, 33.5, and 33.6 ms; 200-key p95 was 150.5, 33.5, and 33.5 ms.
Release Rust p95 was 62 µs at 100 keys and 140 µs at 200. Both older fixtures
contain only switch parts and omit the mounted workbench.

Compare equivalent pre/post fixtures with the median of five session p95 values.
The existing-path allowance is 10% above each baseline, and the existing
outline test still has its 100/200 ms absolute caps. New pointer and group
interaction gates need separate measurements because these paths do not exist
in the baseline editor.

## Candidate gate, 2026-09-23

The candidate was rebuilt from current Rust sources before browser runs. Five
serial Chromium 153.0.8010.47 sessions on the same host used the same mounted
100/200-key three-part fixture, viewport (1280 × 720), DPR (1), ten warmups,
and 100 measured preview edits per case. Values below are medians of session
p95 values; limits are 110% of the pre-change p95.

| Keys | Move | Worker p95 / limit | Painted p95 / limit |
| ---: | --- | ---: | ---: |
| 100 | one switch | 2.4 / 5.2 ms | 33.8 / 37.0 ms |
| 100 | first row, all parts | 2.3 / 4.4 ms | 34.1 / 37.8 ms |
| 200 | one switch | 3.9 / 7.6 ms | 33.9 / 47.0 ms |
| 200 | first row, all parts | 4.1 / 7.7 ms | 38.2 / 53.6 ms |

All four relative gates pass. Real pointer moves on the mounted editor yielded
first-painted-transform p95 of 24.6, 33.1, and 44.1 ms at 30, 100, and 200
keys, respectively, below the 33/50/100 ms limits. Every one of 110 sampled
moves per fixture changed the visible transform. The p95 frame gap was 16.7 ms
with no gap over 50 ms in those traces.

Complete 30/100/200-key generated matrices had 90/300/600 parts, including
diodes and RGB LEDs. At 200 keys, exact matrix/row/column outline previews
painted at 55.2/51.3/54.2 ms p95, below the 200 ms limit. The existing
switch-only Rust release p95 is 63 µs at 100 keys and 123 µs at 200 keys,
within 10% of its pre-change baseline. A 6×5 diode/RGB matrix preview takes
52 µs p95 in the release native benchmark.

The matrix creation measurement is diagnostic: one 200-key creation took
19–23 ms in the worker and 141–147 ms through the mounted paint in three
serial samples. The first 30-key mounted creation took 236 ms; subsequent
30-key samples took 36–37 ms. Browser shell navigation took 25–29 ms and
transferred 87.5 kB in the local preview; that shell figure excludes the
worker's later WASM fetch. The build lists core WASM at 646 kB gzip and the
Design workbench at 71.5 kB gzip. Three.js (182.6 kB gzip) and OpenCascade
(12.36 MB gzip) load on demand in 3D workflows.

The pre-change capture contains equivalent single/group drag baselines, but
not matrix creation, variant switching, pan/zoom, or library opening. Those
new operations are measured or functionally tested after implementation;
there is no defensible pre-change relative p95 for them. The five-session
comparison is limited to the four equivalent drag paths above. Physical
6×5 KiCad DRC tests prove zero local clearance violations for MX and Choc
solder/hotswap with back-side diode/RGB placement. Those geometry fixtures
have no assigned nets; routed connectivity is not claimed.

A separate 200-key row run recorded stage p95: 1.9 ms inside the WASM request
envelope, 0.4 ms reply parsing, 2.7 ms remaining worker transport/queue,
3.0 ms to the React layout commit, and 34.6 ms to the settled frame. The
WASM envelope includes JSON and engine work; native Rust timings above isolate
the geometry path. A 500-key/1,500-part row diagnostic painted at 82.5 ms p95
with 7.7 ms worker p95. In that run the heap reached 20.3 MB, the longest
task was 145 ms, and one task exceeded 100 ms. These are diagnostics, not
release limits.

## Matrix and library repair gate, 2026-09-23

After the matrix resize, selection, tree, and library preview repairs, five
more serial sessions on the same host passed every equivalent interaction gate.
Values are medians of session p95 measurements.

| Keys | Move | Worker p95 / limit | Painted p95 / limit |
| ---: | --- | ---: | ---: |
| 100 | one switch | 2.3 / 5.2 ms | 33.6 / 37.0 ms |
| 100 | first row, all parts | 2.1 / 4.4 ms | 34.6 / 37.8 ms |
| 200 | one switch | 3.8 / 7.6 ms | 33.5 / 47.0 ms |
| 200 | first row, all parts | 4.4 / 7.7 ms | 38.8 / 53.6 ms |

The full browser run measured pointer-to-painted p95 of 24.5, 32.2, and
46.0 ms at 30, 100, and 200 keys. A 200-key matrix/row/column outline
painted at 53.3/51.2/51.9 ms p95. The generated WASM is 647.33 kB gzip;
the Design workbench chunk is 72.57 kB gzip. These browser measurements
used the built app and Chromium 153; they do not cover a deployed site.

## Rust 3D renderer artifact, 2026-09-25

The production build emits the renderer and Cadrum as separate lazy WASM
artifacts. The renderer WASM is 524,023 bytes (524.02 kB; 216.91 kB gzip),
with 23.68 kB of JavaScript glue (6.67 kB gzip). The Cadrum WASM is
12,821,079 bytes (12,821.08 kB; 4,381.46 kB gzip). The earlier workbench
baseline recorded Three.js at 182.6 kB gzip and OpenCascade at 12.36 MB gzip;
these are different artifacts, so the figures are informational and set no
bundle-reduction target.

One production browser run on Chromium 153.0.8010.47, at 1280 × 720 and DPR 1,
measured renderer WASM initialization at 12.1 ms and first-scene setup/render
at 61.9 ms. The renderer WASM resource reported 2.7 ms duration and 524,023
encoded bytes. These are local-preview diagnostics, not release gates. The
historical baseline has no matching 3D cold-start measurement for a direct
timing comparison.

## Split assembly and manual generation, 2026-09-25

Measured on the uncommitted implementation based on `66c6fb08`, using Chromium
153, a production build, a 1280 × 720 viewport and DPR 1. The new fixture in
`app/e2e/manual-generation.spec.ts` contains two mirrored halves, 70 MX hotswap
switches and 70 diodes, with no component models. Both plates contain a total
of 70 square 14 mm openings. These are single-run local diagnostics, not the
five-session interaction gates above.

| Operation | Duration |
| --- | ---: |
| Renderer WASM initialization | 25.4 ms |
| Initial canvas and scene setup | 378.8 ms |
| Initial scene preparation in worker | 163.8 ms |
| Initial scene upload | 102.8 ms |
| Fresh CAD preview after cancellation/restart | 663.6 ms |
| CAD preview using cached bodies | 3.3 ms |
| Subsequent scene preparation in worker | 164.2–183.1 ms |
| Subsequent scene upload | 20.8–23.5 ms |

In the same split fixture, scene upload measured 853.4 ms before replacing
per-float JavaScript iteration with bulk typed-array copies into WASM. The
post-fix warm upload measured 20.8 ms. Camera, display mode, section/exploded
view and visibility changes perform neither scene preparation nor upload.
Rendering requests are coalesced into animation frames; an idle preview does
not continuously redraw. CAD generation is explicit and cancellable, emits
per-body progress, caches unchanged regions and postpones STEP serialization
until export. There is no equivalent pre-change split CAD timing baseline.

The current production renderer WASM is 838,078 bytes (326,229 bytes gzip).
Its generated JavaScript glue is 50,982 bytes (8,734 bytes gzip), before Vite
bundling. The independent Cadrum WASM is 12,846,887 bytes (4,397,513 bytes
gzip). These replace the earlier artifact sizes in the preceding snapshot;
they do not establish a bundle-size reduction target.

The companion gasket browser fixture generates 12 supports across two halves,
24 EVA strips, a removable retainer and 29 CAD bodies. It exercises actual
pointer dragging, mirrored placement, undo/redo and saved-anchor restoration.
A repeat after the final core/app rebuild measured 690.2 ms for fresh CAD,
4.3 ms cached, 100.7 ms initial upload and 22.5 ms subsequent upload. The
production, development and `/boardstudio/` checks also passed local/offline
loading and STEP export. These runs do not measure a public deployment.
