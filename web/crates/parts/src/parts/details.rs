use super::catalogue::{CatalogEntry, preferred_label};
use super::details_logic::{category_label, envelope_size, source_detail, source_name};
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

// Category, source and envelope predicates live in the host-compatible
// `details_logic` module so the same projection runs under `cargo test`.
