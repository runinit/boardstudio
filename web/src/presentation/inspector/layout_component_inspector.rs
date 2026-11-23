//! Contextual Properties and Relationships for one accepted standalone Layout part.
use crate::presentation::layout_workspace::LayoutInspectorTab;
use boardstudio_application::Scope;
use boardstudio_core::model::{
    Constraint, ConstraintRelation, Follow, HalfExceptionState, MirrorAxis, OffsetAxes,
    PartAnchorValueChange, PartOutline, Vec2,
};
pub(crate) use boardstudio_web_layout::component_offset::LayoutConstraintValues;
use boardstudio_web_layout::component_offset::{
    default_axes_for_source, offset_axes_from_value, offset_axes_value, offset_drafts,
    offset_values,
};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{CheckboxField, NumericField, SelectField};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::checkbox::CheckboxState;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use boardstudio_web_ui_shared::ui::{
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldError, FieldLabel},
    select::{Select, SelectOption},
};

use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type InspectorOwnerIdentity = (
    boardstudio_application::Scope,
    u64,
    u64,
    String,
    Vec<String>,
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutComponentInspectorOwnerKey {
    pub scope: Option<Scope>,
    pub workspace: Workspace,
    pub part_id: Option<String>,
    pub selected_part_ids: Vec<String>,
}

#[derive(Default)]
pub struct LayoutComponentInspectorLifetime {
    current: Cell<u64>,
    key: RefCell<Option<LayoutComponentInspectorOwnerKey>>,
}

impl LayoutComponentInspectorLifetime {
    pub fn update(&self, key: Option<LayoutComponentInspectorOwnerKey>) -> u64 {
        let mut current_key = self.key.borrow_mut();
        if *current_key != key {
            self.current.set(self.current.get().wrapping_add(1).max(1));
            *current_key = key;
        }
        self.current.get()
    }

    pub fn current_generation(&self) -> u64 {
        self.current.get()
    }
}

pub use boardstudio_web_layout::inspector_types::{
    LayoutComponentInspectorOwner, LayoutComponentInspectorProjection,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentPositionAxis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutComponentInspectorAction {
    SetPosition {
        owner: LayoutComponentInspectorOwner,
        axis: ComponentPositionAxis,
        value: f64,
        submitted_draft: String,
    },
    AssignLayout {
        owner: LayoutComponentInspectorOwner,
        layout_id: Option<String>,
    },
    SetOutline {
        owner: LayoutComponentInspectorOwner,
        outline: PartOutline,
        submitted_draft: String,
    },
    SetConstraint {
        owner: LayoutComponentInspectorOwner,
        source_part_id: String,
        values: LayoutConstraintValues,
    },
    RemoveConstraint {
        owner: LayoutComponentInspectorOwner,
        constraint_id: String,
    },
    /// Re-expresses the saved offset constraint on the other axes without moving a part.
    ConvertOffsetAxes {
        owner: LayoutComponentInspectorOwner,
        constraint_id: String,
        to: OffsetAxes,
    },
    NavigateElectrical {
        owner: LayoutComponentInspectorOwner,
    },
    SetMirroredHalfState {
        owner: LayoutComponentInspectorOwner,
        pair_id: String,
        mirror_id: String,
        state: HalfExceptionState,
    },
    SetPartMirrored {
        owner: LayoutComponentInspectorOwner,
        part_id: String,
        mirrored: bool,
    },
    /// Ticket 13: edit one carried (Spread/Stagger/Splay) or local
    /// (Offset/Rotation) anchor value of an anchored part.
    SetPartAnchorValue {
        owner: LayoutComponentInspectorOwner,
        change: PartAnchorValueChange,
    },
    /// Ticket 13: change the Follow subset of an anchored part.
    SetPartAnchorFollow {
        owner: LayoutComponentInspectorOwner,
        follow: Follow,
    },
    /// Ticket 13, retarget rule 04: retarget an anchored part's anchor key
    /// (Pick), keeping its stored values so the part moves to the fresh
    /// placement from the new key — or anchor a detached part, which lands
    /// on the picked key with the submitted values.
    SetPartAnchor {
        owner: LayoutComponentInspectorOwner,
        matrix_id: String,
        row: u32,
        column: u32,
        follow: Follow,
    },
    /// Ticket 13: detach an anchored part (Detach), keeping its pose.
    DetachPartAnchor {
        owner: LayoutComponentInspectorOwner,
    },
}

#[derive(Props, Clone, PartialEq)]
pub struct LayoutComponentInspectorProps {
    pub projection: LayoutComponentInspectorProjection,
    pub inspector_tab: Signal<LayoutInspectorTab>,
    pub(in crate::presentation) x: Signal<String>,
    pub(in crate::presentation) y: Signal<String>,
    pub(in crate::presentation) margin: Signal<String>,
    pub(in crate::presentation) x_failure: Signal<Option<String>>,
    pub(in crate::presentation) y_failure: Signal<Option<String>>,
    pub(in crate::presentation) margin_failure: Signal<Option<String>>,
    pub(in crate::presentation) error: Signal<Option<String>>,
    pub on_action: EventHandler<LayoutComponentInspectorAction>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConstraintKind {
    Offset,
    Mirror,
    MirrorLine,
}

/// X/Y are unavailable when the part is locked or when a saved relationship on
/// this board drives it (the relationship would overwrite any typed value).
fn position_unavailable(projection: &LayoutComponentInspectorProjection) -> bool {
    projection.locked || projection.active_constraint.is_some()
}

mod sections;
use sections::render_inspector;

#[component]
pub fn LayoutComponentInspector(props: LayoutComponentInspectorProps) -> Element {
    let mirror_section_open = use_signal(|| false);
    let outline_section_open = use_signal(|| false);

    let projection = props.projection.clone();
    let owner = projection.owner.clone();
    // Selection lifetime is independent from the accepted snapshot. A newer
    // accepted revision must refresh action admission without discarding
    // unrelated dirty fields or the user's selected Inspector tab.
    let identity = (
        owner.scope.clone(),
        owner.context_generation,
        owner.scope_generation,
        owner.part_id.clone(),
        owner.selected_part_ids.clone(),
    );
    let capture_identity = (identity.clone(), owner.snapshot_token, owner.revision);
    let mut latest_capture = use_signal(|| owner.clone());
    use_effect(use_reactive((&capture_identity,), {
        let owner = owner.clone();
        move |_| latest_capture.set(owner.clone())
    }));
    let tab = props.inspector_tab;
    let mut constraint_open = use_signal(|| projection.active_constraint.is_some());
    let mut x = props.x;
    let mut y = props.y;
    let mut margin = props.margin;
    let x_failure = props.x_failure;
    let y_failure = props.y_failure;
    let margin_failure = props.margin_failure;
    let mut constraint_kind = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { .. },
            ..
        }) => ConstraintKind::MirrorLine,
        _ => ConstraintKind::Offset,
    });
    let drafts = offset_drafts(
        projection.active_constraint.as_ref(),
        &projection.board_parts,
        &owner.part_id,
    );
    let mut source_part_id = use_signal(|| drafts.source_part_id.clone());
    let mut offset_axes = use_signal(|| drafts.axes);
    let mut offset_x = use_signal(|| drafts.offset_x.clone());
    let mut offset_y = use_signal(|| drafts.offset_y.clone());
    let mut rotation = use_signal(|| drafts.rotation.clone());
    let mut mirror_axis = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { axis, .. }) => axis.clone(),
        _ => MirrorAxis::Vertical,
    });
    let mut mirror_coordinate = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { coordinate, .. }) => coordinate.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_x = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { at, .. },
            ..
        }) => at.x.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_y = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { at, .. },
            ..
        }) => at.y.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_angle = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { angle, .. },
            ..
        }) => angle.to_string(),
        _ => "90".to_owned(),
    });
    let layout_choice = use_memo(use_reactive((&projection.layout_id,), |(layout,)| {
        Some(layout.unwrap_or_default())
    }));
    let kind_choice = use_memo(move || {
        Some(
            match constraint_kind() {
                ConstraintKind::Offset => "offset",
                ConstraintKind::Mirror => "mirror",
                ConstraintKind::MirrorLine => "mirror-line",
            }
            .to_owned(),
        )
    });
    let source_choice = use_memo(move || Some(source_part_id()));
    let axes_choice = use_memo(move || Some(offset_axes_value(offset_axes()).to_owned()));
    let axis_choice = use_memo(move || {
        Some(
            if mirror_axis() == MirrorAxis::Vertical {
                "vertical"
            } else {
                "horizontal"
            }
            .to_owned(),
        )
    });
    let mut error = props.error;
    let pick_matrix = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.matrix_id.clone())
            .or_else(|| {
                projection
                    .anchor_choices
                    .first()
                    .map(|choice| choice.matrix_id.clone())
            })
            .unwrap_or_default()
    });
    let pick_matrix_choice = use_memo(move || Some(pick_matrix()));
    let pick_row = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| (anchor.row + 1).to_string())
            .unwrap_or_else(|| "1".to_owned())
    });
    let pick_column = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| (anchor.column + 1).to_string())
            .unwrap_or_else(|| "1".to_owned())
    });
    let pick_follow_x = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.follow.x)
            .unwrap_or(true)
    });
    let pick_follow_y = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.follow.y)
            .unwrap_or(true)
    });
    let pick_follow_rotation = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.follow.rotation)
            .unwrap_or(true)
    });
    let mirrored_half_action = |state| {
        let mirrored_half = projection.mirrored_half.clone();
        let owner = latest_capture;
        let action = props.on_action;
        move |_| {
            if let Some(mirrored_half) = mirrored_half.as_ref() {
                action.call(LayoutComponentInspectorAction::SetMirroredHalfState {
                    owner: owner(),
                    pair_id: mirrored_half.pair_id.clone(),
                    mirror_id: mirrored_half.mirror_id.clone(),
                    state,
                });
            }
        }
    };

    let initial_position = projection.position;
    let initial_outline = projection.outline.clone();
    let initial_constraint = projection.active_constraint.clone();
    let has_initial_constraint = initial_constraint.is_some();
    let board_parts = projection.board_parts.clone();
    let part_id = owner.part_id.clone();
    use_effect(use_reactive((&identity,), {
        move |_| {
            constraint_open.set(has_initial_constraint);
            error.set(None);
        }
    }));
    let mut previous_x = use_signal(|| None::<(InspectorOwnerIdentity, f64)>);
    let accepted_x = (identity.clone(), initial_position.x);
    let identity_x = identity.clone();
    use_effect(use_reactive((&accepted_x,), move |((_, value),)| {
        let previous = previous_x.peek().clone();
        let clean = previous.as_ref().is_none_or(|(old_identity, old_value)| {
            old_identity != &identity_x || x.peek().as_str() == format!("{old_value:.2}")
        });
        if clean {
            x.set(format!("{value:.2}"));
        }
        previous_x.set(Some((identity_x.clone(), value)));
    }));
    let mut previous_y = use_signal(|| None::<(InspectorOwnerIdentity, f64)>);
    let accepted_y = (identity.clone(), initial_position.y);
    let identity_y = identity.clone();
    use_effect(use_reactive((&accepted_y,), move |((_, value),)| {
        let previous = previous_y.peek().clone();
        let clean = previous.as_ref().is_none_or(|(old_identity, old_value)| {
            old_identity != &identity_y || y.peek().as_str() == format!("{old_value:.2}")
        });
        if clean {
            y.set(format!("{value:.2}"));
        }
        previous_y.set(Some((identity_y.clone(), value)));
    }));
    let mut previous_margin = use_signal(|| None::<(InspectorOwnerIdentity, Option<f64>)>);
    let accepted_margin = (identity.clone(), initial_outline.margin);
    let identity_margin = identity.clone();
    use_effect(use_reactive((&accepted_margin,), move |((_, value),)| {
        let previous = previous_margin.peek().clone();
        let old_text = previous
            .as_ref()
            .map(|(_, value)| value.map(|value| value.to_string()).unwrap_or_default());
        let clean = previous
            .as_ref()
            .is_none_or(|(old_identity, _)| old_identity != &identity_margin)
            || old_text.as_deref() == Some(margin.peek().as_str());
        if clean {
            margin.set(value.map(|value| value.to_string()).unwrap_or_default());
        }
        previous_margin.set(Some((identity_margin.clone(), value)));
    }));
    let accepted_constraint = (identity.clone(), initial_constraint.clone());
    use_effect(use_reactive((&accepted_constraint,), {
        move |((_, constraint),)| {
            constraint_open.set(constraint.is_some());
            constraint_kind.set(match constraint.as_ref() {
                Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
                Some(Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { .. },
                    ..
                }) => ConstraintKind::MirrorLine,
                _ => ConstraintKind::Offset,
            });
            let drafts = offset_drafts(constraint.as_ref(), &board_parts, &part_id);
            source_part_id.set(drafts.source_part_id);
            offset_axes.set(drafts.axes);
            offset_x.set(drafts.offset_x);
            offset_y.set(drafts.offset_y);
            rotation.set(drafts.rotation);
            match constraint {
                Some(Constraint::Mirror {
                    axis, coordinate, ..
                }) => {
                    mirror_axis.set(axis.clone());
                    mirror_coordinate.set(coordinate.to_string());
                }
                Some(Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { at, angle },
                    ..
                }) => {
                    mirror_line_x.set(at.x.to_string());
                    mirror_line_y.set(at.y.to_string());
                    mirror_line_angle.set(angle.to_string());
                }
                _ => {
                    mirror_axis.set(MirrorAxis::Vertical);
                    mirror_coordinate.set("0".to_owned());
                    mirror_line_x.set("0".to_owned());
                    mirror_line_y.set("0".to_owned());
                    mirror_line_angle.set("90".to_owned());
                }
            }
            error.set(None);
        }
    }));
    let reset_properties_drafts = {
        let mut x = x;
        let mut y = y;
        let mut margin = margin;
        let mut constraint_open = constraint_open;
        let mut constraint_kind = constraint_kind;
        let mut source_part_id = source_part_id;
        let mut offset_axes = offset_axes;
        let mut offset_x = offset_x;
        let mut offset_y = offset_y;
        let mut rotation = rotation;
        let mut mirror_axis = mirror_axis;
        let mut mirror_coordinate = mirror_coordinate;
        let mut mirror_line_x = mirror_line_x;
        let mut mirror_line_y = mirror_line_y;
        let mut mirror_line_angle = mirror_line_angle;
        let mut error = error;
        let initial_margin = initial_outline.margin;
        let initial_constraint = initial_constraint.clone();
        let initial_drafts = drafts.clone();
        move || {
            x.set(format!("{:.2}", initial_position.x));
            y.set(format!("{:.2}", initial_position.y));
            margin.set(
                initial_margin
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            );
            constraint_open.set(initial_constraint.is_some());
            constraint_kind.set(match initial_constraint.as_ref() {
                Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
                Some(Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { .. },
                    ..
                }) => ConstraintKind::MirrorLine,
                _ => ConstraintKind::Offset,
            });
            let drafts = initial_drafts.clone();
            source_part_id.set(drafts.source_part_id);
            offset_axes.set(drafts.axes);
            offset_x.set(drafts.offset_x);
            offset_y.set(drafts.offset_y);
            rotation.set(drafts.rotation);
            match initial_constraint.as_ref() {
                Some(Constraint::Mirror {
                    axis, coordinate, ..
                }) => {
                    mirror_axis.set(axis.clone());
                    mirror_coordinate.set(coordinate.to_string());
                }
                Some(Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { at, angle },
                    ..
                }) => {
                    mirror_line_x.set(at.x.to_string());
                    mirror_line_y.set(at.y.to_string());
                    mirror_line_angle.set(angle.to_string());
                }
                _ => {
                    mirror_axis.set(MirrorAxis::Vertical);
                    mirror_coordinate.set("0".to_owned());
                    mirror_line_x.set("0".to_owned());
                    mirror_line_y.set("0".to_owned());
                    mirror_line_angle.set("90".to_owned());
                }
            }
            error.set(None);
        }
    };
    let commit_position = move |axis, value: f64| {
        let draft = value.to_string();
        match axis {
            ComponentPositionAxis::X => x.set(draft.clone()),
            ComponentPositionAxis::Y => y.set(draft.clone()),
        }
        error.set(None);
        props
            .on_action
            .call(LayoutComponentInspectorAction::SetPosition {
                owner: latest_capture(),
                axis,
                value,
                submitted_draft: draft,
            });
    };
    let set_outline: Rc<dyn Fn(PartOutline)> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move |outline| {
            action.call(LayoutComponentInspectorAction::SetOutline {
                owner: latest_capture(),
                outline,
                submitted_draft: margin(),
            })
        })
    };
    // Ticket 13: column-vocabulary commits. Number fields parse on Enter or
    // blur; Follow, Pick and Detach submit immediately.
    let commit_anchor_value: Rc<dyn Fn(PartAnchorValueChange)> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move |change| {
            action.call(LayoutComponentInspectorAction::SetPartAnchorValue {
                owner: latest_capture(),
                change,
            })
        })
    };
    let commit_anchor_follow: Rc<dyn Fn(Follow)> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move |follow| {
            action.call(LayoutComponentInspectorAction::SetPartAnchorFollow {
                owner: latest_capture(),
                follow,
            })
        })
    };
    let commit_anchor_pick: Rc<dyn Fn(String, u32, u32, Follow)> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move |matrix_id, row, column, follow| {
            action.call(LayoutComponentInspectorAction::SetPartAnchor {
                owner: latest_capture(),
                matrix_id,
                row,
                column,
                follow,
            })
        })
    };
    let commit_anchor_detach: Rc<dyn Fn()> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move || {
            action.call(LayoutComponentInspectorAction::DetachPartAnchor {
                owner: latest_capture(),
            })
        })
    };
    let anchor_follow = projection.anchor.as_ref().map(|anchor| anchor.follow);
    let anchor_follow_text = projection.anchor.as_ref().map(|anchor| {
        let mut parts = Vec::new();
        if anchor.follow.x {
            parts.push("X");
        }
        if anchor.follow.y {
            parts.push("Y");
        }
        if anchor.follow.rotation {
            parts.push("rotation");
        }
        if parts.is_empty() {
            "none".to_owned()
        } else {
            parts.join(", ")
        }
    });
    let anchor_locked = position_unavailable(&projection);
    // Ticket 13: cloned before the render so Pick closures borrow the list,
    // not the projection the anchor sections also read.
    let anchor_choices = projection.anchor_choices.clone();
    let anchor_number_field =
        move |label: &'static str,
              aria: &'static str,
              field: &'static str,
              current: f64,
              change: Rc<dyn Fn(f64) -> PartAnchorValueChange>| {
            let commit = commit_anchor_value.clone();
            let value = format!("{current:.2}").parse::<f64>().unwrap_or(current);
            rsx! {
                NumericField {
                    id: format!("part-anchor-{field}"), label, aria_label: aria,
                    value, precision: 2,
                    disabled_reason: anchor_locked.then(|| "Position is unavailable".to_owned()),
                    on_commit: move |value| commit(change(value)),
                }
            }
        };
    let has_active_constraint = projection.active_constraint.is_some();
    // A choice on a saved offset converts it at once; a new one only sets the default.
    let choose_offset_axes = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        let saved_offset_id = projection
            .active_constraint
            .as_ref()
            .filter(|constraint| constraint.offset_axes().is_some())
            .map(|constraint| constraint.id().to_owned());
        let mut offset_axes = offset_axes;
        move |to: OffsetAxes| {
            offset_axes.set(to);
            if let Some(constraint_id) = saved_offset_id.as_ref() {
                action.call(LayoutComponentInspectorAction::ConvertOffsetAxes {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                    to,
                });
            }
        }
    };
    let save_constraint = {
        let latest_capture = latest_capture;
        let offset_axes = offset_axes;
        let mirror_line_x = mirror_line_x;
        let mirror_line_y = mirror_line_y;
        let mirror_line_angle = mirror_line_angle;
        let retain_legacy_form = projection
            .active_constraint
            .as_ref()
            .is_some_and(|constraint| !matches!(constraint, Constraint::Framed { .. }));
        let action = props.on_action;
        let mut error = error;
        move |_| {
            let source = source_part_id();
            let owner = latest_capture();
            if source.is_empty() || source == owner.part_id {
                error.set(Some("Choose another part on this board.".to_owned()));
                return;
            }
            let values = match constraint_kind() {
                ConstraintKind::Offset => {
                    let (Ok(x), Ok(y), Ok(rotation)) = (
                        offset_x().parse::<f64>(),
                        offset_y().parse::<f64>(),
                        rotation().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !rotation.is_finite() {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    }
                    let offset = Vec2 { x, y };
                    if retain_legacy_form {
                        offset_values(offset_axes(), offset, rotation)
                    } else {
                        LayoutConstraintValues::Framed {
                            relation: match offset_axes() {
                                OffsetAxes::Board => {
                                    ConstraintRelation::Offset { offset, rotation }
                                }
                                OffsetAxes::Part => {
                                    ConstraintRelation::PartOffset { offset, rotation }
                                }
                            },
                        }
                    }
                }
                ConstraintKind::Mirror => {
                    let Ok(coordinate) = mirror_coordinate().parse::<f64>() else {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    };
                    if !coordinate.is_finite() {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    }
                    if retain_legacy_form {
                        LayoutConstraintValues::Mirror {
                            axis: mirror_axis(),
                            coordinate,
                        }
                    } else {
                        let axis = mirror_axis();
                        let at = match axis {
                            MirrorAxis::Vertical => Vec2 {
                                x: coordinate,
                                y: 0.0,
                            },
                            MirrorAxis::Horizontal => Vec2 {
                                x: 0.0,
                                y: coordinate,
                            },
                        };
                        LayoutConstraintValues::Framed {
                            relation: ConstraintRelation::MirrorLine {
                                at,
                                angle: if axis == MirrorAxis::Vertical {
                                    90.0
                                } else {
                                    0.0
                                },
                            },
                        }
                    }
                }
                ConstraintKind::MirrorLine => {
                    let (Ok(x), Ok(y), Ok(angle)) = (
                        mirror_line_x().parse::<f64>(),
                        mirror_line_y().parse::<f64>(),
                        mirror_line_angle().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite mirror line values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !angle.is_finite() {
                        error.set(Some("Enter finite mirror line values.".to_owned()));
                        return;
                    }
                    LayoutConstraintValues::Framed {
                        relation: ConstraintRelation::MirrorLine {
                            at: Vec2 { x, y },
                            angle,
                        },
                    }
                }
            };
            error.set(None);
            action.call(LayoutComponentInspectorAction::SetConstraint {
                owner: owner.clone(),
                source_part_id: source,
                values,
            });
        }
    };
    let active_constraint_id = projection
        .active_constraint
        .as_ref()
        .map(|constraint| constraint.id().to_owned());
    let remove_constraint = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        move |_| {
            if let Some(constraint_id) = active_constraint_id.as_ref() {
                action.call(LayoutComponentInspectorAction::RemoveConstraint {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                });
            }
        }
    };
    let position = projection.position;
    let group_selection = projection.selection_count > 1;
    let outline = projection.outline.clone();
    let active_constraint = projection.active_constraint.clone();
    let board_parts = projection.board_parts.clone();
    let constraint_source_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.source())
                .map(|part| part.reference.clone())
                .or_else(|| Some(constraint.source().to_owned()))
        })
        .unwrap_or_else(|| "A part".to_owned());
    let constraint_target_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.target())
                .map(|part| part.reference.clone())
                .or_else(|| Some(constraint.target().to_owned()))
        })
        .unwrap_or_else(|| projection.reference.clone());

    render_inspector!(
        props,
        mirror_section_open,
        outline_section_open,
        projection,
        owner,
        latest_capture,
        tab,
        constraint_open,
        margin,
        x_failure,
        y_failure,
        margin_failure,
        constraint_kind,
        source_part_id,
        offset_axes,
        offset_x,
        offset_y,
        rotation,
        mirror_axis,
        mirror_coordinate,
        mirror_line_x,
        mirror_line_y,
        mirror_line_angle,
        layout_choice,
        kind_choice,
        source_choice,
        axes_choice,
        axis_choice,
        error,
        pick_matrix,
        pick_matrix_choice,
        pick_row,
        pick_column,
        pick_follow_x,
        pick_follow_y,
        pick_follow_rotation,
        mirrored_half_action,
        board_parts,
        reset_properties_drafts,
        commit_position,
        set_outline,
        commit_anchor_follow,
        commit_anchor_pick,
        commit_anchor_detach,
        anchor_follow,
        anchor_follow_text,
        anchor_locked,
        anchor_choices,
        anchor_number_field,
        has_active_constraint,
        choose_offset_axes,
        save_constraint,
        remove_constraint,
        position,
        group_selection,
        outline,
        active_constraint,
        constraint_source_reference,
        constraint_target_reference
    )
}
