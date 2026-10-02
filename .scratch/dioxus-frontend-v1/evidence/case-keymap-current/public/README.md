# Public browser evidence (verification only)

Candidate: production artifact source `dc81237c` at `http://127.0.0.1:34663/` and `/boardstudio/`.
React reference: existing `http://127.0.0.1:5173/`.
All browser sessions are isolated disk-backed task profiles under `/var/tmp/frontend-run/profiles/`; no production runtime APIs or IndexedDB writes were used. Browser interactions used real public controls.

Fixtures imported via the public import control:
- Sofle archive: `web/target/builds/frontend-parts-context-aria-20261002/site-subpath/boardstudio/assets/fixtures/sofle.boardstudio`, SHA-256 `0e1e06beabfce1c5a2d6bfc472c85ed281435ff2382174b32f9a2bd3d6d58899`.
- Reviung41 archive: `web/target/builds/frontend-parts-context-aria-20261002/site-subpath/boardstudio/assets/fixtures/reviung41.boardstudio`, SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`.

`reviung-candidate-canonical-case.png` captures candidate default `Physical instance=Canonical board`, which exposes `Add case settings` and `+ New case body`. `reviung-candidate-main-case.png` follows selecting the real `main` option; generated case assembly appears and authored controls disappear. React's initial default shows `Main case assembly` and physical assembly controls (`reviung-reference-main-case.png`). This is a public diagnostic scope convergence; it does not erase the initial default-scope mismatch.

Sofle Keymap screenshots compare the same imported document in System theme at desktop viewport: `sofle-candidate-keymap-system.png`, `sofle-reference-keymap-system.png`. The tested visible Base layer, selected key, canvas key buttons and Unassigned options are aligned. Candidate/reference document parity and detailed action outputs are retained in task-run outputs if available; no layered-keymap fixture has yet been produced.

This evidence is not a full milestone acceptance report. Actual assistive-technology testing and a public UI-produced/imported multilayer archive remain outstanding.

Additional paired screenshots use 1280×800 System initially and 1280×800 explicit Dark/Light themes; compact captures are 720×640 explicit Light. Theme settings changed only in task-owned profiles. React uses Project → Workspace settings → Color theme; candidate uses the public Theme selector.
