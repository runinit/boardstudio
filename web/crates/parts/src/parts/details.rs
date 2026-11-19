use super::catalogue::{CatalogEntry, CatalogueSource, preferred_label};
use boardstudio_core::model::PartDefinition;
use dioxus::prelude::*;

#[component]
pub fn SelectedDefinition(entry: Option<CatalogEntry>) -> Element {
    let Some(entry) = entry else {
        return rsx! {
            section { "aria-label": "Selected component",
                h2 { class: "m-0 [font-size:18px]", "Select a component" }
                p { class: "text-muted-foreground [font-size:14px]", "Choose a footprint from the catalogue to inspect its definition." }
            }
        };
    };
    let definition = entry.definition;
    let category = category_label(&definition);
    let source_name = source_name(entry.source, &definition);
    let source_detail = source_detail(&definition);
    let envelope = envelope_size(&definition);

    rsx! {
        section { "aria-label": "Selected component details",
            h2 { class: "m-0 [font-size:18px]", "{preferred_label(&definition)}" }
            p { class: "text-muted-foreground [font-size:14px]", "{category} footprint" }
            dl { class: "grid grid-cols-[minmax(90px,0.8fr)_minmax(0,1.2fr)] gap-x-3 gap-y-2 mx-0 mt-[18px] mb-0 pt-[14px] border-t border-border [font-size:14px] leading-[1.45] [&_dt]:text-muted-foreground [&_dd]:min-w-0 [&_dd]:m-0 [&_dd]:[overflow-wrap:anywhere] [&_dd]:text-foreground [&_code]:font-mono",
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
        CatalogueSource::Generator => "Bundled footprint library",
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

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_style_tests {
    use super::*;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use std::rc::Rc;
    use wasm_bindgen_test::*;

    fn fixture() -> Element {
        let definition = serde_json::from_value(serde_json::json!({
            "id": "fixture-switch", "name": "Fixture switch", "kind": "switch",
            "courtyard": [], "pads": []
        }))
        .unwrap();
        let entry = CatalogEntry {
            definition: Rc::new(definition),
            source: CatalogueSource::Project,
        };
        rsx! {
            SelectedDefinition { entry: None }
            SelectedDefinition { entry: Some(entry) }
        }
    }

    #[wasm_bindgen_test]
    async fn component_details_keep_their_layout_without_legacy_selectors() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
        wait_until("component details mount", || {
            root.query_selector("[aria-label='Selected component details']")
                .unwrap()
                .is_some()
        })
        .await;
        for name in ["Selected component", "Selected component details"] {
            let region = root
                .query_selector(&format!("[aria-label='{name}']"))
                .unwrap()
                .unwrap();
            let heading = region.query_selector("h2").unwrap().unwrap();
            let classes = heading.get_attribute("class").unwrap_or_default();
            for expected in ["m-0", "[font-size:18px]"] {
                assert!(
                    classes.split_whitespace().any(|class| class == expected),
                    "{name} heading: missing {expected}"
                );
            }
            let description = region.query_selector("p").unwrap().unwrap();
            let classes = description.get_attribute("class").unwrap_or_default();
            for expected in ["text-muted-foreground", "[font-size:14px]"] {
                assert!(
                    classes.split_whitespace().any(|class| class == expected),
                    "{name} description: missing {expected}"
                );
            }
        }
        let list = root
            .query_selector("[aria-label='Selected component details'] dl")
            .unwrap()
            .unwrap();
        let classes = list.get_attribute("class").unwrap_or_default();
        for expected in [
            "grid",
            "grid-cols-[minmax(90px,0.8fr)_minmax(0,1.2fr)]",
            "gap-x-3",
            "gap-y-2",
            "mt-[18px]",
            "pt-[14px]",
            "border-t",
            "border-border",
            "[font-size:14px]",
            "leading-[1.45]",
            "[&_dt]:text-muted-foreground",
            "[&_dd]:[overflow-wrap:anywhere]",
            "[&_code]:font-mono",
        ] {
            assert!(
                classes.split_whitespace().any(|class| class == expected),
                "detail list: missing {expected}"
            );
        }
        assert!(
            list.text_content()
                .unwrap_or_default()
                .contains("fixture-switch")
        );
        root.remove();
    }
}
