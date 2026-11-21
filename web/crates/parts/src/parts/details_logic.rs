//! Host-compatible selected-definition projection for the Parts workspace.
//!
//! The production details panel (wasm) delegates to this module so the same
//! category/source/envelope predicates run under `cargo test` without a browser.

use boardstudio_model::model::PartDefinition;
use boardstudio_web_catalogue::catalogue::CatalogueSource;

pub(crate) fn category_label(definition: &PartDefinition) -> &'static str {
    match &definition.kind {
        boardstudio_model::model::PartKind::Switch => "Switch",
        boardstudio_model::model::PartKind::Controller => "Controller",
        boardstudio_model::model::PartKind::Connector => "Connector",
        boardstudio_model::model::PartKind::Encoder => "Encoder",
        boardstudio_model::model::PartKind::Passive => "Passive",
        boardstudio_model::model::PartKind::Utility => "Utility",
        boardstudio_model::model::PartKind::Custom => "Custom",
    }
}

pub(crate) fn source_name(source: CatalogueSource, definition: &PartDefinition) -> &'static str {
    match source {
        CatalogueSource::Generator => "Bundled footprint library",
        CatalogueSource::Imported if definition.kicad_source.is_some() => "Imported KiCad library",
        CatalogueSource::Imported => "Imported library",
        CatalogueSource::Project => "Current project",
    }
}

pub(crate) fn source_detail(definition: &PartDefinition) -> Option<String> {
    if let Some(generator) = &definition.generator {
        Some(format!("{} · {}", generator.source, generator.version))
    } else if definition.kicad_source.is_some() {
        Some(format!(
            "KiCad format v{}",
            definition.kicad_source.as_ref()?.format_version
        ))
    } else {
        None
    }
}

pub(crate) fn envelope_size(definition: &PartDefinition) -> Option<(f64, f64)> {
    let min_x = definition
        .courtyard
        .iter()
        .map(|point| point.x)
        .reduce(f64::min)?;
    let max_x = definition
        .courtyard
        .iter()
        .map(|point| point.x)
        .reduce(f64::max)?;
    let min_y = definition
        .courtyard
        .iter()
        .map(|point| point.y)
        .reduce(f64::min)?;
    let max_y = definition
        .courtyard
        .iter()
        .map(|point| point.y)
        .reduce(f64::max)?;
    Some((max_x - min_x, max_y - min_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(kind: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "fixture", "name": "Fixture", "kind": kind,
            "courtyard": [{"x": -8.0, "y": -4.0}, {"x": 8.0, "y": 4.0}],
            "pads": []
        }))
        .unwrap()
    }

    #[test]
    fn selected_definition_labels_follow_kind_and_source() {
        assert_eq!(category_label(&definition("switch")), "Switch");
        assert_eq!(category_label(&definition("controller")), "Controller");
        assert_eq!(category_label(&definition("connector")), "Connector");
        assert_eq!(category_label(&definition("encoder")), "Encoder");
        assert_eq!(category_label(&definition("custom")), "Custom");

        let plain = definition("custom");
        assert_eq!(
            source_name(CatalogueSource::Generator, &plain),
            "Bundled footprint library"
        );
        assert_eq!(
            source_name(CatalogueSource::Imported, &plain),
            "Imported library"
        );
        assert_eq!(
            source_name(CatalogueSource::Project, &plain),
            "Current project"
        );
        assert_eq!(source_detail(&plain), None);

        let generated = serde_json::from_value::<PartDefinition>(serde_json::json!({
            "id": "fixture", "name": "Fixture", "kind": "custom",
            "courtyard": [], "pads": [],
            "generator": {"source": "ceoloide/switch_mx", "version": "bundled-1", "parameters": {}}
        }))
        .unwrap();
        assert_eq!(
            source_detail(&generated).as_deref(),
            Some("ceoloide/switch_mx · bundled-1")
        );
    }

    #[test]
    fn selected_definition_envelope_spans_the_courtyard() {
        assert_eq!(envelope_size(&definition("custom")), Some((16.0, 8.0)));
        let empty = serde_json::from_value::<PartDefinition>(serde_json::json!({
            "id": "fixture", "name": "Fixture", "kind": "custom",
            "courtyard": [], "pads": []
        }))
        .unwrap();
        assert_eq!(envelope_size(&empty), None);
    }
}
