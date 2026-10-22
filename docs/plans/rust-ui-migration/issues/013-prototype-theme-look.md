# 013: Prototype the Original and shadcn Themes on copied components

Status: claimed
Type: prototype
Blocked by: None
Parent: [Map](../map.md)
Related: [005](005-theme-structure.md), [001](001-compile-rust-ui-against-our-build.md)

## Question

Do the Original (Graphite values in rust-ui names, 10px radius, Source Sans 3) and shadcn (rust-ui's neutral palette, Inter)
Themes look right, light and dark, on the copied Button, Input, Field, Select, Alert, Card, Badge, Toolbar and Sidenav? Build both
token blocks from [theme-contract.md](../theme-contract.md) in the scratch crate with a Theme and Appearance switch, view the
result with the user, and record the adjustments (radius, the authored success / warning / info fills, shared control sizes) that
the contract needs. Throwaway code; the findings are the output.

## Comments

**2026-10-21, prototype built; awaiting the user's verdict (ticket stays claimed).**

Prototype (throwaway, gitignored): `.scratch/theme-proto`, run with `.scratch/theme-proto/run.sh`, then
`http://127.0.0.1:8393/?variant=A` (A Original light, B Original dark, C shadcn light, D shadcn dark; bottom bar or arrow keys).
It is a mini workbench (56px header with workspace links, Command search button and Export, Objects dock and Inspector dock on the
copied Sidenav, Layout toolbar strip, a 2D canvas drawn with the geometry tokens, footer) built from copied rust-ui Button, Input Group,
Field, Select, Checkbox, Alert, Badge, Card, Toggle Group, Kbd, Separator and Sidenav. Tokens come from
[theme-contract.md](../theme-contract.md): Original from today's Graphite values, shadcn verbatim from rust-ui's `tailwind.css`.

Findings:

- **`selection` collided.** rust-ui's `--selection` is saturated orange in the shadcn palette and meant for text highlight; used for the
  selected tree row and the selected key it rendered loud orange in both shadcn variants. Fix tested: add BoardStudio extras
  `selected` / `selected-foreground` (Original: today's selection tint; shadcn: `accent`), and leave `selection` for text highlight.
- **Original's fields went white-on-white.** Today's fields are filled (`#f7f8fa`); rust-ui inputs are transparent. Fix tested: an extra
  `field` token (Original: today's fill; shadcn: transparent, 5% white in dark) used by Input Group and Select trigger.
- **Shipped Sidenav is not an in-flow dock.** Its collapsible forms are `fixed inset-y-0`, full viewport height and hidden below 768px
  (it would cover the header); the `collapsible: None` form is a plain in-flow `aside` and is what the prototype uses.
- **Inter costs 48 KB** (variable woff2, latin, OFL-1.1, `@fontsource-variable/inter` 5.3.0); Source Sans 3 is 170 KB and Source
  Code Pro 90 KB, already bundled.
- No console errors; fonts need copying next to the generated CSS (`run.sh` does it).
