//! Layout Inspector controls and synchronous commit path for accepted keycap size.
use super::keycap_resize::ResizeAxis;
use super::keycap_size_logic::{
    overlap_references, placements, quarter, resize_resolver_for, selected_keycap_ids,
};
use super::{ScopedTreeContext, TreeContext};
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Lifecycle};
use boardstudio_model::model::Vec2;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{SelectField, ToggleField};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::field::FieldError;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct KeySizeItem {
    pub id: String,
    pub size: Vec2,
    pub pitch: Vec2,
    pub gap: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeySizeProjection {
    pub board_id: String,
    pub selected_ids: Vec<String>,
    pub items: Vec<KeySizeItem>,
    pub units: Vec2,
    pub mixed: bool,
    pub mixed_x: bool,
    pub mixed_y: bool,
    pub overlap_references: Vec<String>,
}

#[derive(Clone, PartialEq)]
pub struct KeySizeMount {
    pub projection: Option<KeySizeProjection>,
    pub editable: bool,
    pub error: Option<String>,
    pub on_resize: EventHandler<(Vec2, Option<ResizeAxis>)>,
}

pub fn use_key_size(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
) -> KeySizeMount {
    let _ = version();
    let current_generation = scope_generation();
    let mut error = use_signal(|| None::<String>);
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let (projection, editable) = project(&runtime, selected.as_ref(), current_workspace)
        .map_or((None, false), |(p, e)| (Some(p), e));
    let error_scope = (
        current_workspace,
        current_generation,
        selected.map(|s| (s.scope, s.context)),
        projection.as_ref().map(|p| p.selected_ids.clone()),
    );
    use_effect(use_reactive((&error_scope,), move |_| {
        if error.peek().is_some() {
            error.set(None);
        }
    }));

    let on_resize = use_callback({
        let runtime = runtime.clone();
        move |(units, axis): (Vec2, Option<ResizeAxis>)| {
            if workspace() != Workspace::Layout {
                return;
            }
            let selected = selected_context.peek().clone();
            let Some((current, editable)) = project(&runtime, selected.as_ref(), Workspace::Layout)
            else {
                return;
            };
            if !should_send(&current, units, axis) {
                return;
            }
            if !editable {
                error.set(Some(
                    "The accepted layout is not ready to edit. Wait for it to save, then retry."
                        .into(),
                ));
                return;
            }
            let model = runtime.model();
            let Some(accepted) = model.accepted.as_ref() else {
                return;
            };
            let result = runtime.commit(
                "layout-key-size",
                resize_resolver_for(
                    &accepted.document,
                    current.board_id,
                    current.selected_ids,
                    units,
                    axis,
                ),
            );
            error.set(result.err());
        }
    });

    KeySizeMount {
        projection,
        editable,
        error: error.read().clone(),
        on_resize,
    }
}

fn should_send(projection: &KeySizeProjection, units: Vec2, axis: Option<ResizeAxis>) -> bool {
    match axis {
        Some(ResizeAxis::X) => {
            projection.mixed_x
                || projection
                    .items
                    .iter()
                    .any(|item| quarter((item.size.x + item.gap.x) / item.pitch.x) != units.x)
        }
        Some(ResizeAxis::Y) => {
            projection.mixed_y
                || projection
                    .items
                    .iter()
                    .any(|item| quarter((item.size.y + item.gap.y) / item.pitch.y) != units.y)
        }
        None => projection.mixed || units != projection.units,
    }
}

fn project(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    workspace: Workspace,
) -> Option<(KeySizeProjection, bool)> {
    if workspace != Workspace::Layout {
        return None;
    }
    let model = runtime.model();
    let scope = runtime.scope()?;
    let selected = selected?;
    if scope != selected.scope {
        return None;
    }
    if !matches!(
        selected.context,
        TreeContext::Matrix { .. }
            | TreeContext::Row { .. }
            | TreeContext::Column { .. }
            | TreeContext::Key { .. }
    ) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
    {
        return None;
    }
    if !crate::selection::context_is_current(&model, &scope, &selected.context) {
        return None;
    }
    let placement_list = placements(
        &snapshot.document,
        &snapshot.scene.matrix_scenes,
        &scope.board_id,
    );
    // The tree context identifies the current Inspector section, but Ctrl/Shift canvas
    // selection can contain parts that are not members of the last hit context. Use the
    // accepted selected IDs as the authority, matching React's selectedKeycaps projection.
    let selected_set = selected_keycap_ids(&model.selected_part_ids, &placement_list);
    let overlap_references =
        overlap_references(&snapshot.document.parts, &placement_list, &selected_set);
    let items: Vec<_> = placement_list
        .iter()
        .filter(|placement| selected_set.contains(&placement.id))
        .filter_map(|placement| {
            let matrix = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == placement.matrix_id)?;
            Some(KeySizeItem {
                id: placement.id.clone(),
                size: placement.size,
                pitch: matrix.pitch,
                gap: matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 }),
            })
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    let to_units = |item: &KeySizeItem| Vec2 {
        x: quarter((item.size.x + item.gap.x) / item.pitch.x),
        y: quarter((item.size.y + item.gap.y) / item.pitch.y),
    };
    let units = to_units(&items[0]);
    let unit_items: Vec<_> = items.iter().map(to_units).collect();
    let mixed_x = unit_items.iter().skip(1).any(|c| c.x != units.x);
    let mixed_y = unit_items.iter().skip(1).any(|c| c.y != units.y);
    let mixed = mixed_x || mixed_y;
    // Key-size edits commit synchronously against the accepted snapshot while an earlier save is
    // in flight, so only an unrecoverable, preview/gesture, or unopened session disables controls.
    let editable = model.lifecycle == Lifecycle::Ready
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
        && model.display_preview.is_none()
        && model.gesture.is_none();
    Some((
        KeySizeProjection {
            board_id: scope.board_id,
            selected_ids: items.iter().map(|item| item.id.clone()).collect(),
            items,
            units,
            mixed,
            mixed_x,
            mixed_y,
            overlap_references,
        },
        editable,
    ))
}

