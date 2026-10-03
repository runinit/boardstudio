//! Scoped edits for the authored geometry fields of a project part definition.

use boardstudio_application::{
    AcceptedSnapshot, Event, OperationId, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Pad, PadShape, PartDefinition, PartKind, Vec2,
};

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{Axis, DefinitionEdit, DefinitionFieldsCapture, apply_definition_edit};
    use crate::runtime::Runtime;
    use boardstudio_application::{AcceptedSnapshot, Scope};
    use boardstudio_core::model::{Pad, PadShape, PartDefinition, PartKind};
    use dioxus::prelude::*;
    use dioxus_web::WebEventExt;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlInputElement;

    #[component]
    pub(crate) fn CustomDefinitionFields(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let error = use_signal(String::new);
        let capture = DefinitionFieldsCapture::new(&snapshot, scope.clone(), &definition);
        let submit = use_callback({
            let runtime = runtime.clone();
            let mut error = error;
            move |edit: DefinitionEdit| {
                let model = runtime.model();
                let Some(current) = model.accepted.as_ref() else {
                    return;
                };
                match apply_definition_edit(
                    current,
                    runtime.scope(),
                    selection(),
                    &capture,
                    edit,
                    runtime.operation(),
                ) {
                    Ok(Some(event)) => {
                        error.set(String::new());
                        runtime.submit(event);
                    }
                    Ok(None) => {}
                    Err(message) => error.set(message),
                }
            }
        });

        let courtyard = courtyard_size(&definition);
        let kicad_locked = definition.kicad_source.is_some();
        let initial_width = courtyard.0.clone();
        let initial_height = courtyard.1.clone();
        let mut width = use_signal(move || initial_width);
        let mut height = use_signal(move || initial_height);
        use_effect(use_reactive((&courtyard,), move |(size,)| {
            width.set(size.0);
            height.set(size.1);
        }));

        let kind = kind_name(&definition.kind);
        let on_kind = {
            let submit = submit;
            move |event: FormEvent| {
                if let Some(kind) = parse_kind(&event.value()) {
                    submit.call(DefinitionEdit::Kind(kind));
                }
            }
        };
        let on_add = {
            let submit = submit;
            move |_| submit.call(DefinitionEdit::AddPad)
        };
        let submit_width = submit;
        let on_width_blur = move |_| submit_width.call(DefinitionEdit::CourtyardWidth(width()));
        let submit_height = submit;
        let on_height_blur = move |_| submit_height.call(DefinitionEdit::CourtyardHeight(height()));
        let committed_width = courtyard.0.clone();
        let width_keydown = move |event| draft_keydown(event, width, committed_width.clone());
        let committed_height = courtyard.1.clone();
        let height_keydown = move |event| draft_keydown(event, height, committed_height.clone());

        rsx! {
            div { class: "m1-definition-fields",
                label {
                    "Kind"
                    select {
                        aria_label: "Definition kind",
                        value: "{kind}",
                        onchange: on_kind,
                        option { value: "switch", "Switch" }
                        option { value: "controller", "Controller" }
                        option { value: "connector", "Connector" }
                        option { value: "encoder", "Encoder" }
                        option { value: "passive", "Passive" }
                        option { value: "custom", "Custom" }
                    }
                }
                fieldset { class: "m1-definition-courtyard",
                    legend { "Rectangular courtyard (mm)" }
                    label { "Width", input {
                        aria_label: "Courtyard width",
                        r#type: "number", min: "0.01", step: "0.1", value: "{width()}",
                        oninput: move |event| width.set(event.value()),
                        onblur: on_width_blur,
                        onkeydown: width_keydown,
                    } }
                    label { "Height", input {
                        aria_label: "Courtyard height",
                        r#type: "number", min: "0.01", step: "0.1", value: "{height()}",
                        oninput: move |event| height.set(event.value()),
                        onblur: on_height_blur,
                        onkeydown: height_keydown,
                    } }
                }
                div { class: "m1-definition-pad-heading",
                    strong { "Pads " small { "{definition.pads.len()}" } }
                    button { r#type: "button", disabled: kicad_locked, onclick: on_add, "+ Add pad" }
                }
                if kicad_locked {
                    p { class: "m1-definition-note", "Imported pad geometry stays linked to its original KiCad source." }
                }
                for (index, pad) in definition.pads.iter().enumerate() {
                    PadFields {
                        key: "{pad.id}", index, pad: pad.clone(), locked: kicad_locked,
                        submit: submit.clone(),
                    }
                }
                ul { class: "m1-definition-validation", "aria-live": "polite",
                    for issue in definition_issues(&definition) { li { "{issue}" } }
                }
                if !error().is_empty() {
                    p { class: "m1-definition-error", role: "alert", "{error()}" }
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
        let mut id = use_signal(|| pad.id.clone());
        let mut number = use_signal(|| pad.number.clone());
        let mut x = use_signal(|| pad.at.x.to_string());
        let mut y = use_signal(|| pad.at.y.to_string());
        let mut size_x = use_signal(|| pad.size.x.to_string());
        let mut size_y = use_signal(|| pad.size.y.to_string());
        let mut drill = use_signal(|| pad.drill.map(|value| value.to_string()).unwrap_or_default());
        let identity = (
            pad.id.clone(),
            pad.number.clone(),
            pad.at.x,
            pad.at.y,
            pad.size.x,
            pad.size.y,
            pad.drill,
        );
        use_effect(use_reactive((&identity,), move |(value,)| {
            id.set(value.0.clone());
            number.set(value.1.clone());
            x.set(value.2.to_string());
            y.set(value.3.to_string());
            size_x.set(value.4.to_string());
            size_y.set(value.5.to_string());
            drill.set(value.6.map(|number| number.to_string()).unwrap_or_default());
        }));
        let submit_id = submit;
        let old_id = pad.id.clone();
        let on_id_blur = move |_| {
            submit_id.call(DefinitionEdit::PadId {
                pad_id: old_id.clone(),
                value: id().trim().to_owned(),
            })
        };
        let committed_id = pad.id.clone();
        let id_keydown = move |event| draft_keydown(event, id, committed_id.clone());
        let submit_number = submit;
        let old_id = pad.id.clone();
        let on_number_blur = move |_| {
            submit_number.call(DefinitionEdit::PadNumber {
                pad_id: old_id.clone(),
                value: number().trim().to_owned(),
            })
        };
        let committed_number = pad.number.clone();
        let number_keydown = move |event| draft_keydown(event, number, committed_number.clone());
        let submit_x = submit;
        let old_id = pad.id.clone();
        let on_x_blur = move |_| {
            submit_x.call(DefinitionEdit::PadCoordinate {
                pad_id: old_id.clone(),
                axis: Axis::X,
                value: x(),
            })
        };
        let committed_x = pad.at.x.to_string();
        let x_keydown = move |event| draft_keydown(event, x, committed_x.clone());
        let submit_y = submit;
        let old_id = pad.id.clone();
        let on_y_blur = move |_| {
            submit_y.call(DefinitionEdit::PadCoordinate {
                pad_id: old_id.clone(),
                axis: Axis::Y,
                value: y(),
            })
        };
        let committed_y = pad.at.y.to_string();
        let y_keydown = move |event| draft_keydown(event, y, committed_y.clone());
        let submit_width = submit;
        let old_id = pad.id.clone();
        let on_width_blur = move |_| {
            submit_width.call(DefinitionEdit::PadSize {
                pad_id: old_id.clone(),
                axis: Axis::X,
                value: size_x(),
            })
        };
        let committed_width = pad.size.x.to_string();
        let width_keydown = move |event| draft_keydown(event, size_x, committed_width.clone());
        let submit_height = submit;
        let old_id = pad.id.clone();
        let on_height_blur = move |_| {
            submit_height.call(DefinitionEdit::PadSize {
                pad_id: old_id.clone(),
                axis: Axis::Y,
                value: size_y(),
            })
        };
        let committed_height = pad.size.y.to_string();
        let height_keydown = move |event| draft_keydown(event, size_y, committed_height.clone());
        let submit_drill = submit;
        let old_id = pad.id.clone();
        let on_drill_blur = move |_| {
            submit_drill.call(DefinitionEdit::PadDrill {
                pad_id: old_id.clone(),
                value: drill(),
            })
        };
        let committed_drill = pad
            .drill
            .map(|number| number.to_string())
            .unwrap_or_default();
        let drill_keydown = move |event| draft_keydown(event, drill, committed_drill.clone());
        let submit_shape = submit;
        let old_id = pad.id.clone();
        let on_shape = move |event: FormEvent| {
            if let Some(shape) = parse_shape(&event.value()) {
                submit_shape.call(DefinitionEdit::PadShape {
                    pad_id: old_id.clone(),
                    shape,
                });
            }
        };
        let submit_remove = submit;
        let remove_id = pad.id.clone();
        let on_remove = move |_| {
            submit_remove.call(DefinitionEdit::RemovePad {
                pad_id: remove_id.clone(),
            })
        };

        rsx! {
            fieldset { class: "m1-definition-pad", disabled: locked,
                legend { "Pad {index + 1}" }
                div { class: "m1-definition-pad-grid",
                    label { "ID", input { aria_label: "Pad {index + 1} ID", value: "{id()}", oninput: move |event| id.set(event.value()), onblur: on_id_blur, onkeydown: id_keydown } }
                    label { "Number", input { aria_label: "Pad {index + 1} number", value: "{number()}", oninput: move |event| number.set(event.value()), onblur: on_number_blur, onkeydown: number_keydown } }
                    label { "X", input { aria_label: "Pad {index + 1} X", r#type: "number", step: "0.1", value: "{x()}", oninput: move |event| x.set(event.value()), onblur: on_x_blur, onkeydown: x_keydown } }
                    label { "Y", input { aria_label: "Pad {index + 1} Y", r#type: "number", step: "0.1", value: "{y()}", oninput: move |event| y.set(event.value()), onblur: on_y_blur, onkeydown: y_keydown } }
                    label { "Width", input { aria_label: "Pad {index + 1} width", r#type: "number", min: "0.01", step: "0.1", value: "{size_x()}", oninput: move |event| size_x.set(event.value()), onblur: on_width_blur, onkeydown: width_keydown } }
                    label { "Height", input { aria_label: "Pad {index + 1} height", r#type: "number", min: "0.01", step: "0.1", value: "{size_y()}", oninput: move |event| size_y.set(event.value()), onblur: on_height_blur, onkeydown: height_keydown } }
                    label { "Shape", select { aria_label: "Pad {index + 1} shape", value: "{shape_name(&pad.shape)}", onchange: on_shape,
                        option { value: "circle", "Circle" } option { value: "oval", "Oval" }
                        option { value: "rect", "Rectangle" } option { value: "roundrect", "Rounded rectangle" }
                    } }
                    label { "Drill", input { aria_label: "Pad {index + 1} drill", r#type: "number", min: "0.01", step: "0.1", placeholder: "None", value: "{drill()}", oninput: move |event| drill.set(event.value()), onblur: on_drill_blur, onkeydown: drill_keydown } }
                }
                button { class: "m1-definition-remove-pad", r#type: "button", onclick: on_remove, "Remove pad" }
            }
        }
    }

    fn kind_name(kind: &PartKind) -> &'static str {
        match kind {
            PartKind::Switch => "switch",
            PartKind::Controller => "controller",
            PartKind::Connector => "connector",
            PartKind::Encoder => "encoder",
            PartKind::Passive => "passive",
            PartKind::Custom => "custom",
            PartKind::Utility => "custom",
        }
    }
    fn shape_name(shape: &PadShape) -> &'static str {
        match shape {
            PadShape::Circle => "circle",
            PadShape::Oval => "oval",
            PadShape::Rect => "rect",
            PadShape::Roundrect => "roundrect",
        }
    }
    fn courtyard_size(definition: &PartDefinition) -> (String, String) {
        let bounds = definition
            .courtyard
            .iter()
            .fold(None, |bounds, point| match bounds {
                None => Some((point.x, point.y, point.x, point.y)),
                Some((min_x, min_y, max_x, max_y)) => Some((
                    min_x.min(point.x),
                    min_y.min(point.y),
                    max_x.max(point.x),
                    max_y.max(point.y),
                )),
            });
        match bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                ((max_x - min_x).to_string(), (max_y - min_y).to_string())
            }
            None => ("10".into(), "6".into()),
        }
    }
    fn parse_kind(value: &str) -> Option<PartKind> {
        match value {
            "switch" => Some(PartKind::Switch),
            "controller" => Some(PartKind::Controller),
            "connector" => Some(PartKind::Connector),
            "encoder" => Some(PartKind::Encoder),
            "passive" => Some(PartKind::Passive),
            "custom" => Some(PartKind::Custom),
            _ => None,
        }
    }
    fn parse_shape(value: &str) -> Option<PadShape> {
        match value {
            "circle" => Some(PadShape::Circle),
            "oval" => Some(PadShape::Oval),
            "rect" => Some(PadShape::Rect),
            "roundrect" => Some(PadShape::Roundrect),
            _ => None,
        }
    }
    fn draft_keydown(event: KeyboardEvent, mut draft: Signal<String>, accepted: String) {
        if event.key() == Key::Escape {
            event.prevent_default();
            draft.set(accepted);
        } else if event.key() == Key::Enter
            && let Some(input) = event
                .data()
                .try_as_web_event()
                .and_then(|event| event.target())
                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
        {
            let _ = input.blur();
        }
    }
    fn definition_issues(definition: &PartDefinition) -> Vec<String> {
        let mut issues = Vec::new();
        if definition.name.trim().is_empty() {
            issues.push("Name is required.".to_owned());
        }
        let (width, height, _) = super::courtyard_bounds(&definition.courtyard);
        if definition.courtyard.len() < 3
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            issues.push("Courtyard must have positive width and height.".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for (index, pad) in definition.pads.iter().enumerate() {
            if pad.id.trim().is_empty() || !ids.insert(pad.id.clone()) {
                issues.push(format!("Pad {} needs a unique ID.", index + 1));
            }
            if definition.kicad_source.is_none() && pad.number.trim().is_empty() {
                issues.push(format!("Pad {} needs a number.", index + 1));
            }
            if !pad.at.x.is_finite() || !pad.at.y.is_finite() {
                issues.push(format!("Pad {} position must be finite.", index + 1));
            }
            if !pad.size.x.is_finite()
                || !pad.size.y.is_finite()
                || pad.size.x <= 0.0
                || pad.size.y <= 0.0
            {
                issues.push(format!("Pad {} size must be positive.", index + 1));
            }
            if pad
                .drill
                .is_some_and(|drill| !drill.is_finite() || drill <= 0.0)
            {
                issues.push(format!("Pad {} drill must be positive.", index + 1));
            }
        }
        issues
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use ui::CustomDefinitionFields;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DefinitionFieldsCapture {
    scope: Option<Scope>,
    definition_id: String,
    session_epoch: SessionEpoch,
    document_id: String,
    snapshot_token: SnapshotToken,
    revision: u64,
}

impl DefinitionFieldsCapture {
    pub(crate) fn new(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        definition: &PartDefinition,
    ) -> Self {
        Self {
            scope,
            definition_id: definition.id.clone(),
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DefinitionEdit {
    Kind(PartKind),
    CourtyardWidth(String),
    CourtyardHeight(String),
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
        value: String,
    },
    PadSize {
        pad_id: String,
        axis: Axis,
        value: String,
    },
    PadDrill {
        pad_id: String,
        value: String,
    },
    PadShape {
        pad_id: String,
        shape: PadShape,
    },
    RemovePad {
        pad_id: String,
    },
}

pub(crate) fn apply_definition_edit(
    current: &AcceptedSnapshot,
    current_scope: Option<Scope>,
    current_selection: Option<(Option<Scope>, String)>,
    capture: &DefinitionFieldsCapture,
    edit: DefinitionEdit,
    operation_id: OperationId,
) -> Result<Option<Event>, String> {
    if capture.scope.is_none()
        || current_scope != capture.scope
        || current.token != capture.snapshot_token
        || current.session_epoch != capture.session_epoch
        || current.document.id != capture.document_id
        || current.document.revision != capture.revision
        || current_selection != Some((capture.scope.clone(), capture.definition_id.clone()))
    {
        return Ok(None);
    }
    let mut doc = current.document.as_ref().clone();
    let Some(definition) = doc
        .definitions
        .iter_mut()
        .find(|item| item.id == capture.definition_id)
    else {
        return Ok(None);
    };
    if definition.generator.is_some() {
        return Ok(None);
    }
    let mut target_ids = vec![definition.id.clone()];
    match edit {
        DefinitionEdit::Kind(kind) => definition.kind = kind,
        DefinitionEdit::CourtyardWidth(value) => {
            let dimension = parse_number(
                &value,
                true,
                "Courtyard dimensions must be positive numbers.",
            )?;
            let (_, old_height, center) = courtyard_bounds(&definition.courtyard);
            let width = dimension;
            let height = old_height;
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
            envelope.courtyard = Some(boardstudio_core::model::EnvelopeOrigin::Authored);
        }
        DefinitionEdit::CourtyardHeight(value) => {
            let dimension = parse_number(
                &value,
                true,
                "Courtyard dimensions must be positive numbers.",
            )?;
            let (old_width, _, center) = courtyard_bounds(&definition.courtyard);
            let width = old_width;
            let height = dimension;
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
            envelope.courtyard = Some(boardstudio_core::model::EnvelopeOrigin::Authored);
        }
        DefinitionEdit::AddPad => {
            if definition.kicad_source.is_some() {
                return Ok(None);
            }
            let number = next_pad_number(&definition.pads);
            let id = unique_pad_id(&definition.pads, operation_id.0);
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
            ensure_manual_pads(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == pad_id) {
                return Ok(None);
            }
            if value.trim().is_empty() {
                return Err("Pad IDs and numbers are required. Positions must be finite, sizes and drill must be positive.".into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != pad_id && other.id == value)
            {
                return Err("Pad IDs must be unique within the component.".into());
            }
            let old_id = pad_id.clone();
            if let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == pad_id) {
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
                    if part_ids.contains(&pin.part_id) && pin.pad_id == old_id {
                        pin.pad_id = value.clone();
                    }
                }
            }
            target_ids.extend([old_id, value]);
        }
        DefinitionEdit::PadNumber { pad_id, value } => {
            ensure_manual_pads(definition)?;
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err("Pad IDs and numbers are required. Positions must be finite, sizes and drill must be positive.".into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != pad_id && other.number == value)
            {
                return Err("Pad numbers must be unique within the component.".into());
            }
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == pad_id) else {
                return Ok(None);
            };
            pad.number = value;
        }
        DefinitionEdit::PadCoordinate {
            pad_id,
            axis,
            value,
        } => {
            ensure_manual_pads(definition)?;
            let number = parse_number(
                &value,
                false,
                "Pad positions and dimensions must be finite numbers.",
            )?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == pad_id) else {
                return Ok(None);
            };
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
            ensure_manual_pads(definition)?;
            let number = parse_number(
                &value,
                true,
                "Pad positions and dimensions must be finite numbers.",
            )?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == pad_id) else {
                return Ok(None);
            };
            match axis {
                Axis::X => pad.size.x = number,
                Axis::Y => pad.size.y = number,
            }
        }
        DefinitionEdit::PadDrill { pad_id, value } => {
            ensure_manual_pads(definition)?;
            let drill = if value.trim().is_empty() {
                None
            } else {
                Some(parse_number(
                    &value,
                    true,
                    "Pad positions and dimensions must be finite numbers.",
                )?)
            };
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == pad_id) else {
                return Ok(None);
            };
            pad.drill = drill;
        }
        DefinitionEdit::PadShape { pad_id, shape } => {
            ensure_manual_pads(definition)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == pad_id) else {
                return Ok(None);
            };
            pad.shape = shape;
        }
        DefinitionEdit::RemovePad { pad_id } => {
            ensure_manual_pads(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == pad_id) {
                return Ok(None);
            }
            definition.pads.retain(|pad| pad.id != pad_id);
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                net.pins
                    .retain(|pin| !part_ids.contains(&pin.part_id) || pin.pad_id != pad_id);
            }
            target_ids.push(pad_id);
        }
    }
    if current.document.as_ref() == &doc {
        return Ok(None);
    }
    Ok(Some(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: capture.revision,
            transaction_id: format!("parts-definition-fields-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids,
            operation: EditOperation::ReplaceDocument {
                document: Box::new(doc),
            },
        },
    }))
}

fn ensure_manual_pads(definition: &PartDefinition) -> Result<(), String> {
    if definition.kicad_source.is_some() {
        Err("Imported pad geometry stays linked to its original KiCad source.".into())
    } else {
        Ok(())
    }
}
fn parse_number(value: &str, positive: bool, message: &str) -> Result<f64, String> {
    let number = if value.trim().is_empty() && !positive {
        0.0
    } else {
        value.parse::<f64>().unwrap_or(f64::NAN)
    };
    if !number.is_finite() || (positive && number <= 0.0) {
        Err(message.into())
    } else {
        Ok(number)
    }
}
fn courtyard_bounds(points: &[Vec2]) -> (f64, f64, Vec2) {
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
    let base = format!("pad-{operation}");
    if !pads.iter().any(|pad| pad.id == base) {
        return base;
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !pads.iter().any(|pad| pad.id == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{ProjectDoc, SceneDelta};
    use std::sync::Arc;

    fn definition(pads: serde_json::Value) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"custom","name":"Custom","kind":"custom","courtyard":[],"pads":pads
        }))
        .expect("definition")
    }
    fn snapshot(definition: PartDefinition) -> AcceptedSnapshot {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition);
        snapshot_doc(document)
    }
    fn snapshot_doc(document: ProjectDoc) -> AcceptedSnapshot {
        let scene: SceneDelta = serde_json::from_value(serde_json::json!({
            "revision":0,"transactionId":"fixture","changedIds":[],"transforms":[],
            "matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[],
            "readiness":{"layout":true,"outline":true,"pcb":true,"case":false}
        }))
        .expect("scene");
        AcceptedSnapshot {
            session_epoch: SessionEpoch(1),
            token: SnapshotToken(7),
            document: Arc::new(document),
            scene: Arc::new(scene),
        }
    }
    fn scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        }
    }
    fn pad(id: &str, number: &str) -> serde_json::Value {
        serde_json::json!({ "id":id,"number":number,"at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle" })
    }
    fn replacement(event: Event) -> ProjectDoc {
        let Event::Edit {
            command:
                EditCommand {
                    operation: EditOperation::ReplaceDocument { document },
                    ..
                },
            ..
        } = event
        else {
            panic!("expected replacement edit")
        };
        *document
    }
    fn scoped_event(current: &AcceptedSnapshot, edit: DefinitionEdit, operation: u64) -> Event {
        let owner_scope = scope();
        let capture = DefinitionFieldsCapture::new(
            current,
            Some(owner_scope.clone()),
            &current.document.definitions[0],
        );
        apply_definition_edit(
            current,
            Some(owner_scope.clone()),
            Some((Some(owner_scope), "custom".into())),
            &capture,
            edit,
            OperationId(operation),
        )
        .unwrap()
        .expect("accepted edit")
    }

    #[test]
    fn add_pad_keeps_free_default_and_repairs_collision_as_one_edit() {
        // Expected-red reference oracle: pads.len()+1 is 3 for [1, 3], so literal
        // reference code creates a duplicate. The reviewed correction advances to 4.
        let pads: Vec<Pad> = vec![
            serde_json::from_value(pad("a", "1")).unwrap(),
            serde_json::from_value(pad("b", "3")).unwrap(),
        ];
        let old_reference_candidate = pads.len() + 1;
        assert_eq!(old_reference_candidate.to_string(), "3");
        assert!(
            pads.iter()
                .any(|pad| pad.number == old_reference_candidate.to_string())
        );
        assert_eq!(next_pad_number(&pads), "4");
        let ordinary = vec![
            serde_json::from_value(pad("a", "1")).unwrap(),
            serde_json::from_value(pad("b", "2")).unwrap(),
        ];
        assert_eq!(next_pad_number(&ordinary), "3");

        let current = snapshot(definition(serde_json::json!([
            pad("a", "1"),
            pad("b", "3")
        ])));
        let scope = scope();
        let capture = DefinitionFieldsCapture::new(
            &current,
            Some(scope.clone()),
            &current.document.definitions[0],
        );
        let event = apply_definition_edit(
            &current,
            Some(scope.clone()),
            Some((Some(scope), "custom".into())),
            &capture,
            DefinitionEdit::AddPad,
            OperationId(9),
        )
        .unwrap()
        .unwrap();
        let edited = replacement(event);
        let pads = &edited.definitions[0].pads;
        assert_eq!(pads.len(), 3);
        assert_eq!(pads[2].number, "4");
        assert_eq!(pads[2].id, "pad-9");
    }

    #[test]
    fn numeric_fields_keep_reference_blank_and_validation_semantics() {
        assert_eq!(parse_number("", false, "bad").unwrap(), 0.0);
        assert!(parse_number("", true, "bad").is_err());
        let mut edited_definition = definition(serde_json::json!([pad("a", "1")]));
        edited_definition.pads[0].at.x = 5.0;
        let current = snapshot(edited_definition);
        let scope = scope();
        let capture = DefinitionFieldsCapture::new(
            &current,
            Some(scope.clone()),
            &current.document.definitions[0],
        );
        let event = apply_definition_edit(
            &current,
            Some(scope.clone()),
            Some((Some(scope), "custom".into())),
            &capture,
            DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "".into(),
            },
            OperationId(1),
        )
        .unwrap()
        .unwrap();
        assert_eq!(replacement(event).definitions[0].pads[0].at.x, 0.0);
    }

    #[test]
    fn every_supported_field_uses_an_individual_accepted_edit() {
        let mut initial = definition(serde_json::json!([pad("a", "1")]));
        initial.pads[0].at.x = 1.0;
        initial.courtyard = vec![
            Vec2 { x: -5.0, y: -3.0 },
            Vec2 { x: 5.0, y: -3.0 },
            Vec2 { x: 5.0, y: 3.0 },
            Vec2 { x: -5.0, y: 3.0 },
        ];
        let current = snapshot(initial);

        let kind = replacement(scoped_event(
            &current,
            DefinitionEdit::Kind(PartKind::Connector),
            10,
        ));
        assert!(matches!(kind.definitions[0].kind, PartKind::Connector));
        let width = replacement(scoped_event(
            &snapshot_doc(kind),
            DefinitionEdit::CourtyardWidth("14".into()),
            11,
        ));
        let bounds = courtyard_bounds(&width.definitions[0].courtyard);
        assert_eq!(bounds.0, 14.0);
        assert_eq!(bounds.1, 6.0);
        let height = replacement(scoped_event(
            &snapshot_doc(width),
            DefinitionEdit::CourtyardHeight("8".into()),
            12,
        ));
        assert_eq!(courtyard_bounds(&height.definitions[0].courtyard).1, 8.0);
        let number = replacement(scoped_event(
            &snapshot_doc(height),
            DefinitionEdit::PadNumber {
                pad_id: "a".into(),
                value: "9".into(),
            },
            13,
        ));
        assert_eq!(number.definitions[0].pads[0].number, "9");
        let y = replacement(scoped_event(
            &snapshot_doc(number),
            DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::Y,
                value: "2".into(),
            },
            14,
        ));
        assert_eq!(y.definitions[0].pads[0].at.y, 2.0);
        let size_x = replacement(scoped_event(
            &snapshot_doc(y),
            DefinitionEdit::PadSize {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "3".into(),
            },
            15,
        ));
        let size_y = replacement(scoped_event(
            &snapshot_doc(size_x),
            DefinitionEdit::PadSize {
                pad_id: "a".into(),
                axis: Axis::Y,
                value: "4".into(),
            },
            16,
        ));
        assert_eq!(size_y.definitions[0].pads[0].size, Vec2 { x: 3.0, y: 4.0 });
        let drill = replacement(scoped_event(
            &snapshot_doc(size_y),
            DefinitionEdit::PadDrill {
                pad_id: "a".into(),
                value: "0.5".into(),
            },
            17,
        ));
        assert_eq!(drill.definitions[0].pads[0].drill, Some(0.5));
        let no_drill = replacement(scoped_event(
            &snapshot_doc(drill),
            DefinitionEdit::PadDrill {
                pad_id: "a".into(),
                value: String::new(),
            },
            18,
        ));
        assert_eq!(no_drill.definitions[0].pads[0].drill, None);
        let shape = replacement(scoped_event(
            &snapshot_doc(no_drill),
            DefinitionEdit::PadShape {
                pad_id: "a".into(),
                shape: PadShape::Oval,
            },
            19,
        ));
        assert!(matches!(shape.definitions[0].pads[0].shape, PadShape::Oval));
    }

    #[test]
    fn stale_snapshot_and_selection_cannot_commit_field_drafts() {
        let current = snapshot(definition(serde_json::json!([pad("a", "1")])));
        let owner_scope = scope();
        let capture = DefinitionFieldsCapture::new(
            &current,
            Some(owner_scope.clone()),
            &current.document.definitions[0],
        );
        let mut newer_document = current.document.as_ref().clone();
        newer_document.revision += 1;
        let newer = AcceptedSnapshot {
            token: SnapshotToken(8),
            document: Arc::new(newer_document),
            ..current.clone()
        };
        assert!(
            apply_definition_edit(
                &newer,
                Some(owner_scope.clone()),
                Some((Some(owner_scope.clone()), "custom".into())),
                &capture,
                DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "3".into()
                },
                OperationId(20),
            )
            .unwrap()
            .is_none()
        );
        assert!(
            apply_definition_edit(
                &current,
                Some(owner_scope.clone()),
                Some((Some(owner_scope.clone()), "other".into())),
                &capture,
                DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "3".into()
                },
                OperationId(21),
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn pad_rename_and_removal_keep_instance_pin_relationships_scoped() {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition(serde_json::json!([
            pad("old", "1"),
            pad("keep", "2")
        ])));
        document.parts = serde_json::from_value(serde_json::json!([
            {"id":"instance-a","definitionId":"custom","reference":"U1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},
            {"id":"instance-b","definitionId":"custom","reference":"U2","pose":{"at":{"x":1.0,"y":0.0},"rotation":0.0},"side":"front"},
            {"id":"other","definitionId":"other-definition","reference":"R1","pose":{"at":{"x":2.0,"y":0.0},"rotation":0.0},"side":"front"}
        ])).unwrap();
        document.nets = serde_json::from_value(serde_json::json!([
            {"id":"net-a","name":"A","pins":[{"partId":"instance-a","padId":"old"},{"partId":"instance-b","padId":"old"},{"partId":"other","padId":"old"}]},
            {"id":"net-b","name":"B","pins":[{"partId":"instance-a","padId":"keep"}]}
        ])).unwrap();
        let current = snapshot_doc(document);
        let renamed = replacement(scoped_event(
            &current,
            DefinitionEdit::PadId {
                pad_id: "old".into(),
                value: "renamed".into(),
            },
            2,
        ));
        assert_eq!(renamed.definitions[0].pads[0].id, "renamed");
        assert_eq!(renamed.nets[0].pins[0].pad_id, "renamed");
        assert_eq!(renamed.nets[0].pins[1].pad_id, "renamed");
        assert_eq!(renamed.nets[0].pins[2].pad_id, "old");
        assert_eq!(renamed.nets[1].pins[0].pad_id, "keep");

        let after_rename = snapshot_doc(renamed);
        let removed = replacement(scoped_event(
            &after_rename,
            DefinitionEdit::RemovePad {
                pad_id: "renamed".into(),
            },
            3,
        ));
        assert_eq!(
            removed.definitions[0]
                .pads
                .iter()
                .map(|pad| pad.id.as_str())
                .collect::<Vec<_>>(),
            vec!["keep"]
        );
        assert!(
            removed.nets[0]
                .pins
                .iter()
                .all(|pin| pin.pad_id != "renamed" || pin.part_id == "other")
        );
        assert_eq!(removed.nets[1].pins[0].pad_id, "keep");
        assert_eq!(removed.parts.len(), 3);
    }
}
