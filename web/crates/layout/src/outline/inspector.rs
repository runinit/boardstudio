//! Outline version Inspector.
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};

use super::hook::{OutlineDrawTool, OutlineInspectorProjection, OutlineVersionOwnership};
use super::overlays::{OutlineCoordinate, OutlineDimension};
use super::planner::{
    PerimeterAnchor, attach_connection_point, connection_point_world, move_connection_point,
};
use crate::outline_settings::OutlineEdit;
use boardstudio_model::model::{
    Contour, CornerStyle, EditPhase, Operation, OutlineControlPoint, OutlineFeature, Side, Vec2,
    polygon_signed_area,
};
use boardstudio_web_ui_shared::ui::checkbox::{Checkbox, CheckboxState};
use boardstudio_web_ui_shared::ui::label::Label;
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};

use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::field::FieldError;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;

mod board;
mod vec2_fields;
use board::{BoardInspectorView, board_outline_inspector};

#[component]
pub fn OutlineVersionInspector(projection: OutlineInspectorProjection) -> Element {
    let mut inspector_mounted = projection.inspector_mounted;
    use_effect(move || {
        inspector_mounted.set(true);
    });
    use_drop(move || inspector_mounted.set(false));
    let mut perimeter_open = projection.editing_points;
    let mut selected_point = projection.selected_point;
    let selected_connection_id = projection.selected_connection_id;
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let draft_serial = use_signal(|| 0u64);
    let active_value = projection.active_version_id.clone().unwrap_or_default();
    let perimeter = projection.perimeter.clone();
    let corner_value = match projection.settings.corners {
        CornerStyle::Sharp => "sharp",
        CornerStyle::Fillet => "fillet",
        CornerStyle::Chamfer => "chamfer",
    };
    let active_selection = use_memo(use_reactive((&active_value,), |(value,)| Some(value)));
    let corner_selection = use_memo(use_reactive((&corner_value,), |(value,)| {
        Some(value.to_owned())
    }));
    let point_attachment = use_memo(use_reactive(
        (&projection.connections,),
        move |(connections,)| {
            Some(
                connections
                    .iter()
                    .find(|connection| {
                        Some(&connection.id) == selected_connection_id.read().as_ref()
                    })
                    .and_then(|connection| {
                        connection
                            .points
                            .get(selected_point().min(connection.points.len().saturating_sub(1)))
                    })
                    .and_then(|point| point.part_id.clone())
                    .unwrap_or_default(),
            )
        },
    ));
    let point_index = perimeter
        .as_ref()
        .map(|perimeter| selected_point().min(perimeter.points.len().saturating_sub(1)))
        .unwrap_or_default();
    let point = perimeter
        .as_ref()
        .and_then(|perimeter| perimeter.canvas_points.get(point_index).copied())
        .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
    let point_count = perimeter
        .as_ref()
        .map(|perimeter| perimeter.points.len())
        .unwrap_or_default();
    rsx! {
                    if perimeter_open() {
                        if let Some(perimeter) = perimeter.as_ref() {
                            section {
                                class: "min-h-0 border-b border-border px-4 py-3.5 [&>p]:my-2 [&>p]:text-muted-foreground",
                                "aria-label": "Perimeter",
                                onkeydown: {
                                    let mut selected_context = projection.selected_context;
                                    let scope = projection.scope.clone();
                                    let board_id = projection.board_id.clone();
                                    move |event: KeyboardEvent| {
                                        crate::objects::stop_space_pan(event.clone());
                                        if event.data().key().to_string() != "Escape" {
                                            return;
                                        }
                                        event.prevent_default();
                                        event.stop_propagation();
                                        let Some(selected) = selected_context.read().clone() else {
                                            return;
                                        };
                                        if selected.scope != scope
                                            || !matches!(
                                                selected.context,
                                                crate::objects::TreeContext::Outline { board_id: selected_board }
                                                if selected_board == board_id
                                            )
                                        {
                                            return;
                                        }
                                        selected_context
                                            .set(
                                                Some(crate::objects::ScopedTreeContext {
                                                    scope: scope.clone(),
                                                    context: crate::objects::TreeContext::Board {
                                                        board_id: board_id.clone(),
                                                    },
                                                }),
                                            );
                                        let focus_context = selected_context;
                                        let focus_scope = scope.clone();
                                        let focus_board_id = board_id.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            gloo_timers::future::TimeoutFuture::new(0).await;
                                            if !focus_context
                                                .read()
                                                .as_ref()
                                                .is_some_and(|selected| {
                                                    selected.scope == focus_scope
                                                        && matches!(
                                                            &selected.context,
                                                            crate::objects::TreeContext::Board { board_id }
                                                            if board_id == &focus_board_id
                                                        )
                                                })
                                            {
                                                return;
                                            }
                                            let Some(input) = web_sys::window()
                                                .and_then(|window| window.document())
                                                .and_then(|document| {
                                                    document
                                                        .query_selector("input[aria-label='Board name']")
                                                        .ok()
                                                        .flatten()
                                                })
                                                .and_then(|input| input.dyn_into::<web_sys::HtmlElement>().ok())
                                            else {
                                                return;
                                            };
                                            let _ = input.focus();
                                        });
                                    }
                                },
                                div { class: "flex items-center justify-between gap-2",
                                    h2 { class: "m-0 text-[16px]! font-semibold!", "Perimeter" }
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        button_type: "button",
                                        disabled: !enabled,
                                        onclick: move |_| perimeter_open.set(false),
                                        "Done"
                                    }
                                }
                                p {
                                    if projection.active_version_id.is_some() {
                                        "This outline stays fixed when components move. Changes save as you edit."
                                    } else {
                                        "The first point change creates and activates a fixed copy. Generated stays available."
                                    }
                                }
                                h3 { class: "mt-4 mb-2 text-[13px] font-semibold", "Point {point_index + 1} of {point_count}" }
                                div { class: "grid gap-2.5",
                                    OutlineCoordinate {
                                        key: format!("{:?}:{:?}:{point_index}:x", action_context.envelope.scope, perimeter.target),
                                        label: format!("Point {} X mm", point_index + 1),
                                        value: point.x,
                                        editable: enabled,
                                        on_commit: {
                                            let action_context = action_context.clone();
                                            let target = perimeter.target.clone();
                                            let mut points = perimeter.points.clone();
                                            let world_points = perimeter.canvas_points.clone();
                                            let anchor = perimeter.anchor;
                                            move |value| {
                                                if let (Some(point), Some(world)) = (
                                                    points.get_mut(point_index),
                                                    world_points.get(point_index),
                                                ) {
                                                    let next = Vec2 { x: value, y: world.y };
                                                    *point = anchor.map_or(next, |anchor| anchor.local(next));
                                                }
                                                on_action
                                                    .call(
                                                        action_context
                                                            .edit_perimeter(
                                                                target.clone(),
                                                                points.clone(),
                                                                EditPhase::Commit,
                                                                format!(
                                                                    "outline-point-{}-{point_index}",
                                                                    action_context.envelope.revision,
                                                                ),
                                                            ),
                                                    );
                                            }
                                        },
                                    }
                                    OutlineCoordinate {
                                        key: format!("{:?}:{:?}:{point_index}:y", action_context.envelope.scope, perimeter.target),
                                        label: format!("Point {} Y mm", point_index + 1),
                                        value: point.y,
                                        editable: enabled,
                                        on_commit: {
                                            let action_context = action_context.clone();
                                            let target = perimeter.target.clone();
                                            let mut points = perimeter.points.clone();
                                            let world_points = perimeter.canvas_points.clone();
                                            let anchor = perimeter.anchor;
                                            move |value| {
                                                if let (Some(point), Some(world)) = (
                                                    points.get_mut(point_index),
                                                    world_points.get(point_index),
                                                ) {
                                                    let next = Vec2 { x: world.x, y: value };
                                                    *point = anchor.map_or(next, |anchor| anchor.local(next));
                                                }
                                                on_action
                                                    .call(
                                                        action_context
                                                            .edit_perimeter(
                                                                target.clone(),
                                                                points.clone(),
                                                                EditPhase::Commit,
                                                                format!(
                                                                    "outline-point-{}-{point_index}",
                                                                    action_context.envelope.revision,
                                                                ),
                                                            ),
                                                    );
                                            }
                                        },
                                    }
                                }
                                div { class: "my-3 flex flex-wrap gap-2",
                                    Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    aria_label: "Insert after {point_index + 1}",
    onclick: {
                                        let action_context = action_context.clone();
                                        let target = perimeter.target.clone();
                                        let mut points = perimeter.points.clone();
                                        move |_| {
                                            if point_count < 3 { return; }
                                            let next = (point_index + 1) % point_count;
                                            let a = points[point_index];
                                            let b = points[next];
                                            points.insert(point_index + 1, Vec2 { x: (a.x + b.x) / 2.0, y: (a.y + b.y) / 2.0 });
                                            selected_point.set(point_index + 1);
                                            on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.envelope.revision)));
                                        }
                                    },

                                        "Insert after"

                                     }
                                    Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled || point_count <= 3,
    aria_label: "Remove point {point_index + 1}",
    onclick: {
                                        let action_context = action_context.clone();
                                        let target = perimeter.target.clone();
                                        let mut points = perimeter.points.clone();
                                        move |_| {
                                            if point_count <= 3 { return; }
                                            points.remove(point_index);
                                            selected_point.set(point_index.saturating_sub(1));
                                                on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.envelope.revision)));
                                        }
                                    },

                                        "Remove point"

                                     }
                                }
                                div {
                                    class: "max-h-[220px] overflow-auto border border-border",
                                    role: "group",
                                    aria_label: "Outline points",
                                    div {
                                        class: "sticky top-0 grid grid-cols-[54px_minmax(0,1fr)_minmax(0,1fr)] items-center gap-2 bg-card px-2 py-1.5 text-xs font-semibold text-muted-foreground",
                                        aria_hidden: "true",
                                        span { "Point" }
                                        span { "X · mm" }
                                        span { "Y · mm" }
                                    }
                                    for (index, point) in perimeter.canvas_points.iter().enumerate() {
                                        ToggleGroup {
                                            class: "w-full! rounded-none! p-0!",
                                            aria_label: "Select outline point {index + 1}",
                                            pressed: Some(
                                                if index == point_index {
                                                    std::collections::HashSet::from([0usize])
                                                } else {
                                                    std::collections::HashSet::new()
                                                },
                                            ),
                                            on_pressed_change: move |_| selected_point.set(index),
                                            ToggleItem {
                                                class: "grid! w-full! grid-cols-[54px_minmax(0,1fr)_minmax(0,1fr)] gap-2 rounded-none! px-2 py-1.5 text-left text-xs! font-normal! tabular-nums max-[760px]:min-h-control-touch",
                                                index: 0usize,
                                                key: "outline-point-{index}",
                                                aria_label: "Select outline point {index + 1}",
                                                span { "{index + 1}" }
                                                span { "{point.x:.3}" }
                                                span { "{point.y:.3}" }
                                            }
                                        }
                                    }
                                }
                                if let Some(message) = projection
                                    .feedback
                                    .as_ref()
                                    .and_then(|feedback| feedback.message.as_ref())
                                {
                                    Alert { variant: AlertVariant::Destructive,AlertDescription {  "Outline change failed: {message}"  } }
                                }
                            }
                        }
                    } else {
                        {board_outline_inspector(BoardInspectorView { projection: projection.clone(), draft_serial, active_selection, corner_selection, point_attachment })}
                    }
                }
}

