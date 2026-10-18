//! Pure-Rust Parry 0.31.1 triangle-surface collision and clearance observations.
//!
//! This module consumes world-millimetre f64 triangle soup plus per-triangle
//! 010 face-occurrence indices and explicit physical row transforms. It reports
//! raw mesh observations and witness provenance only; Runtime combines these
//! observations with asset-level error bounds to decide whether a source-level
//! tolerance band is supported.

use js_sys::{Float64Array, Uint32Array};
use parry3d_f64::{
    math::{Pose, Vector},
    query::{ClosestPoints, DefaultQueryDispatcher, QueryDispatcher},
    shape::{SubShapeId, TriMesh, TriMeshFlags, Triangle},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

/// Aggregate byte cap across the four transferred typed buffers (64 MiB).
pub const MAX_PAIR_BUFFER_BYTES: usize = 64 * 1024 * 1024;

/// Conservative coordinate overflow envelope in millimetres so squared distances
/// in f64 cannot overflow. This is a numeric guard, not an accuracy bound.
const MAX_SAFE_COORD_MM: f64 = 1e100;

/// Relative/absolute comparison slack between mesh-level distance and selected
/// triangle-pair closest-point witness distance.
const CLOSEST_POINTS_SLACK_MM: f64 = 1e-6;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadStepImportOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_repair_budget_mm: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadCollisionOperand {
    pub model_id: String,
    pub source_sha256: String,
    #[serde(default)]
    pub import_options: CadStepImportOptions,
    pub row_transform_f64: [f64; 16],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadCollisionQuery {
    pub left: CadCollisionOperand,
    pub right: CadCollisionOperand,
    pub clearance_threshold_mm: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_accuracy_mm: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TriangleProvenance {
    pub triangle_index: u32,
    pub face_occurrence_index: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrianglePairProvenance {
    pub left: TriangleProvenance,
    pub right: TriangleProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContactWitness {
    pub triangles: TrianglePairProvenance,
    pub point1_mm: [f64; 3],
    pub point2_mm: [f64; 3],
    pub normal1: [f64; 3],
    pub normal2: [f64; 3],
    /// Parry's signed distance for this selected triangle-pair contact only.
    /// It is not a global mesh or solid penetration depth.
    pub triangle_pair_contact_dist_mm: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClosestPointsWitness {
    pub triangles: TrianglePairProvenance,
    pub point1_mm: [f64; 3],
    pub point2_mm: [f64; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MeshObservation {
    #[serde(rename_all = "camelCase")]
    SurfaceIntersecting {
        intersection_triangles: TrianglePairProvenance,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contact_witness: Option<ContactWitness>,
    },
    #[serde(rename_all = "camelCase")]
    SurfaceSeparated {
        surface_distance_mm: f64,
        closest_points: ClosestPointsWitness,
    },
    #[serde(rename_all = "camelCase")]
    Uncertain {
        reason: String,
        #[serde(default)]
        partial_intersection: bool,
    },
}

pub fn checked_pair_buffer_bytes(
    left_positions_len: usize,
    left_tags_len: usize,
    right_positions_len: usize,
    right_tags_len: usize,
) -> Result<usize, String> {
    let total = left_positions_len
        .checked_mul(std::mem::size_of::<f64>())
        .and_then(|bytes| {
            left_tags_len
                .checked_mul(std::mem::size_of::<u32>())
                .and_then(|tag_bytes| bytes.checked_add(tag_bytes))
        })
        .and_then(|bytes| {
            right_positions_len
                .checked_mul(std::mem::size_of::<f64>())
                .and_then(|right_bytes| bytes.checked_add(right_bytes))
        })
        .and_then(|bytes| {
            right_tags_len
                .checked_mul(std::mem::size_of::<u32>())
                .and_then(|tag_bytes| bytes.checked_add(tag_bytes))
        })
        .ok_or_else(|| "Collision pair buffer sizes overflow usize".to_string())?;
    if total > MAX_PAIR_BUFFER_BYTES {
        return Err(format!(
            "Collision pair buffers ({total} bytes) exceed the {MAX_PAIR_BUFFER_BYTES} byte limit"
        ));
    }
    Ok(total)
}

impl CadCollisionQuery {
    pub fn validate(&self) -> Result<(), String> {
        if !self.clearance_threshold_mm.is_finite() || self.clearance_threshold_mm < 0.0 {
            return Err(
                "Invalid collision query: clearanceThresholdMm must be finite and non-negative"
                    .into(),
            );
        }
        if let Some(accuracy) = self.requested_accuracy_mm {
            if !accuracy.is_finite() || accuracy <= 0.0 {
                return Err(
                    "Invalid collision query: requestedAccuracyMm must be finite and positive"
                        .into(),
                );
            }
        }
        self.left.validate("left")?;
        self.right.validate("right")?;
        Ok(())
    }
}

impl CadCollisionOperand {
    fn validate(&self, side: &str) -> Result<(), String> {
        if self.model_id.trim().is_empty() {
            return Err(format!(
                "Invalid collision query: {side}.modelId must not be empty"
            ));
        }
        if !is_sha256(&self.source_sha256) {
            return Err(format!(
                "Invalid collision query: {side}.sourceSha256 must be 64 lowercase hexadecimal characters"
            ));
        }
        if let Some(budget) = self.import_options.preview_repair_budget_mm {
            if !budget.is_finite() || budget <= 0.0 {
                return Err(format!(
                    "Invalid collision query: {side}.importOptions.previewRepairBudgetMm must be finite and positive"
                ));
            }
        }
        validate_affine_transform(&self.row_transform_f64, side)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_affine_transform(matrix: &[f64; 16], side: &str) -> Result<(), String> {
    if matrix.iter().any(|value| !value.is_finite()) {
        return Err(format!(
            "Invalid collision query: {side}.rowTransformF64 contains non-finite values"
        ));
    }
    if [matrix[3], matrix[7], matrix[11], matrix[15]] != [0.0, 0.0, 0.0, 1.0] {
        return Err(format!(
            "Invalid collision query: {side}.rowTransformF64 must be an affine 4x4 matrix"
        ));
    }
    let det = matrix[0] * (matrix[5] * matrix[10] - matrix[9] * matrix[6])
        - matrix[4] * (matrix[1] * matrix[10] - matrix[9] * matrix[2])
        + matrix[8] * (matrix[1] * matrix[6] - matrix[5] * matrix[2]);
    if !det.is_finite() || det.abs() <= 1e-12 {
        return Err(format!(
            "Invalid collision query: {side}.rowTransformF64 has a singular or non-finite linear part"
        ));
    }
    Ok(())
}

fn validate_soup_lengths(positions_len: usize, tags_len: usize, side: &str) -> Result<(), String> {
    if positions_len == 0 || !positions_len.is_multiple_of(9) {
        return Err(format!(
            "Invalid collision input: {side} positions must be a non-empty multiple of 9 f64 values"
        ));
    }
    if tags_len != positions_len / 9 {
        return Err(format!(
            "Invalid collision input: {side} triangle tag count ({tags_len}) does not match triangle count ({})",
            positions_len / 9
        ));
    }
    if positions_len / 3 > u32::MAX as usize {
        return Err(format!(
            "Invalid collision input: {side} vertex count exceeds u32 index range"
        ));
    }
    Ok(())
}

struct TriangleSoupMesh {
    mesh: TriMesh,
    face_occurrence_indices: Vec<u32>,
}

impl TriangleSoupMesh {
    fn from_transformed_soup(
        positions_mm: &[f64],
        face_occurrence_indices: &[u32],
        transform: &[f64; 16],
        side: &str,
    ) -> Result<Self, String> {
        validate_soup_lengths(positions_mm.len(), face_occurrence_indices.len(), side)?;
        let vertex_count = positions_mm.len() / 3;
        let triangle_count = positions_mm.len() / 9;
        let mut vertices = Vec::with_capacity(vertex_count);
        for (vertex_index, point) in positions_mm.as_chunks::<3>().0.iter().enumerate() {
            if point.iter().any(|value| !value.is_finite()) {
                return Err(format!(
                    "Invalid collision input: {side} vertex {vertex_index} contains a non-finite coordinate"
                ));
            }
            let x = transform[0] * point[0]
                + transform[4] * point[1]
                + transform[8] * point[2]
                + transform[12];
            let y = transform[1] * point[0]
                + transform[5] * point[1]
                + transform[9] * point[2]
                + transform[13];
            let z = transform[2] * point[0]
                + transform[6] * point[1]
                + transform[10] * point[2]
                + transform[14];
            for (axis, coord) in [x, y, z].into_iter().enumerate() {
                if !coord.is_finite() || coord.abs() > MAX_SAFE_COORD_MM {
                    return Err(format!(
                        "Invalid collision input: {side} transformed vertex {vertex_index} axis {axis} is outside the safe query range"
                    ));
                }
            }
            vertices.push(Vector::new(x, y, z));
        }
        let indices = (0..triangle_count as u32)
            .map(|tri| [tri * 3, tri * 3 + 1, tri * 3 + 2])
            .collect::<Vec<_>>();
        let mesh =
            TriMesh::with_flags(vertices, indices, TriMeshFlags::empty()).map_err(|error| {
                format!("Invalid collision input: {side} TriMesh construction failed: {error}")
            })?;
        Ok(Self {
            mesh,
            face_occurrence_indices: face_occurrence_indices.to_vec(),
        })
    }

    fn provenance(&self, subshape: SubShapeId) -> Option<TriangleProvenance> {
        let index = usize::try_from(subshape).ok()?;
        let face_occurrence_index = *self.face_occurrence_indices.get(index)?;
        Some(TriangleProvenance {
            triangle_index: subshape,
            face_occurrence_index,
        })
    }

    fn triangle(&self, subshape: SubShapeId) -> Option<Triangle> {
        let index = usize::try_from(subshape).ok()?;
        if index >= self.face_occurrence_indices.len() {
            return None;
        }
        Some(self.mesh.triangle(subshape))
    }
}

fn pair_provenance(
    left: &TriangleSoupMesh,
    right: &TriangleSoupMesh,
    subshape1: SubShapeId,
    subshape2: SubShapeId,
) -> Option<TrianglePairProvenance> {
    Some(TrianglePairProvenance {
        left: left.provenance(subshape1)?,
        right: right.provenance(subshape2)?,
    })
}

fn vector_is_finite(v: Vector) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

fn to_array(v: Vector) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn observe_mesh_pair(left: &TriangleSoupMesh, right: &TriangleSoupMesh) -> MeshObservation {
    let dispatcher = DefaultQueryDispatcher;
    let intersection = match dispatcher.intersection_test(&Pose::IDENTITY, &left.mesh, &right.mesh)
    {
        Ok(result) => result,
        Err(_) => {
            return MeshObservation::Uncertain {
                reason: "Parry intersection_test unsupported for triangle mesh pair".into(),
                partial_intersection: false,
            };
        }
    };

    if intersection.intersecting {
        let Some(intersection_triangles) =
            pair_provenance(left, right, intersection.subshape1, intersection.subshape2)
        else {
            return MeshObservation::Uncertain {
                reason: "Parry intersection_test returned an out-of-range triangle ID".into(),
                partial_intersection: true,
            };
        };

        let contact_witness =
            match dispatcher.contact(&Pose::IDENTITY, &left.mesh, &right.mesh, 0.0) {
                Ok(Some(contact)) => {
                    pair_provenance(left, right, contact.subshape1, contact.subshape2)
                        .filter(|_| {
                            vector_is_finite(contact.point1)
                                && vector_is_finite(contact.point2)
                                && vector_is_finite(contact.normal1)
                                && vector_is_finite(contact.normal2)
                                && contact.dist.is_finite()
                        })
                        .map(|triangles| ContactWitness {
                            triangles,
                            point1_mm: to_array(contact.point1),
                            point2_mm: to_array(contact.point2),
                            normal1: to_array(contact.normal1),
                            normal2: to_array(contact.normal2),
                            triangle_pair_contact_dist_mm: contact.dist,
                        })
                }
                Ok(None) | Err(_) => None,
            };

        return MeshObservation::SurfaceIntersecting {
            intersection_triangles,
            contact_witness,
        };
    }

    let distance = match dispatcher.distance(&Pose::IDENTITY, &left.mesh, &right.mesh) {
        Ok(distance) => distance,
        Err(_) => {
            return MeshObservation::Uncertain {
                reason: "Parry distance query unsupported for triangle mesh pair".into(),
                partial_intersection: false,
            };
        }
    };

    if !distance.distance.is_finite() || distance.distance == f64::MAX || distance.distance < 0.0 {
        return MeshObservation::Uncertain {
            reason: "Parry distance query returned a non-finite or sentinel distance".into(),
            partial_intersection: false,
        };
    }

    if distance.distance == 0.0 {
        return MeshObservation::Uncertain {
            reason: "Parry reported zero unsigned distance without surface intersection".into(),
            partial_intersection: false,
        };
    }

    let (Some(triangles), Some(tri1), Some(tri2)) = (
        pair_provenance(left, right, distance.subshape1, distance.subshape2),
        left.triangle(distance.subshape1),
        right.triangle(distance.subshape2),
    ) else {
        return MeshObservation::Uncertain {
            reason: "Parry distance query returned an out-of-range triangle ID".into(),
            partial_intersection: false,
        };
    };

    match dispatcher.closest_points(&Pose::IDENTITY, &tri1, &tri2, f64::MAX) {
        Ok(ClosestPoints::WithinMargin(point1, point2)) => {
            if !vector_is_finite(point1) || !vector_is_finite(point2) {
                return MeshObservation::Uncertain {
                    reason: "Parry closest_points returned non-finite witness coordinates".into(),
                    partial_intersection: false,
                };
            }
            let witness_distance = (point1 - point2).length();
            let slack = CLOSEST_POINTS_SLACK_MM * distance.distance.max(1.0);
            if !witness_distance.is_finite() || (witness_distance - distance.distance).abs() > slack
            {
                return MeshObservation::Uncertain {
                    reason: "Parry closest_points witness distance disagreed with mesh distance"
                        .into(),
                    partial_intersection: false,
                };
            }
            MeshObservation::SurfaceSeparated {
                surface_distance_mm: distance.distance,
                closest_points: ClosestPointsWitness {
                    triangles,
                    point1_mm: to_array(point1),
                    point2_mm: to_array(point2),
                },
            }
        }
        Ok(_) => MeshObservation::Uncertain {
            reason: "Parry closest_points did not return a witness pair".into(),
            partial_intersection: false,
        },
        Err(_) => MeshObservation::Uncertain {
            reason: "Parry closest_points unsupported for selected triangle pair".into(),
            partial_intersection: false,
        },
    }
}

pub fn query_collision_mesh_pair_data(
    left_positions_mm_f64: &[f64],
    left_triangle_face_occurrence_indices: &[u32],
    right_positions_mm_f64: &[f64],
    right_triangle_face_occurrence_indices: &[u32],
    query: &CadCollisionQuery,
) -> Result<MeshObservation, String> {
    checked_pair_buffer_bytes(
        left_positions_mm_f64.len(),
        left_triangle_face_occurrence_indices.len(),
        right_positions_mm_f64.len(),
        right_triangle_face_occurrence_indices.len(),
    )?;
    validate_soup_lengths(
        left_positions_mm_f64.len(),
        left_triangle_face_occurrence_indices.len(),
        "left",
    )?;
    validate_soup_lengths(
        right_positions_mm_f64.len(),
        right_triangle_face_occurrence_indices.len(),
        "right",
    )?;
    query.validate()?;
    let left = TriangleSoupMesh::from_transformed_soup(
        left_positions_mm_f64,
        left_triangle_face_occurrence_indices,
        &query.left.row_transform_f64,
        "left",
    )?;
    let right = TriangleSoupMesh::from_transformed_soup(
        right_positions_mm_f64,
        right_triangle_face_occurrence_indices,
        &query.right.row_transform_f64,
        "right",
    )?;
    Ok(observe_mesh_pair(&left, &right))
}

#[wasm_bindgen]
pub fn query_collision_mesh_pair(
    left_positions: Float64Array,
    left_tags: Uint32Array,
    right_positions: Float64Array,
    right_tags: Uint32Array,
    query_metadata: JsValue,
) -> Result<JsValue, JsValue> {
    checked_pair_buffer_bytes(
        left_positions.length() as usize,
        left_tags.length() as usize,
        right_positions.length() as usize,
        right_tags.length() as usize,
    )
    .map_err(super::js_error)?;
    validate_soup_lengths(
        left_positions.length() as usize,
        left_tags.length() as usize,
        "left",
    )
    .map_err(super::js_error)?;
    validate_soup_lengths(
        right_positions.length() as usize,
        right_tags.length() as usize,
        "right",
    )
    .map_err(super::js_error)?;
    let query: CadCollisionQuery = serde_wasm_bindgen::from_value(query_metadata)
        .map_err(|error| super::js_error(format!("Invalid collision query metadata: {error}")))?;
    query.validate().map_err(super::js_error)?;
    let observation = query_collision_mesh_pair_data(
        &left_positions.to_vec(),
        &left_tags.to_vec(),
        &right_positions.to_vec(),
        &right_tags.to_vec(),
        &query,
    )
    .map_err(super::js_error)?;
    observation
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(super::js_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn cube_soup(min: [f64; 3], max: [f64; 3], base_tag: u32) -> (Vec<f64>, Vec<u32>) {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        let v = [
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ];
        let faces = [
            ([0, 2, 1], [0, 3, 2]),
            ([4, 5, 6], [4, 6, 7]),
            ([0, 1, 5], [0, 5, 4]),
            ([2, 3, 7], [2, 7, 6]),
            ([1, 2, 6], [1, 6, 5]),
            ([3, 0, 4], [3, 4, 7]),
        ];
        let mut positions = Vec::with_capacity(12 * 9);
        let mut tags = Vec::with_capacity(12);
        for (face_idx, (t1, t2)) in faces.into_iter().enumerate() {
            for tri in [t1, t2] {
                for idx in tri {
                    positions.extend_from_slice(&v[idx]);
                }
                tags.push(base_tag + face_idx as u32);
            }
        }
        (positions, tags)
    }

    fn query(left_transform: [f64; 16], right_transform: [f64; 16]) -> CadCollisionQuery {
        CadCollisionQuery {
            left: CadCollisionOperand {
                model_id: "part-left".into(),
                source_sha256: SHA_A.into(),
                import_options: CadStepImportOptions::default(),
                row_transform_f64: left_transform,
            },
            right: CadCollisionOperand {
                model_id: "part-right".into(),
                source_sha256: SHA_B.into(),
                import_options: CadStepImportOptions::default(),
                row_transform_f64: right_transform,
            },
            clearance_threshold_mm: 0.5,
            requested_accuracy_mm: None,
        }
    }

    #[test]
    fn public_query_reports_separated_unsigned_gap_and_original_occurrence_indices() {
        let (left_pos, left_tags) = cube_soup([0.0, 0.0, 0.0], [2.0, 2.0, 2.0], 10);
        let (right_pos, right_tags) = cube_soup([3.0, 0.0, 0.0], [5.0, 2.0, 2.0], 20);
        let observation = query_collision_mesh_pair_data(
            &left_pos,
            &left_tags,
            &right_pos,
            &right_tags,
            &query(IDENTITY, IDENTITY),
        )
        .unwrap();
        match observation {
            MeshObservation::SurfaceSeparated {
                surface_distance_mm,
                closest_points,
            } => {
                assert!((surface_distance_mm - 1.0).abs() < 1e-12);
                assert!((closest_points.point1_mm[0] - 2.0).abs() < 1e-12);
                assert!((closest_points.point2_mm[0] - 3.0).abs() < 1e-12);
                assert_eq!(
                    closest_points.triangles.left.face_occurrence_index,
                    left_tags[closest_points.triangles.left.triangle_index as usize]
                );
                assert_eq!(
                    closest_points.triangles.right.face_occurrence_index,
                    right_tags[closest_points.triangles.right.triangle_index as usize]
                );
            }
            other => panic!("expected separated surface observation, got {other:?}"),
        }
    }

    #[test]
    fn public_query_reports_touching_and_crossing_mesh_observations_with_triangle_ids() {
        let (left_pos, left_tags) = cube_soup([0.0, 0.0, 0.0], [2.0, 2.0, 2.0], 100);
        for x_min in [2.0, 1.25] {
            let (right_pos, right_tags) =
                cube_soup([x_min, 0.0, 0.0], [x_min + 2.0, 2.0, 2.0], 200);
            let observation = query_collision_mesh_pair_data(
                &left_pos,
                &left_tags,
                &right_pos,
                &right_tags,
                &query(IDENTITY, IDENTITY),
            )
            .unwrap();
            match observation {
                MeshObservation::SurfaceIntersecting {
                    intersection_triangles,
                    contact_witness,
                } => {
                    assert_eq!(
                        intersection_triangles.left.face_occurrence_index,
                        left_tags[intersection_triangles.left.triangle_index as usize]
                    );
                    assert_eq!(
                        intersection_triangles.right.face_occurrence_index,
                        right_tags[intersection_triangles.right.triangle_index as usize]
                    );
                    let witness = contact_witness.expect("contact witness present");
                    assert_eq!(
                        witness.triangles.left.face_occurrence_index,
                        left_tags[witness.triangles.left.triangle_index as usize]
                    );
                    assert_eq!(
                        witness.triangles.right.face_occurrence_index,
                        right_tags[witness.triangles.right.triangle_index as usize]
                    );
                }
                other => {
                    panic!("expected intersecting observation at x_min={x_min}, got {other:?}")
                }
            }
        }
    }

    #[test]
    fn public_query_keeps_nested_cubes_as_separated_surfaces_not_a_solid_clearance_claim() {
        let (outer_pos, outer_tags) = cube_soup([0.0, 0.0, 0.0], [10.0, 10.0, 10.0], 1);
        let (inner_pos, inner_tags) = cube_soup([3.0, 3.0, 3.0], [7.0, 7.0, 7.0], 50);
        let observation = query_collision_mesh_pair_data(
            &outer_pos,
            &outer_tags,
            &inner_pos,
            &inner_tags,
            &query(IDENTITY, IDENTITY),
        )
        .unwrap();
        match observation {
            MeshObservation::SurfaceSeparated {
                surface_distance_mm,
                ..
            } => {
                assert!((surface_distance_mm - 3.0).abs() < 1e-12);
            }
            other => panic!("nested cube boundaries are separated by 3mm, got {other:?}"),
        }
    }

    #[test]
    fn public_query_applies_each_final_row_transform_once() {
        let (left_pos, left_tags) = cube_soup([0.0, 0.0, 0.0], [2.0, 2.0, 2.0], 0);
        let (right_pos, right_tags) = cube_soup([0.0, 0.0, 0.0], [2.0, 2.0, 2.0], 12);
        let mut right_transform = IDENTITY;
        right_transform[12] = 4.5;
        let observation = query_collision_mesh_pair_data(
            &left_pos,
            &left_tags,
            &right_pos,
            &right_tags,
            &query(IDENTITY, right_transform),
        )
        .unwrap();
        match observation {
            MeshObservation::SurfaceSeparated {
                surface_distance_mm,
                closest_points,
            } => {
                assert!((surface_distance_mm - 2.5).abs() < 1e-12);
                assert!((closest_points.point1_mm[0] - 2.0).abs() < 1e-12);
                assert!((closest_points.point2_mm[0] - 4.5).abs() < 1e-12);
            }
            other => panic!("expected 2.5mm separated cubes after row transform, got {other:?}"),
        }
    }

    #[test]
    fn buffer_budget_is_checked_from_lengths_before_allocation_or_transfer() {
        let max_f64_elems = MAX_PAIR_BUFFER_BYTES / 8;
        assert!(checked_pair_buffer_bytes(max_f64_elems, 1, 0, 0).is_err());
        assert!(checked_pair_buffer_bytes(usize::MAX, 1, 1, 1).is_err());
        assert!(checked_pair_buffer_bytes(9, 1, 9, 1).is_ok());
    }

    #[test]
    fn public_query_rejects_malformed_tags_nonfinite_values_and_bad_transforms() {
        let (left_pos, left_tags) = cube_soup([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0);
        let (right_pos, right_tags) = cube_soup([2.0, 0.0, 0.0], [3.0, 1.0, 1.0], 0);
        assert!(query_collision_mesh_pair_data(
            &left_pos[..8],
            &left_tags,
            &right_pos,
            &right_tags,
            &query(IDENTITY, IDENTITY),
        )
        .is_err());
        assert!(query_collision_mesh_pair_data(
            &left_pos,
            &left_tags[..left_tags.len() - 1],
            &right_pos,
            &right_tags,
            &query(IDENTITY, IDENTITY),
        )
        .is_err());
        let mut nan_pos = left_pos.clone();
        nan_pos[0] = f64::NAN;
        assert!(query_collision_mesh_pair_data(
            &nan_pos,
            &left_tags,
            &right_pos,
            &right_tags,
            &query(IDENTITY, IDENTITY),
        )
        .is_err());
        let mut singular = IDENTITY;
        singular[0] = 0.0;
        assert!(query_collision_mesh_pair_data(
            &left_pos,
            &left_tags,
            &right_pos,
            &right_tags,
            &query(singular, IDENTITY),
        )
        .is_err());
        let mut non_affine = IDENTITY;
        non_affine[15] = 2.0;
        assert!(query_collision_mesh_pair_data(
            &left_pos,
            &left_tags,
            &right_pos,
            &right_tags,
            &query(non_affine, IDENTITY),
        )
        .is_err());
    }

    #[test]
    fn public_query_rejects_nonfinite_clearance_and_noncanonical_asset_identity() {
        let (left_pos, left_tags) = cube_soup([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0);
        let (right_pos, right_tags) = cube_soup([2.0, 0.0, 0.0], [3.0, 1.0, 1.0], 0);
        for bad_clearance in [f64::NAN, f64::INFINITY, -0.01] {
            let mut q = query(IDENTITY, IDENTITY);
            q.clearance_threshold_mm = bad_clearance;
            assert!(query_collision_mesh_pair_data(
                &left_pos,
                &left_tags,
                &right_pos,
                &right_tags,
                &q
            )
            .is_err());
        }
        for bad_accuracy in [f64::NAN, 0.0, -0.01, f64::INFINITY] {
            let mut q = query(IDENTITY, IDENTITY);
            q.requested_accuracy_mm = Some(bad_accuracy);
            assert!(query_collision_mesh_pair_data(
                &left_pos,
                &left_tags,
                &right_pos,
                &right_tags,
                &q
            )
            .is_err());
        }
        let mut bad_sha = query(IDENTITY, IDENTITY);
        bad_sha.left.source_sha256 = "not-a-sha".into();
        assert!(query_collision_mesh_pair_data(
            &left_pos,
            &left_tags,
            &right_pos,
            &right_tags,
            &bad_sha,
        )
        .is_err());
    }
}
