use super::catalogue::{CatalogEntry, CatalogueSource, preferred_label};
use boardstudio_core::model::PartDefinition;
use dioxus::prelude::*;

#[component]
pub(super) fn SelectedDefinition(entry: Option<CatalogEntry>) -> Element {
    let Some(entry) = entry else {
        return rsx! {
            section { class: "m1-parts-details", "aria-label": "Selected component",
                h2 { "Select a component" }
                p { "Choose a footprint from the catalogue to inspect its definition." }
            }
        };
    };
    let definition = entry.definition;
    let category = category_label(&definition);
    let source_name = source_name(entry.source, &definition);
    let source_detail = source_detail(&definition);
    let envelope = envelope_size(&definition);

    rsx! {
        section { class: "m1-parts-details", "aria-label": "Selected component details",
            h2 { "{preferred_label(&definition)}" }
            p { class: "m1-parts-detail-kind", "{category} footprint" }
            dl { class: "m1-parts-detail-list",
                dt { "Definition ID" }
                dd { code { "{definition.id}" } }
                dt { "Catalogue source" }
                dd { "{source_name}" }
                if let Some(source_detail) = source_detail {
                    dt { "Source detail" }
                    dd { "{source_detail}" }
                }
                dt { "Pads" }
                dd { "{definition.pads.len()}" }
                if let Some((width, height)) = envelope {
                    dt { "Courtyard" }
                    dd { "{width:.1} × {height:.1} mm" }
                }
            }
        }
    }
}

fn category_label(definition: &PartDefinition) -> &'static str {
    match &definition.kind {
        boardstudio_core::model::PartKind::Switch => "Switch",
        boardstudio_core::model::PartKind::Controller => "Controller",
        boardstudio_core::model::PartKind::Connector => "Connector",
        boardstudio_core::model::PartKind::Encoder => "Encoder",
        boardstudio_core::model::PartKind::Passive => "Passive",
        boardstudio_core::model::PartKind::Utility => "Utility",
        boardstudio_core::model::PartKind::Custom => "Custom",
    }
}

fn source_name(source: CatalogueSource, definition: &PartDefinition) -> &'static str {
    match source {
        CatalogueSource::Ergogen => "Bundled Ergogen library",
        CatalogueSource::Imported if definition.kicad_source.is_some() => "Imported KiCad library",
        CatalogueSource::Imported => "Imported library",
        CatalogueSource::Project => "Current project",
    }
}

fn source_detail(definition: &PartDefinition) -> Option<String> {
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

fn envelope_size(definition: &PartDefinition) -> Option<(f64, f64)> {
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
