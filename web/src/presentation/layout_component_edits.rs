pub(super) use boardstudio_web_layout::component_inspector::layout_component_edits::*;
use dioxus::prelude::Signal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InspectorField {
    X,
    Y,
    Margin,
    Layout,
    Constraint,
    RemoveConstraint,
    MirroredHalfState,
    PartMirror,
    PartAnchorValue,
    PartAnchorFollow,
    PartAnchorPick,
    PartAnchorDetach,
}

#[derive(Clone, Copy)]
pub(super) struct ComponentEditViews {
    pub x: (Signal<String>, Signal<Option<String>>),
    pub y: (Signal<String>, Signal<Option<String>>),
    pub margin: (Signal<String>, Signal<Option<String>>),
    pub error: Signal<Option<String>>,
}
