// Existing preview IDs, placement and bounded cache around local mesh templates.

use super::super::{BodyMeshData, MeshData};
use super::{
    empty_mesh,
    mesh_template::{add_triangle, build_keycap_mesh_template, KeycapMeshTemplate},
    validate,
};
use boardstudio_contracts::{KeycapSpec as Spec, Side};
use std::{cell::RefCell, collections::VecDeque};

const MAX_KEYCAPS_PER_REQUEST: usize = 4096;
const MAX_CACHED_TEMPLATES: usize = 64;
const MAX_CACHED_MESH_BYTES: usize = 8 * 1024 * 1024;

thread_local! {
    static CACHE: RefCell<VecDeque<(String, KeycapMeshTemplate)>> = const { RefCell::new(VecDeque::new()) };
}

pub(super) fn build(specs: &[Spec]) -> Result<Vec<BodyMeshData>, String> {
    build_mode(specs, false)
}

pub(super) fn build_printable(specs: &[Spec]) -> Result<Vec<BodyMeshData>, String> {
    if specs.is_empty() {
        return Err("Printing requires at least one keycap".into());
    }
    build_mode(specs, true)
}

fn build_mode(specs: &[Spec], printable: bool) -> Result<Vec<BodyMeshData>, String> {
    if specs.len() > MAX_KEYCAPS_PER_REQUEST {
        return Err("At most 4096 keycaps can be built per request".into());
    }
    let mut bodies = Vec::new();
    for spec in specs {
        validate(spec)?;
        let key = template_key(spec);
        let template = CACHE.with(|cache| -> Result<KeycapMeshTemplate, String> {
            let mut cache = cache.borrow_mut();
            if let Some(index) = cache.iter().position(|(cached_key, _)| cached_key == &key) {
                let Some((_, template)) = cache.remove(index) else {
                    return Err("Keycap template cache changed while reading".into());
                };
                Ok(template)
            } else {
                build_keycap_mesh_template(spec)
            }
        })?;

        let placed = (|| -> Result<(MeshData, Option<MeshData>), String> {
            let cap_mesh = if printable {
                super::printing::mesh(&template, spec)
                    .map_err(|error| format!("Keycap '{}' print failed: {error}", spec.id))?
            } else {
                display_cap_mesh(&template)
            };
            let cap_mesh = placed_mesh(&cap_mesh, spec)?;
            let legend = if printable {
                None
            } else {
                template
                    .legend
                    .as_ref()
                    .map(|legend| placed_mesh(legend, spec))
                    .transpose()?
            };
            Ok((cap_mesh, legend))
        })();
        let (cap_mesh, legend_mesh) = match placed {
            Ok(meshes) => meshes,
            Err(error) => {
                // The red/cache-retention-red.log regression pins this cache reinsert.
                cache_template(key, template);
                return Err(error);
            }
        };

        bodies.push(BodyMeshData {
            id: format!("keycap:{}", spec.id),
            name: spec.reference.clone(),
            mesh: cap_mesh,
        });
        if let Some(legend) = legend_mesh {
            bodies.push(BodyMeshData {
                id: format!("keycap-legend:{}", spec.id),
                name: format!("{} legend", spec.reference),
                mesh: legend,
            });
        }

        cache_template(key, template);
    }
    Ok(bodies)
}

fn template_key(spec: &Spec) -> String {
    format!(
        "{:?}",
        (
            &spec.profile,
            &spec.mount,
            spec.row,
            &spec.size,
            &spec.top_size,
            spec.height,
            spec.tilt,
            spec.dish_depth,
            spec.spherical,
            spec.wall_thickness,
            &spec.legend
        )
    )
}

fn template_bytes(template: &KeycapMeshTemplate) -> usize {
    let bytes = |mesh: &MeshData| (mesh.positions.len() + mesh.normals.len()) * 4;
    bytes(&template.shell)
        + bytes(&template.stem)
        + bytes(&template.socket)
        + template.legend.as_ref().map(bytes).unwrap_or(0)
}

fn cache_template(key: String, template: KeycapMeshTemplate) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.push_back((key, template));
        while cache.len() > MAX_CACHED_TEMPLATES
            || cache
                .iter()
                .map(|(_, template)| template_bytes(template))
                .sum::<usize>()
                > MAX_CACHED_MESH_BYTES
        {
            cache.pop_front();
        }
    });
}

/// Keep shell, stems and socket floors as separate triangle components. They
/// share one existing preview body identity, but are never joined as a print mesh.
fn display_cap_mesh(template: &KeycapMeshTemplate) -> MeshData {
    let mut output = empty_mesh();
    append_mesh(&mut output, &template.shell);
    append_mesh(&mut output, &template.stem);
    append_mesh(&mut output, &template.socket);
    output
}

fn append_mesh(output: &mut MeshData, input: &MeshData) {
    output.positions.extend_from_slice(&input.positions);
    output.normals.extend_from_slice(&input.normals);
}

