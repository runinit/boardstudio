//! Scoped edits for the authored geometry fields of a project part definition.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_model::model::{
    EditOperation, Pad, PadShape, PartDefinition, PartKind, ProjectDoc, Vec2,
};

pub(crate) const DEFINITION_GONE: &str = "This part definition no longer exists.";
pub(crate) const GENERATOR_LOCKED: &str =
    "Generated definitions are edited through their generator settings.";
const PAD_GONE: &str = "This pad no longer exists on the definition.";
const IMPORTED_PADS: &str = "Imported pad geometry stays linked to its original KiCad source.";
const PAD_IDS_REQUIRED: &str =
    "Pad IDs and numbers are required. Positions must be finite, sizes and drill must be positive.";
const PAD_IDS_UNIQUE: &str = "Pad IDs must be unique within the component.";
const PAD_NUMBERS_UNIQUE: &str = "Pad numbers must be unique within the component.";
const COURTYARD_NUMBERS: &str = "Courtyard dimensions must be positive numbers.";
const PAD_NUMBERS: &str = "Pad positions and dimensions must be finite numbers.";

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::*;
    use crate::runtime::Runtime;
    use boardstudio_web_ui_shared::inspector_fields::{
        InspectorSection, NumericField, SelectField, TextCommitField, ToggleField,
    };
    use boardstudio_web_ui_shared::ui::{
        alert::{Alert, AlertVariant},
        button::Button,
    };
    use dioxus::prelude::*;
    use std::rc::Rc;

    #[component]
    pub(crate) fn CustomDefinitionFields(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let capture = DefinitionPanelCapture::new(&snapshot, scope, &definition);
        let mut failure = use_signal(|| None::<String>);
        let submit = EventHandler::new(move |edit: DefinitionEdit| {
            let Some(current) = runtime.model().accepted else {
                return;
            };
            if !capture.owner_matches(&current, runtime.scope(), selection()) {
                return;
            }
            let seed = if matches!(edit, DefinitionEdit::AddPad) {
                runtime.operation().0
            } else {
                0
            };
            let resolver = definition_field_resolver(capture.definition_id.clone(), edit, seed);
            failure.set(runtime.commit("parts-definition-field", resolver).err());
        });
        let (width, height, _) = courtyard_bounds(&definition.courtyard);
        let locked = definition.kicad_source.is_some();
        rsx! {
            InspectorSection { title: "Edit footprint", badge: "Custom geometry",
                default_open: definition.pads.is_empty(),
                TextCommitField { label: "Name", aria_label: "Definition name", value: definition.name,
                    on_commit: move |value| submit.call(DefinitionEdit::Name(value)),
                }
                SelectField::<PartKind> { label: "Kind", aria_label: "Definition kind", value: definition.kind,
                    options: [(PartKind::Switch, "Switch"), (PartKind::Controller, "Controller"),
                        (PartKind::Connector, "Connector"), (PartKind::Encoder, "Encoder"),
                        (PartKind::Passive, "Passive"), (PartKind::Custom, "Custom")]
                        .map(|(kind, label)| (kind, label.to_owned())).to_vec(),
                    on_commit: move |kind| submit.call(DefinitionEdit::Kind(kind)),
                }
                for (label, value, edit) in [
                    ("Courtyard width", width, DefinitionEdit::CourtyardWidth as fn(String) -> DefinitionEdit),
                    ("Courtyard height", height, DefinitionEdit::CourtyardHeight as fn(String) -> DefinitionEdit),
                ] {
                    NumericField { key: "{label}", label, unit: "mm", value, min: Some(0.01), step: Some(0.1),
                        on_commit: move |value: f64| submit.call(edit(value.to_string())),
                    }
                }
                if locked {
                    p { "{IMPORTED_PADS}" }
                }
                Button { disabled: locked, onclick: move |_| submit.call(DefinitionEdit::AddPad), "Add pad" }
                for (index, pad) in definition.pads.into_iter().enumerate() {
                    PadFields { key: "{index}:{pad.id}", index, pad, locked, submit }
                }
                if let Some(error) = failure() {
                    Alert { variant: AlertVariant::Destructive, "{error}" }
                }
            }
        }
    }

    #[component]
    fn PadFields(
        index: usize,
        pad: Pad,
        locked: bool,
        submit: EventHandler<DefinitionEdit>,
    ) -> Element {
        let id = pad.id.clone();
        let reason = locked.then(|| IMPORTED_PADS.to_owned());
        rsx! {
            fieldset { class: "grid gap-2",
                legend { "Pad {index + 1}" }
                TextCommitField { label: "ID", value: pad.id.clone(), disabled_reason: reason.clone(),
                    on_commit: move |value| submit.call(DefinitionEdit::PadId { pad_id: id.clone(), value }),
                }
                TextCommitField { label: "Number", value: pad.number.clone(), disabled_reason: reason.clone(),
                    on_commit: { let id = pad.id.clone(); move |value| submit.call(DefinitionEdit::PadNumber { pad_id: id.clone(), value }) },
                }
                for (label, value, axis, size) in [
                    ("X", pad.at.x, Axis::X, false), ("Y", pad.at.y, Axis::Y, false),
                    ("Width", pad.size.x, Axis::X, true), ("Height", pad.size.y, Axis::Y, true),
                ] {
                    NumericField { key: "{label}", label, value, unit: "mm", min: size.then_some(0.01), step: Some(0.1),
                        disabled_reason: reason.clone(),
                        on_commit: { let id = pad.id.clone(); move |value: f64| {
                            let pad_id = id.clone();
                            let value = value.to_string();
                            submit.call(if size { DefinitionEdit::PadSize { pad_id, axis, value } }
                                else { DefinitionEdit::PadCoordinate { pad_id, axis, value } });
                        }},
                    }
                }
                ToggleField { label: "Drill", checked: pad.drill.is_some(), disabled_reason: reason.clone(),
                    on_commit: { let id = pad.id.clone(); move |enabled| submit.call(DefinitionEdit::PadDrill {
                        pad_id: id.clone(), value: if enabled { "1".into() } else { String::new() },
                    })},
                }
                if let Some(drill) = pad.drill {
                    NumericField { label: "Drill diameter", unit: "mm", value: drill, min: Some(0.01), step: Some(0.1),
                        disabled_reason: reason.clone(),
                        on_commit: { let id = pad.id.clone(); move |value: f64| submit.call(DefinitionEdit::PadDrill {
                            pad_id: id.clone(), value: value.to_string(),
                        })},
                    }
                }
                SelectField::<PadShape> { label: "Shape", value: pad.shape, disabled_reason: reason,
                    options: [(PadShape::Circle, "Circle"), (PadShape::Oval, "Oval"), (PadShape::Rect, "Rectangle"),
                        (PadShape::Roundrect, "Rounded rectangle")].map(|(shape, label)| (shape, label.to_owned())).to_vec(),
                    on_commit: { let id = pad.id.clone(); move |shape| submit.call(DefinitionEdit::PadShape { pad_id: id.clone(), shape }) },
                }
                Button { disabled: locked, onclick: move |_| submit.call(DefinitionEdit::RemovePad { pad_id: pad.id.clone() }), "Remove pad" }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use ui::CustomDefinitionFields;

/// What a definition panel needs to know its owner is still current at dispatch:
/// the scope, the document session and the selected definition. Deliberately not a
/// snapshot token or revision — field edits queue freely and resolve against
/// whatever document is accepted when they run (ADR-0005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionPanelCapture {
    pub scope: Option<Scope>,
    pub definition_id: String,
    session_epoch: SessionEpoch,
    document_id: String,
}

impl DefinitionPanelCapture {
    pub fn new(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        definition: &PartDefinition,
    ) -> Self {
        Self {
            scope,
            definition_id: definition.id.clone(),
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
        }
    }

    /// Owner lifetime at dispatch: the panel still belongs to this scope, document
    /// session and selected definition, so a stale callback is ignored early. A newer
    /// accepted revision is not a liveness answer — the resolver re-checks the target
    /// at execution.
    pub fn owner_matches(
        &self,
        current: &AcceptedSnapshot,
        current_scope: Option<Scope>,
        current_selection: Option<(Option<Scope>, String)>,
    ) -> bool {
        self.scope.is_some()
            && current_scope == self.scope
            && current.session_epoch == self.session_epoch
            && current.document.id == self.document_id
            && current_selection == Some((self.scope.clone(), self.definition_id.clone()))
    }
}

mod edit;
pub use edit::{Axis, DefinitionEdit, definition_field_resolver, validate_definition_edit};
pub(crate) use edit::{courtyard_bounds, replacement_commit};

#[cfg(test)]
mod tests;
