//! Private presentation and navigation for the reference Export workspace.
use super::{
    PanelSettings, RuntimeReportBanner, pin_inspector_on_desktop,
    selection::SelectionAdapter,
    zmk_firmware_export::{ZmkFirmwareExportPanelInput, ZmkFirmwareExportRow},
};
use crate::runtime::Runtime;
use boardstudio_application::{GenerationStatus, ReadModel, SelectionMode};
use boardstudio_core::model::OutlineExportFormat;
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
    let selection = use_context::<SelectionAdapter>();
    let model = runtime.model();
    let board_name = selected_board_name(&model);
    let rows = export_rows(
        &runtime,
        &model,
        zmk_firmware.as_ref().is_some_and(|firmware| firmware.ready),
        zmk_firmware
            .as_ref()
            .is_some_and(|firmware| firmware.wiring_ready),
    );
    let back = return_workspace();
    let key_listener = use_hook(|| {
        let retained = Rc::new(RefCell::new(
            None::<(
                web_sys::Document,
                Closure<dyn FnMut(web_sys::KeyboardEvent)>,
            )>,
        ));
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            let mut workspace = workspace;
            let return_workspace = return_workspace;
            let listener = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |event: web_sys::KeyboardEvent| {
                    if event.key() == "Escape" && workspace() == "Export" {
                        event.prevent_default();
                        workspace.set(return_workspace());
                    }
                },
            );
            let _ = document
                .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref());
            *retained.borrow_mut() = Some((document, listener));
        }
        retained
    });
    use_drop({
        let key_listener = key_listener.clone();
        move || {
            if let Some((document, listener)) = key_listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    listener.as_ref().unchecked_ref(),
                );
            }
        }
    });

    let review_wiring = {
        let runtime = runtime.clone();
        let mut inspect_open = inspect_open;
        let inspector_settings = inspector_settings;
        move |_| {
            let mut selected_context = selection.selected_context;
            selected_context.set(None);
            let mut anchor_scope = selection.anchor_scope;
            anchor_scope.set(None);
            let model = runtime.model();
            if !model.selected_part_ids.is_empty() || model.selection_anchor_id.is_some() {
                runtime.submit(boardstudio_application::Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: vec![],
                    range_part_ids: vec![],
                    mode: SelectionMode::Replace,
                });
            }
            pin_inspector_on_desktop(inspector_settings);
            inspect_open.set(true);
            workspace.set("PCB");
        }
    };
    let review_case = {
        let mut inspect_open = inspect_open;
        let inspector_settings = inspector_settings;
        move |_| {
            pin_inspector_on_desktop(inspector_settings);
            inspect_open.set(true);
            workspace.set("Case");
        }
    };
    let archive = runtime.clone();

    rsx! {
        section { class: "m1-export-panel m1-export-workspace", "aria-label": "Export package",
            header { class: "m1-export-heading",
                button { class: "m1-export-back", r#type: "button", onclick: move |_| workspace.set(back), "Back to {back}" }
                h1 { "Export package" }
                p { "Choose a handoff for {board_name}. Downloads use the current committed design; routing and mechanical fit still need downstream review." }
            }
            h2 { class: "m1-export-section-title", "Design files" }
            div { class: "m1-export-list",
                for row in rows {
                    ExportRowView { row, zmk_firmware: zmk_firmware.clone() }
                }
            }
            div { class: "m1-export-review-links",
                button { r#type: "button", onclick: review_wiring, "Review wiring" }
                button { r#type: "button", onclick: review_case, "Review case" }
            }
            section { class: "m1-export-portable", "aria-label": "Portable project",
                h2 { "Portable project" }
                p { "Keep an editable copy of the whole project, including all boards." }
                label { class: "m1-export-option",
                    input {
                        r#type: "checkbox",
                        aria_label: "Embed used models",
                        checked: runtime.embed_used_models(),
                        onchange: move |event: FormEvent| runtime.set_embed_used_models(event.checked()),
                    }
                    span { strong { "Embed used models" } small { "Include attached 3D model files used in this project." } }
                }
                button {
                    class: "m1-export-action",
                    r#type: "button",
                    disabled: model.accepted.is_none(),
                    onclick: move |_| archive.export_project_copy(),
                    "Save .boardstudio project"
                }
            }
            RuntimeReportBanner {}
        }
    }
}

#[derive(Clone, PartialEq)]
struct ExportRow {
    label: &'static str,
    detail: &'static str,
    ready: bool,
    available: bool,
    reason: Option<&'static str>,
    on_export: Option<EventHandler<()>>,
}

#[component]
fn ExportRowView(row: ExportRow, zmk_firmware: Option<ZmkFirmwareExportPanelInput>) -> Element {
    if row.label == "ZMK firmware"
        && let Some(firmware) = zmk_firmware
    {
        return rsx! { ZmkFirmwareExportRow { ready: firmware.ready, on_export: firmware.on_export } };
    }
    rsx! {
        div { class: "m1-export-row", "data-export-kind": "{row.label}",
            span {
                class: if row.ready { "m1-export-ready-dot is-ready" } else { "m1-export-ready-dot" },
                "aria-hidden": "true",
            }
            span { class: "m1-export-row-copy",
                strong { "{row.label}" }
                small { "{row.detail}" }
                if let Some(reason) = row.reason { small { class: "m1-export-reason", "{reason}" } }
                if row.ready && !row.available {
                    small { class: "m1-export-provider-gap", "This export provider is not connected in this build." }
                }
            }
            button {
                class: "m1-export-row-action",
                r#type: "button",
                aria_label: if row.ready && row.available { format!("Export {}", row.label) } else if row.ready { format!("{} export provider unavailable", row.label) } else { format!("{} needs work", row.label) },
                disabled: !row.ready || !row.available,
                onclick: move |_| {
                    if let Some(on_export) = row.on_export.as_ref() {
                        on_export.call(());
                    }
                },
                if row.ready { "Export" } else { "Needs work" }
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
    let Some(snapshot) = model.accepted.as_ref() else {
        return vec![];
    };
    let document = &snapshot.document;
    let board_count = document.boards.len();
    let board_id = model.active_board_id.as_str();
    let scene = &snapshot.scene;
    let selected_readiness = scene
        .board_readiness
        .iter()
        .find(|item| item.board_id == board_id);
    let pcb_ready =
        selected_readiness.map_or(board_count <= 1 && scene.readiness.pcb, |item| item.pcb);
    let outline_ready = selected_readiness.map_or(scene.readiness.outline, |item| item.outline);
    let outline_blocker =
        (!outline_ready).then_some("Review the board outline and layout findings.");
    let wiring_blocker =
        (!wiring_ready).then_some("Review the layout and resolve controller wiring in PCB.");
    let authored_case_ready =
        boardstudio_core::authored_case_geometry_ready(document, scene, board_id);
    let mechanical_scope = runtime.scope();
    let mechanical_document = mechanical_scope
        .as_ref()
        .and_then(|scope| boardstudio_web::cad_jobs::captured_case_document(snapshot, scope).ok());
    let generated_case = mechanical_document
        .as_ref()
        .and_then(|document| document.mechanical.as_ref())
        .is_some_and(|configuration| configuration.board_id == board_id);
    let step_runtime = runtime.clone();
    let step_scope = runtime.scope();
    let step_token = snapshot.token;
    let step_epoch = snapshot.session_epoch;
    let step_export = EventHandler::new(move |()| {
        let model = step_runtime.model();
        if step_runtime.scope() == step_scope
            && model.accepted.as_ref().is_some_and(|accepted| {
                accepted.token == step_token && accepted.session_epoch == step_epoch
            })
        {
            step_runtime.export_step();
        }
    });
    let mechanical_runtime = runtime.clone();
    let mechanical_callback_scope = mechanical_scope.clone();
    let mechanical_token = snapshot.token;
    let mechanical_session_epoch = snapshot.session_epoch;
    let mechanical_export = EventHandler::new(move |()| {
        let model = mechanical_runtime.model();
        if mechanical_runtime.scope() == mechanical_callback_scope
            && model.accepted.as_ref().is_some_and(|accepted| {
                accepted.token == mechanical_token
                    && accepted.session_epoch == mechanical_session_epoch
            })
        {
            mechanical_runtime.export_mechanical();
        }
    });
    let svg_runtime = runtime.clone();
    let svg_export =
        EventHandler::new(move |()| svg_runtime.export_board_outline(OutlineExportFormat::Svg));
    let dxf_runtime = runtime.clone();
    let dxf_export =
        EventHandler::new(move |()| dxf_runtime.export_board_outline(OutlineExportFormat::Dxf));
    let footprints_runtime = runtime.clone();
    let footprints_export = EventHandler::new(move |()| footprints_runtime.export_footprints());
    let kicad_runtime = runtime.clone();
    let kicad_export = EventHandler::new(move |()| kicad_runtime.export_kicad_board(false));
    let draft_kicad_runtime = runtime.clone();
    let draft_kicad_export =
        EventHandler::new(move |()| draft_kicad_runtime.export_kicad_board(true));
    let mut rows = vec![
        ExportRow {
            label: "KiCad board",
            detail: "Placements, resolved wiring, and board edges for KiCad",
            ready: pcb_ready && wiring_ready,
            available: true,
            reason: if pcb_ready && wiring_ready {
                None
            } else {
                wiring_blocker
            },
            on_export: Some(kicad_export),
        },
        ExportRow {
            label: "Draft KiCad board",
            detail: "Incomplete wiring with a findings report",
            ready: pcb_ready,
            available: true,
            reason: (!pcb_ready).then_some("Review the board outline and layout findings."),
            on_export: Some(draft_kicad_export),
        },
        ExportRow {
            label: "ZMK firmware",
            detail: "ZMK v0.3.0 configuration and editable starter keymap",
            ready: firmware_ready,
            available: true,
            reason: wiring_blocker,
            on_export: None,
        },
        ExportRow {
            label: "KiCad footprints",
            detail: "Component footprint library",
            ready: !document.definitions.is_empty(),
            available: true,
            reason: document
                .definitions
                .is_empty()
                .then_some("Add a component definition before exporting footprints."),
            on_export: Some(footprints_export),
        },
        ExportRow {
            label: "SVG board outline",
            detail: "Resolved board contour",
            ready: outline_ready,
            available: true,
            reason: outline_blocker,
            on_export: Some(svg_export),
        },
        ExportRow {
            label: "DXF board outline",
            detail: "Resolved board contour",
            ready: outline_ready,
            available: true,
            reason: outline_blocker,
            on_export: Some(dxf_export),
        },
        ExportRow {
            label: if generated_case {
                "Authored Case STEP"
            } else {
                "Case STEP"
            },
            detail: "Saved authored case bodies",
            ready: authored_case_ready,
            available: true,
            reason: (!authored_case_ready).then_some("Add and generate case geometry in Case."),
            on_export: Some(step_export),
        },
    ];
    if generated_case {
        let expected_scope = mechanical_scope;
        let ready = outline_ready
            && runtime.cad_scene().is_some_and(|cad| {
                Some(&cad.scope) == expected_scope.as_ref()
                    && cad.token == snapshot.token
                    && cad.exact
                    && matches!(
                        &model.generation,
                        GenerationStatus::Ready { exact: true, .. }
                    )
                    && cad.mechanical.as_ref().is_some_and(|assembly| {
                        assembly.revision == snapshot.document.revision
                            && !assembly.generation_blocked
                            && !assembly.diagnostics.iter().any(|finding| {
                                finding.severity == boardstudio_core::model::Severity::Error
                            })
                    })
            });
        rows.push(ExportRow {
            label: "Generated mechanical package",
            detail: "STEP/STL parts, outlines, specifications and FR4 plate project",
            ready,
            available: true,
            reason: (!ready).then_some("Update the current mechanical preview before export."),
            on_export: Some(mechanical_export),
        });
    }
    rows
}

fn selected_board_name(model: &ReadModel) -> String {
    model
        .accepted
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == model.active_board_id)
        })
        .map(|board| board.name.clone())
        .unwrap_or_else(|| "this board".to_owned())
}
