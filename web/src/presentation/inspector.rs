use crate::runtime::Runtime;
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::ui::{
    callout::{Callout, CalloutVariant},
    field::FieldError,
};
use dioxus::prelude::*;
use std::rc::Rc;

mod companion_placement;
mod layout_component_inspector;
pub(super) use companion_placement::{
    CompanionPlacementAction, CompanionPlacementInspector, CompanionPlacementOwner,
    CompanionPlacementProjection,
};
pub(super) use layout_component_inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorOwner,
    LayoutComponentInspectorOwnerKey, LayoutComponentInspectorProjection, LayoutConstraintValues,
};

#[component]
pub(super) fn Inspector() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let mut failure = use_signal(|| None::<String>);
    let model = runtime.model();
    let selected = model.accepted.as_ref().and_then(|s| {
        s.document
            .parts
            .iter()
            .find(|p| model.selected_part_ids.first() == Some(&p.id))
            .cloned()
    });
    let commit_axis = {
        let rt = runtime.clone();
        move |x: Option<f64>, y: Option<f64>| {
            let m = rt.model();
            let res = rt.commit(
                "layout-old-position",
                super::layout_component_edits::commit_position_resolver(
                    m.selected_part_ids,
                    m.active_board_id,
                    x,
                    y,
                    format!("position-{}", rt.operation().0),
                ),
            );
            failure.set(res.err());
        }
    };
    let (mut cx, mut cy) = (commit_axis.clone(), commit_axis);
    rsx! {
        aside { class: "flex flex-col min-w-0 min-h-0 overflow-auto bg-transparent max-[760px]:overflow-visible", "aria-label": "Inspect",
            header { class: "hidden", h2 { "Inspect" } }
            h2 { class: "hidden", "Position" }
            if let Some(part) = selected {
                p { class: "mx-4 text-muted-foreground", "{part.reference}" }
                NumericField { id: "m1-position-x", label: "X (mm)".to_owned(), unit: "mm", value: part.pose.at.x, on_commit: move |v| cx(Some(v), None) }
                NumericField { id: "m1-position-y", label: "Y (mm)".to_owned(), unit: "mm", value: part.pose.at.y, on_commit: move |v| cy(None, Some(v)) }
                if let Some(msg) = failure() { FieldError { "{msg}" } }
            } else {
                Callout { variant: CalloutVariant::Info, "Select a component to edit its position." }
            }
        }
    }
}
