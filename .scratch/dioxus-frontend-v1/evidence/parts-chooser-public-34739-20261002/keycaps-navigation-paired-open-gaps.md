# Keycaps finding navigation: paired public journey

Status: bounded observation only; **not accepted as camera/Inspector parity**.

## Identity

- Browser session: `keycaps-nav-paired-3f3596fbfe2f`.
- Viewport: 1280×577 in both tabs.
- TypeScript: `http://127.0.0.1:5173/`, Vite process PID 32534 from `/home/chris/01_Projects/ts-boardstudio2/app`; repository HEAD `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus: `http://127.0.0.1:34739/`, static server PID 4038340; build source commit `a8fd8988f649c70c11393066314095447535dce5`, build provenance `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-chooser-pcb-navigation-integrated-20261002/provenance.json` (SHA-256 `183c7cd7dd133c6cdaf9c423d49877e37b1bfbe0ed2469e85a26fb02dd048c52`). This candidate includes issue05 navigation repair and predates Marker07, so no focused-finding marker is expected.
- Both tabs imported the same archive: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- The archive's `project.json` has `case: {}`; it has no saved case bodies or generated mechanical layers. Case body/layer ownership cannot be exercised from this fixture.

## Journey

In each app, open Keycaps, locate `left-keys-SW1 · switch mx`, and activate the first `Select affected geometry` action for the clearance finding `left-keys-SW1 and left-keys-SW2: keycap clearance -18.00 mm (required 0.50 mm)`.

Both apps switch to Layout and select the first affected key, `left-keys-SW1`. TypeScript Inspector identifies `left-keys-SW1`; Dioxus Inspector identifies `keys · Key 1.1`, reports one selected key, and lists `left-keys-SW1`. No browser errors were reported in either tab.

The rendered camera does **not** match. At the same 1280×577 viewport after navigation:

- TypeScript canvas SVG `viewBox`: `-57.933135863135874 21.70348340548339 134.22627172627173 102.75252525252527`.
- Dioxus canvas SVG `viewBox`: `-24.485114801179336 48.975591240875914 67.33022960235863 52.76321167883212`.
- The selected key's bounding box in Dioxus is about 161×161 CSS px, approximately twice the on-screen key size in TypeScript. TypeScript draws the finding outline over the SW1/SW2 pair; the pre-Marker07 Dioxus candidate has no corresponding focused-finding marker.

This demonstrates routing and single-key selection, while retaining a concrete open camera-fit/finding-feedback parity defect. It does not qualify complete Issue05 or F6C.4 acceptance. The source candidate's missing marker is expected for this build and is handled by the separate Marker07 slice.

## Evidence files

- TypeScript screenshot: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-nav-c6-5173-layout-route.png`, SHA-256 `033b752016a73eb676eca9041ce4fe9d5d25160c1df6d348c1507a7f3cce8c27`.
- Dioxus screenshot: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-nav-c6-34739-layout-route.png`, SHA-256 `17d349ba6d5cae73ac8b138f30270c74f74b8f4c34ab759823b6208a991ac03f`.

The unrelated missing original F6C.4 fixture/provenance gate remains open; this c6 fixture is a separate, clearly identified retained archive.
