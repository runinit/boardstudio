use std::collections::BTreeMap;

use boardstudio_core::model::PcbModel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentModelSource {
    Layout,
    Physical,
    Parts,
}

/// Select component-layer models from the active viewer source. Layout never
/// borrows physical-preview models when its own current projection is empty.
pub fn component_models_for_source<'a>(
    source: ComponentModelSource,
    layout_models: Option<&'a [PcbModel]>,
    physical_models: Option<&'a [PcbModel]>,
    parts_models: Option<&'a [PcbModel]>,
) -> Option<&'a [PcbModel]> {
    match source {
        ComponentModelSource::Layout => layout_models,
        ComponentModelSource::Physical => physical_models,
        ComponentModelSource::Parts => parts_models,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaseDisplay {
    pub hidden: Vec<String>,
    pub colors: BTreeMap<String, String>,
}

impl CaseDisplay {
    pub fn color(&self, id: &str) -> Option<&str> {
        preference_ids(id)
            .first()
            .and_then(|id| self.colors.get(id))
            .map(String::as_str)
    }
    pub fn has_color(&self, id: &str) -> bool {
        preference_ids(id)
            .iter()
            .any(|alias| self.colors.contains_key(alias))
    }
    pub fn set_color(&mut self, id: &str, value: &str) {
        for id in preference_ids(id) {
            if value.is_empty() {
                self.colors.remove(&id);
            } else {
                self.colors.insert(id, value.to_owned());
            }
        }
    }
}

pub fn preference_ids(id: &str) -> Vec<String> {
    if id == "gaskets" {
        vec!["Gaskets".to_owned()]
    } else if id == "pcb" {
        ["PCB", "Models", "Keycaps", "Copper", "Mask", "Silkscreen"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    } else if id.starts_with("gasket:") {
        let stem = id
            .strip_suffix(":upper")
            .or_else(|| id.strip_suffix(":lower"))
            .unwrap_or(id);
        vec![format!("{stem}:lower"), format!("{stem}:upper")]
    } else {
        vec![id.to_owned()]
    }
}

pub fn is_visible(display: &CaseDisplay, id: &str) -> bool {
    !preference_ids(id)
        .iter()
        .all(|alias| display.hidden.contains(alias))
}

pub fn is_inspector_visible(display: &CaseDisplay, id: &str) -> bool {
    !preference_ids(id)
        .iter()
        .any(|alias| display.hidden.contains(alias))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str) -> PcbModel {
        use boardstudio_core::model::{Pose2, Side, Vec2, Vec3};
        PcbModel {
            id: id.to_owned(),
            reference: "SW1".to_owned(),
            path: "models/SW1.step".to_owned(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            offset: Vec3::default(),
            rotation: Vec3::default(),
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }

    #[test]
    fn parts_component_layers_select_only_current_sample_models() {
        let layout_models = [model("layout-switch")];
        let sample_models = [
            model("parts-sample-0-model-0"),
            model("parts-sample-0-model-1"),
        ];
        let physical_models = [model("case-switch")];

        let selected = component_models_for_source(
            ComponentModelSource::Parts,
            None,
            None,
            Some(&sample_models),
        );

        assert_eq!(
            selected
                .unwrap()
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["parts-sample-0-model-0", "parts-sample-0-model-1"]
        );
        assert!(
            component_models_for_source(
                ComponentModelSource::Parts,
                None,
                Some(&physical_models),
                None,
            )
            .is_none(),
            "Parts must not fall back to physical Case models when its sample rows are absent"
        );
        assert_eq!(
            component_models_for_source(
                ComponentModelSource::Layout,
                None,
                Some(&physical_models),
                Some(&sample_models),
            ),
            None,
            "an active Layout source must not fall back to physical or Parts models"
        );
        assert_eq!(
            component_models_for_source(
                ComponentModelSource::Layout,
                Some(&layout_models),
                Some(&physical_models),
                Some(&sample_models),
            )
            .unwrap()[0]
                .id,
            "layout-switch"
        );
        assert_eq!(
            component_models_for_source(
                ComponentModelSource::Physical,
                Some(&layout_models),
                Some(&physical_models),
                Some(&sample_models),
            )
            .unwrap()[0]
                .id,
            "case-switch"
        );
    }

    #[test]
    fn pcb_color_reads_reference_alias_and_changes_or_resets_all_aliases() {
        let mut display = CaseDisplay::default();
        display.colors.insert("PCB".into(), "#123456".into());
        display.colors.insert("other-body".into(), "#abcdef".into());
        assert_eq!(display.color("pcb"), Some("#123456"));
        display.set_color("pcb", "#654321");
        for id in preference_ids("pcb") {
            assert_eq!(display.colors.get(&id).map(String::as_str), Some("#654321"));
        }
        display.set_color("pcb", "");
        for id in preference_ids("pcb") {
            assert!(!display.colors.contains_key(&id));
        }
        assert_eq!(
            display.colors.get("other-body").map(String::as_str),
            Some("#abcdef")
        );
    }
    #[test]
    fn partial_reference_alias_colors_remain_resettable() {
        let mut display = CaseDisplay::default();
        display.colors.insert("Models".into(), "#123456".into());
        display
            .colors
            .insert("gasket:pair:upper".into(), "#112233".into());
        assert!(display.has_color("pcb"));
        assert!(display.has_color("gasket:pair:lower"));
        display.set_color("pcb", "");
        display.set_color("gasket:pair:lower", "");
        assert!(!display.has_color("pcb"));
        assert!(!display.has_color("gasket:pair:upper"));
    }
    #[test]
    fn gasket_upper_lower_share_color_without_changing_visibility() {
        let mut display = CaseDisplay {
            hidden: vec!["PCB".into()],
            colors: BTreeMap::new(),
        };
        display.set_color("gasket:pair:upper", "#112233");
        assert_eq!(display.color("gasket:pair:lower"), Some("#112233"));
        display.set_color("gasket:pair:lower", "");
        assert!(display.colors.is_empty());
        assert_eq!(display.hidden, vec!["PCB"]);
    }

    #[test]
    fn inspector_visibility_requires_every_alias_to_be_visible_while_tree_uses_all_hidden() {
        let display = CaseDisplay {
            hidden: vec!["PCB".into()],
            ..CaseDisplay::default()
        };

        assert!(is_visible(&display, "pcb"));
        assert!(!is_inspector_visible(&display, "pcb"));
    }
}
