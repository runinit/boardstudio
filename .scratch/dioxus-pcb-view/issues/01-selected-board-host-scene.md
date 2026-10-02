# 01: Show host PCB geometry in the selected-board canvas

**What to build:** Opening PCB displays the selected board’s actual host scene in the canvas, including its parts and supported PCB artwork, so a designer can inspect the real board in its own scope.

**Blocked by:** INT.1 is the parent start edge. Before feature implementation, the coordinator must assign a PCB-specific page mount and prove the accepted scene/definition inputs, board scope, canvas owner and selection callback. INT.1 does not supply this PCB contract.

**Status:** ready-for-agent

- [ ] A private Dioxus PCB canvas/controller composes the existing accepted board/scene and definition sources, using the reference’s front/back layer mapping and transforms; no reliance on Layout’s existing `FootprintGraphics` as the full PCB path.
- [ ] Show supported host parts, pads/drills, copper, Edge.Cuts, courtyards, references and technical artwork from their real existing sources; unavailable data is reported truthfully rather than drawn as a fake provider result.
- [ ] Board changes and workspace exit reject stale scene publication; inspection does not edit project data, history, or export inputs.
- [ ] Paired public browser evidence checks geometry and selection on single- and multi-board fixtures against React, including loading/error/empty cases; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
