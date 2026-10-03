# 02: Toggle host PCB layers independently

**What to build:** A designer can independently show or hide each supported host PCB layer in the PCB canvas and retains the same transient layer choices while moving between Layout, PCB, boards, and back again.

**Blocked by:** 01: selected-board-host-scene (the real selected-board host scene is composed in the Dioxus canvas).

**Status:** ready-for-agent

- [ ] Layer controls include the dynamically derived footprint layers with reference front/back mapping and the standard Edge.Cuts, Courtyards, Pads, Holes and References groups.
- [ ] The PCB menu also exposes React's separate **Mounted modules → Footprints** control. It filters only actual mounted-module source footprint geometry when the selected board has module scenes; it does not alias host PCB parts, pads, or the Layout workspace's separate `Footprints` control. Keep module visibility in its existing separate owner/state.
- [ ] Copper, pads, drills, host footprint graphics, outlines and references respond independently as in React. The Mounted modules row is present even when no module scene exists; its geometry effect is qualified with a selected board that actually contains a mounted module, rather than inferred from the empty Sofle fixture.
- [ ] Host visibility uses the reference’s transient shared `hiddenLayers` lifetime across Design/PCB and board/workspace transitions; it is not reset on each view or board change and does not persist to the document.
- [ ] Labels, pressed state, group behavior, keyboard/focus, close/Escape and supported compact layout match the reference; stale scenes remain rejected during scope changes.
- [ ] Public paired browser evidence covers all host layer controls, the separate module Footprints row and its effect on a module-bearing fixture, shared visibility lifetime and reference fixture outputs; record RF observations.

**Paired candidate evidence (2026-10-02):** the current packaged candidate passes the host-layer controls and shared `hiddenLayers` lifetime on the original 70-part Sofle fixture. The React reference additionally shows a `Mounted modules` group with `Footprints`; the candidate does not. This is an open parity criterion, not an exception. The Sofle fixture has no `.wb-module-*` geometry, so it proves the control affordance and its separate state in React but cannot prove the geometry effect. See [public paired qualification](../evidence/pcb02-public-qualification-20261002/paired-walkthrough.md) for exact build, project and before/after archive identity.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
