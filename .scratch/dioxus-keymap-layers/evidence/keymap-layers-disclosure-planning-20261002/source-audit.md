# Keymap Layers disclosure source audit

## Scope and provenance

This audit is limited to the right-side Keymap Layers section from the paired root browser capture. The same layered fixture was used in both applications: SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

- React oracle: pinned commit `5a472a9426e6e38993361da402cd4ec730feb369`; capture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keymap-Q-react-current.png`; SHA-256 `b0a0f662fd1f163fd65e94ee334252e5ecbfa19649d0c59bf650f607bb1f9f14`.
- Dioxus candidate: source `b9e74e37fc34948be9fee97c918b644200778bca`; build `frontend-layout-case-transport-integrated-20261002`; capture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keymap-Q-dioxus-34737.png`; SHA-256 `3f287666ccc38df2373e38b6e0d1cf76d08c7ce820b51155b1c348a83c8237fa`. Candidate was served at `http://127.0.0.1:34737/` and `/boardstudio/`.

The candidate source relevant to this observation is exactly present at the frozen planning base `ef4fa71a24287a5cdd7b9fd975e758d1614ed407`: `web/src/presentation/keymap/panel.rs` blob SHA-256 `c03740226a058d960858f2027cfea33ad2aa4db96438d0859d92df300a6987dc`. Candidate `web/assets/m1.css` blob SHA-256 is `096f24e191223580878d00d9a34458b0a54f031b2f1ef4ca357413f8ef756977`. The planning worktree is clean at that base before these draft files.

## Source behavior

Pinned React `app/src/ui/KeymapPanel.tsx` hash `60c062945d563db25605b327edf71c43cc5fb339` wraps the entire layer list and controls in `<InspectorSection title="Layers" defaultOpen>`. `app/src/ui/InspectorSection.tsx` hash `d4645c517d2013730d48fab1838f70fd9dc3d876` renders a native `<details>` with a `<summary>` and defaults it open; child content is part of that disclosure. `app/src/ui/keymap.css` hash `1b65a4290b7739cb822b2c29700875b7a69d278b` supplies section and layer spacing.

The candidate Dioxus Keymap panel instead uses a static `<section aria-label="Layers"><h3>Layers</h3>…</section>`. The section includes the existing list, Add layer, rename and conditional removal controls, help text, and a Dioxus-only Base status paragraph (“The first layer cannot be removed.”). The Base Remove button is already omitted. The observed compact and desktop section consequently has no collapse affordance, and displays an extra line of Base-only content.

## Observed owned gap

1. React shows an expanded disclosure summary with the chevron, “Layers”; Dioxus shows a non-interactive heading. Dioxus users cannot collapse the content.
2. React hides all section descendants on collapse by native details behavior. The new disclosure should contain the same Dioxus layer content as one unit.
3. Dioxus adds an explanatory Base protection sentence absent from React. The Remove layer action is already absent for Base in both, so remove only the extra presentation sentence and keep current operation behavior.
4. The section’s vertical spacing differs in the paired captures. Limit tuning to the owned Layers block so it does not collide with shared Inspector/shell work.

## Excluded paired gaps

The React workspace context bar and “2D / 3D assembly / Footprints” control group, Dioxus physical-instance/group selectors, shared Objects navigation/control arrangement, and footer differences are not assigned to this child. They are tracked by existing shared-shell, viewer, Objects, and footer owners. No new dependency or task-graph edge is justified by this observation.

## Validation boundary

This is a planning/source audit only. No Dioxus source was edited, no new test was run, and no behavior beyond the retained static screenshots/source inspection is claimed. Implementation must add a production-mounted component test plus paired browser verification before this child or any parent can be considered complete.
