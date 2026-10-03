# Keycaps finding-navigation fixture witness — 2026-10-02

This is fixture and React-oracle preparation for issue [05 — Keycaps fit finding navigation](../issues/05-keycap-fit-finding-navigation.md). It does not close issue 05, F6C.4, or any paired acceptance gate.

## Fixture provenance

- React source identity: pinned workbench commit `5a472a9426e6e38993361da402cd4ec730feb369`.
- React route: `http://127.0.0.1:5173/`, re-opened in isolated agent-browser session `keycaps-findings-reopen`.
- Base retained c6 Sofle archive: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/dioxus-34726-fixture-models-false.boardstudio`, SHA-256 `c6ea3c0f72f999ee6736d1e65b6b2c36105b52c5a8d511285ffdc01695aa9d7e`.
- Derived project archive: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Embedded `project.json` SHA-256: `60ba2928d2dea11755e078a594c813d9467f302267c209f174854c8abec0e2ea`.
- Derivation: matrix `keys` profile set to SA; part `matrix/left-keys/r0c0` (`left-keys-SW1`) has keycap dimensions 7×1 units and SA profile. This saved project copy came from actual React Keycaps controls and is distinct from the missing original f2 fixture.

## Reopen and action observation

The saved derivative was imported into a fresh React browser session. The Keycaps Inspector showed 27 actionable Core-generated findings: 24 unsupported diode inputs and 3 keycap clearance collisions. No finding definitions were fabricated. Clicking the `left-keys-SW1 and left-keys-SW2` clearance finding routed to Layout, selected `left-keys-SW1` (React's first matching part target), and opened its Properties Inspector. The browser error list was empty after the journey.

- Re-open Keycaps screenshot: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-reopen.png`, 1280×577, SHA-256 `b74519dc5baeb7a47b0636c9feee81beefacfd61e0e420a7a4cd5af005c28477`.
- After navigation screenshot: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-navigation-react.png`, SHA-256 `9cd35248082725040c686462bda6381ba7a049e58e0663876712461096584a05`.

This verifies fixture reproducibility and the React target-selection oracle only. It is not a Dioxus comparison, camera-framing parity proof, keyboard-activation proof, or edit/history acceptance. The original f2 fixture identity/replay gate remains missing and unwaived.
