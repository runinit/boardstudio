//! Scoped edits for the authored geometry fields of a project part definition.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_model::model::{
    EditOperation, Pad, PadShape, PartDefinition, PartKind, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::Workspace;

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
        let capture =
            PartsOwnerGuard::capture(&snapshot, scope.clone(), &definition.id).require_scope();
        let mut failure = use_signal(|| None::<String>);
        let submit = EventHandler::new(move |edit: DefinitionEdit| {
            let Some(current) = runtime.model().accepted else {
                return;
            };
            if !capture.is_current(&current, runtime.scope(), selection(), None, None, None) {
                return;
            }
            let seed = if matches!(edit, DefinitionEdit::AddPad) {
                runtime.operation().0
            } else {
                0
            };
            let resolver =
                definition_field_resolver(capture.selection_id().unwrap_or_default(), edit, seed);
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
                    ("Courtyard width", width, DefinitionEdit::CourtyardWidth as fn(f64) -> DefinitionEdit),
                    ("Courtyard height", height, DefinitionEdit::CourtyardHeight as fn(f64) -> DefinitionEdit),
                ] {
                    NumericField { key: "{label}", label, unit: "mm", value, min: Some(0.01), step: Some(0.1),
                        on_commit: move |value: f64| submit.call(edit(value)),
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
                            submit.call(if size { DefinitionEdit::PadSize { pad_id, axis, value } }
                                else { DefinitionEdit::PadCoordinate { pad_id, axis, value } });
                        }},
                    }
                }
                ToggleField { label: "Drill", checked: pad.drill.is_some(), disabled_reason: reason.clone(),
                    on_commit: { let id = pad.id.clone(); move |enabled: bool| submit.call(DefinitionEdit::PadDrill {
                        pad_id: id.clone(), value: enabled.then_some(1.0),
                    })},
                }
                if let Some(drill) = pad.drill {
                    NumericField { label: "Drill diameter", unit: "mm", value: drill, min: Some(0.01), step: Some(0.1),
                        disabled_reason: reason.clone(),
                        on_commit: { let id = pad.id.clone(); move |value: f64| submit.call(DefinitionEdit::PadDrill {
                            pad_id: id.clone(), value: Some(value),
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

/// The one Parts owner/liveness guard: the captured workspace scope, selection,
/// document session and (where the panel captured them) selection/scope
/// generations. Deliberately not a snapshot token or revision — field edits queue
/// freely and resolve against whatever document is accepted when they run
/// (ADR-0005); the resolver re-checks the target at execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartsOwnerGuard {
    pub scope: Option<Scope>,
    pub selection: Option<(Option<Scope>, String)>,
    pub session_epoch: SessionEpoch,
    pub document_id: String,
    pub selection_generation: Option<u64>,
    pub scope_generation: Option<u64>,
    require_scope: bool,
    check_selection: bool,
}

impl PartsOwnerGuard {
    /// Capture against the selected id (`definition.id`, or `module:<id>` for modules).
    pub fn capture(snapshot: &AcceptedSnapshot, scope: Option<Scope>, selected_id: &str) -> Self {
        Self {
            selection: Some((scope.clone(), selected_id.to_owned())),
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            selection_generation: None,
            scope_generation: None,
            require_scope: false,
            check_selection: true,
        }
    }

    /// Capture for owners that track the document/session/scope only (assembly drafts).
    pub fn document(
        scope: Option<Scope>,
        session_epoch: SessionEpoch,
        document_id: String,
    ) -> Self {
        Self {
            scope,
            selection: None,
            session_epoch,
            document_id,
            selection_generation: None,
            scope_generation: None,
            require_scope: false,
            check_selection: false,
        }
    }

    pub fn with_selection(mut self, selection: Option<(Option<Scope>, String)>) -> Self {
        self.selection = selection;
        self.check_selection = true;
        self
    }

    pub fn generations(mut self, selection: Option<u64>, scope: Option<u64>) -> Self {
        self.selection_generation = selection;
        self.scope_generation = scope;
        self
    }

    pub fn require_scope(mut self) -> Self {
        self.require_scope = true;
        self
    }

    pub fn selection_id(&self) -> Option<String> {
        self.selection.as_ref().map(|(_, id)| id.clone())
    }

    /// Owner lifetime at dispatch. `workspace: None` skips the Parts-workspace check
    /// for callers that gate the workspace themselves; a captured generation must be
    /// supplied and equal, an uncaptured one is not checked.
    pub fn is_current(
        &self,
        current: &AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Option<(Option<Scope>, String)>,
        selection_generation: Option<u64>,
        scope_generation: Option<u64>,
        workspace: Option<Workspace>,
    ) -> bool {
        workspace.is_none_or(|workspace| workspace == Workspace::Parts)
            && (!self.require_scope || self.scope.is_some())
            && scope == self.scope
            && (!self.check_selection || selection == self.selection)
            && self
                .selection_generation
                .is_none_or(|g| Some(g) == selection_generation)
            && self
                .scope_generation
                .is_none_or(|g| Some(g) == scope_generation)
            && current.session_epoch == self.session_epoch
            && current.document.id == self.document_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
enum DefinitionEdit {
    Name(String),
    Kind(PartKind),
    CourtyardWidth(f64),
    CourtyardHeight(f64),
    AddPad,
    PadId {
        pad_id: String,
        value: String,
    },
    PadNumber {
        pad_id: String,
        value: String,
    },
    PadCoordinate {
        pad_id: String,
        axis: Axis,
        value: f64,
    },
    PadSize {
        pad_id: String,
        axis: Axis,
        value: f64,
    },
    PadDrill {
        pad_id: String,
        value: Option<f64>,
    },
    PadShape {
        pad_id: String,
        shape: PadShape,
    },
    RemovePad {
        pad_id: String,
    },
}

/// The first free identity among `root`, `root-2`, `root-3`, … — the shared
/// suffix-allocation loop for pads, assets, assemblies and generator models.
pub(crate) fn unique_suffixed_id(root: &str, used: impl Fn(&str) -> bool) -> String {
    if !used(root) {
        return root.to_owned();
    }
    (2_u64..)
        .map(|suffix| format!("{root}-{suffix}"))
        .find(|candidate| !used(candidate))
        .expect("suffix space is unbounded")
}

/// [`unique_suffixed_id`] bounded to suffixes 2..=1024 (`None` when exhausted).
pub(crate) fn unique_suffixed_id_capped(root: &str, used: impl Fn(&str) -> bool) -> Option<String> {
    if !used(root) {
        return Some(root.to_owned());
    }
    (2_u64..=1024)
        .map(|suffix| format!("{root}-{suffix}"))
        .find(|candidate| !used(candidate))
}

pub(crate) fn replacement_commit(operation: EditOperation, target_ids: Vec<String>) -> Resolution {
    Resolution::submit(target_ids, operation)
}

/// Replace the definition with the same id, or append it as a new project definition.
pub(crate) fn upsert_definition(document: &mut ProjectDoc, definition: PartDefinition) {
    if let Some(existing) = document
        .definitions
        .iter_mut()
        .find(|item| item.id == definition.id)
    {
        *existing = definition;
    } else {
        document.definitions.push(definition);
    }
}

/// The shared `ReplaceDocument` resolver: clone the accepted document, apply `edit`
/// (returning submit target ids or the retire reason), resolve `Unchanged` when the
/// clone still equals the accepted document (ADR-0005), else submit the replacement.
pub(crate) fn replace_document_resolver(
    name: &'static str,
    edit: impl Fn(&AcceptedSnapshot, &mut ProjectDoc) -> Result<Vec<String>, String> + 'static,
) -> EditResolver {
    EditResolver::new(name, move |accepted: &AcceptedSnapshot| {
        let mut replacement = accepted.document.as_ref().clone();
        match edit(accepted, &mut replacement) {
            Err(reason) => Resolution::Retire(reason),
            Ok(_) if replacement == *accepted.document => Resolution::Unchanged,
            Ok(target_ids) => replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                target_ids,
            ),
        }
    })
}

fn definition_field_resolver(
    definition_id: String,
    edit: DefinitionEdit,
    pad_id_seed: u64,
) -> EditResolver {
    replace_document_resolver("parts-definition-field", move |_, replacement| {
        let mut target_ids = vec![definition_id.clone()];
        apply_definition_edit(
            replacement,
            &definition_id,
            &mut target_ids,
            &edit,
            pad_id_seed,
        )?;
        Ok(target_ids)
    })
}

/// Apply one field edit in place. `Err` is the retire reason.
fn apply_definition_edit(
    doc: &mut ProjectDoc,
    definition_id: &str,
    target_ids: &mut Vec<String>,
    edit: &DefinitionEdit,
    pad_id_seed: u64,
) -> Result<(), String> {
    let Some(definition) = doc
        .definitions
        .iter_mut()
        .find(|item| item.id == definition_id)
    else {
        return Err(DEFINITION_GONE.into());
    };
    if definition.generator.is_some() {
        return Err(GENERATOR_LOCKED.into());
    }
    match edit {
        DefinitionEdit::Name(name) if name.trim().is_empty() => {
            return Err("Name is required.".into());
        }
        DefinitionEdit::Name(name) => definition.name = name.trim().to_owned(),
        DefinitionEdit::Kind(kind) => definition.kind = kind.clone(),
        DefinitionEdit::CourtyardWidth(value) => {
            let width = valid_number(*value, true, COURTYARD_NUMBERS)?;
            let (_, height, center) = courtyard_bounds(&definition.courtyard);
            authored_rectangle(definition, width, height, center);
        }
        DefinitionEdit::CourtyardHeight(value) => {
            let height = valid_number(*value, true, COURTYARD_NUMBERS)?;
            let (width, _, center) = courtyard_bounds(&definition.courtyard);
            authored_rectangle(definition, width, height, center);
        }
        DefinitionEdit::AddPad => {
            imported_pads_retire(definition)?;
            let number = next_pad_number(&definition.pads);
            let id = unique_pad_id(&definition.pads, pad_id_seed);
            definition.pads.push(Pad {
                id,
                number,
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 2.0, y: 2.0 },
                shape: PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            });
        }
        DefinitionEdit::PadId { pad_id, value } => {
            imported_pads_retire(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == *pad_id) {
                return Err(PAD_GONE.into());
            }
            if value.trim().is_empty() {
                return Err(PAD_IDS_REQUIRED.into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != *pad_id && other.id == *value)
            {
                return Err(PAD_IDS_UNIQUE.into());
            }
            if let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) {
                pad.id = value.clone();
            }
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                for pin in &mut net.pins {
                    if part_ids.contains(&pin.part_id) && pin.pad_id == *pad_id {
                        pin.pad_id = value.clone();
                    }
                }
            }
            target_ids.extend([pad_id.clone(), value.clone()]);
        }
        DefinitionEdit::PadNumber { pad_id, value } => {
            imported_pads_retire(definition)?;
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err(PAD_IDS_REQUIRED.into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != *pad_id && other.number == value)
            {
                return Err(PAD_NUMBERS_UNIQUE.into());
            }
            let pad = find_pad(definition, pad_id)?;
            pad.number = value;
        }
        DefinitionEdit::PadCoordinate {
            pad_id,
            axis,
            value,
        } => {
            imported_pads_retire(definition)?;
            let number = valid_number(*value, false, PAD_NUMBERS)?;
            let pad = find_pad(definition, pad_id)?;
            match axis {
                Axis::X => pad.at.x = number,
                Axis::Y => pad.at.y = number,
            }
        }
        DefinitionEdit::PadSize {
            pad_id,
            axis,
            value,
        } => {
            imported_pads_retire(definition)?;
            let number = valid_number(*value, true, PAD_NUMBERS)?;
            let pad = find_pad(definition, pad_id)?;
            match axis {
                Axis::X => pad.size.x = number,
                Axis::Y => pad.size.y = number,
            }
        }
        DefinitionEdit::PadDrill { pad_id, value } => {
            imported_pads_retire(definition)?;
            let drill = value
                .map(|value| valid_number(value, true, PAD_NUMBERS))
                .transpose()?;
            let pad = find_pad(definition, pad_id)?;
            pad.drill = drill;
        }
        DefinitionEdit::PadShape { pad_id, shape } => {
            imported_pads_retire(definition)?;
            let pad = find_pad(definition, pad_id)?;
            pad.shape = shape.clone();
        }
        DefinitionEdit::RemovePad { pad_id } => {
            imported_pads_retire(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == *pad_id) {
                return Err(PAD_GONE.into());
            }
            definition.pads.retain(|pad| pad.id != *pad_id);
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                net.pins
                    .retain(|pin| !part_ids.contains(&pin.part_id) || pin.pad_id != *pad_id);
            }
            target_ids.push(pad_id.clone());
        }
    }
    Ok(())
}

/// Rewrite the courtyard as an authored rectangle of the given size around `center`.
fn find_pad<'a>(definition: &'a mut PartDefinition, id: &str) -> Result<&'a mut Pad, String> {
    definition
        .pads
        .iter_mut()
        .find(|pad| pad.id == id)
        .ok_or_else(|| PAD_GONE.into())
}

