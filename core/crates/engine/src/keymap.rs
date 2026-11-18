//! Persisted keymap semantics and deterministic ZMK source. UI never emits DTS.
use crate::model::ProjectDoc;
pub use boardstudio_model::keymap::*;
use std::collections::BTreeMap;

pub(crate) fn apply_edit(
    doc: &mut ProjectDoc,
    change: &KeymapChange,
) -> Result<Vec<String>, String> {
    let mut map = doc.keymap.clone().unwrap_or_default();
    let targets = match change {
        KeymapChange::EditMacro { macro_id, change } => {
            let mac = map
                .macros
                .iter_mut()
                .find(|mac| mac.id == *macro_id)
                .ok_or("Unknown macro")?;
            match change {
                MacroChange::Name { value } => mac.name = value.clone(),
                MacroChange::TapMs { value } => mac.tap_ms = *value,
                MacroChange::WaitMs { value } => mac.wait_ms = *value,
                MacroChange::AddStep { value } => mac.steps.push(value.clone()),
                MacroChange::Step { index, value } => {
                    *mac.steps.get_mut(*index).ok_or("Unknown macro step")? = value.clone()
                }
                MacroChange::RemoveStep { index } => {
                    if *index >= mac.steps.len() {
                        return Err("Unknown macro step".into());
                    }
                    mac.steps.remove(*index);
                }
            }
            vec![doc.id.clone()]
        }
        KeymapChange::Binding {
            layer_id,
            key_id,
            binding,
        } => {
            let id = key_id.strip_suffix("/push").unwrap_or(key_id);
            let part = doc
                .parts
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| format!("Unknown key {key_id}"))?;
            let def = doc
                .definitions
                .iter()
                .find(|d| d.id == part.definition_id)
                .ok_or("Key definition missing")?;
            let push = key_id.ends_with("/push")
                && doc.boards.iter().any(|b| {
                    crate::electrical_peripherals::describe(doc, &b.id)
                        .iter()
                        .any(|p| {
                            (p.kind == "encoder" || p.kind == "press")
                                && p.part_id == id
                                && p.press_key_id.as_deref() == Some(key_id.as_str())
                        })
                });
            let matrix_press = crate::inputs::profile(def).press.is_some()
                && crate::inputs::matrix_member(doc, part);
            if (key_id.ends_with("/push") && !push)
                || (!key_id.ends_with("/push")
                    && def.kind != crate::model::PartKind::Switch
                    && !matrix_press)
            {
                return Err("Keymap bindings require switches or encoder push inputs".into());
            }
            map.layers
                .iter_mut()
                .find(|l| l.id == *layer_id)
                .ok_or("Unknown layer")?
                .bindings
                .insert(key_id.clone(), binding.clone());
            vec![key_id.clone()]
        }
        KeymapChange::AddLayer { id, name } => {
            map.layers.push(KeymapLayer {
                id: id.clone(),
                name: name.clone(),
                bindings: BTreeMap::new(),
                sensors: BTreeMap::new(),
            });
            vec![doc.id.clone()]
        }
        KeymapChange::RenameLayer { id, name } => {
            map.layers
                .iter_mut()
                .find(|l| l.id == *id)
                .ok_or("Unknown layer")?
                .name = name.clone();
            vec![doc.id.clone()]
        }
        KeymapChange::RemoveLayer { id } => {
            if map.layers.first().is_some_and(|l| l.id == *id) {
                return Err("The base layer cannot be removed".into());
            }
            map.layers.retain(|l| l.id != *id);
            vec![doc.id.clone()]
        }
        KeymapChange::SaveMacro { value } => {
            if let Some(m) = map.macros.iter_mut().find(|m| m.id == value.id) {
                *m = value.clone()
            } else {
                map.macros.push(value.clone())
            }
            vec![doc.id.clone()]
        }
        KeymapChange::RemoveMacro { id } => {
            map.macros.retain(|m| m.id != *id);
            vec![doc.id.clone()]
        }
        KeymapChange::Encoder {
            layer_id,
            encoder_id,
            direction,
            binding,
        } => {
            if !doc.boards.iter().any(|b| {
                crate::electrical_peripherals::describe(doc, &b.id)
                    .iter()
                    .any(|p| p.kind == "encoder" && p.part_id == *encoder_id)
            }) {
                return Err("Unknown physical encoder".into());
            }
            let value = map
                .layers
                .iter_mut()
                .find(|l| l.id == *layer_id)
                .ok_or("Unknown layer")?
                .sensors
                .entry(encoder_id.clone())
                .or_insert(EncoderBinding {
                    clockwise: KeyBinding::None,
                    counterclockwise: KeyBinding::None,
                });
            match direction {
                EncoderDirection::Clockwise => value.clockwise = binding.clone(),
                EncoderDirection::Counterclockwise => value.counterclockwise = binding.clone(),
            }
            vec![encoder_id.clone()]
        }
    };
    validate(&map)?;
    doc.keymap = Some(map);
    Ok(targets)
}
