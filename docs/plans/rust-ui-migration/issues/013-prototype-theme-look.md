# 013: Prototype the Original and shadcn Themes on copied components

Status: resolved
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

**2026-10-21, round 2 (user feedback: "looks good except the white outline around everything; focus more on a cards style").**

Rebuilt the prototype as a cards style (`.scratch/theme-proto`, same URL and variants): a "desk" background with floating rounded cards for
the header, Objects dock, canvas and Inspector; filled controls instead of outlined ones (Secondary and Ghost buttons, a filled Toggle
Group, filled input fields, tinted Alerts, soft inner cards); the drawing surface is an inset rounded panel; separation by fill and
elevation, no borders. What it needed:

- Copied rust-ui components hard-code borders (Card, Alert, Input Group, Select trigger, Toggle Group, Outline Button), including
  `dark:`-prefixed ones that a plain override does not replace (the dark Alert kept a red outline until `dark:border-transparent`).
  The build edits the copies, not call sites.
- The page background must be the darker "desk" so cards read: Original `background` would become today's canvas colour (light `#E9EEF2`,
  dark `#181B21`) with `card` white / `#242831`, instead of `#f7f8fa` / `#1B1E25`.
- The prototype header is a 48px card; ADR-0040 says 56px, solid and bordered. To be settled with the user.

## Answer

Decided with the user, 2026-10-21, after two rounds on the prototype (`.scratch/theme-proto`, captured on branch `prototype/theme-look`):

- **Both Themes approved in light and dark, in a cards style.** Round 1 looked good except "the white outline around everything"; round 2
  moved to cards on a desk, filled controls and no borders, and was approved. Details in
  [theme-contract.md](../theme-contract.md) section 8 and [ADR-0040](../../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md).
- **Contract changes accepted:** extras `selected` / `selected-foreground` (rust-ui's `selection` stays for text highlight, since in the
  shadcn palette it is saturated orange) and `field`; `background` becomes the darker desk; the copied components' built-in borders,
  including `dark:` rules, are removed in the copies.
- **Header:** a 56px borderless card (the prototype used 48px).
- Inter costs 48 KB; the shipped Sidenav is not an in-flow dock, only its `collapsible: None` form is usable and needs adapting.

Prototype captured on the throwaway branch `prototype/theme-look` (pushed, no pull request, never merged): source, token file, `run.sh`,
screenshots and a README. The main plan branch keeps only the decision.
