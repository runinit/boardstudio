# 001: Walking skeleton: one copied Button, both Themes, one migrated control

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [spec](../../rust-ui-migration/spec.md) section 3, [theme contract](../../rust-ui-migration/theme-contract.md), [shared contract](../map.md#shared-contract)

## What to build

The smallest end-to-end path through every layer of the new design. In the running app, a user can choose a Theme (Original or shadcn) next to the existing Appearance choice in Workspace settings (ticket 010 later moves both into the header), and the choice restyles a real control that now comes from the copied rust-ui `Button`. One test finds that control by accessible role and name through a new test helper. This proves the dependencies, the copy-and-provenance routine, the token pipeline, the attributes, the saved preferences and the test rule before anything else builds on them.

## Acceptance criteria

- [ ] rust-ui `Button` is copied into the shared UI crate with a provenance note naming the upstream path, commit `7120636e68556ddd023a4ec8e083663c7b745318` and the MIT notice; `tw_merge` and `strum` are added; the router `Link` branch becomes a plain `a`; the page and wasm builds pass on `dioxus =0.7.10`
- [ ] `<html>` carries `data-appearance` (light or dark, resolved) and `data-theme` (`original` or `shadcn`) from saved preferences; Appearance is stored under `boardstudio:v2:appearance` and reads the legacy `boardstudio:v2:theme` value once; Theme is stored under `boardstudio:v2:workbench-theme` and defaults to Original
- [ ] A minimal set of the contract's tokens exists for Original and shadcn in light and dark, enough for `Button`, plus the single `dark` variant line keyed to `data-appearance`
- [ ] All the buttons of one small existing component use the copied `Button`; choosing each Theme and Appearance visibly restyles them without reload
- [ ] A role-and-name test helper exists and one browser test uses it to find and click a migrated button; the existing tests for the touched files pass
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
