use boardstudio_web_catalogue::catalogue::{CatalogEntry, catalogue_choices};

pub(super) fn add_object_search_results<'a>(
    entries: &'a [CatalogEntry],
    search: &str,
) -> Vec<&'a CatalogEntry> {
    add_object_choices(entries)
        .into_iter()
        .filter(|entry| entry.matches_library_search(search))
        .collect()
}

pub(super) fn add_object_choices(entries: &[CatalogEntry]) -> Vec<&CatalogEntry> {
    catalogue_choices(entries)
}

pub(super) fn add_object_groups<'a>(
    entries: &[&'a CatalogEntry],
) -> Vec<(&'static str, Vec<&'a CatalogEntry>)> {
    type GroupMatcher = fn(&CatalogEntry) -> bool;
    let groups: [(&str, GroupMatcher); 4] = [
        ("Components", |entry| {
            let name = entry.definition.name.to_lowercase();
            ["power", "reset", "battery"]
                .iter()
                .any(|needle| name.contains(needle))
        }),
        ("Controllers", |entry| {
            matches!(
                entry.definition.kind,
                boardstudio_model::model::PartKind::Controller
            )
        }),
        ("Displays", |entry| {
            let name = entry.definition.name.to_lowercase();
            ["display", "oled", "nice.view"]
                .iter()
                .any(|needle| name.contains(needle))
        }),
        ("Encoders", |entry| {
            matches!(
                entry.definition.kind,
                boardstudio_model::model::PartKind::Encoder
            ) || entry.definition.name.to_lowercase().contains("encoder")
        }),
    ];
    groups
        .into_iter()
        .map(|(label, matches)| {
            (
                label,
                entries
                    .iter()
                    .copied()
                    .filter(|entry| matches(entry))
                    .collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_catalogue::catalogue;
    use std::rc::Rc;

    fn entry(id: &str, name: &str, kind: &str) -> CatalogEntry {
        CatalogEntry {
        definition: Rc::new(
            serde_json::from_value(serde_json::json!({
                "id": id,
                "name": name,
                "kind": kind,
                "courtyard": [{"x": -2.0, "y": -2.0}, {"x": 2.0, "y": -2.0}, {"x": 2.0, "y": 2.0}],
                "pads": []
            }))
            .unwrap(),
        ),
        source: catalogue::CatalogueSource::Imported,
    }
    }

    fn retired_entry() -> CatalogEntry {
        let mut definition =
            (*entry("retired", "Battery legacy controller", "controller").definition).clone();
        definition.generator = Some(boardstudio_model::model::PartGenerator {
            source: "infused-kim/nice_nano_pretty".into(),
            version: "legacy".into(),
            parameters: Default::default(),
        });
        CatalogEntry {
            definition: Rc::new(definition),
            source: catalogue::CatalogueSource::Project,
        }
    }

    fn assembly_snapshot_entry() -> CatalogEntry {
        entry(
            "assembly-keyboard/definition/battery",
            "Battery assembly snapshot",
            "custom",
        )
    }

    #[test]
    fn add_menu_uses_the_four_reference_groups_for_available_definitions() {
        let entries = vec![
            entry("battery", "Battery connector", "connector"),
            entry("controller", "nice!nano", "controller"),
            entry("display", "OLED display", "custom"),
            entry("encoder", "Rotary encoder EC11", "encoder"),
            entry("stabilizer", "MX stabilizer", "custom"),
            retired_entry(),
            assembly_snapshot_entry(),
        ];

        let choices = add_object_choices(&entries);
        assert_eq!(choices.len(), 5);
        assert!(choices.iter().all(|entry| {
            !entry.definition.id.starts_with("assembly-")
                && entry
                    .definition
                    .generator
                    .as_ref()
                    .is_none_or(|generator| generator.source != "infused-kim/nice_nano_pretty")
        }));
        let groups = add_object_groups(&choices);

        assert_eq!(
            groups.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            ["Components", "Controllers", "Displays", "Encoders"]
        );
        let group_ids = |label: &str| {
            groups
                .iter()
                .find(|(group, _)| *group == label)
                .unwrap()
                .1
                .iter()
                .map(|entry| entry.definition.id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(group_ids("Components"), ["battery"]);
        assert_eq!(group_ids("Controllers"), ["controller"]);
        assert_eq!(group_ids("Displays"), ["display"]);
        assert_eq!(group_ids("Encoders"), ["encoder"]);
        let battery_matches = add_object_search_results(&entries, "battery");
        assert_eq!(
            battery_matches
                .iter()
                .map(|entry| entry.definition.id.as_str())
                .collect::<Vec<_>>(),
            ["battery"]
        );
    }
}
