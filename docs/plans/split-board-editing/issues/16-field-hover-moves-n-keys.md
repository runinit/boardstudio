# 16: Restore per-field amber "Moves N keys" preview

Status: ready-for-agent
Type: build
Blocked by: none (decision in [06](06-decide-relationship-highlighting-home.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Hovering or focusing an Inspector number field marks, in amber, the keys that a change to that
field would move, and the canvas hint reads "Moves N keys (n per half)". The preview clears on
leave, blur or selection change and never writes the document. This is the lost variant-D
behavior (ticket 02); drag previews from ticket 24 already ship.

## Acceptance

- [ ] Column, row, key and matrix fields each preview the correct key set, including mirrored keys.
- [ ] A mounted browser test covers hover and focus, then clearing.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
