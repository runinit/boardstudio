# VIK modules

## Current availability

VIK remains a deferred hardware program. The Dioxus Parts catalogue filters VIK
definitions out of new choices while retaining imported definitions for document
resolution; see `is_vik_part` in the
[catalogue module](../../web/src/presentation/parts/catalogue.rs).

## Retained qualification evidence

The following catalogue counts and workflow descriptions record the hardware
work before the frontend cutover. They do not establish current UI availability
or completed hardware qualification.

The researched Parts library included all 29 VIK catalog rows and their 38
imported definitions, including the explicit DRV2605L circuit repair. Select a
variant before choosing mounted placement, constituent placement, or an editable
circuit copy. Purchased breakout internals absent from the source remain
unavailable for reconstruction.

Mounted modules retain daughterboard copper separately from the host. Embedded
circuits receive independent host part, pad and net identities; rail/bus joins
are explicit. Placement uses a facing-surface gap in millimetres, host and module
faces, and one transform from the PCB midplane. Source drill positions identify
mounts; support dimensions are designer-selected geometry rather than a
supplier hardware specification.

The model ledger includes 27 STEP and four STL assets. Public CAD tests import
all STEP assets and compare their meshes with independent FreeCAD bounds. Tight
extrema comparisons use the importer's 0.1 mm preview mesh detail; conservative
source bounds remain separate. The original THQ model checks retain their tighter
0.03 mm assertions. Raw asset bounds alone do not establish a model's assembly
position, full purchased stack, swept movement or an active display aperture.

Physical testing is deferred. Acceptance uses source-backed model/footprint
checks, exact exported geometry, public edit/electrical/firmware tests, and
human review in the app. Unknown facts can be resolved from source, supplied as
explicit configuration, or left as an output-specific blocker. Upstream Untested
status is retained as provenance rather than a requirement to obtain hardware.

The [archived review checklist](../archive/vik-module-human-review.md) retains the
former app scenarios and retired test commands. Adapt its scenarios to the current
workbench before using it for review. Record the revision and saved project with
any failure. Human sign-off remains pending; automated browser tests do not supply it.

The splitter is not yet a qualified assembled reference. Its BOM names HDGC
C2919557 while PCB annotations name ATOM C479750, including a distinct J1002
breakout. The supplied generic connector model has no verified datum against
the placed pads. Cable bend/service space and the installed fastener stack are
also unspecified. Its source PCB contour, thickness and drill geometry remain
available; assembled-envelope and model gates remain open.
