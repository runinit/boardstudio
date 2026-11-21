//! Pure projections for Case assembly layer controls.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerAvailability {
    Available,
    Pending,
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseAssemblyLayer {
    pub id: String,
    pub label: String,
    pub availability: LayerAvailability,
}

pub fn standard_assembly_layers(
    generated: impl IntoIterator<Item = (String, String)>,
) -> Vec<CaseAssemblyLayer> {
    let mut layers = [
        ("PCB", "PCB"),
        ("Copper", "Copper"),
        ("Mask", "Mask openings"),
        ("Silkscreen", "Silkscreen"),
        ("Models", "Models"),
        ("Keycaps", "Keycaps"),
    ]
    .into_iter()
    .map(|(id, label)| CaseAssemblyLayer {
        id: id.to_owned(),
        label: label.to_owned(),
        availability: LayerAvailability::Available,
    })
    .collect::<Vec<_>>();
    for (id, label) in generated {
        if id != "pcb" && !layers.iter().any(|layer| layer.id == id) {
            let label = assembly_layer_label(&id, &label);
            layers.push(CaseAssemblyLayer {
                id,
                label,
                availability: LayerAvailability::Available,
            });
        }
    }
    layers
}

/// Keep configured mechanical stack entries visible even when the current CAD
/// result has no corresponding body. Only generated bodies can be toggled.
pub fn assembly_layers_with_stack(
    generated: impl IntoIterator<Item = (String, String)>,
    configured_stack_ids: impl IntoIterator<Item = String>,
) -> Vec<CaseAssemblyLayer> {
    let generated = generated.into_iter().collect::<Vec<_>>();
    let generated_ids = generated
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let mut layers = standard_assembly_layers(generated);

    for id in configured_stack_ids {
        let row_id = if id == "pcb" { "PCB" } else { id.as_str() };
        if layers.iter().any(|layer| layer.id == row_id) {
            continue;
        }
        let label = assembly_layer_label(&id, &id);
        layers.push(CaseAssemblyLayer {
            id: row_id.to_owned(),
            label,
            availability: if generated_ids.contains(&id) || id == "pcb" {
                LayerAvailability::Available
            } else {
                LayerAvailability::Unavailable("Not generated".to_owned())
            },
        });
    }
    layers
}

fn assembly_layer_label(id: &str, fallback: &str) -> String {
    match id {
        "plate" => "Plate".to_owned(),
        "plate-foam" => "Plate foam".to_owned(),
        "bottom-foam" => "Bottom foam".to_owned(),
        "bottom" => "Bottom".to_owned(),
        _ => fallback.to_owned(),
    }
}

pub(crate) fn case_display_selection_id(id: &str, selectable_layers: &[String]) -> Option<String> {
    let id = match id {
        "PCB" => "pcb",
        "Gaskets" => "gaskets",
        id => id,
    };
    selectable_layers
        .iter()
        .any(|selectable| selectable == id)
        .then(|| id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_controls_keep_distinct_visibility_ids_and_generated_rows() {
        let layers = standard_assembly_layers([
            ("pcb".into(), "legacy PCB alias".into()),
            ("plate:lower".into(), "Plate".into()),
        ]);
        assert_eq!(layers.len(), 7);
        assert_eq!(layers[0].id, "PCB");
        assert_eq!(layers[1].id, "Copper");
        assert_eq!(layers[2].label, "Mask openings");
        assert_eq!(layers.last().unwrap().id, "plate:lower");
    }

    #[test]
    fn configured_stack_keeps_unmeshed_battery_unavailable_and_generated_plate_available() {
        let layers = assembly_layers_with_stack(
            [("plate".into(), "Plate body".into())],
            ["battery".into(), "plate".into()],
        );

        let battery = layers.iter().find(|layer| layer.id == "battery").unwrap();
        assert_eq!(battery.label, "battery");
        assert_eq!(
            battery.availability,
            LayerAvailability::Unavailable("Not generated".into())
        );
        let plate = layers.iter().find(|layer| layer.id == "plate").unwrap();
        assert_eq!(plate.label, "Plate");
        assert_eq!(plate.availability, LayerAvailability::Available);
    }

    #[test]
    fn display_selection_normalizes_pcb_and_only_exposes_supported_layers() {
        let selectable = vec!["pcb".to_owned(), "plate".to_owned()];
        assert_eq!(
            case_display_selection_id("PCB", &selectable),
            Some("pcb".into())
        );
        assert_eq!(
            case_display_selection_id("plate", &selectable),
            Some("plate".into())
        );
        assert_eq!(case_display_selection_id("Copper", &selectable), None);
    }
}