fn contour_preview(contours: &[Contour]) -> Option<(String, Vec<(String, bool)>)> {
    let mut points = contours.iter().flat_map(|contour| &contour.points);
    let first = points.next()?;
    let (min_x, max_x, min_y, max_y) = points.fold(
        (first.x, first.x, first.y, first.y),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            )
        },
    );
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let padding = width.max(height) * 0.08;
    let view_box = format!(
        "{} {} {} {}",
        min_x - padding,
        -(max_y + padding),
        width + padding * 2.0,
        height + padding * 2.0,
    );
    let paths = contours
        .iter()
        .map(|contour| {
            let points = contour
                .points
                .iter()
                .map(|point| format!("{},{}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(" ");
            (points, contour.hole)
        })
        .collect();
    Some((view_box, paths))
}

/// The Inspector's one-line ownership summary for the selected outline.
fn ownership_summary(ownership: OutlineVersionOwnership) -> &'static str {
    match ownership {
        OutlineVersionOwnership::Generated => {
            "Generated follows your keycaps and included components."
        }
        OutlineVersionOwnership::Linked => {
            "Linked refinement: it follows the board's current components and its settings stay editable. Freeze makes a separate edited copy and keeps this link."
        }
        OutlineVersionOwnership::Edited => {
            "Edited outline: an authored, fixed copy. Its geometry stays put when components move."
        }
    }
}