fn authored_rectangle(definition: &mut PartDefinition, width: f64, height: f64, center: Vec2) {
    let half_x = width / 2.0;
    let half_y = height / 2.0;
    definition.courtyard = vec![
        Vec2 {
            x: center.x - half_x,
            y: center.y - half_y,
        },
        Vec2 {
            x: center.x + half_x,
            y: center.y - half_y,
        },
        Vec2 {
            x: center.x + half_x,
            y: center.y + half_y,
        },
        Vec2 {
            x: center.x - half_x,
            y: center.y + half_y,
        },
    ];
    let envelope = definition
        .envelope_source
        .get_or_insert_with(Default::default);
    envelope.courtyard = Some(boardstudio_model::model::EnvelopeOrigin::Authored);
}

fn imported_pads_retire(definition: &PartDefinition) -> Result<(), String> {
    if definition.kicad_source.is_some() {
        Err(IMPORTED_PADS.into())
    } else {
        Ok(())
    }
}
fn valid_number(value: f64, positive: bool, message: &str) -> Result<f64, String> {
    if !value.is_finite() || (positive && value <= 0.0) {
        Err(message.into())
    } else {
        Ok(value)
    }
}
pub(crate) fn courtyard_bounds(points: &[Vec2]) -> (f64, f64, Vec2) {
    if points.is_empty() {
        return (10.0, 6.0, Vec2 { x: 0.0, y: 0.0 });
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    (
        (max_x - min_x),
        (max_y - min_y),
        Vec2 {
            x: (min_x + max_x) / 2.0,
            y: (min_y + max_y) / 2.0,
        },
    )
}
fn next_pad_number(pads: &[Pad]) -> String {
    let mut candidate = pads.len() + 1;
    while pads.iter().any(|pad| pad.number == candidate.to_string()) {
        candidate += 1;
    }
    candidate.to_string()
}
fn unique_pad_id(pads: &[Pad], operation: u64) -> String {
    unique_suffixed_id(&format!("pad-{operation}"), |candidate| {
        pads.iter().any(|pad| pad.id == candidate)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(pads: serde_json::Value) -> PartDefinition {
        serde_json::from_value(serde_json::json!({"id":"custom","name":"Custom","kind":"custom","courtyard":[],"pads":pads})).expect("definition")
    }
    fn pad(id: &str, number: &str) -> serde_json::Value {
        serde_json::json!({ "id":id,"number":number,"at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle" })
    }

    #[test]
    fn pad_rename_and_removal_keep_instance_pin_relationships_scoped() {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition(serde_json::json!([
            pad("old", "1"),
            pad("keep", "2")
        ])));
        document.parts = serde_json::from_value(serde_json::json!([{"id":"instance-a","definitionId":"custom","reference":"U1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},{"id":"instance-b","definitionId":"custom","reference":"U2","pose":{"at":{"x":1.0,"y":0.0},"rotation":0.0},"side":"front"},{"id":"other","definitionId":"other-definition","reference":"R1","pose":{"at":{"x":2.0,"y":0.0},"rotation":0.0},"side":"front"}])).unwrap();
        document.nets = serde_json::from_value(serde_json::json!([{"id":"net-a","name":"A","pins":[{"partId":"instance-a","padId":"old"},{"partId":"instance-b","padId":"old"},{"partId":"other","padId":"old"}]},{"id":"net-b","name":"B","pins":[{"partId":"instance-a","padId":"keep"}]}])).unwrap();
        let mut targets = vec!["custom".to_owned()];
        apply_definition_edit(
            &mut document,
            "custom",
            &mut targets,
            &DefinitionEdit::PadId {
                pad_id: "old".into(),
                value: "renamed".into(),
            },
            1,
        )
        .unwrap();
        assert_eq!(document.definitions[0].pads[0].id, "renamed");
        let first_net_pads: Vec<_> = document.nets[0]
            .pins
            .iter()
            .map(|pin| pin.pad_id.as_str())
            .collect();
        assert_eq!(first_net_pads, ["renamed", "renamed", "old"]);

        apply_definition_edit(
            &mut document,
            "custom",
            &mut targets,
            &DefinitionEdit::RemovePad {
                pad_id: "renamed".into(),
            },
            2,
        )
        .unwrap();
        assert_eq!(document.nets[0].pins.len(), 1);
        assert_eq!(document.nets[0].pins[0].part_id, "other");
        assert_eq!(document.nets[1].pins[0].pad_id, "keep");
    }

    #[test]
    fn courtyard_edit_preserves_the_existing_center_and_other_dimension() {
        let mut component = definition(serde_json::json!([]));
        component.courtyard = vec![Vec2 { x: 1.0, y: 2.0 }, Vec2 { x: 3.0, y: 6.0 }];
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(component);
        apply_definition_edit(
            &mut document,
            "custom",
            &mut vec![],
            &DefinitionEdit::CourtyardWidth(8.0),
            0,
        )
        .unwrap();
        assert_eq!(
            courtyard_bounds(&document.definitions[0].courtyard),
            (8.0, 4.0, Vec2 { x: 2.0, y: 4.0 })
        );
    }
}
