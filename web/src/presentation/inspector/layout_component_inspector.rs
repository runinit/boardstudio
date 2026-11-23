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
use boardstudio_web_ui_shared::inspector_fields::{
    CheckboxField, InspectorSection, NumericField, SelectField,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::checkbox::CheckboxState;
use boardstudio_web_ui_shared::ui::field::FieldError;
use boardstudio_web_ui_shared::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use dioxus::prelude::*;
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

pub use boardstudio_web_layout::component_inspector::layout_component_edits::ComponentPositionAxis;

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

impl LayoutComponentInspectorAction {
    pub fn owner(&self) -> &LayoutComponentInspectorOwner {
        match self {
            LayoutComponentInspectorAction::SetPosition { owner, .. }
            | LayoutComponentInspectorAction::AssignLayout { owner, .. }
            | LayoutComponentInspectorAction::SetOutline { owner, .. }
            | LayoutComponentInspectorAction::SetConstraint { owner, .. }
            | LayoutComponentInspectorAction::RemoveConstraint { owner, .. }
            | LayoutComponentInspectorAction::ConvertOffsetAxes { owner, .. }
            | LayoutComponentInspectorAction::NavigateElectrical { owner }
            | LayoutComponentInspectorAction::SetMirroredHalfState { owner, .. }
            | LayoutComponentInspectorAction::SetPartMirrored { owner, .. }
            | LayoutComponentInspectorAction::SetPartAnchorValue { owner, .. }
            | LayoutComponentInspectorAction::SetPartAnchorFollow { owner, .. }
            | LayoutComponentInspectorAction::SetPartAnchor { owner, .. }
            | LayoutComponentInspectorAction::DetachPartAnchor { owner } => owner,
        }
    }
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

mod anchor;
mod constraint;
mod mirror;
mod outline;
mod properties;

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
    let mut tab = props.inspector_tab;
    let mut x = props.x;
    let mut y = props.y;
    let mut margin = props.margin;
    let x_failure = props.x_failure;
    let y_failure = props.y_failure;
    let margin_failure = props.margin_failure;
    let layout_choice = use_memo(use_reactive((&projection.layout_id,), |(layout,)| {
        Some(layout.unwrap_or_default())
    }));
    let error = props.error;
    let initial_position = projection.position;
    let initial_outline = projection.outline.clone();
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
    let mut position_error = error;
    let commit_position = move |axis, value: f64| {
        let draft = value.to_string();
        match axis {
            ComponentPositionAxis::X => x.set(draft.clone()),
            ComponentPositionAxis::Y => y.set(draft.clone()),
        }
        position_error.set(None);
        props
            .on_action
            .call(LayoutComponentInspectorAction::SetPosition {
                owner: latest_capture(),
                axis,
                value,
                submitted_draft: draft,
            });
    };
    let group_selection = projection.selection_count > 1;
    let mirrors = mirror::mirror_controls(
        &projection,
        latest_capture,
        props.on_action,
        mirror_section_open,
    );
    let anchors = anchor::use_anchor_controls(&projection, latest_capture, props.on_action, error);
    let outline = outline::outline_controls(
        &projection,
        latest_capture,
        props.on_action,
        outline_section_open,
        margin,
        error,
    );
    let (constraint, mut reset_constraint) = constraint::use_constraint_controls(
        &projection,
        &identity,
        latest_capture,
        props.on_action,
        error,
    );
    let reset_properties_drafts = move || {
        x.set(format!("{:.2}", initial_position.x));
        y.set(format!("{:.2}", initial_position.y));
        margin.set(
            initial_outline
                .margin
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
        reset_constraint();
    };
    rsx! {
        section {
            class: "grid min-w-0 content-start gap-3 px-4 pt-3 pb-5 text-foreground [&_h2]:m-0 [&_h3]:m-0 [&_p]:m-0 [&_fieldset_small]:text-[11px] [&_fieldset_small]:leading-[1.45] [&_fieldset_small]:text-muted-foreground",
            aria_label: "Component inspector",
            onkeydown: crate::presentation::view_helpers::stop_space_propagation,
            header { class: "flex items-start justify-between gap-2 [&_h2]:text-base [&_h2]:font-semibold [&_h2]:[overflow-wrap:anywhere]",
                h2 { "{projection.reference}" }
                if projection.locked {
                    span { class: "flex-none rounded-[9px] border border-border px-1.5 py-0.5 text-[11px] text-muted-foreground",
                        "Locked"
                    }
                }
            }
            Tabs {
                horizontal: true,
                value: Some(
                    if tab() == LayoutInspectorTab::Properties {
                        "properties".to_owned()
                    } else {
                        "relations".to_owned()
                    },
                ),
                on_value_change: {
                    let mut reset = reset_properties_drafts;
                    move |value: String| {
                        let next = if value == "relations" {
                            LayoutInspectorTab::Relations
                        } else {
                            LayoutInspectorTab::Properties
                        };
                        if next == LayoutInspectorTab::Properties && *tab.peek() != next {
                            reset();
                        }
                        tab.set(next);
                    }
                },
                TabsList {
                    aria_label: "Inspector details",
                    class: "layout-inspector-tabs",
                    TabsTrigger { index: 0usize, value: "properties", "Properties" }
                    TabsTrigger { index: 1usize, value: "relations", "Relations" }
                }
                TabsContent { index: 0usize, value: "properties",
                    if group_selection {
                        p {
                            "{projection.selection_count} parts selected. Position edits apply to the selection."
                        }
                    } else {
                        p { class: "grid min-w-0 grid-cols-[1fr_auto] items-baseline gap-[5px] text-[13px] text-foreground [&_span]:text-xs [&_span]:leading-[1.45] [&_span]:text-muted-foreground",
                            "{projection.definition_name}"
                            span { "{projection.definition_kind}" }
                        }
                        if let Some(notice) = projection.envelope_notice.as_ref() {
                            Callout {
                                class: "text-xs leading-[1.45] text-muted-foreground",
                                variant: CalloutVariant::Info,
                                "{notice}"
                            }
                        }
                    }
                    {mirrors}
                    if !group_selection && !projection.layouts.is_empty() {
                        {
                            properties::layout_assignment(
                                &projection,
                                layout_choice,
                                latest_capture,
                                props.on_action,
                            )
                        }
                    }
                    {anchors}
                    {properties::position_fields(&projection, commit_position)}
                    if !group_selection {
                        {outline}
                        {constraint}
                    }
                    if let Some(message) = x_failure()
                        .or_else(&*y_failure)
                        .or_else(&*margin_failure)
                        .or_else(&*error)
                    {
                        FieldError { "{message}" }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        class: "min-h-[34px] justify-self-start text-left",
                        button_type: "button",
                        onclick: {
                            let action = props.on_action;
                            move |_| {
                                action
                                    .call(LayoutComponentInspectorAction::NavigateElectrical {
                                        owner: latest_capture(),
                                    })
                            }
                        },
                        "Edit electrical connections"
                    }
                }
                TabsContent { index: 1usize, value: "relations",
                    h2 { "Relationships" }
                    p { class: "text-xs leading-normal", "{projection.relationship_summary}" }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        onclick: move |_| tab.set(LayoutInspectorTab::Properties),
                        "Go to Properties"
                    }
                    p { class: "text-xs leading-[1.45] text-muted-foreground",
                        "Matrix rows and columns share pitch, stagger and splay. Edit those in Properties."
                    }
                }
            }
        }
    }
}
