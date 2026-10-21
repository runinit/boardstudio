# 004: Give PCB ownership of wiring and plan delivery

Status: ready-for-agent
Type: build
Blocked by: [003: Keymap workspace ownership pilot](003-keymap-workspace-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Keep PCB editing and its current electrical plan usable across workspaces while PCB
owns its page-shell controllers, projections, admission and surfaces. Keymap, guide
and Export continue consuming the same resolved plan when PCB is not visible.

## Scope and constraints

- Move wiring-controller/action mounts, board-only versus UI source construction,
  firmware-position/input projections, Inspector choice and PCB-specific hit/action
  admission into the existing PCB workspace module.
- Retain one unconditional Editor-lifetime plan query and expose its actual shared
  result/signal. Rewire consumers rather than mounting another query for each.
- Preserve readiness, locked-item and modified-hit behavior. Common drag, pointer
  capture and camera mechanics remain shared; do not extract Layout's gesture chain.
- Keep existing Parts/module and Add-to-Layout routes operational through explicit
  ports. Parts will own its side of those routes in the next slice.

## Acceptance criteria

- [ ] PCB owns controller/source/projection assembly and constructs its Objects, toolbar, canvas and Inspector inputs; Editor no longer rebuilds their controller details.
- [ ] Mounted PCB net/input and firmware-position actions still target the correct board, update accepted values and retain pending/failure and Undo behavior.
- [ ] A current plan reaches Keymap, guide and Export through one shared Editor-lifetime query, including when another workspace is active. Existing downstream readiness decisions remain unchanged.
- [ ] With a filtered plan or edit reply held, change boards and release it; obsolete plan/action results cannot replace the current board's projection or write another control's feedback.
- [ ] PCB hit selection, empty selection, modified hits, locked parts and readiness refusal retain existing outcomes. PCB-specific admission is not replaced with Layout's different rules.
- [ ] Add-to-Layout, controller choosing and mounted-module browsing still reach their original destination through the existing shared placement/navigation capabilities.
- [ ] Shared surface/drag ownership and physical-instance admission remain intact; electrical plan identity is not changed into a physical-instance-specific query.
- [ ] Obsolete PCB forwarding/input assembly is removed after consumers migrate, without changing lower feature contracts or dependency direction.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite
and relevant PCB wiring/controller/Inspector tests. Include a real PCB action, shared
plan consumption outside PCB and a stale-board completion case at the composition or
existing controlled-adapter seam. Follow the plan's verification contract.
