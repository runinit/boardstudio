# 011: Consolidate shared camera and surface coordination

Status: ready-for-agent
Type: build
Blocked by: [008: Keycaps finding navigation](008-keycaps-assessment-and-finding-navigation.md), [009: Layout selection and Inspector ownership](009-layout-selection-and-inspector-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Keep pan, zoom, fit, measured canvas size and shared 2D/3D controls consistent across
workspaces while moving remaining camera/surface coordination behind its existing
shared responsibility rather than leaving workspace-specific math in Editor.

## Scope and constraints

- Reuse existing navigation/camera modules and established workspace projections.
  Move surface observation, camera basis/fit coordination and shared event plumbing
  coherently without inventing a second camera or changing formula semantics.
- Preserve the single active SVG reference, measured size, Runtime surface and observer
  lifecycle. Keep Layout and PCB feature transaction admission separate from common pan.
- Preserve shared snap/selection-kind/3D-mode identity, explicit panel-reveal priority,
  layer-dock ownership and the selection-cleanup bridge's scope/drop ownership.
- This is not a camera policy redesign or an opportunity to unify intentionally
  different accepted/display bounds, Keymap zoom ratios or finding source bases.

## Acceptance criteria

- [ ] Editor no longer contains workspace-specific camera-basis/fit assembly or observer lifecycle details that belong to existing shared navigation responsibility.
- [ ] Mounted measured-canvas pan, wheel zoom and explicit fit produce the same camera behavior in Layout, PCB, Keymap and Keycaps; view-only actions do not change the accepted document.
- [ ] Keycaps finding navigation retains its captured source basis and waits for the live destination surface; obsolete deferred fits/focus remain rejected after the move.
- [ ] Active SVG, Runtime surface and size stay synchronized through workspace and 2D/3D transitions. Replacing/unmounting a surface disconnects its old observer and cannot let an old owner clear the new surface/dock.
- [ ] Selection cleanup remains synchronously invoked by production scope changes and removes only its own registration on drop; no duplicate cleanup bridge is installed per workspace.
- [ ] Shared snap, view mode and selection-kind identities retain existing sharing/reset rules; workspace switching does not create per-workspace preference copies or reset Session camera implicitly.
- [ ] Desktop and compact transitions preserve panel widths/modes, ordinary defaults and explicit guide/placement reveals. Controls, labels and focus semantics remain accessible.
- [ ] Real unmount leaves no observer/listener updates into dropped Signals or stale camera/focus writes; DOM-root removal alone is not used as cleanup evidence.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite
and existing shared navigation/panel/layer-dock/canvas-prop tests. Use attached DOM,
nonzero dimensions and production styles where measured layout matters. Record
desktop/compact and 2D/3D evidence; defer actual GPU/provider qualification only to
the final packaged acceptance, not to an explanatory model.
