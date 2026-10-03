# 16: Case workspace shell and contextual Objects parity

**Parent:** [09: Case assembly Objects tree and contextual Inspector](09-case-contextual-objects-inspector.md); F7.2 authored Case workspace.

**What to build:** Make the Case workspace canvas-first and match the reference's contextual assembly navigation and common viewer controls, while preserving current Case selection, visibility, editing, and preview owners.

**Blocked by:** None (Issue 09's tree, selection and panel owners are already mounted; this is a composition follow-on).

**Status:** ready-for-agent; implementation is in progress in the Case queue.

- [ ] Case Objects shows real assembly roots and only the active root's contextual authored/generated and PCB rows. Selecting or expanding another real root navigates through the existing accepted board/instance scope; duplicate generic board and physical-instance selectors do not occupy the tree.
- [ ] Each represented assembly, layer, and component visibility control uses an eye affordance with an accessible Show/Hide name and the existing scoped Case display preference callback.
- [ ] The Case viewer uses the available work-area height. Fit, camera navigation, and Shaded/Wireframe/Hybrid controls are inline for Case without changing other SharedViewer consumers; specialist section/explosion/layer settings remain available.
- [ ] Status, pending, and failure feedback stays truthful and available without redundant ready-state paragraphs displacing the canvas.
- [ ] The current selected-part Inspector breadcrumb, Objects footer, canvas badge, and existing mechanical/body editor remain scoped and functioning.
- [ ] Preserve exact accepted identities, selection and display-preference behavior; no new Session, renderer, geometry, or project-format authority is introduced.
- [ ] Retain the paired source/build/fixture/browser receipt and an explicit scoped RF disposition. Keep all F7.2/F7.3/F7.4 and downstream acceptance gates open until their parents are independently joined.