fn placed_mesh(mesh: &MeshData, spec: &Spec) -> Result<MeshData, String> {
    if mesh.positions.len() != mesh.normals.len() {
        return Err("Placed keycap mesh has mismatched position and normal buffers".into());
    }
    if !mesh.positions.len().is_multiple_of(9) {
        return Err("Placed keycap mesh does not contain complete triangles".into());
    }

    let angle = spec.pose.rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    let side_sign = if spec.side == Side::Back { -1.0 } else { 1.0 };
    let transform = |point: [f32; 3], normal: bool| {
        let x = f64::from(point[0]);
        let y = f64::from(point[1]) * side_sign;
        let z = f64::from(point[2]) * side_sign;
        [
            x * cos - y * sin + if normal { 0.0 } else { spec.pose.at.x },
            x * sin + y * cos + if normal { 0.0 } else { spec.pose.at.y },
            z + if normal { 0.0 } else { spec.z },
        ]
    };

    let mut output = empty_mesh();
    let mut checked = empty_mesh();
    for (positions, normals) in mesh
        .positions
        .as_chunks::<9>()
        .0
        .iter()
        .zip(mesh.normals.as_chunks::<9>().0.iter())
    {
        let points = [0, 1, 2].map(|vertex| {
            let start = vertex * 3;
            transform(
                [positions[start], positions[start + 1], positions[start + 2]],
                false,
            )
        });
        checked.positions.clear();
        checked.normals.clear();
        add_triangle(&mut checked, points).map_err(|error| {
            format!("Placed keycap geometry is unusable at f32 render precision: {error}")
        })?;
        output.positions.extend_from_slice(&checked.positions);

        for vertex in 0..3 {
            let start = vertex * 3;
            let normal = transform(
                [normals[start], normals[start + 1], normals[start + 2]],
                true,
            )
            .map(|coordinate| coordinate as f32);
            output.normals.extend_from_slice(&normal);
        }
    }

    if output
        .positions
        .iter()
        .chain(&output.normals)
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("Placed keycap mesh contains non-finite f32 values".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_contracts::{KeycapMount, KeycapProfile, Pose2, Vec2};

    fn spec() -> Spec {
        Spec {
            id: "adapter".into(),
            reference: "A1".into(),
            profile: KeycapProfile::Dsa,
            mount: KeycapMount::Mx,
            row: 3,
            size: Vec2 { x: 18.2, y: 18.2 },
            top_size: Vec2 { x: 13.2, y: 13.2 },
            height: 7.5,
            tilt: 0.0,
            dish_depth: 0.5,
            spherical: true,
            wall_thickness: 1.2,
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            z: 10.0,
            travel: 4.0,
            legend: "O".into(),
            color: "#eeeeee".into(),
            legend_color: "#222222".into(),
        }
    }

    fn flat_g20() -> Spec {
        let mut spec = spec();
        spec.profile = KeycapProfile::G20;
        spec.height = 6.5;
        spec.dish_depth = 0.0;
        spec.spherical = false;
        spec.legend = "A1".into();
        spec
    }

    #[test]
    fn preview_keeps_cap_and_legend_ids_and_cached_local_geometry() {
        let mut first = spec();
        first.pose.at.x = 30.0;
        let first_bodies = build(&[first.clone()]).expect("valid preview");
        assert_eq!(first_bodies.len(), 2);
        assert_eq!(first_bodies[0].id, "keycap:adapter");
        assert_eq!(first_bodies[1].id, "keycap-legend:adapter");
        assert_eq!(first_bodies[0].name, "A1");
        assert_eq!(first_bodies[1].name, "A1 legend");
        assert!(!first_bodies[0].mesh.positions.is_empty());
        assert_eq!(
            first_bodies[0].mesh.positions.len(),
            first_bodies[0].mesh.normals.len()
        );

        let original_x = first_bodies[0].mesh.positions[0];
        let mut moved = first;
        moved.pose.at.x += 10.0;
        moved.z += 2.0;
        let moved_bodies = build(&[moved]).expect("cached local template remains reusable");
        assert!((moved_bodies[0].mesh.positions[0] - original_x - 10.0).abs() < 1e-5);
        assert!(
            (moved_bodies[1].mesh.positions[2] - first_bodies[1].mesh.positions[2] - 2.0).abs()
                < 1e-5
        );
    }

    #[test]
    fn preview_applies_back_side_rotation_and_translation_to_points_and_normals() {
        let mut keycap = spec();
        keycap.pose = Pose2 {
            at: Vec2 { x: 7.0, y: -4.0 },
            rotation: 90.0,
        };
        keycap.side = Side::Back;
        keycap.z = 3.0;
        let local = build_keycap_mesh_template(&keycap).expect("valid local template");
        let mut local_cap = display_cap_mesh(&local);
        let preview = build(&[keycap.clone()]).expect("valid placed preview");
        let (sin, cos) = keycap.pose.rotation.to_radians().sin_cos();
        let sign = -1.0;
        let point = [
            f64::from(local_cap.positions[0]),
            f64::from(local_cap.positions[1]) * sign,
            f64::from(local_cap.positions[2]) * sign,
        ];
        let expected = [
            (point[0] * cos - point[1] * sin + keycap.pose.at.x) as f32,
            (point[0] * sin + point[1] * cos + keycap.pose.at.y) as f32,
            (point[2] + keycap.z) as f32,
        ];
        assert_eq!(&preview[0].mesh.positions[..3], &expected);

        let normal = [
            f64::from(local_cap.normals[0]),
            f64::from(local_cap.normals[1]) * sign,
            f64::from(local_cap.normals[2]) * sign,
        ];
        let expected_normal = [
            (normal[0] * cos - normal[1] * sin) as f32,
            (normal[0] * sin + normal[1] * cos) as f32,
            normal[2] as f32,
        ];
        assert_eq!(&preview[0].mesh.normals[..3], &expected_normal);
        local_cap.positions.clear();
    }

    #[test]
    fn preview_rejects_over_limit_and_preserves_mount_components_in_cap_payload() {
        let mut cap = spec();
        cap.legend.clear();
        let template = build_keycap_mesh_template(&cap).expect("valid cap template");
        let shell_vertices = template.shell.positions.len();
        let mount_vertices = template.stem.positions.len() + template.socket.positions.len();
        let bodies = build(std::slice::from_ref(&cap)).expect("valid preview");
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].id, "keycap:adapter");
        assert_eq!(
            bodies[0].mesh.positions.len(),
            shell_vertices + mount_vertices
        );

        let too_many = vec![cap; MAX_KEYCAPS_PER_REQUEST + 1];
        assert!(matches!(
            build(&too_many),
            Err(ref message) if message.contains("At most 4096")
        ));
    }

    #[test]
    fn cache_hit_revalidates_pose_and_travel_and_returns_finite_meshes() {
        let mut valid = spec();
        valid.size.x = 19.31;
        valid.top_size.x = 14.31;
        valid.height = 8.21;
        valid.legend = "A1".into();
        let warmed = build(std::slice::from_ref(&valid)).expect("warm valid template");
        assert_eq!(warmed.len(), 2);

        let mut invalid_pose = valid.clone();
        invalid_pose.pose.at.x = f64::NAN;
        assert_eq!(template_key(&valid), template_key(&invalid_pose));
        let pose_result = build(&[invalid_pose]);

        let mut invalid_travel = valid.clone();
        invalid_travel.travel = f64::NAN;
        assert_eq!(template_key(&valid), template_key(&invalid_travel));
        let travel_result = build(&[invalid_travel]);

        assert!(
            pose_result.is_err(),
            "a cached keycap must reject non-finite placement before returning meshes"
        );
        assert!(
            travel_result.is_err(),
            "a cached keycap must reject non-finite travel before returning meshes"
        );

        let mut moved = valid;
        moved.pose.at.x = 17.0;
        moved.pose.at.y = -4.0;
        moved.pose.rotation = 31.0;
        moved.z = 2.25;
        let cached = build(&[moved]).expect("valid moved cache hit");
        assert_eq!(cached.len(), 2);
        for body in &cached {
            assert!(body.mesh.positions.iter().all(|value| value.is_finite()));
            assert!(body.mesh.normals.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn placement_rejects_f32_overflow_and_render_precision_collapse() {
        let mut overflow = flat_g20();
        overflow.pose.at.x = 1.0e300;
        let overflow_result = build(&[overflow]);

        let valid = flat_g20();
        let warmed = build(std::slice::from_ref(&valid)).expect("warm valid template");
        assert_eq!(warmed.len(), 2);
        for body in &warmed {
            assert!(body.mesh.positions.iter().all(|value| value.is_finite()));
            assert!(body.mesh.normals.iter().all(|value| value.is_finite()));
        }

        let mut collapsed = valid.clone();
        collapsed.pose.at.x = 1.0e10;
        let key = template_key(&valid);
        assert_eq!(key, template_key(&collapsed));
        let collapse_result = build(&[collapsed]);
        let cached_after_rejection = CACHE.with(|cache| {
            cache
                .borrow()
                .iter()
                .any(|(cached_key, _)| cached_key == &key)
        });

        assert!(
            matches!(overflow_result, Err(ref message) if message.contains("render precision")),
            "finite placement that overflows f32 must be rejected"
        );
        assert!(
            matches!(collapse_result, Err(ref message) if message.contains("render precision")),
            "finite placement that collapses a triangle at f32 precision must be rejected"
        );
        assert!(
            cached_after_rejection,
            "a failed placement must preserve the valid local template in cache"
        );
    }
}
