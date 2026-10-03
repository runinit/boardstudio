# F8.2a KiCad footprints ZIP — bounded browser receipt

- Candidate URL: `http://127.0.0.1:34767/boardstudio/` (the verified root and subpath routes both returned HTTP 200 with COEP/COOP).
- Candidate source: `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`.
- Candidate provenance: `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`.
- Fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Journey: import fixture → title `Sofle v2` → Export → Export KiCad footprints. The button was enabled for the 14-definition project.
- Scope: project `m1-sofle-v2-copy`, revision 12, selected board `Left PCB`; export used the committed accepted snapshot. No all-format checks were run.
- Download: source names the file `Sofle v2-footprints.zip` and uses MIME `application/zip`; captured bytes saved by the named browser session to `Sofle-v2-footprints.zip` because the download command takes an explicit destination path. The download completed from the verified `/boardstudio/` route with no Runtime error.
- ZIP: 1,584,592 bytes; SHA-256 `4d0417f389f61352dcee19eacf3e4341b0664be5b1a99d996b6edf45b28edb83`; ZIP signature `PK\x03\x04`; `zipfile.testzip()` returned no error.
- Entries: 22 total — 14 `BoardStudio.pretty/*.kicad_mod`, six `models/{sha256}.{step|stp}` model assets, `fp-lib-table`, and `BOARD-UTILITIES.txt`.
- Duplicate-name correction: four switch definitions and four diode definitions received distinct stable ID-derived filename suffixes. Each of those eight file stems matches its internal KiCad root name. Unique legacy filenames were preserved, including their existing Ergogen-generated namespace names.
- Models: all six fixture model assets were present; each model payload SHA-256 matched its filename digest (Nice Nano, OLED, PJ-320A, Kailh hot-swap switch, Cherry MX switch, 1N4148W diode).
- `fp-lib-table` points at `${KIPRJMOD}/BoardStudio.pretty`. `BOARD-UTILITIES.txt` contains the standalone-library warning and generator-skip section if applicable. In this fixture it contains the standard two-line notice and no skipped utilities.

The downloaded archive is retained outside Git at `/home/chris/.local/share/boardstudio/reviews/export-workspace-20261003/Sofle-v2-footprints.zip` (SHA-256 `4d0417f389f61352dcee19eacf3e4341b0664be5b1a99d996b6edf45b28edb83`). The original external receipt has SHA-256 `cc7c93bae672ba9991b955315309f26c42cbfbc2f6c0cf07612c2a2867be3a84`. React baseline pin and its observed duplicate-path error are in `/home/chris/.local/share/boardstudio/reviews/export-workspace-20261003/react-footprints-baseline.md`. This is a bounded F8.2a changed journey, not acceptance of F8.2/F8.3 or the broader Export parent. The shared Ergogen worker still cannot cancel already-running per-request work; the captured Runtime owner suppresses stale ZIP delivery.
