# 013: Toasts

Status: ready-for-agent
Type: build
Blocked by: [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [mapping](../../rust-ui-migration/mapping.md), [spec](../../rust-ui-migration/spec.md), [shared contract](../map.md#shared-contract)

## What to build

Transient results of one-shot actions appear as Toasts instead of inline text.

## Acceptance criteria

- [ ] A Toast provider is mounted in the shell; saving, copying, export started or finished and one-shot failures use it
- [ ] Blocking and persistent states stay inline (Export blockers, Case readiness, field errors); the classification rule in the mapping is applied to the shell's messages
- [ ] Toasts are announced to assistive technology and dismissible by keyboard; tests cover both
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