#[component]
pub fn KeySizeControls(mount: ReadSignal<KeySizeMount>) -> Element {
    let on_size_commit = use_callback(move |choice: String| {
        let Ok(size) = choice.parse::<f64>() else {
            return;
        };
        let current = mount.peek();
        let Some(projection) = current.projection.as_ref() else {
            return;
        };
        let next = if projection.units.y > projection.units.x {
            Vec2 { x: 1.0, y: size }
        } else {
            Vec2 { x: size, y: 1.0 }
        };
        current.on_resize.call((next, None));
    });
    let on_rotate_commit = use_callback(move |checked: bool| {
        let current = mount.peek();
        let Some(projection) = current.projection.as_ref() else {
            return;
        };
        let long = projection.units.x.max(projection.units.y);
        let short = projection.units.x.min(projection.units.y);
        let next = if checked {
            Vec2 { x: short, y: long }
        } else {
            Vec2 { x: long, y: short }
        };
        current.on_resize.call((next, None));
    });

    let current = mount.read();
    let Some(projection) = current.projection.as_ref() else {
        return rsx! {};
    };
    let units = projection.units;
    let mixed = projection.mixed;
    let overlap_warning = (!projection.overlap_references.is_empty()).then(|| {
        let shown = projection.overlap_references.len().min(8);
        let references = projection.overlap_references[..shown].join(", ");
        let remaining = projection.overlap_references.len().saturating_sub(8);
        if remaining > 0 {
            format!("Keycaps overlap: {references} and {remaining} more. Adjust their rows or columns to clear the overlap.")
        } else {
            format!("Keycaps overlap: {references}. Adjust their rows or columns to clear the overlap.")
        }
    });
    let value_label = if mixed {
        "Mixed".to_owned()
    } else {
        format!("{}u × {}u", units.x, units.y)
    };
    let custom = mixed || units.x.min(units.y) != 1.0;
    let selected_size = if custom {
        "custom".to_owned()
    } else {
        units.x.max(units.y).to_string()
    };
    let mut options = Vec::new();
    if custom {
        options.push(("custom".to_owned(), value_label.clone()));
    }
    for step in 4..=28 {
        let size = f64::from(step) / 4.0;
        options.push((size.to_string(), format!("{size}u")));
    }
    let scale = 80.0 / units.y.max(units.x / 2.0).max(1.0);
    let width = units.x * scale;
    let height = units.y * scale;

    rsx! {
        section { class: "grid min-w-0 gap-2.5 border-b border-border p-0 text-foreground", aria_label: "Key size",
            onkeydown: super::stop_space_pan,
            figure { class: "m-0 mb-3 text-center",
                svg {
                    class: "block h-[100px] w-full",
                    view_box: "0 0 200 120",
                    role: "img",
                    "aria-label": "Keycap size {value_label}",
                    rect {
                        class: "fill-secondary stroke-primary stroke-[1]",
                        x: "{100.0 - width / 2.0}",
                        y: "{60.0 - height / 2.0}",
                        width: "{width}",
                        height: "{height}",
                        rx: "5",
                    }
                    rect {
                        class: "fill-field stroke-border-strong stroke-[1]",
                        x: "{104.0 - width / 2.0}",
                        y: "{64.0 - height / 2.0}",
                        width: "{(width - 8.0).max(1.0)}",
                        height: "{(height - 8.0).max(1.0)}",
                        rx: "3",
                    }
                }
                figcaption { output { "{value_label}" } }
            }
            SelectField {
                label: "Size".to_owned(),
                value: selected_size,
                options,
                aria_label: "Key size",
                disabled: !current.editable,
                on_commit: on_size_commit,
            }
            ToggleField {
                label: "Rotate 90°".to_owned(),
                checked: !mixed && units.y > units.x,
                aria_label: "Rotate keycap",
                disabled_reason: (!current.editable).then(|| "Placement is unavailable".to_owned()),
                on_commit: on_rotate_commit,
            }
            if let Some(error) = current.error.as_deref() {
                FieldError { "{error}" }
            }
        }
        if let Some(warning) = overlap_warning {
            div { role: "status", class: "-mt-1 mx-4 mb-3 rounded-[4px] border border-warning-dark bg-warning-light px-2.5 py-2 text-[14px] leading-[1.55] text-warning-dark",
                Callout { variant: CalloutVariant::Warning, "{warning}" }
            }
        }
    }
}
