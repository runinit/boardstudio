# 161: Curated layout-only library demos

Status: ready-for-agent
Type: build
Blocked by: none
Related: [138](138-retire-shipped-demos.md)

## Scope

The user reopened a narrow demo scope on 2026-10-16. Keep the library launchpad and saved-keyboard shelf as the first screen. Give demo, new, and import equal weight; keep the right-side starter guidance visible. Ship three synthetic editable layout examples (Split 42, Studio 60, Macro 12). Opening a demo creates a separate local project copy. Examples demonstrate geometry only and make no electrical or fabrication-readiness claim. Do not restore retired hardware-specific boards, assets, recipes, or sign-off work from ticket 138.

## Acceptance

- No saved keyboards: demo, new, and import actions have equal visual weight; shelf empty state and starter guidance remain visible.
- Saved keyboards remain resumable from the shelf.
- Each shipped example opens as an independent editable local copy with its stated layout; user projects are preserved.
- Import remains limited to `.boardstudio` project files.
- Desktop and mobile layouts have no clipping or horizontal overflow; controls remain keyboard accessible.
- Finish review reaches `ship` against `.impeccable/mocks/decision/library-launchpad.png`; reconcile residual type, diagram, and ground differences called out in `.impeccable/review/diff/hero/report.json`.
- Update product documentation to describe the curated layout-only examples without hardware claims.
