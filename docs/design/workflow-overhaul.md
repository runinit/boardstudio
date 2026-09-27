# Board Studio workflow overhaul

Mode: Operate. Scope: the shared workbench, setup, editing controls, settings,
and export. This extends the Keyboard Lab identity; it does not replace the
palette, typography, document model, geometry tools, or exporter contracts.

## Direction contract

**THESIS:** Navigation must survive every panel state. Keep view navigation
outside the object tree, and provide visible returns from temporary tasks.

**OWN-WORLD:** Retain Source Sans 3, precise measured fields, neutral light and
dark panels, blue actions, and the existing geometry colors. Use compact labeled
controls and rule-separated content.

**STORY:** Choose Layout, PCB, or Case; select an object; edit its properties;
review and export. The optional guide can be left and resumed without losing work.
Project files, setup, and workspace preferences are reached through the project
menu; preferences remain a separate page with an explicit return.

**FIRST VIEWPORT:** One 44px header contains the project identity, save state,
Layout/PCB/Case/Parts navigation, and Export. Below 821px, the view tabs become a
native Workspace selector beside the drawer controls. Undo/redo sit beside the
canvas controls on desktop and in the project menu on compact screens. The Add
action stays with Objects. Parts and Export provide a return within their content.

**FORM:** Code-led extension of the existing workbench, with explicit task exits
and contextual object lists. No replacement visual world or concept roll applies.

**FINISH:** unreviewed and undocumented is unfinished; this build ends with the
finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Interaction rules

- Layout, PCB, Case, and Parts remain reachable with either panel collapsed. The object
  tree shows the active editing context; parts and geometry retain their identities.
- Parts and Export return to the previous design view. Temporary inspector pages
  expose Back to selection. Command popovers and compact drawers expose Close.
- Project → Setup guide is the entry point. Back to objects leaves it.
  A guided matrix operation returns to the current guide step when cancelled or
  placed. Creating a new project from Parts returns to Design and opens setup.
- Project → Workspace settings owns appearance and restoring panel layout, with
  Back to project menu and Close always available. Project also owns naming,
  opening, and saving portable copies. The Project browser lists saved keyboards
  with search and current-project priority, while its separate Demo keyboards
  gallery starts independent editable copies.
- 3D previews do not expose 2D transform commands. Footprint visibility belongs
  beside the preview switch, rather than among snap parameters.
- Export shows formats, availability, reasons, and links to wiring and case
  review. Existing revision and readiness checks remain authoritative.
- Compact drawers and their backdrop are bounded by the actual workspace,
  including when the footer wraps. An unmounted panel never creates a backdrop.
- The viewport shell cannot scroll when focus enters a moving drawer; scrolling
  belongs to the panel content and the Export page.

## Validation scope

The workflow browser regressions cover returns, collapsed navigation, export,
panel restoration, 3D toolbar scope, compact close controls, and guided creation.
Existing browser checks are updated to use the persistent view navigation.
Pixel-based renderer checks retain their thresholds at an explicit viewport.

## Keyboard browser extension

The project menu opens a responsive 740px keyboard browser with a fixed close
control and workspace footer. Your keyboards are sourced from browser storage,
searchable, and shown with measured layout previews; the current project is
identified first. Demo keyboards are a separate 18-layout gallery that creates
independent editable copies. New-project and portable-backup actions stay with
the browser, while the gallery remains the scrolling region on small screens.


## Integration refinements

The simplified shell retains the current dev onboarding and generation behavior.
Only the guide offers matrix creation while onboarding is visible; the empty
canvas offers it after the guide is dismissed. Reset local projects remains an
explicit, confirmed action in Workspace settings. Export's Back action, Escape,
and header toggle share the same return behavior.

The keyboard browser tolerates damaged saved records without hiding healthy
projects or blocking recovery. Failed saved-project acceptance restores the
previous core document. Saved records remain untouched until an explicit action.

The shell intentionally uses 14px navigation, 15px guide/gallery headings, and
16px export subsection headings to distinguish navigation and browsing from the
13px dense editing controls. Export's page heading is 28px on desktop and 24px on
compact screens. Compact project inputs and preference selectors use 16px to
avoid browser focus zoom. Secondary gallery labels use the existing 12px label
token, and controls use the existing 4px radius. The keyboard browser's offset
neutral shadow is confined to that transient overlay.
