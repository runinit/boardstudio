//! Private presentation and navigation for the reference Export workspace.
use super::{
    PanelSettings, RuntimeReportBanner, pin_inspector_on_desktop,
    selection::SelectionAdapter,
    zmk_firmware_export::{ZmkFirmwareExportPanelInput, ZmkFirmwareExportRow},
};
use crate::runtime::Runtime;
use boardstudio_application::{GenerationStatus, ReadModel, SelectionMode};
use boardstudio_core::model::OutlineExportFormat;
use boardstudio_web_ui_shared::inspector_fields::ToggleField;
use boardstudio_web_ui_shared::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::{Callout, CalloutVariant},
    command::{Command, use_commands},
    item::{Item, ItemContent, ItemDescription},
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};

#[component]
pub(super) fn ExportWorkspace(
    zmk_firmware: Option<ZmkFirmwareExportPanelInput>,
    workspace: Signal<&'static str>,
    return_workspace: Signal<&'static str>,
    mut inspect_open: Signal<bool>,
    inspector_settings: Signal<PanelSettings>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let selection = use_context::<SelectionAdapter>();
    let model = runtime.model();
    let board_name = model
        .accepted
        .as_ref()
        .and_then(|s| {
            s.document
                .boards
                .iter()
                .find(|b| b.id == model.active_board_id)
        })
        .map_or_else(|| "this board".into(), |b| b.name.clone());
    let rows = export_rows(
        &runtime,
        &model,
        zmk_firmware.as_ref().is_some_and(|f| f.ready),
        zmk_firmware.as_ref().is_some_and(|f| f.wiring_ready),
    );
    let (project_name, revision) = model.accepted.as_ref().map_or_else(
        || (String::new(), 0),
        |s| (s.document.name.clone(), s.document.revision),
    );
    let durability = boardstudio_web_ui_shared::project_menu::durability_label(&model.durability);
    let back = return_workspace();
    let listener = use_hook(|| {
        let slot = Rc::new(RefCell::new(
            None::<(
                web_sys::Document,
                Closure<dyn FnMut(web_sys::KeyboardEvent)>,
            )>,
        ));
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let mut ws = workspace;
            let cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |e: web_sys::KeyboardEvent| {
                    if e.key() == "Escape" && ws() == "Export" {
                        e.prevent_default();
                        ws.set(return_workspace());
                    }
                },
            );
            let _ = doc.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            *slot.borrow_mut() = Some((doc, cb));
        }
        slot
    });
    use_drop(move || {
        if let Some((d, cb)) = listener.borrow_mut().take() {
            let _ = d.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
    });
    let (rt_w, rt_a) = (runtime.clone(), runtime.clone());
    rsx! {
        section { class: "grid w-full grid-cols-[minmax(0,1fr)_minmax(320px,34%)] content-start gap-x-8 gap-y-4 bg-card p-[clamp(24px,3vw,48px)] text-foreground max-[760px]:flex max-[760px]:flex-col", "aria-label": "Export package",
            header { class: "col-start-1",
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| workspace.set(back), "Back to {back}" }
                h1 { class: "mt-2 mb-3 text-[32px]", "Export package" }
                p { class: "m-0 text-base text-muted-foreground", "Choose a handoff for {board_name}. Downloads use the current committed design." }
            }
            div { class: "col-start-2 row-start-1 row-span-6 flex min-w-0 flex-col gap-4",
                aside { class: "grid gap-3 rounded-xl bg-card p-5", aria_label: "Current design",
                    h2 { class: "m-0 text-xl", "Current design" }
                    dl { class: "m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-2",
                        dt { "Project" } dd { "{project_name}" }
                        dt { "Active board" } dd { "{board_name}" }
                        dt { "Revision" } dd { "{revision}" }
                        dt { "Local save" } dd { "{durability}" }
                    }
                }
                section { class: "grid gap-3 rounded-xl bg-card p-5", "aria-label": "Portable project",
                    h2 { class: "m-0 text-xl", "Portable project" }
                    ToggleField {
                        label: "Embed used models".to_owned(), aria_label: "Embed used models",
                        description: "Include attached 3D model files used in this project.",
                        checked: runtime.embed_used_models(),
                        on_commit: { let rt = runtime.clone(); move |v: bool| rt.set_embed_used_models(v) },
                    }
                    Button { variant: ButtonVariant::Secondary, class: "min-h-12 w-full", button_type: "button", disabled: model.accepted.is_none(),
                        onclick: move |_| rt_a.export_project_copy(), "Download .boardstudio project" }
                }
            }
            h2 { class: "col-start-1 mt-2 border-b border-border pb-2.5 text-lg", "Design files" }
            div { class: "col-start-1 grid gap-0",
                for row in rows { ExportRowView { row, zmk_firmware: zmk_firmware.clone() } }
            }
            div { class: "col-start-1 flex flex-wrap gap-2.5",
                Button { variant: ButtonVariant::Secondary, onclick: move |_| {
                    let (mut sc, mut an) = (selection.selected_context, selection.anchor_scope);
                    sc.set(None); an.set(None);
                    let m = rt_w.model();
                    if !m.selected_part_ids.is_empty() || m.selection_anchor_id.is_some() {
                        rt_w.submit(boardstudio_application::Event::SelectParts { operation_id: rt_w.operation(), part_ids: vec![], range_part_ids: vec![], mode: SelectionMode::Replace });
                    }
                    pin_inspector_on_desktop(inspector_settings); inspect_open.set(true); workspace.set("PCB");
                }, "Review wiring" }
                Button { variant: ButtonVariant::Secondary, onclick: move |_| { pin_inspector_on_desktop(inspector_settings); inspect_open.set(true); workspace.set("Case"); }, "Review case" }
            }
            RuntimeReportBanner {}
        }
    }
}

