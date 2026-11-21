//! Pure mechanical layer presentation projection.
use boardstudio_model::model::MechanicalAssembly;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalLayerRow {
    pub id: String,
    pub label: String,
    pub z: f64,
    pub thickness: f64,
    /// Present only when the displayed completed Case assembly contains a body with this ID.
    pub resolved_body_thickness: Option<f64>,
    /// The row is retained from the last completed same-owner assembly while
    /// the accepted physical inputs have changed.
    pub is_previous: bool,
}

pub(crate) fn project_layer_rows(
    display_assembly: Option<&MechanicalAssembly>,
    current_assembly: Option<&MechanicalAssembly>,
    internal_gasket: bool,
    is_previous: bool,
) -> Rc<[MechanicalLayerRow]> {
    // Current assemblies have already passed the exact scope/token/revision admission above.
    // Prefer their rows so the settings view can show the accepted resolved stack before the
    // viewport publishes a current scene; only fall back to display geometry as explicitly
    // previous rows when no current assembly is available.
    let (assembly, rows_are_previous) = current_assembly
        .map(|assembly| (Some(assembly), false))
        .unwrap_or((display_assembly, is_previous));
    let layers: Vec<_> = assembly
        .into_iter()
        .flat_map(|assembly| assembly.stack.iter().map(move |layer| (assembly, layer)))
        .map(|(assembly, layer)| MechanicalLayerRow {
            id: layer.id.clone(),
            label: mechanical_layer_label(&layer.id, internal_gasket),
            z: layer.z,
            thickness: layer.thickness,
            resolved_body_thickness: assembly
                .case
                .bodies
                .iter()
                .find(|body| body.body.id == layer.id)
                .map(|body| body.body.thickness),
            is_previous: rows_are_previous,
        })
        .collect();
    Rc::from(layers)
}

pub(crate) fn mechanical_layer_label(id: &str, internal_gasket: bool) -> String {
    if id == "retainer" && internal_gasket {
        return "Top case".into();
    }
    if id == "pcb" {
        return "PCB".into();
    }
    let label = id.replace('-', " ");
    let mut chars = label.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assembly_with_plate(thickness: f64) -> MechanicalAssembly {
        serde_json::from_value(serde_json::json!({
            "revision": 1,
            "case": { "revision": 1, "bodies": [] },
            "stack": [{ "id": "plate", "z": 3.5, "thickness": thickness }],
            "suggestedMounts": [],
            "nominalPlateContours": [],
            "plateContours": [],
            "diagnostics": []
        }))
        .expect("valid minimal mechanical assembly")
    }

    #[test]
    fn current_resolution_layers_precede_previous_display_layers() {
        let previous = assembly_with_plate(1.5);
        let current = assembly_with_plate(2.0);
        let rows = project_layer_rows(Some(&previous), Some(&current), false, true);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].thickness, 2.0);
        assert!(!rows[0].is_previous);

        let fallback = project_layer_rows(Some(&previous), None, false, true);
        assert_eq!(fallback.len(), 1);
        assert_eq!(fallback[0].thickness, 1.5);
        assert!(fallback[0].is_previous);
    }
}
