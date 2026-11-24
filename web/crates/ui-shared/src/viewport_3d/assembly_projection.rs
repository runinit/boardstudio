//! Pure projections for Case assembly layer controls.

use boardstudio_model::model::PcbModel;
use boardstudio_web_runtime::model_delivery::ModelDeliveryRows;

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
    let mut layers = standard_assembly_layers(generated);

    for id in configured_stack_ids {
        if id == "pcb" || layers.iter().any(|layer| layer.id == id) {
            continue;
        }
        let label = assembly_layer_label(&id, &id);
        layers.push(CaseAssemblyLayer {
            id,
            label,
            availability: LayerAvailability::Unavailable("Not generated".to_owned()),
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

pub type CaseComponentLayer = CaseAssemblyLayer;

/// Project the accepted preview rows into layer controls without changing their
/// order or substituting references/asset paths for renderer model IDs.
pub fn physical_component_layers(
    models: &[PcbModel],
    delivery: Option<&ModelDeliveryRows>,
) -> Vec<CaseComponentLayer> {
    models
        .iter()
        .map(|model| {
            let availability = match delivery {
                Some(rows) if rows.delivered.iter().any(|row| row.id == model.id) => {
                    LayerAvailability::Available
                }
                Some(rows) if rows.pending.iter().any(|id| id == &model.id) => {
                    LayerAvailability::Pending
                }
                Some(_) => LayerAvailability::Unavailable("Model is not available".to_owned()),
                None => LayerAvailability::Pending,
            };
            let filename = model.path.rsplit('/').next().unwrap_or(&model.path);
            CaseComponentLayer {
                id: model.id.clone(),
                label: format!("{} · {filename}", model.reference),
                availability,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use boardstudio_model::model::{Pose2, Side, Vec2, Vec3};
    use boardstudio_web_runtime::model_delivery::{DeliveredModel, ValidatedMesh};
    use std::rc::Rc;

    fn model(id: &str, reference: &str, path: &str) -> PcbModel {
        PcbModel {
            id: id.into(),
            reference: reference.into(),
            path: path.into(),
            pose: Pose2 {
                at: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.25,
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
    fn physical_rows_keep_preview_order_and_exact_model_id_join() {
        let preview_models = vec![
            model("mesh-id-switch", "SW1", "models/switch.step"),
            model("mesh-id-diode", "D1", "models/diode.step"),
        ];
        let delivered = ModelDeliveryRows {
            delivered: vec![DeliveredModel {
                id: "mesh-id-diode".into(),
                mesh: Rc::new(ValidatedMesh {
                    positions: Rc::from([0.0_f32; 9]),
                    normals: Rc::from([0.0_f32; 9]),
                    colors: None,
                    step_bounds: None,
                    vendor_step: None,
                }),
                matrix: None,
            }],
            pending: vec!["mesh-id-switch".into()],
            failures: Vec::new(),
        };

        let rows = physical_component_layers(&preview_models, Some(&delivered));

        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["mesh-id-switch", "mesh-id-diode"]
        );
        assert_eq!(rows[0].label, "SW1 · switch.step");
        assert_eq!(rows[0].availability, LayerAvailability::Pending);
        assert_eq!(rows[1].label, "D1 · diode.step");
        assert_eq!(rows[1].availability, LayerAvailability::Available);
    }

    #[test]
    fn missing_delivery_never_marks_a_preview_model_available() {
        let models = [model("mesh-id", "SW1", "assets/switch.step")];
        let rows = physical_component_layers(&models, None);
        assert_eq!(rows[0].availability, LayerAvailability::Pending);
    }

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
