//! Private generated analytic verifier, consumed by the sealed qualification owner.

use crate::{ExactModel, GeneratedAnalysisMesh};
use monstertruck_modeling::{Curve, InnerSpace, Point3, Surface, Vector3, Wire};
#[cfg(test)]
use monstertruck_modeling::{Plane, Solid};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedQualification {
    Bounded {
        version: u32,
        class: &'static str,
        direct_bound_mm: f64,
        source_precision_mm: f64,
        total_bound_mm: f64,
    },
    Unknown {
        reasons: Vec<String>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NativeFaceKey {
    solid_index: usize,
    shell_index: usize,
    face_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeSupportKind {
    Plane,
    Revolution,
    Other,
}

fn native_support_kind(surface: &Surface) -> NativeSupportKind {
    match surface {
        Surface::Plane(_) => NativeSupportKind::Plane,
        Surface::RevolutionSurface(_) => NativeSupportKind::Revolution,
        _ => NativeSupportKind::Other,
    }
}

fn support_kind_name(kind: NativeSupportKind) -> &'static str {
    match kind {
        NativeSupportKind::Plane => "PLANE",
        NativeSupportKind::Revolution => "REVOLUTION_SURFACE",
        NativeSupportKind::Other => "SURFACE",
    }
}

fn native_face_boundaries(model: &ExactModel, key: NativeFaceKey) -> Option<Vec<Wire>> {
    let solid = model.solids.get(key.solid_index)?;
    let shell = solid.boundaries().get(key.shell_index)?;
    let face = shell.face_iter().nth(key.face_index)?;
    Some(face.boundaries())
}

fn nonzero_axis(delta: Vector3) -> Option<usize> {
    let axes = [delta.x != 0.0, delta.y != 0.0, delta.z != 0.0];
    if axes.iter().filter(|value| **value).count() != 1 {
        return None;
    }
    axes.iter().position(|value| *value)
}

fn native_face_surface(model: &ExactModel, key: NativeFaceKey) -> Option<Surface> {
    let solid = model.solids.get(key.solid_index)?;
    let shell = solid.boundaries().get(key.shell_index)?;
    let face = shell.face_iter().nth(key.face_index)?;
    Some(face.surface().clone())
}

fn exact_plane_rectangle_corners(model: &ExactModel, key: NativeFaceKey) -> Option<Vec<Point3>> {
    let surface = native_face_surface(model, key)?;
    let plane = match surface {
        Surface::Plane(plane) => plane,
        _ => return None,
    };

    let origin = plane.origin();
    let axis_u = plane.axis_u();
    let axis_v = plane.axis_v();
    if !origin.x.is_finite()
        || !origin.y.is_finite()
        || !origin.z.is_finite()
        || !axis_u.x.is_finite()
        || !axis_u.y.is_finite()
        || !axis_u.z.is_finite()
        || !axis_v.x.is_finite()
        || !axis_v.y.is_finite()
        || !axis_v.z.is_finite()
    {
        return None;
    }

    let boundaries = native_face_boundaries(model, key)?;
    if boundaries.len() != 1 {
        return None;
    }
    let wire = &boundaries[0];
    let edges: Vec<_> = wire.edge_iter().collect();
    if edges.len() != 4 {
        return None;
    }

    let mut corners = Vec::with_capacity(4);
    for (index, edge) in edges.iter().enumerate() {
        let (start, end) = edge.ends();
        let start_point = start.point();
        let end_point = end.point();
        let _ = (start, end);
        if !start_point.x.is_finite()
            || !start_point.y.is_finite()
            || !start_point.z.is_finite()
            || !end_point.x.is_finite()
            || !end_point.y.is_finite()
            || !end_point.z.is_finite()
        {
            return None;
        }

        let curve = edge.curve();
        let line = match curve {
            Curve::Line(line) => line,
            _ => return None,
        };
        if !((line.0 == start_point && line.1 == end_point)
            || (line.0 == end_point && line.1 == start_point))
        {
            return None;
        }
        if index == 0 {
            corners.push(start_point);
        } else if corners.last().copied() != Some(start_point) {
            return None;
        }
        corners.push(end_point);
    }
    if corners.len() == 5 && corners[0] == corners[4] {
        corners.pop();
    }
    if corners.len() != 4 {
        return None;
    }

    let distinct: BTreeSet<[u64; 3]> = corners
        .iter()
        .map(|point| [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()])
        .collect();
    if distinct.len() != 4 {
        return None;
    }

    // The supported plane is axis-aligned: one coordinate is fixed for all
    // corners. Use the corners themselves, not a normalized normal, to identify
    // that coordinate. Both retained parametric axes must be zero in it.
    let fixed_axis = (0..3).find(|&axis| {
        corners
            .iter()
            .map(|point| match axis {
                0 => point.x.to_bits(),
                1 => point.y.to_bits(),
                _ => point.z.to_bits(),
            })
            .collect::<BTreeSet<_>>()
            .len()
            == 1
    })?;
    let fixed_coord = match fixed_axis {
        0 => corners[0].x,
        1 => corners[0].y,
        _ => corners[0].z,
    };
    let origin_fixed = match fixed_axis {
        0 => origin.x,
        1 => origin.y,
        _ => origin.z,
    };
    if origin_fixed != fixed_coord {
        return None;
    }
    let axis_u_fixed = match fixed_axis {
        0 => axis_u.x,
        1 => axis_u.y,
        _ => axis_u.z,
    };
    let axis_v_fixed = match fixed_axis {
        0 => axis_v.x,
        1 => axis_v.y,
        _ => axis_v.z,
    };
    if axis_u_fixed != 0.0 || axis_v_fixed != 0.0 {
        return None;
    }
    let axis_u_inplane = nonzero_axis(axis_u)?;
    let axis_v_inplane = nonzero_axis(axis_v)?;
    if axis_u_inplane == axis_v_inplane {
        return None;
    }

    let mut axes = Vec::with_capacity(4);
    let mut lengths = Vec::with_capacity(4);
    for edge in &edges {
        let (start, end) = edge.ends();
        let delta = end.point() - start.point();
        let axis = nonzero_axis(delta)?;
        axes.push(axis);
        lengths.push(match axis {
            0 => delta.x.abs(),
            1 => delta.y.abs(),
            2 => delta.z.abs(),
            _ => return None,
        });
    }
    for index in 0..4 {
        if axes[index] == axes[(index + 1) % 4] {
            return None;
        }
        if axes[index] != axes[(index + 2) % 4] {
            return None;
        }
        if lengths[index] != lengths[(index + 2) % 4] {
            return None;
        }
        if lengths[index] == 0.0 {
            return None;
        }
    }

    Some(corners)
}

pub fn qualify_generated(
    model: &ExactModel,
    analysis: &GeneratedAnalysisMesh,
) -> GeneratedQualification {
    let mut reasons: Vec<String> = Vec::new();

    if model.solids.is_empty() {
        reasons.push("missing exact solid".into());
    }
    if analysis.positions_mm_f64.is_empty() || !analysis.positions_mm_f64.len().is_multiple_of(9) {
        reasons.push("invalid final buffer".into());
    }
    if analysis
        .positions_mm_f64
        .iter()
        .any(|value| !value.is_finite())
    {
        reasons.push("non-finite coordinate".into());
    }
    if analysis.triangle_face_occurrence_indices.len() != analysis.positions_mm_f64.len() / 9 {
        reasons.push("tag count mismatch".into());
    }
    if analysis.source_faces.is_empty() {
        reasons.push("missing source face".into());
    }
    if !analysis.deflection_mm.is_finite() || analysis.deflection_mm <= 0.0 {
        reasons.push("invalid deflection setting".into());
    }
    if !analysis.angular_rad.is_finite() || analysis.angular_rad <= 0.0 {
        reasons.push("invalid angular parameter".into());
    }
    if analysis
        .triangle_face_occurrence_indices
        .iter()
        .any(|tag| *tag >= analysis.source_faces.len())
    {
        reasons.push("invalid triangle tag".into());
    }

    // Independent native face census.
    let mut native_keys = BTreeSet::new();
    let mut native_key_to_support = BTreeMap::new();
    let mut native_accepted = BTreeMap::new();
    for (solid_index, solid) in model.solids.iter().enumerate() {
        for (shell_index, shell) in solid.boundaries().iter().enumerate() {
            for (face_index, face) in shell.face_iter().enumerate() {
                let key = NativeFaceKey {
                    solid_index,
                    shell_index,
                    face_index,
                };
                native_keys.insert(key);
                let surface = face.surface().clone();
                let support = native_support_kind(&surface);
                native_key_to_support.insert(key, support);
                native_accepted.insert(key, exact_plane_rectangle_corners(model, key).is_some());
            }
        }
    }

    // Payload keys must form a bijection with the native face census.
    let mut payload_keys = Vec::new();
    for face in &analysis.source_faces {
        payload_keys.push(NativeFaceKey {
            solid_index: face.solid_index,
            shell_index: face.shell_index,
            face_index: face.face_index,
        });
    }
    let payload_key_set: BTreeSet<NativeFaceKey> = payload_keys.iter().copied().collect();
    if payload_keys.len() != payload_key_set.len() {
        reasons.push("duplicate payload source face".into());
    }
    if payload_key_set != native_keys {
        reasons.push("incomplete native source face census".into());
    }

    // Actual support type must match metadata; string metadata is never trusted.
    for (payload_index, face) in analysis.source_faces.iter().enumerate() {
        let key = NativeFaceKey {
            solid_index: face.solid_index,
            shell_index: face.shell_index,
            face_index: face.face_index,
        };
        if let Some(actual) = native_key_to_support.get(&key) {
            if support_kind_name(*actual) != face.support_kind {
                reasons.push("source support kind mismatch".into());
            }
            if *actual != NativeSupportKind::Plane {
                reasons.push("unsupported generated source face".into());
            }
        } else {
            reasons.push("missing source face".into());
        }
        let _ = payload_index;
    }

    if native_accepted.values().any(|accepted| !*accepted) {
        reasons.push("unsupported generated source face".into());
    }

    let mut distinct = BTreeSet::new();
    for tag in &analysis.triangle_face_occurrence_indices {
        distinct.insert(*tag);
    }
    if distinct.len() != analysis.source_faces.len() {
        reasons.push("incomplete face coverage".into());
    }
    for index in 0..analysis.source_faces.len() {
        if !distinct.contains(&index) {
            reasons.push("missing face occurrence".into());
            break;
        }
    }

    if !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return GeneratedQualification::Unknown { reasons };
    }

    // Map each payload occurrence to its native key.
    let mut occurrence_to_key = BTreeMap::new();
    for (index, face) in analysis.source_faces.iter().enumerate() {
        occurrence_to_key.insert(
            index,
            NativeFaceKey {
                solid_index: face.solid_index,
                shell_index: face.shell_index,
                face_index: face.face_index,
            },
        );
    }

    let triangle_count = analysis.positions_mm_f64.len() / 9;
    let mut local_reasons: Vec<String> = Vec::new();

    for occurrence in 0..analysis.source_faces.len() {
        let key = occurrence_to_key[&occurrence];
        let corners = match exact_plane_rectangle_corners(model, key) {
            Some(corners) => corners,
            None => {
                local_reasons.push("unsupported generated source face".into());
                continue;
            }
        };

        // Collect final triangles for this face.
        let mut face_triangles: Vec<[[f64; 3]; 3]> = Vec::new();
        for triangle_index in 0..triangle_count {
            if analysis.triangle_face_occurrence_indices[triangle_index] != occurrence {
                continue;
            }
            let base = triangle_index * 9;
            let mut tri = [[0.0; 3]; 3];
            for (corner, vertex) in tri.iter_mut().enumerate() {
                vertex[0] = analysis.positions_mm_f64[base + corner * 3];
                vertex[1] = analysis.positions_mm_f64[base + corner * 3 + 1];
                vertex[2] = analysis.positions_mm_f64[base + corner * 3 + 2];
            }
            face_triangles.push(tri);
        }

        // The supported class is exactly two triangles, with no subdivision or
        // interior vertices.
        if face_triangles.len() != 2 {
            local_reasons.push("unsupported final triangulation".into());
            continue;
        }

        let corner_keys: BTreeSet<[u64; 3]> = corners
            .iter()
            .map(|point| [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()])
            .collect();

        let mut vertices_ok = true;
        for tri in &face_triangles {
            for vertex in tri {
                if !vertex[0].is_finite() || !vertex[1].is_finite() || !vertex[2].is_finite() {
                    vertices_ok = false;
                    break;
                }
                let key = [
                    vertex[0].to_bits(),
                    vertex[1].to_bits(),
                    vertex[2].to_bits(),
                ];
                if !corner_keys.contains(&key) {
                    vertices_ok = false;
                    break;
                }
            }
            if !vertices_ok {
                break;
            }
            // Nondegenerate triangle.
            let a = Point3::new(tri[0][0], tri[0][1], tri[0][2]);
            let b = Point3::new(tri[1][0], tri[1][1], tri[1][2]);
            let c = Point3::new(tri[2][0], tri[2][1], tri[2][2]);
            if (b - a).cross(c - a).magnitude() == 0.0 {
                vertices_ok = false;
                break;
            }
        }
        if !vertices_ok {
            local_reasons.push("incomplete plane trim coverage".into());
            continue;
        }

        // The two triangles must share one diagonal and together contain all
        // four corners.
        let first: BTreeSet<[u64; 3]> = face_triangles[0]
            .iter()
            .map(|v| [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()])
            .collect();
        let second: BTreeSet<[u64; 3]> = face_triangles[1]
            .iter()
            .map(|v| [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()])
            .collect();
        let shared: Vec<_> = first.intersection(&second).copied().collect();
        if shared.len() != 2 {
            local_reasons.push("incomplete plane trim coverage".into());
            continue;
        }
        let union: BTreeSet<[u64; 3]> = first.union(&second).copied().collect();
        if union != corner_keys {
            local_reasons.push("incomplete plane trim coverage".into());
            continue;
        }
        // Shared edge must be a diagonal: the two shared vertices must be
        // opposite in the corner cycle. Check by ensuring the two non-shared
        // vertices are distinct and not adjacent via an edge? We approximate by
        // requiring the shared pair not be one of the original four edges.
        let shared_set: BTreeSet<[u64; 3]> = shared.iter().copied().collect();
        let mut is_edge_of_rectangle = false;
        for index in 0..4 {
            let edge_pair: BTreeSet<[u64; 3]> = [
                [
                    corners[index].x.to_bits(),
                    corners[index].y.to_bits(),
                    corners[index].z.to_bits(),
                ],
                [
                    corners[(index + 1) % 4].x.to_bits(),
                    corners[(index + 1) % 4].y.to_bits(),
                    corners[(index + 1) % 4].z.to_bits(),
                ],
            ]
            .into_iter()
            .collect();
            if edge_pair == shared_set {
                is_edge_of_rectangle = true;
                break;
            }
        }
        if is_edge_of_rectangle {
            local_reasons.push("incomplete plane trim coverage".into());
            continue;
        }
    }

    if !local_reasons.is_empty() {
        local_reasons.sort();
        local_reasons.dedup();
        return GeneratedQualification::Unknown {
            reasons: local_reasons,
        };
    }

    // Exact source/final correspondence for the supported class gives a zero
    // symmetric bound. No floating-point residual or area approximation is used
    // as proof.
    GeneratedQualification::Bounded {
        version: 1,
        class: "analytic-generated",
        direct_bound_mm: 0.0,
        source_precision_mm: 0.0,
        total_bound_mm: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_profile_for_analysis;
    use crate::test_support::{ProfileInput, ProfileKind, ProfileRegion, build_profile};
    use boardstudio_contracts::Vec2;

    fn point(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    fn plate_model() -> ExactModel {
        let input = ProfileInput {
            kind: ProfileKind::Plate,
            base_z: 0.0,
            thickness: 2.0,
            wall_height: 0.0,
            regions: vec![ProfileRegion {
                outer: vec![
                    point(0.0, 0.0),
                    point(20.0, 0.0),
                    point(20.0, 10.0),
                    point(0.0, 10.0),
                ],
                holes: vec![],
            }],
        };
        build_profile(input).expect("plate")
    }

    #[test]
    fn plane_plate_qualifies_with_zero_direct_bound() {
        let model = plate_model();
        let analysis = mesh_profile_for_analysis(&model, Some(0.01)).expect("mesh");
        let result = qualify_generated(&model, &analysis);
        match result {
            GeneratedQualification::Bounded {
                direct_bound_mm,
                source_precision_mm,
                total_bound_mm,
                ..
            } => {
                assert_eq!(source_precision_mm, 0.0);
                assert_eq!(direct_bound_mm, 0.0);
                assert_eq!(total_bound_mm, 0.0);
            }
            other => panic!("expected bounded, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_triangle_coverage_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        // Replace the top face's two triangles with two identical copies.
        // Find top face occurrence (support plane z=2).

        // Mutate all triangles belonging to occurrence 1 (top face in this mesher's order)
        // by copying the first triangle twice. Simpler: rewrite occurrence 1 triangles.
        let top_triangles: Vec<usize> = analysis
            .triangle_face_occurrence_indices
            .iter()
            .enumerate()
            .filter(|(_, tag)| **tag == 1)
            .map(|(i, _)| i)
            .collect();
        if top_triangles.len() >= 2 {
            let first = top_triangles[0];
            let base_first = first * 9;
            let copy: Vec<f64> = analysis.positions_mm_f64[base_first..base_first + 9].to_vec();
            let second = top_triangles[1];
            let base_second = second * 9;
            analysis.positions_mm_f64[base_second..base_second + 9].copy_from_slice(&copy);
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn translated_triangle_coverage_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        // Translate top-face triangles in X, preserving area but creating a gap.
        for (index, tag) in analysis.triangle_face_occurrence_indices.iter().enumerate() {
            if *tag == 1 {
                let base = index * 9;
                for corner in 0..3 {
                    analysis.positions_mm_f64[base + corner * 3] += 100.0;
                }
            }
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn representable_corner_gap_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        let delta = 2.0_f64.powi(-50);
        for (index, tag) in analysis.triangle_face_occurrence_indices.iter().enumerate() {
            if *tag == 1 {
                let base = index * 9;
                for corner in 0..3 {
                    if analysis.positions_mm_f64[base + corner * 3] == 0.0 {
                        analysis.positions_mm_f64[base + corner * 3] = delta;
                    }
                }
            }
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn support_kind_mismatch_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        if !analysis.source_faces.is_empty() {
            analysis.source_faces[0].support_kind = "PLANE".into();
        }
        // Force a revolution surface by using a PCB model? Instead, mutate metadata only:
        // actual support remains plane, so this case stays valid. To test mismatch,
        // mark metadata as revolution while actual is plane.
        if !analysis.source_faces.is_empty() {
            analysis.source_faces[0].support_kind = "REVOLUTION_SURFACE".into();
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn removed_source_face_is_unknown_by_native_census() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        let removed = analysis.source_faces.len() - 1;
        analysis.source_faces.pop();
        // Reindex tags and drop removed face's triangles/positions.
        let mut new_positions = Vec::new();
        let mut new_tags = Vec::new();
        for (triangle_index, tag) in analysis.triangle_face_occurrence_indices.iter().enumerate() {
            if *tag == removed {
                continue;
            }
            let adjusted = if *tag > removed { *tag - 1 } else { *tag };
            let base = triangle_index * 9;
            new_positions.extend_from_slice(&analysis.positions_mm_f64[base..base + 9]);
            new_tags.push(adjusted);
        }
        analysis.positions_mm_f64 = new_positions;
        analysis.triangle_face_occurrence_indices = new_tags;
        let result = qualify_generated(&model, &analysis);
        match result {
            GeneratedQualification::Unknown { reasons } => {
                assert!(reasons.iter().any(|reason| reason.contains("census")));
            }
            other => panic!("expected unknown, got {other:?}"),
        }
    }

    #[test]
    fn invalid_tag_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        if !analysis.triangle_face_occurrence_indices.is_empty() {
            analysis.triangle_face_occurrence_indices[0] = analysis.source_faces.len() + 1;
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn unsupported_support_kind_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        if !analysis.source_faces.is_empty() {
            analysis.source_faces[0].support_kind = "SURFACE".into();
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn duplicate_native_descriptor_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        analysis.source_faces.push(analysis.source_faces[0].clone());
        let result = qualify_generated(&model, &analysis);
        match result {
            GeneratedQualification::Unknown { reasons } => {
                assert!(
                    reasons
                        .iter()
                        .any(|reason| reason.contains("duplicate") || reason.contains("census"))
                );
            }
            other => panic!("expected unknown, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_plane_support_is_unknown() {
        let original = plate_model();
        let analysis = mesh_profile_for_analysis(&original, None).expect("mesh");
        let mut model = original;
        let mut solids = Vec::new();
        for solid in model.solids {
            let mut shells = solid.into_boundaries();
            for shell in &mut shells {
                for face in shell.face_iter_mut() {
                    face.set_surface(Surface::Plane(Plane::new(
                        Point3::new(100.0, 0.0, 0.0),
                        Point3::new(101.0, 0.0, 0.0),
                        Point3::new(100.0, 1.0, 0.0),
                    )));
                }
            }
            solids.push(Solid::new_unchecked(shells));
        }
        model.solids = solids;
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn reversed_line_endpoint_is_unknown() {
        let original = plate_model();
        let analysis = mesh_profile_for_analysis(&original, None).expect("mesh");
        let mut model = original;
        let mut solids = Vec::new();
        for solid in model.solids {
            let mut shells = solid.into_boundaries();
            for shell in &mut shells {
                for face in shell.face_iter_mut() {
                    let mut boundaries = face.boundaries();
                    if let Some(wire) = boundaries.get_mut(0)
                        && let Some(edge) = wire.edge_iter_mut().next()
                    {
                        let (_start, _end) = edge.ends();
                        edge.set_curve(Curve::Line(monstertruck_modeling::Line(
                            Point3::new(999.0, 999.0, 999.0),
                            Point3::new(1000.0, 1000.0, 1000.0),
                        )));
                    }
                    face.set_surface(face.surface().clone());
                    // Reconstruct face with mutated boundaries.
                    let surface = face.surface().clone();
                    // Replace boundaries by creating a new face is not possible in place;
                    // set_curve already mutated the shared edge, so keep face.
                    let _ = (boundaries, surface);
                }
            }
            solids.push(Solid::new_unchecked(shells));
        }
        model.solids = solids;
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn tiny_off_plane_parametric_component_is_unknown() {
        let original = plate_model();
        let analysis = mesh_profile_for_analysis(&original, None).expect("mesh");

        // Select the native bottom face by its exact z=0 corners, not by ordinal.
        let mut bottom_keys = Vec::new();
        let mut native_keys = BTreeSet::new();
        for (solid_index, solid) in original.solids.iter().enumerate() {
            for (shell_index, shell) in solid.boundaries().iter().enumerate() {
                for (face_index, _) in shell.face_iter().enumerate() {
                    native_keys.insert(NativeFaceKey {
                        solid_index,
                        shell_index,
                        face_index,
                    });
                }
            }
        }
        for key in &native_keys {
            if let Some(corners) = exact_plane_rectangle_corners(&original, *key)
                && corners.iter().all(|point| point.z == 0.0)
            {
                bottom_keys.push(*key);
            }
        }
        assert_eq!(bottom_keys.len(), 1);
        let bottom_key = bottom_keys[0];

        // A plane whose normalized normal rounds to +Z, but whose retained
        // parametric axis has a tiny, nonzero out-of-plane component.
        let mutated_surface = Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0_f64.powi(-100), 0.0, 0.0),
            Point3::new(0.0, 2.0_f64.powi(100), 5e-324),
        ));

        let mut model = original;
        let mut solids = Vec::new();
        for (solid_index, solid) in model.solids.into_iter().enumerate() {
            let mut shells = solid.into_boundaries();
            for (shell_index, shell) in shells.iter_mut().enumerate() {
                for (face_index, face) in shell.face_iter_mut().enumerate() {
                    if solid_index == bottom_key.solid_index
                        && shell_index == bottom_key.shell_index
                        && face_index == bottom_key.face_index
                    {
                        face.set_surface(mutated_surface.clone());
                    }
                }
            }
            solids.push(Solid::new_unchecked(shells));
        }
        model.solids = solids;

        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn triangular_plane_face_is_unknown() {
        // A triangular region has three edges, not the supported rectangle class.
        let input = ProfileInput {
            kind: ProfileKind::Plate,
            base_z: 0.0,
            thickness: 1.0,
            wall_height: 0.0,
            regions: vec![ProfileRegion {
                outer: vec![point(0.0, 0.0), point(10.0, 0.0), point(0.0, 10.0)],
                holes: vec![],
            }],
        };
        let model = build_profile(input).expect("triangle");
        let analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn wrong_coverage_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        if analysis.source_faces.len() > 1 {
            for tag in &mut analysis.triangle_face_occurrence_indices {
                if *tag == analysis.source_faces.len() - 1 {
                    *tag = 0;
                }
            }
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn missing_face_occurrence_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        analysis.source_faces.pop();
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn shrunk_plane_triangles_are_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        for triangle in 0..(analysis.positions_mm_f64.len() / 9) {
            let base = triangle * 9;
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            for corner in 0..3 {
                cx += analysis.positions_mm_f64[base + corner * 3];
                cy += analysis.positions_mm_f64[base + corner * 3 + 1];
                cz += analysis.positions_mm_f64[base + corner * 3 + 2];
            }
            cx /= 3.0;
            cy /= 3.0;
            cz /= 3.0;
            for corner in 0..3 {
                let index = base + corner * 3;
                analysis.positions_mm_f64[index] = (analysis.positions_mm_f64[index] + cx) * 0.5;
                analysis.positions_mm_f64[index + 1] =
                    (analysis.positions_mm_f64[index + 1] + cy) * 0.5;
                analysis.positions_mm_f64[index + 2] =
                    (analysis.positions_mm_f64[index + 2] + cz) * 0.5;
            }
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn pcb_hole_is_unknown_until_uv_trim_proof_exists() {
        let input: boardstudio_contracts::PreparedCaseIR = serde_json::from_str(include_str!(
            "../../tests/fixtures/pcb-source/circle-tracer.json"
        ))
        .expect("pcb fixture");
        let entry = crate::build_case(&input).expect("build");
        let analysis = mesh_profile_for_analysis(&entry, Some(0.01)).expect("mesh");
        let result = qualify_generated(&entry, &analysis);
        match result {
            GeneratedQualification::Unknown { reasons } => {
                assert!(
                    reasons
                        .iter()
                        .any(|reason| reason.contains("unsupported generated source face"))
                );
            }
            other => panic!("expected unknown, got {other:?}"),
        }
    }
}
