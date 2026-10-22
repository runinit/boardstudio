# PROTOTYPE: Theme look (wayfinder ticket 013), throwaway

Never merged. Question: do the Original and shadcn Themes, light and dark, look right on copied rust-ui components in a mini workbench?
Answer (2026-10-21): yes, in a cards style (cards on a desk, filled controls, no borders). See
`docs/plans/rust-ui-migration/theme-contract.md` section 8 and `issues/013-prototype-theme-look.md` on the main plan branch.

Run: `./run.sh` then open http://127.0.0.1:8393/?variant=A (A Original light, B Original dark, C shadcn light, D shadcn dark).
Needs `dx` 0.7.10 and network for the Tailwind CLI. Screenshots in `shots/` (`cards2-*` is the approved round).
Components under `src/ui` are copied from rust-ui/ui at 7120636e (MIT) with one edit: `Button`'s `Link` became `a`.
Fonts: Source Sans 3, Source Code Pro (bundled with BoardStudio) and Inter Variable (OFL-1.1, @fontsource-variable/inter 5.3.0).
