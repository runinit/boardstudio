use super::{
    InstanceSelection, active_board_scope_matches,
    pcb_wiring::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity},
};
use crate::runtime::Runtime;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    command::{Command, use_commands},
    item::{Item, ItemContent, ItemDescription},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub(crate) struct ZmkFirmwareExportPanelInput {
    pub ready: bool,
    pub wiring_ready: bool,
    pub on_export: EventHandler<()>,
}

pub(crate) fn use_export_panel_input(
    runtime: Rc<Runtime>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
    instance_selection: InstanceSelection,
) -> ZmkFirmwareExportPanelInput {
    let ready = source
        .as_ref()
        .is_some_and(|s| ready_for_export(&s.identity, &resolution()));
    let wiring_ready = source
        .as_ref()
        .is_some_and(|s| wiring_ready_for_board_export(&s.identity, &resolution()));
    let on_export = use_callback(move |()| {
        let Some(src) = source.as_ref() else {
            return;
        };
        let model = runtime.model();
        let ok = workspace() == Workspace::Export
            && scope_generation() == src.scope_generation
            && runtime.scope().as_ref() == Some(&src.ui_scope)
            && active_board_scope_matches(&model, &src.ui_scope)
            && instance_selection.is_current(&model)
            && model.accepted.as_ref().is_some_and(|a| {
                a.session_epoch == src.identity.scope.session_epoch
                    && a.document.id == src.identity.scope.document_id
                    && a.token == src.identity.token
                    && a.document.revision == src.identity.revision
                    && a.scene.revision == src.identity.revision
            })
            && ready_for_export(&src.identity, &resolution());
        if ok {
            runtime.export_firmware();
        }
    });
    ZmkFirmwareExportPanelInput {
        ready,
        wiring_ready,
        on_export,
    }
}

fn wiring_ready_for_board_export(id: &WiringPlanIdentity, res: &PcbWiringResolution) -> bool {
    matches!(res, PcbWiringResolution::Current { identity, plan }
        if identity == id && plan.revision == id.revision
            && plan.board_id.as_deref() == Some(id.scope.board_id.as_str())
            && plan.instance_id.is_none()
            && !plan.diagnostics.iter().any(|d| d.severity == "error"))
}

pub(crate) fn ready_for_export(id: &WiringPlanIdentity, res: &PcbWiringResolution) -> bool {
    matches!(res, PcbWiringResolution::Current { identity, plan }
        if identity == id && plan.revision == id.revision
            && plan.board_id.as_deref() == Some(id.scope.board_id.as_str())
            && plan.instance_id.is_none()
            && !plan.diagnostics.iter().any(|d| d.severity == "error")
            && plan.controller_part_id.is_some()
            && !plan.assignments.is_empty())
}

#[component]
pub(crate) fn ZmkFirmwareExportRow(ready: bool, on_export: EventHandler<()>) -> Element {
    use_commands(vec![Command {
        group: "Current Workspace",
        label: "Export ZMK firmware".into(),
        reason: (!ready).then(|| "Review the layout and resolve controller wiring in PCB.".into()),
        hint: None,
        on_select: on_export,
    }]);
    rsx! {
        div { "data-export-kind": "ZMK firmware",
            Item { class: "grid grid-cols-[minmax(0,1fr)_auto] gap-3 rounded-none border-0 border-t border-border p-4 bg-card max-[760px]:grid-cols-1",
                ItemContent { class: "flex-row flex-wrap items-start gap-1 min-w-0 max-[760px]:flex-col",
                    strong { class: "text-[18px]", "ZMK firmware" }
                    Badge { variant: if ready { BadgeVariant::Success } else { BadgeVariant::Warning },
                        if ready { "Available" } else { "Needs controller wiring" }
                    }
                    ItemDescription { class: "line-clamp-none", "ZMK v0.3.0 configuration and editable starter keymap" }
                    if !ready { Callout { variant: CalloutVariant::Warning, "Review the layout and resolve controller wiring in PCB." } }
                }
                Button { variant: ButtonVariant::Secondary, class: "min-w-[104px] min-h-9", button_type: "button",
                    aria_label: "Export ZMK firmware", disabled: !ready, onclick: move |_| on_export.call(()), "Export firmware" }
            }
        }
    }
}
