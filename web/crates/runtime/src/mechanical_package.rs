//! Native-testable formatting for mechanical export package files.

use boardstudio_core::model::{MechanicalAssembly, ProjectDoc};
use std::collections::BTreeSet;

pub(crate) fn push_mechanical_file(
    files: &mut Vec<(String, Vec<u8>)>,
    paths: &mut BTreeSet<String>,
    path: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!(
            "Mechanical exporter produced an unsafe path: {path}"
        ));
    }
    if !paths.insert(path.clone()) {
        return Err(format!(
            "Mechanical exporter produced a duplicate path: {path}"
        ));
    }
    files.push((path, bytes));
    Ok(())
}

pub(crate) fn mechanical_filename_component(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_replacement = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
            result.push(character);
            in_replacement = false;
        } else if !in_replacement {
            result.push('-');
            in_replacement = true;
        }
    }
    result
}

pub(crate) fn mechanical_stl(
    mesh: &boardstudio_web_host::cad_jobs::CadMesh,
) -> Result<Vec<u8>, String> {
    if !mesh.positions.len().is_multiple_of(9) || mesh.positions.len() != mesh.normals.len() {
        return Err("CAD returned incomplete mechanical STL triangles.".into());
    }
    let triangles = mesh.positions.len() / 9;
    let triangle_count = u32::try_from(triangles)
        .map_err(|_| "Mechanical STL exceeds the supported triangle count.".to_owned())?;
    let byte_length = 84usize
        .checked_add(
            triangles
                .checked_mul(50)
                .ok_or_else(|| "Mechanical STL is too large.".to_owned())?,
        )
        .ok_or_else(|| "Mechanical STL is too large.".to_owned())?;
    let mut bytes = vec![0u8; byte_length];
    const HEADER: &[u8] = b"Board Studio mechanical part; coordinates in millimetres";
    bytes[..HEADER.len()].copy_from_slice(HEADER);
    bytes[80..84].copy_from_slice(&triangle_count.to_le_bytes());
    for triangle in 0..triangles {
        let offset = 84 + triangle * 50;
        for component in 0..3 {
            let value = mesh.normals[triangle * 9 + component];
            bytes[offset + component * 4..offset + component * 4 + 4]
                .copy_from_slice(&value.to_le_bytes());
        }
        for component in 0..9 {
            let value = mesh.positions[triangle * 9 + component];
            let index = offset + 12 + component * 4;
            bytes[index..index + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(bytes)
}

pub(crate) fn mechanical_xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn serialized_enum_label<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub(crate) fn mechanical_fabrication_notes(
    document: &ProjectDoc,
    assembly: &MechanicalAssembly,
) -> String {
    let Some(configuration) = document.mechanical.as_ref() else {
        return String::new();
    };
    let mut hardware = configuration.hardware.clone().unwrap_or_default();
    hardware.extend(assembly.generated_hardware.iter().cloned());
    let mounts = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "plate")
        .and_then(|body| body.body.mounts.as_ref())
        .cloned()
        .unwrap_or_default();
    let method = serialized_enum_label(&configuration.method);
    let mount = serialized_enum_label(&configuration.mount);
    let plate_to_pcb = assembly
        .stack
        .iter()
        .find(|layer| layer.id == "plate")
        .map_or(configuration.plate_to_pcb, |layer| layer.z);
    let material_note = if method == "pcb-fr4" {
        "Plate substrate: FR4, no copper or plated holes. Confirm grade, finish and thickness tolerance with the fabricator."
    } else {
        "Material grade, finish and mechanical properties must be selected with the fabricator; no material grade is inferred from the process choice."
    };
    let mount_notes = if mounts.is_empty() {
        "none".to_owned()
    } else {
        mounts
            .iter()
            .map(|item| {
                format!(
                    "{}: diameter {} mm at ({}, {}) mm",
                    item.id, item.hole_diameter, item.at.x, item.at.y
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let hardware_notes = if hardware.is_empty() {
        "- No hardware specifications recorded.".to_owned()
    } else {
        hardware
            .iter()
            .map(|item| {
                format!(
                    "- {}: {} × {}; thread {}; length {} mm; part {}, mount {}{}{}",
                    item.id,
                    item.quantity,
                    item.designation,
                    item.thread,
                    item.length,
                    item.part_id,
                    item.feature_id,
                    item.tolerance
                        .as_ref()
                        .map(|value| format!("; tolerance {value}"))
                        .unwrap_or_default(),
                    item.notes
                        .as_ref()
                        .map(|value| format!("; {value}"))
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let processes = configuration
        .part_processes
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|part| {
                    format!(
                        "- {}: {}; material {}; finished thickness {} mm; constraint set {}",
                        part.part_id,
                        serialized_enum_label(&part.method),
                        part.material,
                        part.thickness,
                        part.constraints_version
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "- No per-part overrides.".into());
    let gasket_note = if mount == "gasket" {
        let thickness = configuration
            .gasket_layout
            .as_ref()
            .map_or(2.0, |layout| layout.thickness);
        let compression = configuration
            .gasket_layout
            .as_ref()
            .map_or(0.15, |layout| layout.compression);
        format!(
            "\nGasket stock: EVA, {thickness} mm uncompressed; {}% nominal assembly compression. Exported assembly strips depict compressed thickness; cut the strip outlines from the specified uncompressed stock.\n",
            100.0 * compression
        )
    } else {
        String::new()
    };
    let critical_fits = configuration
        .critical_fits
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|fit| {
                    let length = (fit.to.x - fit.from.x).hypot(fit.to.y - fit.from.y);
                    format!(
                        "- {}: {} — {}: {:.3} mm, {}; from ({}, {}) to ({}, {}) mm.",
                        fit.id,
                        fit.part_id,
                        fit.label,
                        length,
                        fit.tolerance,
                        fit.from.x,
                        fit.from.y,
                        fit.to.x,
                        fit.to.y
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "- No critical dimension annotations recorded.".into());
    let openings = assembly
        .nominal_plate_contours
        .iter()
        .filter(|contour| contour.hole)
        .enumerate()
        .map(|(index, contour)| {
            let min_x = contour.points.iter().map(|point| point.x).fold(f64::INFINITY, f64::min);
            let max_x = contour.points.iter().map(|point| point.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = contour.points.iter().map(|point| point.y).fold(f64::INFINITY, f64::min);
            let max_y = contour.points.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max);
            format!(
                "- Opening {}: {:.3} × {:.3} mm; review the full contour for corner radii and retention tabs.",
                index + 1,
                max_x - min_x,
                max_y - min_y
            )
        })
        .collect::<Vec<_>>();
    let openings = if openings.is_empty() {
        "- No profile openings.".to_owned()
    } else {
        openings.join("\n")
    };
    let profiles = if configuration.profiles.is_empty() {
        "- No mechanical profiles configured.".to_owned()
    } else {
        configuration
            .profiles
            .iter()
            .map(|profile| format!("- {}: {}", profile.definition_id, profile.source))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let diagnostics = if assembly.diagnostics.is_empty() {
        "- No resolver diagnostics.".to_owned()
    } else {
        assembly
            .diagnostics
            .iter()
            .map(|finding| {
                format!(
                    "- {}: {}",
                    serialized_enum_label(&finding.severity),
                    finding.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# Mechanical fabrication specification\n\nRevision: {}\nProcess: {method}\nMount system: {mount}\nPlate finished thickness: {} mm\nPCB nominal thickness: {} mm\nPlate-to-PCB distance: {plate_to_pcb} mm\nPlate foam thickness: {} mm\nBottom foam thickness: {} mm\nWall thickness: {} mm\n\nThe assembled STEP includes a nominal, unpopulated PCB reference. Component solids are not included. This reference is excluded from manufacturing part exports.\n\n## Material and hardware\n\n{material_note}\nPlate mounting holes: {mount_notes}.\nHardware specifications (metadata only; threads are not modeled):\n{hardware_notes}\nFastener compatibility, washers, inserts, torque, gasket material and adhesive specifications are not inferred from hole diameter. Review recorded hardware against the assembled stack before ordering.\n\nPer-part process specifications:\n{processes}\n{gasket_note}\n## Critical fit and process allowances\n\nPlate STEP, STL, SVG, DXF and KiCad geometry uses the same resolved millimetre design. Explicit radial opening allowance: {} mm. Foam retains nominal exclusions. No automatic shrinkage, kerf or tool-radius compensation has been applied. Account for the recorded opening allowance before applying any additional reviewed CAM compensation; preserve the nominal source. CNC internal corners require a compatible tool radius or explicitly reviewed relief. Printed shrinkage and cut-sheet kerf require a measured process coupon. Critical interfaces: switch retention, stabilizer cutouts, plate-to-PCB distance, fastener fit and battery clearance. Confirm each before fabrication.\n\nRecorded critical dimensions:\n{critical_fits}\n\nNominal opening extents (bounding dimensions, not replacement profiles):\n{openings}\n\nProfile sources:\n{profiles}\n\nDiagnostics:\n{diagnostics}\n",
        assembly.revision,
        configuration.plate_thickness,
        configuration.pcb_thickness,
        configuration.plate_foam_thickness,
        configuration.bottom_foam_thickness,
        configuration.wall_thickness,
        configuration.opening_allowance.unwrap_or(0.0)
    )
}

pub(crate) fn critical_fit_drawing(
    assembly: &MechanicalAssembly,
    configuration: &boardstudio_core::model::MechanicalConfiguration,
) -> Result<String, String> {
    use boardstudio_core::model::Vec2;

    let contours = &assembly.nominal_plate_contours;
    let fits = configuration.critical_fits.as_deref().unwrap_or_default();
    let mut hardware = configuration.hardware.clone().unwrap_or_default();
    hardware.extend(assembly.generated_hardware.iter().cloned());
    let referenced_parts = fits
        .iter()
        .map(|fit| fit.part_id.as_str())
        .chain(hardware.iter().map(|item| item.part_id.as_str()))
        .collect::<BTreeSet<_>>();
    let reference_bodies = assembly
        .case
        .bodies
        .iter()
        .filter(|entry| {
            entry.body.id != "plate" && referenced_parts.contains(entry.body.id.as_str())
        })
        .collect::<Vec<_>>();
    let points = contours
        .iter()
        .flat_map(|contour| contour.points.iter())
        .chain(fits.iter().flat_map(|fit| [&fit.from, &fit.to]))
        .chain(reference_bodies.iter().flat_map(|entry| {
            entry
                .contours
                .iter()
                .flat_map(|contour| contour.points.iter())
        }))
        .collect::<Vec<_>>();
    if points.is_empty() {
        return Err("No plate geometry for critical-fit drawing.".into());
    }
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err("Invalid critical-fit coordinates.".into());
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let width = max_x - min_x;
    let height = max_y - min_y;
    let path_data = |points: &[Vec2]| {
        points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                format!(
                    "{} {} {}",
                    if index == 0 { "M" } else { "L" },
                    point.x,
                    -point.y
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let paths = contours
        .iter()
        .map(|contour| format!("<path d=\"{} Z\"/>", path_data(&contour.points)))
        .collect::<String>();
    let reference_paths = reference_bodies
        .iter()
        .map(|entry| {
            let paths = entry
                .contours
                .iter()
                .map(|contour| format!("<path d=\"{} Z\"/>", path_data(&contour.points)))
                .collect::<String>();
            format!(
                "<g data-part=\"{}\" fill=\"none\" stroke=\"#8c959b\" stroke-width=\"0.1\" stroke-dasharray=\"0.7 0.5\">{paths}</g>",
                mechanical_xml_escape(&entry.body.id)
            )
        })
        .collect::<String>();
    let mounts = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "plate")
        .and_then(|body| body.body.mounts.as_ref())
        .into_iter()
        .flatten()
        .map(|mount| {
            format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                mount.at.x,
                -mount.at.y,
                mount.hole_diameter / 2.0
            )
        })
        .collect::<String>();
    let mut dimensions = String::new();
    for fit in fits {
        let dx = fit.to.x - fit.from.x;
        let dy = fit.to.y - fit.from.y;
        let length = dx.hypot(dy);
        if !length.is_finite() || length <= 0.0 {
            return Err("Critical-fit endpoints must be distinct.".into());
        }
        let offset = Vec2 {
            x: -dy / length * 5.0,
            y: dx / length * 5.0,
        };
        let a = Vec2 {
            x: fit.from.x + offset.x,
            y: fit.from.y + offset.y,
        };
        let b = Vec2 {
            x: fit.to.x + offset.x,
            y: fit.to.y + offset.y,
        };
        let label = format!(
            "{}: {} — {:.3} mm {}",
            fit.part_id, fit.label, length, fit.tolerance
        );
        dimensions.push_str(&format!(
            "<g data-fit=\"{}\"><path d=\"M {} {} L {} {} M {} {} L {} {}\" fill=\"none\" stroke=\"#42657a\" stroke-width=\"0.12\"/><path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"#42657a\" stroke-width=\"0.15\" marker-start=\"url(#dimension-arrow)\" marker-end=\"url(#dimension-arrow)\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"2.2\" fill=\"#23495e\">{}</text></g>",
            mechanical_xml_escape(&fit.id),
            fit.from.x, -fit.from.y, a.x, -a.y,
            fit.to.x, -fit.to.y, b.x, -b.y,
            a.x, -a.y, b.x, -b.y,
            (a.x + b.x) / 2.0,
            -(a.y + b.y) / 2.0 - 1.0,
            mechanical_xml_escape(&label),
        ));
    }
    let mut callouts = String::new();
    for (index, item) in hardware.iter().enumerate() {
        let mount = assembly
            .case
            .bodies
            .iter()
            .find(|entry| entry.body.id == item.part_id)
            .and_then(|entry| entry.body.mounts.as_ref())
            .and_then(|mounts| mounts.iter().find(|mount| mount.id == item.feature_id))
            .ok_or_else(|| {
                format!(
                    "Hardware {} is not linked to a generated mounting feature.",
                    item.id
                )
            })?;
        let x = max_x + 12.0;
        let y = -max_y + index as f64 * 9.0;
        let label = format!(
            "{} × {}; {} × {} mm",
            item.quantity, item.designation, item.thread, item.length
        );
        let feature = format!(
            "{}/{}{}",
            item.part_id,
            item.feature_id,
            item.tolerance
                .as_ref()
                .map(|value| format!("; {value}"))
                .unwrap_or_default()
        );
        callouts.push_str(&format!(
            "<g data-hardware=\"{}\"><path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"#705b35\" stroke-width=\"0.12\"/><circle cx=\"{}\" cy=\"{}\" r=\"0.4\" fill=\"#705b35\"/><text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"2.3\" fill=\"#493a21\">{}<tspan x=\"{}\" dy=\"3\">{}</tspan></text></g>",
            mechanical_xml_escape(&item.id),
            mount.at.x, -mount.at.y, x - 2.0, y,
            mount.at.x, -mount.at.y,
            x, y, mechanical_xml_escape(&label), x, mechanical_xml_escape(&feature),
        ));
    }
    let drawing_width = width + if hardware.is_empty() { 40.0 } else { 135.0 };
    let drawing_height = (height + 50.0).max(hardware.len() as f64 * 9.0 + 40.0);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{drawing_width}mm\" height=\"{drawing_height}mm\" viewBox=\"{} {} {drawing_width} {drawing_height}\"><defs><marker id=\"dimension-arrow\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" markerWidth=\"3\" markerHeight=\"3\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 Z\" fill=\"#42657a\"/></marker></defs>{reference_paths}<g fill=\"none\" stroke=\"#111\" stroke-width=\"0.15\">{paths}{mounts}<path d=\"M {min_x} {} v 4 M {max_x} {} v 4 M {min_x} {} H {max_x}\"/></g>{dimensions}{callouts}<g font-family=\"sans-serif\" font-size=\"2.5\" fill=\"#111\"><text x=\"{min_x}\" y=\"{}\">NOMINAL ASSEMBLY XY — CRITICAL FIT REVIEW</text><text x=\"{min_x}\" y=\"{}\">Extents: {:.3} × {:.3} mm</text><text x=\"{min_x}\" y=\"{}\">Nominal geometry; see specification for opening allowance.</text><text x=\"{min_x}\" y=\"{}\">Hardware callouts are specifications; threads are not modeled.</text><text x=\"{min_x}\" y=\"{}\">See FABRICATION.md for fit and hardware specifications.</text></g></svg>",
        min_x - 20.0,
        -max_y - 20.0,
        -min_y + 5.0,
        -min_y + 5.0,
        -min_y + 7.0,
        -max_y - 10.0,
        -min_y + 12.0,
        width,
        height,
        -min_y + 17.0,
        -min_y + 22.0,
        -min_y + 27.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_host::cad_jobs::CadMesh;

    #[test]
    fn xml_escaping_protects_all_markup_delimiters() {
        assert_eq!(
            mechanical_xml_escape("A&B <C> \"D\" 'E'"),
            "A&amp;B &lt;C&gt; &quot;D&quot; &apos;E&apos;"
        );
    }

    #[test]
    fn mechanical_paths_reject_traversal_and_duplicates() {
        let mut files = Vec::new();
        let mut paths = BTreeSet::new();
        assert!(push_mechanical_file(&mut files, &mut paths, "../escape".into(), vec![]).is_err());
        assert!(
            push_mechanical_file(&mut files, &mut paths, "plate/plate.stl".into(), vec![1]).is_ok()
        );
        assert!(
            push_mechanical_file(&mut files, &mut paths, "plate/plate.stl".into(), vec![2])
                .is_err()
        );
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn stl_writer_emits_the_binary_triangle_record() {
        let mesh = CadMesh {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
        };
        let bytes = mechanical_stl(&mesh).unwrap();
        assert_eq!(bytes.len(), 134);
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 1);
        assert_eq!(f32::from_le_bytes(bytes[108..112].try_into().unwrap()), 1.0);
    }
}
