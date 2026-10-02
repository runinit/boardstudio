# 02: Toggle host PCB layers independently

**What to build:** A designer can independently show or hide each supported host PCB layer in the PCB canvas and retains the same transient layer choices while moving between Layout, PCB, boards, and back again.

**Blocked by:** 01: selected-board-host-scene (the real selected-board host scene is composed in the Dioxus canvas).

**Status:** ready-for-agent

- [ ] Layer controls include the dynamically derived footprint layers with reference front/back mapping and the standard Edge.Cuts, Courtyards, Pads, Holes and References groups.
- [ ] Copper, pads, drills, footprint graphics, outlines and references respond independently as in React; module-specific layer state remains separately owned.
- [ ] Host visibility uses the reference’s transient shared `hiddenLayers` lifetime across Design/PCB and board/workspace transitions; it is not reset on each view or board change and does not persist to the document.
- [ ] Labels, pressed state, group behavior, keyboard/focus, close/Escape and supported compact layout match the reference; stale scenes remain rejected during scope changes.
- [ ] Public browser evidence covers all host layer controls, shared visibility lifetime and reference fixture outputs; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