#[derive(Clone, PartialEq)]
struct ExportRow {
    label: &'static str,
    detail: &'static str,
    status: &'static str,
    action: &'static str,
    ready: bool,
    reason: Option<&'static str>,
    on_export: Option<EventHandler<()>>,
}

#[component]
fn ExportRowView(row: ExportRow, zmk_firmware: Option<ZmkFirmwareExportPanelInput>) -> Element {
    let fw_owned = row.label == "ZMK firmware" && zmk_firmware.is_some();
    use_commands(if fw_owned {
        vec![]
    } else {
        vec![Command {
            group: "Current Workspace",
            label: format!("Export {}", row.label),
            reason: (!row.ready).then(|| row.reason.unwrap_or(row.status).to_owned()),
            hint: None,
            on_select: EventHandler::new(move |()| {
                if let Some(cb) = row.on_export {
                    cb.call(());
                }
            }),
        }]
    });
    if let (true, Some(fw)) = (row.label == "ZMK firmware", zmk_firmware) {
        return rsx! { ZmkFirmwareExportRow { ready: fw.ready, on_export: fw.on_export } };
    }
    rsx! {
        div { "data-export-kind": "{row.label}",
            Item { class: "grid grid-cols-[minmax(0,1fr)_auto] gap-3 bg-card p-4 max-[760px]:grid-cols-1",
                ItemContent { class: "flex min-w-0 flex-wrap items-start gap-x-3 gap-y-1",
                    strong { "{row.label}" }
                    Badge { variant: if row.ready { BadgeVariant::Success } else { BadgeVariant::Warning }, "{row.status}" }
                    ItemDescription { "{row.detail}" }
                    if let Some(reason) = row.reason { Callout { variant: CalloutVariant::Warning, "{reason}" } }
                }
                Button { variant: ButtonVariant::Secondary, class: "min-h-9 min-w-[104px]", button_type: "button",
                    aria_label: format!("Export {}", row.label), disabled: !row.ready,
                    onclick: move |_| if let Some(cb) = row.on_export { cb.call(()); }, "{row.action}" }
            }
        }
    }
}

