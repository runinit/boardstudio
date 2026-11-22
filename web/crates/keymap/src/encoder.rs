//! Pure encoder input projection.
use crate::binding::{BindingLayerChoice, BindingMacroChoice};
use boardstudio_core_mechanical::electrical::ElectricalPlan;
use boardstudio_model::model::{KeyBinding, ProjectDoc};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderInputChoice {
    pub id: Rc<str>,
    pub label: Rc<str>,
    pub push_key_id: Option<Rc<str>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderInputProjection {
    pub encoders: Rc<[EncoderInputChoice]>,
    pub electrical_plan_settled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderBindingRow {
    pub id: Rc<str>,
    pub label: Rc<str>,
    pub clockwise: KeyBinding,
    pub counterclockwise: KeyBinding,
    pub push_key_id: Option<Rc<str>>,
    pub push_binding: Option<KeyBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderEditorProjection {
    pub effective_layer_id: String,
    pub rows: Rc<[EncoderBindingRow]>,
    pub layers: Rc<[BindingLayerChoice]>,
    pub macros: Rc<[BindingMacroChoice]>,
}

pub(crate) fn encoder_choices(
    doc: &ProjectDoc,
    board_id: &str,
    plan: Option<&ElectricalPlan>,
) -> Vec<EncoderInputChoice> {
    let mut rows: Vec<_> = plan
        .into_iter()
        .flat_map(|p| p.peripherals.iter())
        .filter(|r| r.kind == "encoder" && r.rotary.is_some())
        .map(|r| EncoderInputChoice {
            id: Rc::from(r.part_id.as_str()),
            label: Rc::from(
                doc.parts
                    .iter()
                    .find(|p| p.id == r.part_id)
                    .map_or(r.part_id.as_str(), |p| p.reference.as_str()),
            ),
            push_key_id: r.press_key_id.as_deref().map(Rc::from),
        })
        .collect();
    for m in doc
        .modules
        .iter()
        .filter(|m| m.host_board_id == board_id && !m.detached)
    {
        let Some(def) = doc
            .module_definitions
            .iter()
            .find(|d| d.id == m.definition_id)
        else {
            continue;
        };
        if def.catalogue_row.as_deref() == Some("ec11-evqwgd001")
            && def.electrical.rotary_profile.is_some()
            && !rows.iter().any(|r| r.id.as_ref() == m.id)
        {
            rows.push(EncoderInputChoice {
                id: Rc::from(m.id.as_str()),
                label: Rc::from(def.name.as_str()),
                push_key_id: None,
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document_has_no_encoders() {
        let doc = ProjectDoc::empty("d", "P");
        assert!(encoder_choices(&doc, "b1", None).is_empty());
    }
}
