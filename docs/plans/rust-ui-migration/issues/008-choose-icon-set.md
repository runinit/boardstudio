# 008: Choose the workbench icon set

Status: claimed
Type: grilling
Blocked by: None
Parent: [Map](../map.md)
Related: [006](006-solar-icons-for-rust-ui.md)

## Question

With no Solar icons in the app today, which icon set do rust-ui components and the
workbench use: rust-ui's Lucide `icons` crate as shipped, newly vendored Solar (35 icons
for the catalogue plus the app's own, CC BY 4.0 attribution), or today's inline SVGs?
And does the chosen set differ between the Original and shadcn Themes?

## Comments

**2026-10-21, findings (grilling in progress; ticket claimed).**

- **rust-ui's `icons` crate (0.18.3, MIT, Lucide artwork)** renders every icon through one component, `DioxusIcon`, that looks the icon
  up in a static registry of about 1,680 entries. Measured in the compile spike: the release wasm is 2,472 KB with the crate and
  2,013 KB with four stub icons instead, so the crate adds about 459 KB uncompressed to a demo.
- **Accessibility:** the component hard-codes `<title>Rust/UI Icons - {name}</title>` inside every SVG and sets no `aria-hidden`, so
  the text becomes part of a button's accessible name (seen in the spike: a Select trigger read "bRust/UI Icons - ChevronDown").
  The planned role-and-name tests would break. The SVG is also fixed at 24px with stroke width 2.
- **Our icons today:** 84 inline `svg` elements with 71 distinct path strings, mostly 20x20 viewBox (47 uses) and stroke width 1.5,
  already `aria-hidden` (91 uses in the code). Some are geometry drawings, not icons.
- **Solar:** every one of rust-ui's 35 icons has a Solar counterpart in Iconify's `solar` set (CC BY 4.0, 480 Design, attribution
  required); BoardStudio has never imported any Solar icons (see [006](006-solar-icons-for-rust-ui.md)).
