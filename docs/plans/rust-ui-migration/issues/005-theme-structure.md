# 005: How Themes and Appearance are structured

Status: resolved
Type: grilling
Blocked by: [002](002-rust-ui-catalogue-facts.md)
Parent: [Map](../map.md)

## Question

How do two Themes × light/dark Appearance reach components: which colour, radius and
font names form the contract (rust-ui's set, plus any BoardStudio additions such as
canvas or selection colours); how a Theme and Appearance are applied to the page; how the
preference is stored next to Appearance; and how the picker presents both choices (it lives in the header, [ADR-0040](../../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md)). Also: control sizing. Ours are 40px (36px compact, 44px touch); rust-ui's default is 36px and its 40px size is `Lg`, so does the Original Theme tighten them or keep today's sizes through size tokens?

## Answer

Decided with the user, 2026-10-21, in [theme-contract.md](../theme-contract.md); all seven proposals accepted.

- **Attributes:** `data-appearance="light|dark"` and `data-theme="original|shadcn"` on `<html>`, renaming today's `data-theme`
  (which holds Appearance) to match the glossary; one `@custom-variant dark` line keys copied `dark:` classes to it.
- **Token contract:** rust-ui's token names are the base, mapped by meaning (our `accent` brand colour becomes `primary`, our
  `muted` text becomes `muted-foreground`, rust-ui's `accent` is the hover surface), plus a short list of BoardStudio extras
  (canvas, border-strong, toolbar, 2D geometry, grid, keycap). `--wb-*` retires. Graphite has no success / warning / info fills,
  `destructive-foreground` or slider / scrollbar colours, so those are authored.
- **A Theme sets** colours, `--radius`, fonts and shadows, **never layout metrics**: control sizes (40 / 36 / 44px), header,
  docks, spacing and type scale are shared and identical in both Themes. Copied rust-ui sizes read those tokens. This answers the
  control-sizing question.
- **shadcn Theme:** rust-ui's own neutral oklch palette verbatim with its three gaps filled, and bundled Inter (OFL).
  **Original:** today's Graphite values in rust-ui names, Source Sans 3 and Source Code Pro, `--radius` 10px.
- **Preferences:** Appearance in `boardstudio:v2:appearance` (reads the legacy `boardstudio:v2:theme` once); Theme in
  `boardstudio:v2:workbench-theme`, default Original; browser preferences, not in the project file.
- **3D renderer** keeps its own light and dark palettes in both Themes; the 2D canvases follow the Theme through tokens.
- **Header control:** one button with Appearance and Theme radio groups (ADR-0040).
- **Follow-on:** [013](013-prototype-theme-look.md) renders both Themes before the spec fixes them.
