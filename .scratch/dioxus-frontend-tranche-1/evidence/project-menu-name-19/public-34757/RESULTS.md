# Project menu visual delta on 34757

The 34757 replay verifies the changed menu layering and saved-key outline only.
The unchanged Current project rename/history journey is reused from the paired
34740 and 34756 evidence; it was not repeated for this visual check.

## Candidate and fixture

- URL: `http://127.0.0.1:34757/` (also served at `/boardstudio/`).
- Exact source: `d7742d46ad0e4dc05d5eb229c7c7de50e6f7945c`.
- Root reports eight fresh package commands and the affected strict page check
  passing (17.66 s strict check, 98.04 s package, zero drift). Provenance
  SHA-256:
  `f5e98f1b521a06c1b8cc91631e8fcdee550972489d8d5e3830a1a4c9e1c0f2e3`.
- Same exported imported-definition fixture as the prior paired journey:
  `layered-sofle-export.boardstudio`, SHA-256
  `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Candidate session: `parts19-candidate-936481b4b15c`; React reference session:
  `parts19-936481b4b15c`, pinned to `5a472a9426e6e38993361da402cd4ec730feb369`.

## Changed visible behavior

After opening the actual Project menu, the page topbar computed to
`position: relative; z-index: 40`. A hit-test over the former overlap region
resolved to the visible Project menu content, above the 2D/3D workspace
controls. The saved key outline computed to a 1 px stroke. These checks passed
on desktop and compact viewports in both light and dark themes.

The menu remained usable with its fixed Setup guide action and New/Open row;
the key outlines stayed legible on the preview.

## Captures

The retained candidate screenshots are under
`/home/chris/.local/share/boardstudio/retained-tmp/20261003/`:

| View | SHA-256 |
| --- | --- |
| Candidate desktop light | `b8c958ef0563b79f55a7ad0284a0089734c912a18ac397736763d474a1aacc48` |
| Candidate desktop dark | `1540bdadeaa935f3e44a6a7bcc64a5167d994430e30afc2d9d792d67a42c3ef4` |
| Candidate compact light | `b0cf0f86b636b98f6536c9f3c45e5981493847861bb2ad58039f018e0d6a1a07` |
| Candidate compact dark | `515b6c2822d0e707902e998b9e4bc2bba3ba67d513680f0005b3ff0f3c681475` |
| React desktop light | `f7bd19e9442c19ad4d8c261cd0e874af709fd24acffe14d4e31194319ecfb6f3` |
| React desktop dark | `6f4f715835c7156d78c0b400ac68d991b78f93838236dfa949c3e29a0c34d3c7` |
| React compact light | `c860f5f3e7b1961cca531d7306df8e92c69aa630562de87015756abe475c274d` |
| React compact dark | `d34e1fddf7769a6636432039d39bb4de2084bc73d84ea379893cc80d942cff75` |

The preceding Enter/trim, Project trigger and saved-card synchronization,
Undo/Redo, and reload behavior remain supported by the retained paired journey
in `/home/chris/.local/share/boardstudio/retained-tmp/20261003/project-name-*`
and `.scratch/dioxus-frontend-tranche-1/evidence/project-menu-name-19/public-34740/`.
This receipt closes only the changed menu layering/outline delta; it does not
close F2.2, F2.1, INT.2, or the overall frontend acceptance joins.

RF-009 evidence accounting was updated by retaining the exact candidate,
fixture, provenance, and changed-only replay boundary. No new refactoring
takeaway was observed.
