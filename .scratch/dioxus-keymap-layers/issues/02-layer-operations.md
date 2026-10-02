# 02: Add, rename, and remove permitted keymap layers

**What to build:** A designer can add a layer, rename any permitted layer including Base, and remove only a non-base layer while preserving keymap history and stable layer identity.

**Blocked by:** 01: keymap-projection-selection (proven layer-ID selection and keymap read projection). The parent F3.1 selection/read-model acceptance join remains required for integrated acceptance.

**Status:** ready-for-agent

- [ ] Use existing `EditKeymap` changes and normal edit/history flow; Base may be renamed; only the first/base layer is protected from removal.
- [ ] Preserve rename-on-blur, 32-layer add limit, 32-character name input limit, existing core validation/errors, and current invalid-input recovery.
- [ ] Keep selection by layer ID; after removing the selected non-base layer, select the established valid fallback. Preserve other layer bindings/sensors and all keymap data.
- [ ] Changes survive Undo/Redo and save/reopen; no new command, schema field, or public API is added.
- [ ] Public browser evidence covers add/rename Base and non-base/remove/fallback, protected removal, limit/invalid feedback, identity, Undo/Redo and reload; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
