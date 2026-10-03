# Keymap 3D keycap consumer — paired 2026-10-03

- React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34759/`, source `04c85b88eae2894b416979f785a1f0060d2eb27d`; build provenance SHA-256 `81ee87aead500473ae22f79834a2bc921618442a081719860a08188c063e49aa`.
- Both runs used the retained c6 fixture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`, at 1280×577.
- After selecting Keymap and setting SW1’s Main/base behavior to Key press `A`, both shared viewers rendered the accepted keycap geometry. The expanded layer lists exposed global **Keycaps**, per-key cap rows, and the independent **left-keys-SW1 legend** layer. Dioxus showed no preparation/error state after the CAD worker delivered the mesh.
- Captures: [`react-keymap-3d.png`](react-keymap-3d.png), SHA-256 `50e1818a3e82d1d69e385a7f292637cfb0326665b75c5b37a54561b6e575b885`; [`dioxus-keymap-3d.png`](dioxus-keymap-3d.png), SHA-256 `2c4c0822e662f83298248002a3f1237417d90caba96fdfb17a8b3f47324d99dd`.

This qualifies the changed Keymap-to-existing-Keycaps-fit consumer and its visible cap/legend layer journey only. It adds no second fit resolver, CAD worker, scene, or document authority and does not accept broader F6C.5 criteria.
