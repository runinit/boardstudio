//! The board-outline (non-perimeter) section of the outline Inspector.
use super::super::OutlineActionKind;
use super::vec2_fields::vec2_fields;
use super::*;

mod connections;
mod drawing;
mod features;
mod settings;
mod versions;

/// Build an `on_commit` closure that maps the committed value to one outline
/// settings edit directly through the projection.
pub(super) fn commit_setting<T>(
    projection: OutlineInspectorProjection,
    edit: impl Fn(T) -> OutlineEdit,
) -> impl Fn(T) {
    move |value| projection.update(edit(value))
}

/// Build an `on_commit` closure that rewrites one saved feature: the committed
/// value mutates a copy of `before`, then the before/after pair is submitted.
pub(super) fn commit_feature_edit(
    projection: OutlineInspectorProjection,
    before: OutlineFeature,
    version_id: Option<String>,
    mutate: impl Fn(&mut OutlineFeature, f64),
) -> impl Fn(f64) {
    move |value| {
        let mut after = before.clone();
        mutate(&mut after, value);
        projection.dispatch(OutlineActionKind::SetFeature {
            version_id: version_id.clone(),
            before: before.clone(),
            after,
        });
    }
}

pub(super) struct BoardInspectorView {
    pub(super) projection: OutlineInspectorProjection,
    pub(super) draft_serial: Signal<u64>,
    pub(super) active_selection: Memo<Option<String>>,
    pub(super) corner_selection: Memo<Option<String>>,
    pub(super) point_attachment: Memo<Option<String>>,
}

pub(super) fn board_outline_inspector(view: BoardInspectorView) -> Element {
    let projection = &view.projection;
    let contour_view = contour_preview(&projection.contours);
    let mut perimeter_open = projection.ui.editing_points;
    let mut canvas_ui = projection.ui.canvas_ui;
    let enabled = projection.enabled;
    let active_choice = projection
        .versions
        .iter()
        .find(|item| Some(&item.id) == projection.active_version_id.as_ref());
    let active_ownership =
        active_choice.map_or(OutlineVersionOwnership::Generated, |item| item.ownership);
    let copy_projection = projection.clone();
    let freeze_projection = projection.clone();
    let refine_projection = projection.clone();
    let delete_projection = projection.clone();
    let can_freeze = projection.has_linked_active_version();
    let can_refine = projection.has_generated && projection.active_version_id.is_none();
    let delete_version_id = projection.active_version_id.clone();
    rsx! {
        section { class: "border-b border-border px-4 py-3.5 [&>p]:my-2 [&>p]:text-muted-foreground", "aria-label": "Board outline",
        onkeydown: crate::objects::stop_space_pan,
            div { class: "flex items-center justify-between gap-2",
                h2 { class: "m-0 text-[16px]! font-semibold!", "Board outline" }
                span { class: "text-muted-foreground text-xs", "{projection.board_name}" }
            }
            p { "{ownership_summary(active_ownership)}" }
            p { "{projection.contours.len()} accepted contours" }
            if let Some((view_box, paths)) = contour_view {
                svg {
                    class: "my-3 block h-[150px] w-full border border-border bg-canvas",
                    role: "img",
                    "aria-label": "Accepted active board outline",
                    view_box: "{view_box}",
                    g { transform: "scale(1,-1)",
                        for (index, (points, hole)) in paths.iter().enumerate() {
                            polygon {
                                key: "outline-preview-{index}",
                                class: if *hole { "fill-canvas stroke-destructive stroke-[0.3] [stroke-dasharray:0.8_0.55]" } else { "fill-geometry-fill stroke-geometry-board stroke-[0.3]" },
                                points: "{points}",
                            }
                        }
                    }
                }
            }
            div { class: "grid gap-3 mt-3 text-foreground",
                {versions::version_controls(&view, active_choice, active_ownership)}
                {settings::outline_settings(&view, active_ownership)}
            }
            if projection.perimeter.as_ref().is_some_and(|perimeter| perimeter.points.len() >= 3) {
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    onclick: move |_| {
                canvas_ui.write().selected_point = 0;
                perimeter_open.set(true);
            },

                    "Edit perimeter points"

                 }
            }
            section {
                class: "mt-4 grid gap-2 border-t border-border pt-3 [&_h3]:m-0 [&_h3]:text-xs [&>p]:m-0 [&>p]:text-xs [&>p]:text-muted-foreground",
                "aria-label": "Manual geometry",
                h3 { "Manual geometry" }
                {features::manual_features(projection)}
                {connections::connection_controls(&view)}
                {drawing::drawing_controls(&view)}
            }
            div { class: "mt-3 flex flex-wrap gap-2 [&_button]:min-h-9",
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    disabled: !projection.enabled,
                    onclick: move |_| copy_projection.dispatch(OutlineActionKind::Copy),
                    "Copy outline"
                }
                if can_freeze {
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        disabled: !projection.enabled,
                        onclick: {
                            let version_id = projection.active_version_id.clone().unwrap_or_default();
                            move |_| freeze_projection.dispatch(OutlineActionKind::Freeze { version_id: version_id.clone() })
                        },

                        "Freeze into an edited copy"

                    }
                }
                if can_refine {
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        disabled: !projection.enabled,
                        onclick: move |_| refine_projection.dispatch(OutlineActionKind::Refine),

                        "Refine outline"

                    }
                }
                if let Some(version_id) = delete_version_id {
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        disabled: !projection.enabled,
                        onclick: move |_| delete_projection.dispatch(OutlineActionKind::Delete { version_id: version_id.clone() }),
                        "Delete outline"
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
}
