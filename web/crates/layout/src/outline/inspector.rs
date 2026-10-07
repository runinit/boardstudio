//! Outline version Inspector.

use super::hook::{OutlineDrawTool, OutlineInspectorProjection};
use super::overlays::{OutlineCoordinate, OutlineDimension, polygon_area};
use super::planner::{
    PerimeterAnchor, attach_connection_point, connection_point_world, move_connection_point,
};
use crate::outline_settings::OutlineEdit;
use boardstudio_core::model::{
    Contour, CornerStyle, EditPhase, Operation, OutlineControlPoint, OutlineFeature, Side, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;

#[component]
pub fn OutlineVersionInspector(projection: OutlineInspectorProjection) -> Element {
    let contour_view = contour_preview(&projection.contours);
    let copy = projection.copy_action();
    let delete = projection.delete_action();
    let copy_handler = projection.on_action;
    let delete_handler = projection.on_action;
    let mut version_name = use_signal(|| None::<(String, String, String)>);
    let mut perimeter_open = projection.editing_points;
    let mut selected_point = projection.selected_point;
    let mut selected_feature_id = projection.selected_feature_id;
    let mut selected_connection_id = projection.selected_connection_id;
    let name_draft = version_name()
        .filter(|(id, baseline, _)| {
            Some(id) == projection.active_version_id.as_ref()
                && baseline == &projection.version_name
        })
        .map(|(_, _, draft)| draft)
        .unwrap_or_else(|| projection.version_name.clone());
    let active_version = projection.active_version_id.clone();
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let one_shot_enabled = enabled && !projection.one_shot_pending;
    let mut drawing_operation = projection.drawing_operation;
    let mut drawing_points = projection.drawing_points;
    let mut draft_serial = use_signal(|| 0u64);
    let active_value = projection.active_version_id.clone().unwrap_or_default();
    let perimeter = projection.perimeter.clone();
    let corner_value = match projection.settings.corners {
        CornerStyle::Sharp => "sharp",
        CornerStyle::Fillet => "fillet",
        CornerStyle::Chamfer => "chamfer",
    };
    let size_label: &'static str = match projection.settings.corners {
        CornerStyle::Fillet => "Fillet radius",
        _ => "Chamfer size",
    };
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
                section { class: "m1-outline-inspector m1-outline-point-editor", "aria-label": "Perimeter",
                    onkeydown: {
                        let mut selected_context = projection.selected_context;
                        let scope = projection.scope.clone();
                        let board_id = projection.board_id.clone();
                        move |event: KeyboardEvent| {
                            if event.data().key().to_string() != "Escape" { return; }
                            event.prevent_default();
                            event.stop_propagation();
                            let Some(selected) = selected_context.read().clone() else { return; };
                            if selected.scope != scope
                                || !matches!(selected.context, crate::objects::TreeContext::Outline { board_id: selected_board } if selected_board == board_id)
                            { return; }
                            selected_context.set(Some(crate::objects::ScopedTreeContext {
                                scope: scope.clone(),
                                context: crate::objects::TreeContext::Board { board_id: board_id.clone() },
                            }));
                            let focus_context = selected_context;
                            let focus_scope = scope.clone();
                            let focus_board_id = board_id.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                gloo_timers::future::TimeoutFuture::new(0).await;
                                if !focus_context.read().as_ref().is_some_and(|selected| {
                                    selected.scope == focus_scope
                                        && matches!(
                                            &selected.context,
                                            crate::objects::TreeContext::Board { board_id }
                                                if board_id == &focus_board_id
                                        )
                                }) {
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
                    div { class: "m1-outline-inspector-heading",
                        h2 { "Perimeter" }
                        button { r#type: "button", disabled: !enabled, onclick: move |_| perimeter_open.set(false), "Done" }
                    }
                    p { if projection.active_version_id.is_some() { "This outline stays fixed when components move. Changes save as you edit." } else { "The first point change creates and activates a fixed copy. Generated stays available." } }
                    h3 { class: "m1-outline-point-heading", "Point {point_index + 1} of {point_count}" }
                    div { class: "m1-outline-coordinate-fields",
                        OutlineCoordinate {
                            key: format!("{:?}:{:?}:{point_index}:x", action_context.scope, perimeter.target),
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
                                    if let (Some(point), Some(world)) = (points.get_mut(point_index), world_points.get(point_index)) {
                                        let next = Vec2 { x: value, y: world.y };
                                        *point = anchor.map_or(next, |anchor| anchor.local(next));
                                    }
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                        }
                        OutlineCoordinate {
                            key: format!("{:?}:{:?}:{point_index}:y", action_context.scope, perimeter.target),
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
                                    if let (Some(point), Some(world)) = (points.get_mut(point_index), world_points.get(point_index)) {
                                        let next = Vec2 { x: world.x, y: value };
                                        *point = anchor.map_or(next, |anchor| anchor.local(next));
                                    }
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                        }
                    }
                    div { class: "m1-outline-point-actions",
                        button {
                            class: "m1-outline-insert-point",
                            r#type: "button",
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
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                            "Insert after"
                        }
                        button {
                            class: "m1-outline-remove-point",
                            r#type: "button",
                            disabled: !enabled || point_count <= 3,
                            aria_label: "Remove point {point_index + 1}",
                            title: if point_count <= 3 { "Keep at least three points." } else { "Remove the selected point" },
                            onclick: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |_| {
                                    if point_count <= 3 { return; }
                                    points.remove(point_index);
                                    selected_point.set(point_index.saturating_sub(1));
                                        on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                            "Remove point"
                        }
                    }
                    div { class: "m1-outline-point-list", role: "group", aria_label: "Outline points",
                        div { class: "m1-outline-point-columns", aria_hidden: "true",
                            span { "Point" } span { "X · mm" } span { "Y · mm" }
                        }
                        for (index, point) in perimeter.canvas_points.iter().enumerate() {
                            button {
                                key: "outline-point-{index}",
                                r#type: "button",
                                aria_label: "Select outline point {index + 1}",
                                aria_pressed: "{index == point_index}",
                                onclick: move |_| selected_point.set(index),
                                span { "{index + 1}" }
                                span { "{point.x:.3}" }
                                span { "{point.y:.3}" }
                            }
                        }
                    }
                    if let Some(feedback) = projection.feedback.as_ref() {
                        p { role: if feedback.state == "pending" || feedback.state == "saved" { "status" } else { "alert" }, "data-state": feedback.state,
                            if feedback.state == "pending" { "Saving outline…" }
                            else if feedback.state == "saved" { "Saved" }
                            else { "Outline change failed: {feedback.message.as_deref().unwrap_or_default()}" }
                        }
                    }
                }
            }
        } else {
        section { class: "m1-outline-inspector", "aria-label": "Board outline",
            div { class: "m1-outline-inspector-heading", h2 { "Board outline" } span { class: "m1-outline-board-name", "{projection.board_name}" } }
            p { if projection.active_version_id.is_some() { "A fixed outline; component placement is shared with every version." } else { "Generated follows your keycaps and included components." } }
            p { "{projection.contours.len()} accepted contours" }
            if let Some((view_box, paths)) = contour_view {
                svg { class: "m1-outline-preview", role: "img", "aria-label": "Accepted active board outline", view_box: "{view_box}",
                    g { transform: "scale(1,-1)",
                        for (index, (points, hole)) in paths.iter().enumerate() {
                            polygon { key: "outline-preview-{index}", class: if *hole { "m1-outline is-hole" } else { "m1-outline" }, points: "{points}" }
                        }
                    }
                }
            }
            div { class: "m1-outline-settings",
                label { class: "m1-outline-field",
                    span { "Active outline" }
                    select {
                        aria_label: "Active outline",
                        value: "{active_value}",
                        disabled: !one_shot_enabled,
                        onchange: {
                            let action_context = action_context.clone();
                            move |event: FormEvent| {
                                let value = event.value();
                                on_action.call(action_context.activate_action((!value.is_empty()).then_some(value)));
                            }
                        },
                        option { value: "", selected: active_value.is_empty(), "Generated" }
                        for item in projection.versions.iter() {
                            option { key: "{item.id}", value: "{item.id}", selected: item.id == active_value, "{item.name}" }
                        }
                    }
                }
                if let Some(version_id) = active_version.as_ref() {
                    label { class: "m1-outline-field",
                        span { "Version name" }
                        input {
                            aria_label: "Outline version name",
                            maxlength: "120",
                            value: "{name_draft}",
                            disabled: !enabled,
                            aria_invalid: name_draft.trim().is_empty(),
                            oninput: {
                                let version_id = version_id.clone();
                                let baseline = projection.version_name.clone();
                                move |event: FormEvent| version_name.set(Some((version_id.clone(), baseline.clone(), event.value())))
                            },
                            onblur: {
                                let version_id = version_id.clone();
                                let accepted_name = projection.version_name.clone();
                                let action_context = action_context.clone();
                                move |_| {
                                    let Some((draft_id, baseline, draft)) = version_name() else { return; };
                                    if draft_id != version_id || baseline != accepted_name { return; }
                                    let name = draft.trim().to_owned();
                                    if !name.is_empty() && name.len() <= 120 && name != accepted_name {
                                        on_action.call(action_context.action(OutlineEdit::RenameVersion { version_id: version_id.clone(), name }));
                                    }
                                }
                            },
                            onkeydown: {
                                move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                    "Enter" => {
                                        event.prevent_default();
                                        if let Some(input) = event.data().try_as_web_event()
                                            .and_then(|event| event.target())
                                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                        { let _ = input.blur(); }
                                    }
                                    "Escape" => {
                                        event.prevent_default();
                                        version_name.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if !projection.has_generated && projection.active_version_id.is_none() {
                    button {
                        class: "m1-outline-action",
                        disabled: !one_shot_enabled,
                        onclick: {
                            let action_context = action_context.clone();
                            move |_| on_action.call(action_context.action(OutlineEdit::CreateAutomatic))
                        },
                        "Generate automatic outline"
                    }
                }
                if projection.has_generated || projection.active_version_id.is_some() {
                    label { class: "m1-outline-field",
                        span { "Corners" }
                        select {
                            aria_label: "Outline corners",
                            value: "{corner_value}",
                            disabled: !enabled,
                            onchange: {
                                let action_context = action_context.clone();
                                move |event: FormEvent| {
                                let corner = match event.value().as_str() {
                                    "fillet" => CornerStyle::Fillet,
                                    "chamfer" => CornerStyle::Chamfer,
                                    _ => CornerStyle::Sharp,
                                };
                                on_action.call(action_context.action(OutlineEdit::SetCorners(corner)));
                            }},
                            option { value: "sharp", "Sharp" }
                            option { value: "fillet", "Fillet" }
                            option { value: "chamfer", "Chamfer" }
                        }
                    }
                    if projection.settings.corners != CornerStyle::Sharp {
                        OutlineDimension {
                            label: size_label,
                            value: projection.settings.size,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetSize(value)))
                            },
                        }
                    }
                    if projection.active_version_id.is_none() {
                        if let Some(margin) = projection.generated_margin {
                            OutlineDimension {
                                label: "Outline margin",
                                value: margin,
                                minimum: 0.0,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    move |value| on_action.call(action_context.action(OutlineEdit::SetMargin(value)))
                                },
                            }
                        }
                        OutlineDimension {
                            label: "Bridge width",
                            value: projection.settings.bridge_width,
                            minimum: 0.001,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetBridgeWidth(value)))
                            },
                        }
                    }
                    if projection.active_version_id.is_none() && !projection.gaps.is_empty() {
                        fieldset { class: "m1-outline-controls", disabled: !enabled,
                            legend { "Gap repair" }
                            p { "Keep gap preserves an intentional recess and follows its source components." }
                            for (index, gap) in projection.gaps.iter().enumerate() {
                                div { class: "m1-outline-gap-row",
                                    button {
                                        class: "m1-outline-gap-focus",
                                        r#type: "button",
                                        aria_label: "Show gap {index + 1}",
                                        disabled: !enabled,
                                        onclick: {
                                            let action_context = action_context.clone();
                                            let gap_id = gap.id.clone();
                                            move |_| on_action.call(action_context.focus_gap(gap_id.clone()))
                                        },
                                        "Gap {index + 1} · {gap.span:.1} mm span"
                                    }
                                    label { class: "m1-outline-field m1-outline-gap",
                                        input {
                                            r#type: "checkbox",
                                            aria_label: "Keep gap {index + 1}",
                                            checked: gap.protected,
                                            onchange: {
                                                let gap_id = gap.id.clone();
                                                let action_context = action_context.clone();
                                                move |event: FormEvent| on_action.call(action_context.action(OutlineEdit::SetProtectedGap { gap_id: gap_id.clone(), protected: event.checked() }))
                                            }
                                        }
                                        span { "Keep gap" }
                                    }
                                }
                            }
                        }
                    }
                    fieldset { class: "m1-outline-controls", disabled: !enabled,
                        legend { "Advanced cleanup and clearance" }
                        if projection.active_version_id.is_none() {
                            label { class: "m1-outline-field m1-outline-gap",
                                input {
                                    r#type: "checkbox",
                                    aria_label: "Automatic gap cleanup",
                                    checked: projection.repair.enabled,
                                    onchange: {
                                        let action_context = action_context.clone();
                                        move |event: FormEvent| on_action.call(action_context.action(OutlineEdit::SetRepairEnabled(event.checked())))
                                    },
                                }
                                span { "Automatic cleanup" }
                            }
                            OutlineDimension {
                                label: "Maximum gap span",
                                value: projection.repair.maximum_gap_span,
                                minimum: 0.0,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    move |value| on_action.call(action_context.action(OutlineEdit::SetMaximumGapSpan(value)))
                                },
                            }
                        }
                        OutlineDimension {
                            label: "Minimum connection width",
                            value: projection.repair.minimum_connection_width,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetMinimumConnectionWidth(value)))
                            },
                        }
                        OutlineDimension {
                            label: "PCB edge clearance",
                            value: projection.repair.edge_clearance,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetEdgeClearance(value)))
                            },
                        }
                        p { "Support and clearance findings block affected fabrication exports. Editing and project saving stay available." }
                        if projection.active_version_id.is_none() {
                            for (index, gap) in projection.repair.keep_gaps.iter().enumerate() {
                                button {
                                    class: "m1-outline-remove-gap",
                                    aria_label: "Remove protected gap {index + 1}",
                                    onclick: {
                                        let gap_id = gap.id.clone();
                                        let action_context = action_context.clone();
                                        move |_| on_action.call(action_context.action(OutlineEdit::RemoveProtectedGap { gap_id: gap_id.clone() }))
                                    },
                                    "Remove protected gap {index + 1}"
                                }
                            }
                        }
                    }
                }
            }
            if projection.perimeter.as_ref().is_some_and(|perimeter| perimeter.points.len() >= 3) {
                button {
                    class: "m1-outline-action",
                    r#type: "button",
                    disabled: !enabled,
                    onclick: move |_| {
                        selected_point.set(0);
                        perimeter_open.set(true);
                    },
                    "Edit perimeter points"
                }
            }
            section { class: "m1-outline-manual", "aria-label": "Manual geometry",
                h3 { "Manual geometry" }
                if !projection.geometry_features.is_empty() {
                    div { class: "m1-outline-feature-list", role: "group", aria_label: "Saved outline geometry",
                        h4 { "Saved geometry" }
                        for (index, feature) in projection.geometry_features.iter().enumerate() {
                            {
                                let feature_id = feature.id().to_owned();
                                let selected = selected_feature_id().as_deref() == Some(feature_id.as_str());
                                let feature_name = match feature {
                                    OutlineFeature::Polygon { operation, .. } => match operation { Operation::Add => "Addition", Operation::Subtract => "Cutout" },
                                    OutlineFeature::Rect { operation, .. } => match operation { Operation::Add => "Rectangle addition", Operation::Subtract => "Rectangle cutout" },
                                    OutlineFeature::PartEnvelope { .. } => "Generated perimeter",
                                };
                                rsx! {
                                    div { class: "m1-outline-feature-row", key: "outline-feature-{feature_id}",
                                        button {
                                            r#type: "button",
                                            aria_pressed: "{selected}",
                                            disabled: !enabled,
                                            onclick: {
                                                let feature_id = feature_id.clone();
                                                move |_| selected_feature_id.set(Some(feature_id.clone()))
                                            },
                                            "{feature_name} {index + 1}"
                                        }
                                        if matches!(feature, OutlineFeature::Polygon { .. }) {
                                            button {
                                                r#type: "button",
                                                disabled: !enabled,
                                                onclick: {
                                                    let feature_id = feature_id.clone();
                                                    move |_| {
                                                        selected_feature_id.set(Some(feature_id.clone()));
                                                        selected_point.set(0);
                                                        perimeter_open.set(true);
                                                    }
                                                },
                                                "Edit points"
                                            }
                                        }
                                        if let Some(version_id) = active_version.as_ref().filter(|_| index > 0) {
                                            button {
                                                r#type: "button",
                                                class: "m1-outline-remove-feature",
                                                disabled: !one_shot_enabled,
                                                aria_label: "Remove {feature_name} {index + 1}",
                                                onclick: {
                                                    let action_context = action_context.clone();
                                                    let version_id = version_id.clone();
                                                    let feature_id = feature_id.clone();
                                                    move |_| on_action.call(action_context.remove_feature(version_id.clone(), feature_id.clone()))
                                                },
                                                "Remove"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(feature) = projection.geometry_features.iter().find(|feature| Some(feature.id()) == selected_feature_id().as_deref()) {
                    if let OutlineFeature::Rect { center, size, anchor_part_id, .. } = feature {
                        fieldset { class: "m1-outline-feature-editor", disabled: !enabled,
                            legend { "Selected rectangle" }
                            div { class: "m1-outline-coordinate-fields",
                                OutlineCoordinate {
                                    key: "rectangle-{feature.id()}-center-x",
                                    label: "Rectangle center X mm",
                                    value: anchor_part_id.as_deref()
                                        .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                        .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back }.world(*center))
                                        .unwrap_or(*center).x,
                                    editable: enabled,
                                    on_commit: {
                                        let action_context = action_context.clone();
                                        let before = feature.clone();
                                        let version_id = active_version.clone();
                                        let anchor = anchor_part_id.as_deref()
                                            .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                            .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back });
                                        let world_center = anchor.map_or(*center, |anchor| anchor.world(*center));
                                        move |value| {
                                            let local = anchor.map_or(Vec2 { x: value, y: world_center.y }, |anchor| anchor.local(Vec2 { x: value, y: world_center.y }));
                                            let mut after = before.clone();
                                            if let OutlineFeature::Rect { center, .. } = &mut after { *center = local; }
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                        }
                                    },
                                }
                                OutlineCoordinate {
                                    key: "rectangle-{feature.id()}-center-y",
                                    label: "Rectangle center Y mm",
                                    value: anchor_part_id.as_deref()
                                        .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                        .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back }.world(*center))
                                        .unwrap_or(*center).y,
                                    editable: enabled,
                                    on_commit: {
                                        let action_context = action_context.clone();
                                        let before = feature.clone();
                                        let version_id = active_version.clone();
                                        let anchor = anchor_part_id.as_deref()
                                            .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                            .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back });
                                        let world_center = anchor.map_or(*center, |anchor| anchor.world(*center));
                                        move |value| {
                                            let local = anchor.map_or(Vec2 { x: world_center.x, y: value }, |anchor| anchor.local(Vec2 { x: world_center.x, y: value }));
                                            let mut after = before.clone();
                                            if let OutlineFeature::Rect { center, .. } = &mut after { *center = local; }
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                        }
                                    },
                                }
                            }
                            OutlineDimension {
                                label: "Rectangle width",
                                value: size.x,
                                minimum: 0.001,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    let before = feature.clone();
                                    let version_id = active_version.clone();
                                    move |value| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::Rect { size, .. } = &mut after { size.x = value; }
                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    }
                                },
                            }
                            OutlineDimension {
                                label: "Rectangle height",
                                value: size.y,
                                minimum: 0.001,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    let before = feature.clone();
                                    let version_id = active_version.clone();
                                    move |value| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::Rect { size, .. } = &mut after { size.y = value; }
                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    }
                                },
                            }
                            if let OutlineFeature::Rect { radius, .. } = feature {
                                OutlineDimension {
                                    label: "Rectangle corner radius",
                                    value: *radius,
                                    minimum: 0.0,
                                    editable: enabled,
                                    on_commit: {
                                        let action_context = action_context.clone();
                                        let before = feature.clone();
                                        let version_id = active_version.clone();
                                        move |value| {
                                            let mut after = before.clone();
                                            if let OutlineFeature::Rect { radius, .. } = &mut after { *radius = value; }
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
                if !projection.connections.is_empty() {
                    div { class: "m1-outline-connection-list", role: "group", aria_label: "Outline connections",
                        h4 { "Connections" }
                        for (index, connection) in projection.connections.iter().enumerate() {
                            div { class: "m1-outline-feature-row", key: "outline-connection-{connection.id}",
                                button {
                                    r#type: "button",
                                    aria_pressed: "{selected_connection_id().as_deref() == Some(connection.id.as_str())}",
                                    onclick: {
                                        let id = connection.id.clone();
                                        move |_| selected_connection_id.set(Some(id.clone()))
                                    },
                                    "Connection {index + 1} · {connection.points.len()} points · {connection.width:.2} mm"
                                }
                                if let Some(before) = projection.connection_feature.as_ref() {
                                    button {
                                        r#type: "button",
                                        class: "m1-outline-remove-feature",
                                        disabled: !enabled,
                                        aria_label: "Remove connection {index + 1}",
                                        onclick: {
                                            let action_context = action_context.clone();
                                            let before = before.clone();
                                            let version_id = active_version.clone();
                                            let connection_id = connection.id.clone();
                                            move |_| {
                                                let mut after = before.clone();
                                                if let OutlineFeature::PartEnvelope { connections, .. } = &mut after {
                                                    connections.retain(|item| item.id != connection_id);
                                                }
                                                on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                                selected_connection_id.set(None);
                                            }
                                        },
                                        "Remove"
                                    }
                                }
                            }
                        }
                    }
                }
                if let (Some(connection_id), Some(before)) = (selected_connection_id().as_ref(), projection.connection_feature.as_ref()) {
                    if let OutlineFeature::PartEnvelope { connections, .. } = before {
                        if let Some(connection) = connections.iter().find(|connection| &connection.id == connection_id) {
                            OutlineDimension {
                                label: "Connection width",
                                value: connection.width,
                                minimum: 0.001,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    let before = before.clone();
                                    let version_id = active_version.clone();
                                    let connection_id = connection_id.clone();
                                    move |value| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after {
                                            if let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id) {
                                                connection.width = value;
                                            }
                                        }
                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    }
                                },
                            }
                            if !connection.points.is_empty() {
                                {
                                    let index = selected_point().min(connection.points.len() - 1);
                                    let point = &connection.points[index];
                                    let world = connection_point_world(point, &projection.outline_parts);
                                    rsx! {
                                        h4 { "Connection point {index + 1} of {connection.points.len()}" }
                                        div { class: "m1-outline-coordinate-fields",
                                            OutlineCoordinate {
                                                key: "connection-{connection.id}-{index}-x",
                                                label: format!("Point {} X mm", index + 1),
                                                value: world.x,
                                                editable: enabled,
                                                on_commit: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |value| {
                                                        let next = move_connection_point(&before, &connection_id, index, Vec2 { x: value, y: world.y }, &parts);
                                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                                    }
                                                },
                                            }
                                            OutlineCoordinate {
                                                key: "connection-{connection.id}-{index}-y",
                                                label: format!("Point {} Y mm", index + 1),
                                                value: world.y,
                                                editable: enabled,
                                                on_commit: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |value| {
                                                        let next = move_connection_point(&before, &connection_id, index, Vec2 { x: world.x, y: value }, &parts);
                                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                                    }
                                                },
                                            }
                                        }
                                        label { class: "m1-outline-field",
                                            span { "Point attachment" }
                                            select {
                                                aria_label: "Point {index + 1} attachment",
                                                value: "{point.part_id.as_deref().unwrap_or_default()}",
                                                disabled: !enabled,
                                                onchange: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |event: FormEvent| {
                                                        let value = event.value();
                                                        let next = attach_connection_point(&before, &connection_id, index, (!value.is_empty()).then_some(value.as_str()), &parts);
                                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                                    }
                                                },
                                                option { value: "", "Fixed on board" }
                                                if let Some(id) = point.part_id.as_ref().filter(|id| !projection.outline_parts.iter().any(|part| &part.id == *id)) {
                                                    option { value: "{id}", "Missing component" }
                                                }
                                                for part in projection.outline_parts.iter() {
                                                    option { key: "{part.id}", value: "{part.id}", "{part.reference}" }
                                                }
                                            }
                                        }
                                        div { class: "m1-outline-point-actions",
                                            button {
                                                r#type: "button",
                                                disabled: !enabled || index + 1 == connection.points.len(),
                                                aria_label: "Insert after connection point {index + 1}",
                                                title: if index + 1 == connection.points.len() { "A connection needs its final endpoint." } else { "Insert a fixed point after this point." },
                                                onclick: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |_| {
                                                        let mut after = before.clone();
                                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                                            && let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id)
                                                            && index + 1 < connection.points.len()
                                                        {
                                                            let a = connection_point_world(&connection.points[index], &parts);
                                                            let b = connection_point_world(&connection.points[index + 1], &parts);
                                                            connection.points.insert(index + 1, OutlineControlPoint {
                                                                at: Vec2 { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 },
                                                                part_id: None,
                                                            });
                                                            selected_point.set(index + 1);
                                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                                        }
                                                    }
                                                },
                                                "Insert after"
                                            }
                                            button {
                                                r#type: "button",
                                                disabled: !enabled || connection.points.len() <= 2,
                                                aria_label: "Remove connection point {index + 1}",
                                                title: if connection.points.len() <= 2 { "Keep at least two points." } else { "Remove the selected point" },
                                                onclick: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    move |_| {
                                                        let mut after = before.clone();
                                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                                            && let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id)
                                                            && connection.points.len() > 2 && index < connection.points.len()
                                                        {
                                                            connection.points.remove(index);
                                                            selected_point.set(index.saturating_sub(1));
                                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                                        }
                                                    }
                                                },
                                                "Remove point"
                                            }
                                        }
                                        div { class: "m1-outline-point-list", role: "group", aria_label: "Connection points",
                                            for (index, control) in connection.points.iter().enumerate() {
                                                {
                                                    let point = connection_point_world(control, &projection.outline_parts);
                                                    rsx! {
                                                        button {
                                                            key: "connection-point-{index}",
                                                            r#type: "button",
                                                            aria_label: "Select connection point {index + 1}",
                                                            aria_pressed: "{index == selected_point()}",
                                                            onclick: move |_| selected_point.set(index),
                                                            span { "{index + 1}" }
                                                            span { "{point.x:.3}" }
                                                            span { "{point.y:.3}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                        }
                    }
                }
                }
                if let Some(tool) = drawing_operation() {
                    p { role: "status", "Click the canvas to add points. Enter finishes; Escape cancels." }
                    p { "{drawing_points.read().len()} points" }
                    div { class: "m1-outline-actions",
                        button {
                            r#type: "button",
                            disabled: drawing_points.read().is_empty(),
                            onclick: move |_| { let mut points = drawing_points.read().clone(); points.pop(); drawing_points.set(points); },
                            "Undo point"
                        }
                        button { r#type: "button", onclick: move |_| { drawing_operation.set(None); drawing_points.set(Vec::new()); }, "Cancel drawing" }
                        button {
                            r#type: "button",
                            disabled: !one_shot_enabled || drawing_points.read().len() < if tool == OutlineDrawTool::Connect { 2 } else { 3 }
                                || matches!(tool, OutlineDrawTool::Polygon(_)) && polygon_area(&drawing_points.read()).abs() < 1e-6,
                            onclick: {
                                let action_context = action_context.clone();
                                let board_id = projection.board_id.clone();
                                let revision = projection.revision;
                                move |_| {
                                    let minimum_points = if tool == OutlineDrawTool::Connect { 2 } else { 3 };
                                    let points = drawing_points.read().clone();
                                    if points.len() < minimum_points { return; }
                                    match tool {
                                        OutlineDrawTool::Polygon(operation) => {
                                            if polygon_area(&points).abs() < 1e-6 { return; }
                                            let serial = draft_serial() + 1;
                                            draft_serial.set(serial);
                                            let feature = OutlineFeature::Polygon {
                                                id: format!("outline-manual-{board_id}-{}-{serial}", revision),
                                                points,
                                                anchor_part_id: None,
                                                operation,
                                            };
                                            on_action.call(action_context.add_feature(feature));
                                        }
                                        OutlineDrawTool::Connect => on_action.call(action_context.add_connection(points)),
                                    }
                                    drawing_operation.set(None);
                                    drawing_points.set(Vec::new());
                                }
                            },
                            "Finish drawing"
                        }
                    }
                } else {
                    div { class: "m1-outline-actions",
                        button {
                            r#type: "button", disabled: !one_shot_enabled,
                            onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Polygon(Operation::Add))); },
                            "Draw addition"
                        }
                        button {
                            r#type: "button", disabled: !one_shot_enabled,
                            onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Polygon(Operation::Subtract))); },
                            "Draw cutout"
                        }
                        button {
                            r#type: "button",
                            disabled: !one_shot_enabled || projection.active_version_id.is_some() || !projection.has_generated,
                            title: if projection.active_version_id.is_some() { "Select Generated to add a linked connection." } else if !projection.has_generated { "Generate an automatic outline first." } else { "" },
                            onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Connect)); },
                            "Connect points"
                        }
                    }
                }
            }
            div { class: "m1-outline-actions",
                button { r#type: "button", disabled: !projection.enabled || projection.one_shot_pending, onclick: move |_| copy_handler.call(copy.clone()), "Copy outline" }
                if let Some(delete) = delete {
                    button { r#type: "button", disabled: !projection.enabled || projection.one_shot_pending, onclick: move |_| delete_handler.call(delete.clone()), "Delete outline" }
                }
            }
            if let Some(feedback) = projection.feedback.as_ref() {
                p { role: if feedback.state == "pending" || feedback.state == "saved" { "status" } else { "alert" }, "data-state": feedback.state,
                    if feedback.state == "pending" { "Saving outline…" }
                    else if feedback.state == "saved" { "Saved" }
                    else { "Outline change failed: {feedback.message.as_deref().unwrap_or_default()}" }
                }
            }
        }
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
