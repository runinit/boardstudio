# rust-ui icons and their Solar replacements

Research for [ticket 006](../plans/rust-ui-migration/issues/006-solar-icons-for-rust-ui.md)
of the rust-ui migration map. Date: 2026-10-21.

## Sources (pinned)

- rust-ui/ui at `7120636e68556ddd023a4ec8e083663c7b745318` (`main` on 2026-10-21).
  Components: `app_crates/registry/src/ui/*.rs`. Icons crate: `crates/icons/`.
  Workspace dependency (root `Cargo.toml:58`):
  `icons = { version = "0.18", features = ["dioxus", "dioxus_animated"] }`.
- Solar icon names and licence: Iconify `json/solar.json` at iconify/icon-sets
  `34ae9f5b718f9edafa4fba1f205e10b4bf802e5d` (last change to that file, 2026-09-26).
  Its `info` block gives author 480 Design and licence CC BY 4.0, with 8,706 icons.
- BoardStudio at `94766ab0b` (this branch's base).

## Main finding: BoardStudio has no Solar wrapper or Solar assets

The ticket and map assume BoardStudio already has an "icon wrapper" and a "local Solar
asset set". It has neither, on this commit or on any branch:

- `git grep -i solar` matches nothing at `94766ab0b`. `git log --all -S solar` touches
  only planning docs: `docs/plans/ui-design/plan.md`, `docs/plans/delivery-queue.md`,
  `typed-core-edits/issues/140-ui-design-impeccable.md`, and the rust-ui map/ADR-0034.
- `docs/plans/ui-design/plan.md` (added in `9d3904c80`) planned "a reusable accessible
  icon wrapper" and Solar assets, then says outright: "No icons have been imported."
- Icons today are hand-drawn inline `svg { … }` elements, each carrying its own paths:
  `TabIcon` in `web/src/presentation.rs:727`, panel buttons in
  `web/crates/ui-shared/src/panels.rs:90,111,457,468`, and toolbar icons in
  `web/crates/layout/src/objects/layout_toolbar.rs:673,729` and
  `web/crates/case/src/shared_viewer.rs`. They use a mix of 16 and 20 unit viewBoxes
  with `stroke="currentColor"`, mostly `stroke-width 1.5`, and `aria-hidden`.
- `web/assets/licenses/README.txt` covers Cadrum, Open CASCADE, DejaVu and ttf-parser.
  It does not mention Solar.

So "vendored" is **no** for every row below. Keeping Solar (ADR-0034) means first
building the wrapper and the asset set. See "What adding Solar needs".

## How rust-ui wires icons

- 21 of the 87 files in `registry/src/ui/` use the crate. Each imports named components
  with `use icons::{…}`. The one exception is `dropzone.rs:231`, which uses the path
  `icons::Upload`.
- Every icon is a Dioxus component, `fn Name(class: Option<String>) -> Element`
  (`crates/icons/src/dioxus/compatibility.rs`). It forwards to `DioxusIcon`, then to
  `SvgIcon` (`dioxus/svg_icon.rs`). `SvgIcon` renders a 24×24 `viewBox="0 0 24 24"`
  SVG with `fill="none" stroke="currentColor" stroke-width="2"`, round caps and joins,
  `data-name`, and a `<title>Rust/UI Icons - Name</title>`. It does not set
  `aria-hidden`. `class` is merged with `tw_merge`.
- `class` is the only prop anywhere. Size and colour come only from Tailwind classes
  (`size-4`, `text-muted-foreground`, `opacity-*`, `group-aria-*:opacity-100`), or from
  the parent's `[&_svg]` / `[&>svg]:size-4` selectors when no class is passed
  (`X {}` in dialog and sheet, `ChevronRight {}` in breadcrumb).
- **Animated: none.** The `dioxus_animated` feature only adds `*Animate` components
  (`AnimatedIcon`, `PlusAnimate`, `SearchAnimate`, …), and no registry component uses
  one. Motion comes only from Tailwind on static icons: `animate-spin` (spinner),
  `animate-bounce` (dropzone Upload), and `rotate-180` transitions (header and
  navigation_menu ChevronDown).
- The icon artwork is Lucide (`crates/icons/README.md`: "Dioxus port of lucide.dev
  icons"). The crate is MIT. The in-repo `crates/icons` is version 0.19.0, while the
  workspace pulls 0.18 from crates.io.

A Solar drop-in therefore only needs a component per icon that takes
`class: Option<String>` and renders `stroke="currentColor"`. Solar Linear bodies already
use `fill="none" stroke="currentColor" stroke-width="1.5"` on a 24-unit grid. That
matches BoardStudio's current 1.5 stroke, but is thinner than Lucide's 2.

## Mapping

There are 35 distinct icons. Solar names are Iconify `solar:` names in the Linear style.
Each was checked to exist in the pinned `solar.json`. "Vendored" means present in
BoardStudio today; it is no for all 35.

| rust-ui component (file:line) | Lucide icon | How wired | Solar replacement | Vendored |
|---|---|---|---|---|
| checkbox (`checkbox.rs:43`), stepper (`stepper.rs:153`), select (`:258`), multi_select (`:154`), dropdown_menu (`:392`), menubar (`:84,135`), command (`:664`) | `Check` | `class` (size-3.5/4; select-style items hide it with `opacity-0 group-aria-*:opacity-100`) | `check-linear` | no |
| select (`:65,193`), multi_select (`:214,340`), header (`:250`), navigation_menu (`:298`), data_grid (`:680`) | `ChevronDown` | `class`; header and navigation_menu rotate it 180° on open | `alt-arrow-down-linear` | no |
| select (`:188`), multi_select (`:335`) | `ChevronUp` | `class` (scroll-up button) | `alt-arrow-up-linear` | no |
| context_menu (`:379`), dropdown_menu (`:337`), menubar (`:426`), carousel (`:192`), breadcrumb (`:83`, no class) | `ChevronRight` | `class` (sub-menu caret, next) | `alt-arrow-right-linear` | no |
| carousel (`:165`) | `ChevronLeft` | `class` | `alt-arrow-left-linear` | no |
| input_phone (`:486`) | `ChevronsUpDown` | `class` | `sort-vertical-linear` (judgement call) | no |
| data_grid (`:685`) | `ArrowUpNarrowWide` | `class` (sort ascending) | `sort-from-bottom-to-top-linear` | no |
| data_grid (`:689`) | `ArrowDownWideNarrow` | `class` (sort descending) | `sort-from-top-to-bottom-linear` | no |
| data_grid (`:697`) | `CircleX` | `class` (clear sort) | `close-circle-linear` | no |
| data_grid (`:720`) | `PanelLeft` | `class` (pin column) | `panel-left-linear` | no |
| data_grid (`:710`) | `PanelLeftClose` | `class` (unpin) | `panel-left-close-linear` | no |
| data_grid (`:734`) | `EyeOff` | `class` (hide column) | `eye-closed-linear` | no |
| dialog (`:171`), sheet (`:152`), input_phone (`:540`) | `X` | none in dialog and sheet (sized by parent selector); `class` in input_phone | `close-linear` | no |
| breadcrumb (`:70`) | `Ellipsis` | passes the component's own `class` prop through | `menu-dots-linear` | no |
| input_phone (`:492`) | `Search` | `class` | `magnifer-linear` (Solar's spelling) | no |
| input_otp (`:89`) | `Minus` | `class` (separator) | `minus-linear` | no |
| spinner, `Spinner` (`spinner.rs:10`) | `LoaderCircle` | `class` merged with `size-4 animate-spin` | `loader-linear` (an open arc) | no |
| spinner, `SpinnerCircle` (`spinner.rs:19`) | `Loader` | `class` merged with `size-4 animate-spin` | `loader-linear` (Solar has no rayed spinner) | no |
| dropzone (`:231`) | `Upload` | `icons::Upload` path, `class` `size-10 animate-bounce` | `upload-linear` | no |
| dropzone (`:334`) | `FileImage` | file-kind enum match, passes `class` through | `file-image-linear` | no |
| dropzone (`:335`) | `FileAudio` | same | `file-audio-linear` | no |
| dropzone (`:336,340`) | `FileText` | same (Pdf, Text, Other) | `file-text-linear` | no |
| dropzone (`:337`) | `FileArchive` | same | `file-zip-linear` | no |
| dropzone (`:338`) | `FileCode` | same | `file-code-linear` | no |
| dropzone (`:339`) | `FileSpreadsheet` | same | `file-spreadsheet-linear` | no |
| dropzone (`:427`) | `LayoutList` | `class` (list view toggle) | `layout-list-linear` | no |
| dropzone (`:434`) | `LayoutGrid` | `class` (grid view toggle) | `grid-2x2-linear` | no |
| theme_toggle (`:48`) | `Sun` | `class` | `sun-linear` | no |
| theme_toggle (`:50`) | `Moon` | `class` | `moon-linear` | no |
| workflow (`:437,483`) | `Pencil` | `class` | `pen-linear` | no |
| workflow (`:443`) | `Copy` | `class` | `copy-linear` | no |
| workflow (`:456,495`) | `Trash2` | `class` | `trash-bin-minimalistic-linear` | no |
| workflow (`:552`) | `Plus` | `class` | `add-linear` | no |
| workflow (`:558`) | `SquareDashedMousePointer` | `class` | `cursor-square-linear` | no |
| workflow (`:565`) | `Clipboard` | `class` | `clipboard-linear` | no |

Imported but never rendered: none. Every name in a `use icons::` line has a render site.
The other components either draw no icon or take icon `children` from the caller:
`button` / `item` / `empty` / `sidenav` `Icon` variants, `DropzoneIcon`, and
`MarkerIcon`. Those slots will hold BoardStudio's own Solar icons.

## What adding Solar needs

- **Assets:** copy the 35 bodies above (fewer if the migration drops `workflow`,
  `dropzone`, `data_grid` or `input_phone`) from the pinned Iconify `solar.json`, or
  from the upstream that `docs/plans/ui-design/plan.md` names (saoudi-h/solar-icons).
  Copy only the icons in use, as that plan requires.
- **Wrapper:** one component per icon (or one `Icon { name }`) with
  `class: Option<String>`, which makes it a drop-in for the `use icons::X` sites.
  rust-ui's icons emit a `<title>` and no `aria-hidden`. The plan asks for an
  *accessible* wrapper, so decorative use should set `aria-hidden="true"`, as
  BoardStudio's inline SVGs do now.
- **Licence and provenance:** Solar artwork is CC BY 4.0 by 480 Design, which requires
  attribution. Add a Solar entry to `web/assets/licenses/README.txt` (source, pinned
  commit, licence), plus the CC BY 4.0 text or link. If the wrapper code comes from
  saoudi-h/solar-icons, also add its MIT notice and LICENSE-THIRD-PARTY
  (`docs/plans/ui-design/plan.md` cites both). The upstream-commit provenance note for
  each copied rust-ui file (map, settled scope) should also record that the `icons`
  imports were swapped for Solar.

## Caveats

- Solar picks are visual judgement calls wherever names differ. Check these on screen:
  `ChevronsUpDown` → `sort-vertical`, `Loader` → `loader`, and
  `SquareDashedMousePointer` → `cursor-square`.
- I did not check whether `panel-left-*`, `file-audio`, `file-code`, `loader` and
  `grid-2x2` (all present in Iconify's set) also exist in saoudi-h/solar-icons or in
  480 Design's original Figma. If the assets come from a source other than Iconify,
  re-check those names.