fn export_rows(
    runtime: &Rc<Runtime>,
    model: &ReadModel,
    firmware_ready: bool,
    wiring_ready: bool,
) -> Vec<ExportRow> {
    let Some(snap) = model.accepted.as_ref() else {
        return vec![];
    };
    let (doc, scene, bid) = (&snap.document, &snap.scene, model.active_board_id.as_str());
    let sel = scene.board_readiness.iter().find(|r| r.board_id == bid);
    let pcb_ready = sel.map_or(doc.boards.len() <= 1 && scene.readiness.pcb, |r| r.pcb);
    let outline_ready = sel.map_or(scene.readiness.outline, |r| r.outline);
    let pcb_blk = (!pcb_ready).then_some("Review the board outline and layout findings.");
    let out_blk = (!outline_ready).then_some("Review the board outline and layout findings.");
    let wire_blk =
        (!wiring_ready).then_some("Review the layout and resolve controller wiring in PCB.");
    let case_ready = boardstudio_core::authored_case_geometry_ready(doc, scene, bid);
    let scope = runtime.scope();
    let gen_case = scope
        .as_ref()
        .and_then(|s| boardstudio_web_host::cad_jobs::captured_case_document(snap, s).ok())
        .and_then(|d| d.mechanical)
        .is_some_and(|c| c.board_id == bid);
    let token = snap.token;
    let epoch = snap.session_epoch;
    let guarded = |f: fn(&Rc<Runtime>)| {
        let (rt, sc) = (runtime.clone(), scope.clone());
        EventHandler::new(move |()| {
            if rt.scope() == sc
                && rt
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|a| a.token == token && a.session_epoch == epoch)
            {
                f(&rt);
            }
        })
    };
    let (rt_k, rt_dk, rt_fp, rt_svg, rt_dxf) = (
        runtime.clone(),
        runtime.clone(),
        runtime.clone(),
        runtime.clone(),
        runtime.clone(),
    );
    let keycaps_ready = scene.revision == doc.revision && doc.boards.iter().any(|b| b.id == bid);
    let mut rows = vec![
        ExportRow {
            label: "KiCad board",
            detail: "Placements, resolved wiring, and board edges for KiCad",
            status: if !wiring_ready {
                "Needs controller wiring"
            } else if !pcb_ready {
                "Needs board review"
            } else {
                "Available"
            },
            action: "Export KiCad",
            ready: pcb_ready && wiring_ready,
            reason: if pcb_ready && wiring_ready {
                None
            } else if wiring_ready {
                pcb_blk
            } else {
                wire_blk
            },
            on_export: Some(EventHandler::new(move |()| rt_k.export_kicad_board(false))),
        },
        ExportRow {
            label: "Draft KiCad board",
            detail: "Incomplete wiring with a findings report",
            status: if pcb_ready {
                "Available"
            } else {
                "Needs board review"
            },
            action: "Export draft",
            ready: pcb_ready,
            reason: pcb_blk,
            on_export: Some(EventHandler::new(move |()| rt_dk.export_kicad_board(true))),
        },
        ExportRow {
            label: "Keycap STLs",
            detail: "One fused mesh per keycap, with a millimetre manifest in a ZIP",
            status: if keycaps_ready {
                "Available"
            } else {
                "Needs board"
            },
            action: "Export keycap STLs",
            ready: keycaps_ready,
            reason: (!keycaps_ready)
                .then_some("Select a board with a current design before exporting keycaps."),
            on_export: Some(guarded(Runtime::export_keycaps_stl)),
        },
        ExportRow {
            label: "ZMK firmware",
            detail: "ZMK v0.3.0 configuration and editable starter keymap",
            status: if firmware_ready {
                "Available"
            } else {
                "Needs controller wiring"
            },
            action: "Export firmware",
            ready: firmware_ready,
            reason: wire_blk,
            on_export: None,
        },
        ExportRow {
            label: "KiCad footprints",
            detail: "Component footprint library",
            status: if !doc.definitions.is_empty() {
                "Available"
            } else {
                "Needs component definition"
            },
            action: "Export footprints",
            ready: !doc.definitions.is_empty(),
            reason: doc
                .definitions
                .is_empty()
                .then_some("Add a component definition before exporting footprints."),
            on_export: Some(EventHandler::new(move |()| rt_fp.export_footprints())),
        },
        ExportRow {
            label: "SVG board outline",
            detail: "Resolved board contour",
            status: if outline_ready {
                "Available"
            } else {
                "Needs outline review"
            },
            action: "Export SVG",
            ready: outline_ready,
            reason: out_blk,
            on_export: Some(EventHandler::new(move |()| {
                rt_svg.export_board_outline(OutlineExportFormat::Svg)
            })),
        },
        ExportRow {
            label: "DXF board outline",
            detail: "Resolved board contour",
            status: if outline_ready {
                "Available"
            } else {
                "Needs outline review"
            },
            action: "Export DXF",
            ready: outline_ready,
            reason: out_blk,
            on_export: Some(EventHandler::new(move |()| {
                rt_dxf.export_board_outline(OutlineExportFormat::Dxf)
            })),
        },
        ExportRow {
            label: if gen_case {
                "Authored Case STEP"
            } else {
                "Case STEP"
            },
            detail: "Saved authored case bodies",
            status: if case_ready {
                "Available"
            } else {
                "Needs case geometry"
            },
            action: "Export STEP",
            ready: case_ready,
            reason: (!case_ready).then_some("Add and generate case geometry in Case."),
            on_export: Some(guarded(Runtime::export_step)),
        },
    ];
    if gen_case {
        let ready = outline_ready
            && runtime.cad_scene().is_some_and(|c| {
                Some(&c.scope) == scope.as_ref()
                    && c.token == token
                    && c.exact
                    && matches!(
                        &model.generation,
                        GenerationStatus::Ready { exact: true, .. }
                    )
                    && c.mechanical.as_ref().is_some_and(|a| {
                        a.revision == doc.revision
                            && !a.generation_blocked
                            && !a
                                .diagnostics
                                .iter()
                                .any(|f| f.severity == boardstudio_core::model::Severity::Error)
                    })
            });
        rows.push(ExportRow {
            label: "Generated mechanical package",
            detail: "STEP/STL parts, outlines, specifications and FR4 plate project",
            status: if ready {
                "Available"
            } else {
                "Preview needs update"
            },
            action: "Export package",
            ready,
            reason: (!ready).then_some("Update the current mechanical preview before export."),
            on_export: Some(guarded(Runtime::export_mechanical)),
        });
    }
    rows
}

#[cfg(test)]
#[component]
pub(crate) fn ExportPanel(
    #[props(default)] zmk_firmware: Option<ZmkFirmwareExportPanelInput>,
) -> Element {
    let workspace = use_context::<super::WorkspaceState>().0;
    let return_workspace = use_context::<super::ExportReturnWorkspace>().0;
    let compact = use_context::<super::CompactPanelState>();
    let settings = use_signal(PanelSettings::default);
    rsx! { ExportWorkspace { zmk_firmware, workspace, return_workspace, inspect_open: compact.inspector_open, inspector_settings: settings } }
}
