use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CaseDisplay {
    pub(crate) hidden: Vec<String>,
    pub(crate) colors: BTreeMap<String, String>,
}

impl CaseDisplay {
    pub(crate) fn color(&self, id: &str) -> Option<&str> {
        preference_ids(id)
            .first()
            .and_then(|id| self.colors.get(id))
            .map(String::as_str)
    }
    pub(crate) fn has_color(&self, id: &str) -> bool {
        preference_ids(id)
            .iter()
            .any(|alias| self.colors.contains_key(alias))
    }
    pub(crate) fn set_color(&mut self, id: &str, value: &str) {
        for id in preference_ids(id) {
            if value.is_empty() {
                self.colors.remove(&id);
            } else {
                self.colors.insert(id, value.to_owned());
            }
        }
    }
}

pub(crate) fn preference_ids(id: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
