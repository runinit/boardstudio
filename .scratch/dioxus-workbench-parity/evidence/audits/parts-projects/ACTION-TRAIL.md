# Paired browser action trail

Date: 2026-10-02. This is a visual/interaction audit, not implementation or acceptance certification.

## Sessions and provenance

- React reference: `http://127.0.0.1:5175/`, isolated agent-browser session `pp-react2`, profile under `/var/tmp/pp-102/rp`.
- Dioxus candidate: `http://127.0.0.1:34687/`, isolated agent-browser session `pp-dioxus2`, profile under `/var/tmp/pp-102/dp`.
- Fresh Dioxus start screen: session `pp-dioxus-start`, separate profile `/var/tmp/pp-102/dpstart`.
- React checkout HEAD observed: `5a472a94`; integration checkout HEAD observed: `89b1de8a28fdf02db91d972c90a69235bfbbbffb`.
- The served Dioxus candidate was `frontend-encoder-select-fixed-20261002`, provenance source commit `868edfcbdf93315e866962c9c57543d26672f379`. It is not the current integration HEAD. Treat all screenshots as evidence for that exact served candidate, not a rebuild of `89b1de8a`.
- Browser page-error buffers were empty for both current sessions at the end of the audit.

## React

1. Opened fresh Project start. Observed `New project`, `Open project…`, saved library, search, `Create new keyboard`, and 18 demo actions. Saved screenshot `react-project-start.png`.
2. Started Sofle v2. Opened Parts. Observed searchable seven-category component library, eight Key Assemblies presets, asynchronously loaded VIK module catalogue, KiCad import and New custom component. Selected an MCU: canvas displayed Mechanical fit / 2D footprint / 3D model / Layers; Inspector displayed saved assemblies and per-definition side/reversible/hotswap/solder/keycap/Choc settings, dimensions, and Apply generator settings. Saved initial screenshots `react-parts-mx-2d.png`, `react-parts-mx-3d.png`, `react-parts-mcu.png`, `react-parts-vik-module.png`, `react-parts-module-variants.png`.
3. Opened and canceled Mechanical fit editor; it exposes the measured plate-to-PCB height and Add cutout/Add clearance actions. Saved `react-mechanical-profile.png`.
4. Opened New custom component; form exposed name, kind, courtyard width/height, pad addition. Used Undo to remove the just-created draft. Saved `react-custom-component.png`.
5. Opened New assembly and closed without saving. Observed name, add component, X/Y pose, place-on-selected-board (disabled without a selected board). Saved `react-new-assembly.png`.
6. Created a small valid `DemoImport.kicad_mod` under this evidence directory and imported it into the isolated Sofle project. The imported definition appeared in Custom. Saved `react-import-kicad.png`.
7. Opened Project menu. Observed current project name editor, New project, Open/import file, Save project copy, saved keyboards, and all demo cards. Created New project and reached Setup guide step 1 (name, one/split, reversible); advanced to step 2 (Add key matrix / Edit existing objects / Continue to wiring), then separately used Create new keyboard and confirmed it opens the same setup flow. Saved `react-new-project.png`, `react-create-keyboard-step2.png`, `react-create-new-keyboard.png`.
8. Used Save project copy to download `untitled-copy.boardstudio` (611 bytes); imported that exact file through the Project menu. Then started REVIUNG41 and opened the saved Sofle v2 from the library. Saved `react-reviung-demo.png` and `react-open-saved-project.png`.

## Dioxus

1. Opened a fresh profile. Start screen showed Open a keyboard, two fixed fixture copies (REVIUNG41 copy and Sofle v2 copy), and Import .boardstudio. It showed no New project/Create new keyboard action or project-card library. Saved `dioxus-project-start.png`.
2. Opened Sofle v2 copy, then Parts. Observed the seven-category catalogue and search. Selected a controller and then searched `sod-123`; the catalogue filtered to Matrix diode while inspector continued showing the currently selected MCU. Saved `dioxus-parts-current-build.png` and `dioxus-parts-mcu-selected.png`.
3. Uploaded the React-exported portable archive into the fresh Dioxus session. The app opened an editable zero-part Main board. Saved `dioxus-imported-project.png`.
4. No placement or project mutation actions were invoked in Dioxus; all actions were limited to opening a fixture/archive, selecting a catalogue result, and filtering search.
