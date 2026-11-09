use std::iter;

use super::*;
use boundary::{PolyBoundary, SurfacePoint, UvKey, boundary_segment_parameter, uv_key};

const MAX_SURFACE_SPAN_INTERVALS: usize = 32;
const SURFACE_SPAN_TOLERANCE_FACTOR: f64 = 8.0;

/// Ensures the mesh winding direction globally matches the stored vertex normals.
///
/// Sums the area-weighted dot product of each face's geometric normal against its
/// vertex normals. If the total is negative the entire mesh is inverted, keeping
/// all faces consistently wound.
///
/// # Why the weight is the raw cross-product sum and not `FaceNormal`
///
/// Ledger class C15, measured 2026-08-01. `FaceNormal::new` NORMALIZES, so a
/// face of zero area answers `0/0 = NaN`. A sphere's parameter grid ends in a
/// row of such faces at each pole, and one NaN term makes the whole `vote`
/// NaN -- at which point `vote < 0.0` is FALSE and the mesh is never corrected.
///
/// `occt-sphere.step` measured **-523.253857** on a `+523.5988` ball for
/// exactly this reason: 158 of its 24,964 mesh faces (one pole strip) voted
/// NaN, the raw grid winding is `-Su x Sv`, and the correction that exists to
/// catch that never ran. The torus next to it in the same fixture set has no
/// degenerate row, voted finite, and was right all along.
///
/// The raw sum is also what the doc line above always CLAIMED -- it is
/// area-weighted, so a degenerate face contributes exactly zero instead of
/// poisoning the sum, and a large face outvotes a sliver.
pub(super) fn ensure_winding_matches_normals(mesh: &mut PolygonMesh) {
    let positions = mesh.positions();
    let normals = mesh.normals();
    let mut vote = 0.0f64;
    for face in mesh.faces().face_iter() {
        let center = face
            .iter()
            .fold(Vector3::zero(), |sum, v| sum + positions[v.pos].to_vec())
            / face.len() as f64;
        // Twice the area-weighted normal of the (possibly non-planar) polygon.
        // Deliberately NOT normalized: see the note above.
        let geo = face
            .windows(2)
            .chain(iter::once([face[face.len() - 1], face[0]].as_ref()))
            .fold(Vector3::zero(), |sum, v| {
                let vec0 = positions[v[0].pos].to_vec() - center;
                let vec1 = positions[v[1].pos].to_vec() - center;
                sum + vec0.cross(vec1)
            });
        let vtx: Vector3 = face.iter().fold(Vector3::zero(), |sum, v| {
            sum + v.nor.map(|i| normals[i]).unwrap_or(Vector3::zero())
        });
        let term = geo.dot(vtx);
        if term.is_finite() {
            vote += term;
        }
    }
    if vote < 0.0 {
        mesh.invert();
    }
}

pub(super) fn spade_round(x: f64) -> f64 {
    match f64::abs(x) < MIN_ALLOWED_VALUE {
        true => 0.0,
        false => x,
    }
}

fn add_constraint_through_existing_vertices(
    triangulation: &mut Cdt,
    split_vertices: &[(FixedVertexHandle, Point2)],
    front: (FixedVertexHandle, Point2),
    back: (FixedVertexHandle, Point2),
) {
    if front.0 == back.0 {
        return;
    }
    let mut chain = split_vertices
        .iter()
        .filter_map(|(handle, point)| {
            if *handle == front.0 || *handle == back.0 {
                None
            } else {
                boundary_segment_parameter(*point, front.1, back.1)
                    .map(|parameter| (parameter, *handle))
            }
        })
        .collect::<Vec<_>>();
    chain.sort_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0));
    let handles = iter::once(front.0)
        .chain(chain.into_iter().map(|(_, handle)| handle))
        .chain(iter::once(back.0))
        .collect::<Vec<_>>();
    handles.windows(2).for_each(|window| {
        if window[0] != window[1] && triangulation.can_add_constraint(window[0], window[1]) {
            triangulation.add_constraint(window[0], window[1]);
        }
    });
}

/// Canonicalizes only parameter rows or columns explicitly declared by the
/// analytic surface implementation as point singularities.
fn canonicalize_declared_singularities<S: PreMeshableSurface>(
    surface: &S,
    udiv: &[f64],
    vdiv: &[f64],
    positions: &mut [Point3],
    nv: usize,
) -> (Vec<bool>, Vec<bool>) {
    let mut collapsed_u = vec![false; udiv.len()];
    let mut collapsed_v = vec![false; vdiv.len()];
    for singularity in surface.analytic_singularities() {
        match singularity.axis {
            ParameterAxis::U => {
                for (i, u) in udiv.iter().enumerate() {
                    if *u == singularity.parameter {
                        collapsed_u[i] = true;
                        let anchor = positions[i * nv];
                        for j in 0..nv {
                            positions[i * nv + j] = anchor;
                        }
                    }
                }
            }
            ParameterAxis::V => {
                for (j, v) in vdiv.iter().enumerate() {
                    if *v == singularity.parameter {
                        collapsed_v[j] = true;
                        let anchor = positions[j];
                        for i in 0..udiv.len() {
                            positions[i * nv + j] = anchor;
                        }
                    }
                }
            }
        }
    }
    (collapsed_u, collapsed_v)
}

fn same_position(a: Point3, b: Point3) -> bool { a.x == b.x && a.y == b.y && a.z == b.z }

/// Tessellates a bounded surface without any trimming.
///
/// Generates a structured grid from parameter division, then triangulates
/// cells adjacent to source-declared analytic point singularities. Generic
/// surfaces retain their sampled positions and face topology unchanged.
pub(super) fn untrimmed_tessellation<S>(
    surface: &S,
    range: ((f64, f64), (f64, f64)),
    tolerance: f64,
    primitive_mode: TessellationPrimitiveMode,
) -> PolygonMesh
where
    S: PreMeshableSurface,
{
    let (udiv, vdiv) = surface.parameter_division(range, tolerance);
    let nu = udiv.len();
    let nv = vdiv.len();
    let mut positions = Vec::with_capacity(nu * nv);
    let mut uv_coords = Vec::with_capacity(nu * nv);
    let mut normals = Vec::with_capacity(nu * nv);
    for u in &udiv {
        for v in &vdiv {
            positions.push(surface.subs(*u, *v));
            uv_coords.push(Vector2::new(*u, *v));
            normals.push(surface.normal(*u, *v));
        }
    }
    let (collapsed_u, collapsed_v) =
        canonicalize_declared_singularities(surface, &udiv, &vdiv, &mut positions, nv);
    let idx = |i: usize, j: usize| -> usize { i * nv + j };
    let sv = |k: usize| -> StandardVertex { [k, k, k].into() };
    let mut tri_faces = Vec::new();
    let mut quad_faces = Vec::new();
    for i in 0..nu - 1 {
        for j in 0..nv - 1 {
            let a = idx(i, j);
            let b = idx(i, j + 1);
            let c = idx(i + 1, j + 1);
            let d = idx(i + 1, j);
            let touches_declared_singularity =
                collapsed_u[i] || collapsed_u[i + 1] || collapsed_v[j] || collapsed_v[j + 1];
            if primitive_mode == TessellationPrimitiveMode::Triangles
                || touches_declared_singularity
            {
                for face in [[a, b, c], [a, c, d]] {
                    if !touches_declared_singularity
                        || !(same_position(positions[face[0]], positions[face[1]])
                            || same_position(positions[face[1]], positions[face[2]])
                            || same_position(positions[face[2]], positions[face[0]]))
                    {
                        tri_faces.push(face.map(sv));
                    }
                }
            } else {
                quad_faces.push([sv(a), sv(b), sv(c), sv(d)]);
            }
        }
    }
    let mut mesh = PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords,
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, quad_faces),
    );
    ensure_winding_matches_normals(&mut mesh);
    mesh
}

/// Tessellates one surface trimmed by polyline.
pub(super) fn trimming_tessellation<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tolerance: f64,
    primitive_config: TessellationPrimitiveOptions,
    face_idx: usize,
) -> std::result::Result<PolygonMesh, MetricRefinementError>
where
    S: PreMeshableSurface,
{
    if primitive_config.mode == TessellationPrimitiveMode::IsoQuads {
        if let Some(mut mesh) =
            iso_quad_trimmed_tessellation(surface, polyboundary, tolerance, face_idx)?
        {
            ensure_winding_matches_normals(&mut mesh);
            Ok(mesh)
        } else {
            let mut mesh = cdt_trimming_tessellation(surface, polyboundary, tolerance, face_idx)?;
            ensure_winding_matches_normals(&mut mesh);
            Ok(mesh)
        }
    } else {
        let mut mesh = cdt_trimming_tessellation(surface, polyboundary, tolerance, face_idx)?;
        // `triangulation_into_polymesh` preserves Spade's counterclockwise UV face order.
        // Keep that producer-defined orientation for triangle meshes: a global area-weighted
        // physical-normal vote can reverse valid CDT triangles when coarse chords dominate it.
        if primitive_config.mode != TessellationPrimitiveMode::Triangles {
            ensure_winding_matches_normals(&mut mesh);
        }
        apply_primitive_mode(&mut mesh, primitive_config);
        Ok(mesh)
    }
}

fn cdt_trimming_tessellation<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tolerance: f64,
    face_idx: usize,
) -> std::result::Result<PolygonMesh, MetricRefinementError>
where
    S: PreMeshableSurface,
{
    let mut triangulation = Cdt::new();
    let mut boundary_map = HashMap::<FixedVertexHandle, SurfacePoint>::default();
    let insert_boundary_start = Instant::now();
    let (boundary_vertices, added_constraints, skipped_constraints) =
        polyboundary.insert_to(&mut triangulation, &mut boundary_map);
    log_mesh_trace(
        face_idx,
        "cdt_boundary",
        format!(
            "loops={} boundary_vertices={} constraints={} skipped_constraints={}",
            polyboundary.loops.len(),
            boundary_vertices,
            added_constraints,
            skipped_constraints,
        ),
        insert_boundary_start,
    );
    let insert_surface_start = Instant::now();
    insert_surface(
        &mut triangulation,
        surface,
        polyboundary,
        tolerance,
        face_idx,
    );
    log_mesh_trace(
        face_idx,
        "cdt_surface",
        format!("vertices={}", triangulation.num_vertices()),
        insert_surface_start,
    );
    triangulation_into_polymesh(
        triangulation.vertices(),
        triangulation.inner_faces(),
        surface,
        polyboundary,
        &boundary_map,
    )
}

fn iso_quad_trimmed_tessellation<S>(
    surface: &S,
    polyboundary: &PolyBoundary,
    tolerance: f64,
    face_idx: usize,
) -> std::result::Result<Option<PolygonMesh>, MetricRefinementError>
where
    S: PreMeshableSurface,
{
    let range = (
        (polyboundary.uv_min.x, polyboundary.uv_max.x),
        (polyboundary.uv_min.y, polyboundary.uv_max.y),
    );
    let division_start = Instant::now();
    let (udiv, vdiv) = densify_surface_divisions(
        &surface,
        range,
        surface.parameter_division(range, tolerance),
        tolerance,
    );
    log_mesh_trace(
        face_idx,
        "iso_division",
        format!("udiv={} vdiv={}", udiv.len(), vdiv.len()),
        division_start,
    );
    let (nu, nv) = (udiv.len(), vdiv.len());
    if nu < 2 || nv < 2 {
        return Ok(None);
    }
    let idx = |i: usize, j: usize| -> usize { i * nv + j };
    let sv = |k: usize| -> StandardVertex { [k, k, k].into() };
    let positions = udiv
        .iter()
        .flat_map(|u| vdiv.iter().map(move |v| surface.subs(*u, *v)))
        .collect::<Vec<_>>();
    let uv_coords = udiv
        .iter()
        .flat_map(|u| vdiv.iter().map(move |v| Vector2::new(*u, *v)))
        .collect::<Vec<_>>();
    let normals = udiv
        .iter()
        .flat_map(|u| vdiv.iter().map(move |v| surface.normal(*u, *v)))
        .collect::<Vec<_>>();
    let inside = udiv
        .iter()
        .flat_map(|u| {
            vdiv.iter()
                .map(move |v| polyboundary.include(Point2::new(*u, *v)))
        })
        .collect::<Vec<_>>();
    let nv_cells = nv - 1;
    let (udiv_ref, vdiv_ref, inside_ref) = (&udiv, &vdiv, &inside);
    let full_cells = (0..nu - 1)
        .flat_map(|i| {
            (0..nv - 1).map(move |j| {
                let corner_indices = [idx(i, j), idx(i, j + 1), idx(i + 1, j + 1), idx(i + 1, j)];
                let corners_inside = corner_indices.into_iter().all(|k| inside_ref[k]);
                if !corners_inside {
                    false
                } else {
                    let (u0, u1) = (udiv_ref[i], udiv_ref[i + 1]);
                    let (v0, v1) = (vdiv_ref[j], vdiv_ref[j + 1]);
                    let center = Point2::new((u0 + u1) * 0.5, (v0 + v1) * 0.5);
                    let edge_midpoints = [
                        Point2::new((u0 + u1) * 0.5, v0),
                        Point2::new(u1, (v0 + v1) * 0.5),
                        Point2::new((u0 + u1) * 0.5, v1),
                        Point2::new(u0, (v0 + v1) * 0.5),
                    ];
                    polyboundary.include(center)
                        && edge_midpoints
                            .into_iter()
                            .all(|midpoint| polyboundary.include(midpoint))
                }
            })
        })
        .collect::<Vec<_>>();
    let interior_quads = full_cells
        .iter()
        .enumerate()
        .filter_map(|(cell_index, is_full)| {
            if *is_full {
                let i = cell_index / nv_cells;
                let j = cell_index % nv_cells;
                let a = idx(i, j);
                let b = idx(i, j + 1);
                let c = idx(i + 1, j + 1);
                let d = idx(i + 1, j);
                Some([sv(a), sv(b), sv(c), sv(d)])
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if interior_quads.is_empty() {
        Ok(None)
    } else {
        let mut interior_mesh = PolygonMesh::debug_new(
            StandardAttributes {
                positions,
                uv_coords,
                normals,
            },
            Faces::from_tri_and_quad_faces(Vec::new(), interior_quads),
        );
        let mut boundary_mesh =
            cdt_trimming_tessellation(surface, polyboundary, tolerance, face_idx)?;
        let boundary_triangles = boundary_mesh
            .tri_faces()
            .iter()
            .copied()
            .filter(|triangle| {
                !triangle_is_in_full_cell(
                    *triangle,
                    boundary_mesh.uv_coords(),
                    &udiv,
                    &vdiv,
                    &full_cells,
                )
            })
            .collect::<Vec<_>>();
        let boundary_quads = boundary_mesh
            .quad_faces()
            .iter()
            .copied()
            .filter(|quad| {
                !quad_is_in_full_cell(*quad, boundary_mesh.uv_coords(), &udiv, &vdiv, &full_cells)
            })
            .collect::<Vec<_>>();
        {
            let editor = boundary_mesh.debug_editor();
            *editor.faces = Faces::from_tri_and_quad_faces(boundary_triangles, boundary_quads);
        }
        interior_mesh.merge(boundary_mesh);
        Ok(Some(interior_mesh))
    }
}

fn triangle_is_in_full_cell(
    triangle: [StandardVertex; 3],
    uv_coords: &[Vector2],
    udiv: &[f64],
    vdiv: &[f64],
    full_cells: &[bool],
) -> bool {
    triangle_center_uv(triangle, uv_coords)
        .is_some_and(|uv| uv_is_in_full_cell(uv, udiv, vdiv, full_cells))
}

fn quad_is_in_full_cell(
    quad: [StandardVertex; 4],
    uv_coords: &[Vector2],
    udiv: &[f64],
    vdiv: &[f64],
    full_cells: &[bool],
) -> bool {
    quad_center_uv(quad, uv_coords).is_some_and(|uv| uv_is_in_full_cell(uv, udiv, vdiv, full_cells))
}

fn triangle_center_uv(triangle: [StandardVertex; 3], uv_coords: &[Vector2]) -> Option<Vector2> {
    let uv0 = uv_coords[*triangle[0].uv.as_ref()?];
    let uv1 = uv_coords[*triangle[1].uv.as_ref()?];
    let uv2 = uv_coords[*triangle[2].uv.as_ref()?];
    Some((uv0 + uv1 + uv2) / 3.0)
}

fn quad_center_uv(quad: [StandardVertex; 4], uv_coords: &[Vector2]) -> Option<Vector2> {
    let uv0 = uv_coords[*quad[0].uv.as_ref()?];
    let uv1 = uv_coords[*quad[1].uv.as_ref()?];
    let uv2 = uv_coords[*quad[2].uv.as_ref()?];
    let uv3 = uv_coords[*quad[3].uv.as_ref()?];
    Some((uv0 + uv1 + uv2 + uv3) / 4.0)
}

fn uv_is_in_full_cell(uv: Vector2, udiv: &[f64], vdiv: &[f64], full_cells: &[bool]) -> bool {
    if vdiv.len() < 2 {
        false
    } else {
        let nv_cells = vdiv.len() - 1;
        if let (Some(i), Some(j)) = (
            parameter_cell_index(udiv, uv.x),
            parameter_cell_index(vdiv, uv.y),
        ) {
            full_cells.get(i * nv_cells + j).copied().unwrap_or(false)
        } else {
            false
        }
    }
}

fn parameter_cell_index(parameters: &[f64], value: f64) -> Option<usize> {
    if parameters.len() < 2 || value < parameters[0] || value > parameters[parameters.len() - 1] {
        None
    } else {
        let upper = parameters.partition_point(|sample| *sample <= value);
        if upper == 0 {
            None
        } else if upper >= parameters.len() {
            Some(parameters.len() - 2)
        } else {
            Some(upper - 1)
        }
    }
}

fn apply_primitive_mode(mesh: &mut PolygonMesh, primitive_config: TessellationPrimitiveOptions) {
    if primitive_config.mode != TessellationPrimitiveMode::Triangles {
        mesh.quadrangulate(
            primitive_config.plane_tolerance,
            primitive_config.score_tolerance,
        );
        if primitive_config.mode == TessellationPrimitiveMode::AllQuads {
            force_all_triangles_to_quads(mesh, primitive_config);
        }
    }
}

fn force_all_triangles_to_quads(
    mesh: &mut PolygonMesh,
    primitive_config: TessellationPrimitiveOptions,
) {
    if mesh.tri_faces().is_empty() {
        return;
    }
    let normal_blend_angle = primitive_config.normal_blend_angle;
    let triangles = mesh.tri_faces().clone();
    let mut quadrangles = mesh.quad_faces().clone();
    let mut midpoint_cache = HashMap::<(VertexKey, VertexKey), StandardVertex>::default();
    triangles.into_iter().for_each(|triangle| {
        let [vertex0, vertex1, vertex2] = triangle;
        let point0 = mesh.positions()[vertex0.pos];
        let point1 = mesh.positions()[vertex1.pos];
        let point2 = mesh.positions()[vertex2.pos];
        let point01 = point0.midpoint(point1);
        let point12 = point1.midpoint(point2);
        let point20 = point2.midpoint(point0);
        let center_point =
            Point3::from_vec((point0.to_vec() + point1.to_vec() + point2.to_vec()) / 3.0);
        let split_quads = [
            [point0, point01, center_point, point20],
            [point1, point12, center_point, point01],
            [point2, point20, center_point, point12],
        ];
        let quality_ok = split_quads
            .into_iter()
            .all(|quad| quad_passes_quality_gates(quad, primitive_config));
        if quality_ok {
            let midpoint01 = midpoint_for_edge(
                mesh,
                &mut midpoint_cache,
                vertex0,
                vertex1,
                normal_blend_angle,
            );
            let midpoint12 = midpoint_for_edge(
                mesh,
                &mut midpoint_cache,
                vertex1,
                vertex2,
                normal_blend_angle,
            );
            let midpoint20 = midpoint_for_edge(
                mesh,
                &mut midpoint_cache,
                vertex2,
                vertex0,
                normal_blend_angle,
            );
            let center = create_centroid_vertex(mesh, triangle, normal_blend_angle);
            quadrangles.extend([
                [vertex0, midpoint01, center, midpoint20],
                [vertex1, midpoint12, center, midpoint01],
                [vertex2, midpoint20, center, midpoint12],
            ]);
        } else {
            quadrangles.push([vertex0, vertex1, vertex2, vertex2]);
        }
    });
    {
        let editor = mesh.debug_editor();
        *editor.faces = Faces::from_tri_and_quad_faces(Vec::new(), quadrangles);
    }
    ensure_winding_matches_normals(mesh);
}

type VertexKey = (usize, Option<usize>, Option<usize>);

fn midpoint_for_edge(
    mesh: &mut PolygonMesh,
    midpoint_cache: &mut HashMap<(VertexKey, VertexKey), StandardVertex>,
    vertex0: StandardVertex,
    vertex1: StandardVertex,
    normal_blend_angle: f64,
) -> StandardVertex {
    let key = edge_key(vertex0, vertex1);
    if let Some(vertex) = midpoint_cache.get(&key) {
        *vertex
    } else {
        let midpoint = create_midpoint_vertex(mesh, vertex0, vertex1, normal_blend_angle);
        midpoint_cache.insert(key, midpoint);
        midpoint
    }
}

fn edge_key(vertex0: StandardVertex, vertex1: StandardVertex) -> (VertexKey, VertexKey) {
    let key0 = (vertex0.pos, vertex0.uv, vertex0.nor);
    let key1 = (vertex1.pos, vertex1.uv, vertex1.nor);
    if key0 <= key1 {
        (key0, key1)
    } else {
        (key1, key0)
    }
}

fn quad_passes_quality_gates(
    quad: [Point3; 4],
    primitive_config: TessellationPrimitiveOptions,
) -> bool {
    let area = quad_area(quad);
    let convex = is_convex_quad(quad);
    let corner_angles_ok = quad.iter().enumerate().all(|(index, _)| {
        corner_angle(quad, index)
            .is_some_and(|angle| angle <= primitive_config.maximum_corner_angle)
    });
    area >= primitive_config.minimum_area && convex && corner_angles_ok
}

fn quad_area(quad: [Point3; 4]) -> f64 {
    triangle_area(quad[0], quad[1], quad[2]) + triangle_area(quad[0], quad[2], quad[3])
}

fn triangle_area(point0: Point3, point1: Point3, point2: Point3) -> f64 {
    let vector0 = point1.to_vec() - point0.to_vec();
    let vector1 = point2.to_vec() - point0.to_vec();
    vector0.cross(vector1).magnitude() * 0.5
}

fn is_convex_quad(quad: [Point3; 4]) -> bool {
    let diagonal0 = quad[1].to_vec() - quad[0].to_vec();
    let diagonal1 = quad[2].to_vec() - quad[0].to_vec();
    let normal = diagonal0.cross(diagonal1);
    if normal.so_small() {
        false
    } else {
        let (has_positive, has_negative, has_nonzero) = quad
            .iter()
            .enumerate()
            .map(|(index, _)| corner_turn(quad, index, normal))
            .fold(
                (false, false, false),
                |(positive, negative, nonzero), turn| {
                    (
                        positive || turn > TOLERANCE,
                        negative || turn < -TOLERANCE,
                        nonzero || f64::abs(turn) > TOLERANCE,
                    )
                },
            );
        has_nonzero && !(has_positive && has_negative)
    }
}

fn corner_turn(quad: [Point3; 4], index: usize, normal: Vector3) -> f64 {
    let previous = quad[(index + 3) % 4];
    let current = quad[index];
    let next = quad[(index + 1) % 4];
    let edge0 = current.to_vec() - previous.to_vec();
    let edge1 = next.to_vec() - current.to_vec();
    edge0.cross(edge1).dot(normal)
}

fn corner_angle(quad: [Point3; 4], index: usize) -> Option<f64> {
    let previous = quad[(index + 3) % 4];
    let current = quad[index];
    let next = quad[(index + 1) % 4];
    let vector0 = previous.to_vec() - current.to_vec();
    let vector1 = next.to_vec() - current.to_vec();
    let length0 = vector0.magnitude();
    let length1 = vector1.magnitude();
    if length0.so_small() || length1.so_small() {
        None
    } else {
        let cosine = (vector0.dot(vector1) / (length0 * length1)).clamp(-1.0, 1.0);
        Some(cosine.acos())
    }
}

fn create_midpoint_vertex(
    mesh: &mut PolygonMesh,
    vertex0: StandardVertex,
    vertex1: StandardVertex,
    normal_blend_angle: f64,
) -> StandardVertex {
    let point0 = mesh.positions()[vertex0.pos];
    let point1 = mesh.positions()[vertex1.pos];
    let position = point0.midpoint(point1);
    let position_index = mesh.positions().len();
    mesh.push_position(position);

    let uv_coord = match (vertex0.uv, vertex1.uv) {
        (Some(index0), Some(index1)) => {
            Some((mesh.uv_coords()[index0] + mesh.uv_coords()[index1]) / 2.0)
        }
        _ => None,
    };
    let uv_index = uv_coord.map(|uv_coord| {
        let index = mesh.uv_coords().len();
        mesh.push_uv_coord(uv_coord);
        index
    });

    let normal = match (vertex0.nor, vertex1.nor) {
        (Some(index0), Some(index1)) => Some(blend_normals(
            mesh.normals()[index0],
            mesh.normals()[index1],
            normal_blend_angle,
        )),
        _ => None,
    };
    let normal_index = normal.map(|normal| {
        let index = mesh.normals().len();
        mesh.extend_normals([normal]);
        index
    });

    StandardVertex {
        pos: position_index,
        uv: uv_index,
        nor: normal_index,
    }
}

fn create_centroid_vertex(
    mesh: &mut PolygonMesh,
    triangle: [StandardVertex; 3],
    normal_blend_angle: f64,
) -> StandardVertex {
    let [vertex0, vertex1, vertex2] = triangle;
    let point0 = mesh.positions()[vertex0.pos];
    let point1 = mesh.positions()[vertex1.pos];
    let point2 = mesh.positions()[vertex2.pos];
    let position = Point3::from_vec((point0.to_vec() + point1.to_vec() + point2.to_vec()) / 3.0);
    let position_index = mesh.positions().len();
    mesh.push_position(position);

    let uv_coord = match (vertex0.uv, vertex1.uv, vertex2.uv) {
        (Some(index0), Some(index1), Some(index2)) => Some(
            (mesh.uv_coords()[index0] + mesh.uv_coords()[index1] + mesh.uv_coords()[index2]) / 3.0,
        ),
        _ => None,
    };
    let uv_index = uv_coord.map(|uv_coord| {
        let index = mesh.uv_coords().len();
        mesh.push_uv_coord(uv_coord);
        index
    });

    let normal = match (vertex0.nor, vertex1.nor, vertex2.nor) {
        (Some(index0), Some(index1), Some(index2)) => {
            let normal01 = blend_normals(
                mesh.normals()[index0],
                mesh.normals()[index1],
                normal_blend_angle,
            );
            Some(blend_normals(
                normal01,
                mesh.normals()[index2],
                normal_blend_angle,
            ))
        }
        _ => None,
    };
    let normal_index = normal.map(|normal| {
        let index = mesh.normals().len();
        mesh.extend_normals([normal]);
        index
    });

    StandardVertex {
        pos: position_index,
        uv: uv_index,
        nor: normal_index,
    }
}

fn blend_normals(normal0: Vector3, normal1: Vector3, normal_blend_angle: f64) -> Vector3 {
    let cosine = normal0.dot(normal1);
    let min_cosine = f64::cos(normal_blend_angle);
    let blended = if cosine < min_cosine {
        normal0
    } else {
        normal0 + normal1
    };
    let magnitude = blended.magnitude();
    if magnitude.so_small() {
        normal0
    } else {
        blended / magnitude
    }
}

fn surface_axis_point<S: ParametricSurface3D>(
    surface: &S,
    axis: usize,
    axis_value: f64,
    other_value: f64,
) -> Point3 {
    if axis == 0 {
        surface.subs(axis_value, other_value)
    } else {
        surface.subs(other_value, axis_value)
    }
}

fn division_axis_length<S: ParametricSurface3D>(
    surface: &S,
    divisions: &[f64],
    axis: usize,
    other_value: f64,
) -> f64 {
    divisions
        .windows(2)
        .map(|window| {
            surface_axis_point(surface, axis, window[0], other_value).distance(surface_axis_point(
                surface,
                axis,
                window[1],
                other_value,
            ))
        })
        .sum()
}

fn desired_span_intervals(length: f64, tolerance: f64) -> usize {
    let max_segment_length = f64::max(tolerance * SURFACE_SPAN_TOLERANCE_FACTOR, TOLERANCE);
    if length.is_finite() && max_segment_length.is_finite() && max_segment_length > 0.0 {
        f64::ceil(length / max_segment_length)
            .max(1.0)
            .min(MAX_SURFACE_SPAN_INTERVALS as f64) as usize
    } else {
        1
    }
}

fn densify_divisions(
    mut divisions: Vec<f64>,
    range: (f64, f64),
    desired_intervals: usize,
) -> Vec<f64> {
    if divisions.len() < 2 {
        divisions = vec![range.0, range.1];
    }
    if let (Some(start), Some(end)) = (divisions.first().copied(), divisions.last().copied())
        && desired_intervals > divisions.len().saturating_sub(1)
        && start.is_finite()
        && end.is_finite()
        && !(end - start).so_small()
    {
        divisions = (0..=desired_intervals)
            .map(|index| start + (end - start) * index as f64 / desired_intervals as f64)
            .collect();
    }
    divisions
}

fn densify_surface_divisions<S: ParametricSurface3D>(
    surface: &S,
    range: ((f64, f64), (f64, f64)),
    divisions: (Vec<f64>, Vec<f64>),
    tolerance: f64,
) -> (Vec<f64>, Vec<f64>) {
    let (udiv, vdiv) = divisions;
    let vmid = (range.1.0 + range.1.1) * 0.5;
    let umid = (range.0.0 + range.0.1) * 0.5;
    let u_length = division_axis_length(surface, &udiv, 0, vmid);
    let v_length = division_axis_length(surface, &vdiv, 1, umid);
    (
        densify_divisions(udiv, range.0, desired_span_intervals(u_length, tolerance)),
        densify_divisions(vdiv, range.1, desired_span_intervals(v_length, tolerance)),
    )
}

/// Inserts parameter divisions into triangulation.
fn insert_surface(
    triangulation: &mut Cdt,
    surface: impl PreMeshableSurface,
    polyline: &PolyBoundary,
    tolerance: f64,
    face_idx: usize,
) {
    let range = (
        (polyline.uv_min.x, polyline.uv_max.x),
        (polyline.uv_min.y, polyline.uv_max.y),
    );
    // Only the surface definition can certify that boundary chords fill a plane.
    if surface.is_affine_plane() {
        return;
    }
    let division_start = Instant::now();
    let (udiv, vdiv) = densify_surface_divisions(
        &surface,
        range,
        surface.parameter_division(range, tolerance),
        tolerance,
    );
    log_mesh_trace(
        face_idx,
        "parameter_division",
        format!(
            "range=({:.6},{:.6})x({:.6},{:.6}) udiv={} vdiv={}",
            range.0.0,
            range.0.1,
            range.1.0,
            range.1.1,
            udiv.len(),
            vdiv.len(),
        ),
        division_start,
    );
    let insert_start = Instant::now();
    let split_vertices = triangulation
        .vertices()
        .map(|vertex| {
            let point = *vertex.as_ref();
            (vertex.fix(), Point2::new(point.x, point.y))
        })
        .collect::<Vec<_>>();
    let insert_res: Vec<Vec<Option<_>>> = udiv
        .into_iter()
        .map(|u| {
            vdiv.iter()
                .map(|v| {
                    let uv = Point2::new(u, *v);
                    match polyline.include(uv) {
                        true => {
                            let cdt_uv = polyline.uv_to_cdt(uv);
                            triangulation
                                .insert(SPoint2::new(cdt_uv.x, cdt_uv.y))
                                .ok()
                                .map(|handle| (handle, cdt_uv))
                        }
                        false => None,
                    }
                })
                .collect()
        })
        .collect();
    insert_res.windows(2).for_each(|vec| {
        vec[0].windows(2).zip(&vec[1]).for_each(|(a, z)| {
            if let Some(x) = a[0] {
                if let Some(y) = a[1] {
                    add_constraint_through_existing_vertices(triangulation, &split_vertices, x, y);
                }
                if let Some(z) = z {
                    add_constraint_through_existing_vertices(triangulation, &split_vertices, x, *z);
                }
            }
        });
        let idx = vec[0].len() - 1;
        if let (Some(x), Some(y)) = (vec[0][idx], vec[1][idx]) {
            add_constraint_through_existing_vertices(triangulation, &split_vertices, x, y);
        }
    });
    let inserted_points = insert_res
        .iter()
        .flatten()
        .filter(|vertex| vertex.is_some())
        .count();
    log_mesh_trace(
        face_idx,
        "insert_surface",
        format!("inserted_points={inserted_points}"),
        insert_start,
    );
}

/// Converts triangulation into `PolygonMesh`.
fn triangulation_into_polymesh<'a>(
    vertices: VertexIterator<'a, SPoint2, (), CdtEdge<()>, ()>,
    triangles: InnerFaceIterator<'a, SPoint2, (), CdtEdge<()>, ()>,
    surface: &impl ParametricSurface3D,
    polyline: &PolyBoundary,
    boundary_map: &HashMap<FixedVertexHandle, SurfacePoint>,
) -> std::result::Result<PolygonMesh, MetricRefinementError> {
    let mut positions = Vec::<Point3>::new();
    let mut uv_coords = Vec::<Vector2>::new();
    let mut normals = Vec::<Vector3>::new();
    let mut surface_point_cache = HashMap::<UvKey, Point3>::default();
    let mut normal_cache = HashMap::<UvKey, Vector3>::default();
    let vmap: HashMap<_, _> = vertices
        .enumerate()
        .map(|(i, v)| {
            let p = *v.as_ref();
            let uv = polyline.cdt_to_uv(Point2::new(p.x, p.y));
            let key = uv_key(uv);
            let idx = v.fix();
            let point = match boundary_map.get(&idx) {
                Some(point) => point.point,
                None => *surface_point_cache
                    .entry(key)
                    .or_insert_with(|| surface.subs(uv.x, uv.y)),
            };
            let normal = *normal_cache
                .entry(key)
                .or_insert_with(|| surface.normal(uv.x, uv.y));
            positions.push(point);
            uv_coords.push(uv.to_vec());
            normals.push(normal);
            (idx, i)
        })
        .collect();
    let source_vertices = boundary_map.iter().filter_map(|(handle, point)| {
        point.source_vertex.map(|source| (vmap[handle], source))
    }).collect::<HashMap<_, _>>();
    let mut constraints = std::collections::BTreeSet::new();
    let mut tri_faces: Vec<[StandardVertex; 3]> = triangles
        .map(|tri| {
            for edge in tri.adjacent_edges() {
                if edge.is_constraint_edge() {
                    let [a, b] = edge.vertices().map(|v| vmap[&v.fix()]);
                    constraints.insert(ordered_edge(a, b));
                }
            }
            (
                tri.vertices(),
                tri.adjacent_edges()
                    .into_iter()
                    .any(|edge| edge.is_constraint_edge()),
            )
        })
        .filter(|(tri, constrained)| {
            fn sp2cg(p: SPoint2) -> Point2 {
                Point2::new(p.x, p.y)
            }
            let uv_tri = tri.map(|vertex| sp2cg(*vertex.as_ref()));
            let (a, b) = (uv_tri[1] - uv_tri[0], uv_tri[2] - uv_tri[0]);
            let centroid = polyline.cdt_to_uv(uv_tri[0] + (a + b) / 3.0);
            let area = a.x * b.y - a.y * b.x;
            if polyline.include(centroid) && !area.so_small2() {
                return true;
            }
            if !constrained {
                return false;
            }
            let points = tri.map(|vertex| positions[vmap[&vertex.fix()]]);
            if (0..3).any(|i| points[i].near(&points[(i + 1) % 3])) {
                return false;
            }
            let physical_normal = (points[1] - points[0]).cross(points[2] - points[0]);
            let physical_area = physical_normal.magnitude2();
            let oriented_area = physical_normal.dot(surface.normal(centroid.x, centroid.y));
            let source_uv = uv_tri.map(|uv| polyline.cdt_to_uv(uv));
            let (a, b) = (source_uv[1] - source_uv[0], source_uv[2] - source_uv[0]);
            let du = surface.derivative_u(centroid.x, centroid.y);
            let dv = surface.derivative_v(centroid.x, centroid.y);
            let tangent_area = (du * a.x + dv * a.y).cross(du * b.x + dv * b.y).magnitude();
            // A curved boundary chord can enclose area even when its UV triangle
            // is only roundoff. Require resolved area in the local surface metric
            // as well as a chord normal that agrees with the source surface.
            tangent_area.is_finite()
                && !tangent_area.so_small2()
                && physical_area.is_finite()
                && physical_area > 0.0
                && oriented_area.is_finite()
                && oriented_area > 0.0
                && polyline.include_cdt_centroid(centroid)
        })
        .map(|(tri, _)| {
            let idcs = array![i => vmap[&tri[i].fix()]; 3];
            array![i => [idcs[i], idcs[i], idcs[i]].into(); 3]
        })
        .collect();
    refine_tangent_metric(&mut tri_faces, &uv_coords, &constraints, surface, 1000)?;
    // Distinct UV representatives of one source vertex cannot form a physical edge.
    tri_faces.retain(|triangle| !(0..3).any(|i| {
        source_vertices.get(&triangle[i].pos).is_some_and(|source| {
            source_vertices.get(&triangle[(i + 1) % 3].pos) == Some(source)
        })
    }));
    Ok(PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords,
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    ))
}

fn ordered_edge(a: usize, b: usize) -> (usize, usize) { (a.min(b), a.max(b)) }

/// Changes only unconstrained diagonals. The source parameters and positions never move.
/// A varying metric need not converge, so exhaustion is an explicit face failure.
fn refine_tangent_metric(
    triangles: &mut [[StandardVertex; 3]],
    uv: &[Vector2],
    constraints: &std::collections::BTreeSet<(usize, usize)>,
    surface: &impl ParametricSurface3D,
    max_sweeps: usize,
) -> std::result::Result<(), MetricRefinementError> {
    for _ in 0..max_sweeps {
        let mut adjacency = std::collections::BTreeMap::<_, Vec<_>>::new();
        for (i, t) in triangles.iter().enumerate() {
            for k in 0..3 {
                adjacency
                    .entry(ordered_edge(t[k].pos, t[(k + 1) % 3].pos))
                    .or_default()
                    .push((i, k));
            }
        }
        let mut used = vec![false; triangles.len()];
        let mut changed = false;
        for (edge, pair) in &adjacency {
            if pair.len() > 2 {
                return Err(MetricRefinementError::InvalidTopology);
            }
            if pair.len() != 2 || constraints.contains(edge) {
                continue;
            }
            let (i, k) = pair[0];
            let (j, l) = pair[1];
            if used[i] || used[j] {
                continue;
            }
            let t = triangles[i];
            let other = triangles[j];
            let (a, b, c, d) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3], other[(l + 2) % 3]);
            if other[l].pos != b.pos || other[(l + 1) % 3].pos != a.pos {
                return Err(MetricRefinementError::InvalidTopology);
            }
            if adjacency.contains_key(&ordered_edge(c.pos, d.pos)) {
                continue;
            }
            let proposed = [[c, d, b], [d, c, a]];
            // Only accept confidently convex UV quadrilaterals. Ambiguous roundoff
            // leaves the existing valid diagonal untouched; it never removes a face.
            let positive = |t: [StandardVertex; 3]| {
                let [a, b, c] = t.map(|v| uv[v.pos]);
                let x = (b.x - a.x) * (c.y - a.y);
                let y = (b.y - a.y) * (c.x - a.x);
                x.is_finite() && y.is_finite() && x - y > 8.0 * f64::EPSILON * (x.abs() + y.abs())
            };
            if !proposed.iter().all(|t| positive(*t)) {
                continue;
            }
            let center = uv[a.pos] * 0.25 + uv[b.pos] * 0.25 + uv[c.pos] * 0.25 + uv[d.pos] * 0.25;
            if !center.x.is_finite() || !center.y.is_finite() {
                return Err(MetricRefinementError::InvalidMetric);
            }
            let du = surface.derivative_u(center.x, center.y);
            let dv = surface.derivative_v(center.x, center.y);
            let scale =
                du.x.abs()
                    .max(du.y.abs())
                    .max(du.z.abs())
                    .max(dv.x.abs())
                    .max(dv.y.abs())
                    .max(dv.z.abs());
            if ![du.x, du.y, du.z, dv.x, dv.y, dv.z]
                .iter()
                .all(|v| v.is_finite())
                || scale == 0.0
            {
                return Err(MetricRefinementError::InvalidMetric);
            }
            let du = du / scale;
            let dv = dv / scale;
            let determinant = du.cross(dv).magnitude2();
            if !determinant.is_finite() || determinant <= 0.0 {
                return Err(MetricRefinementError::InvalidMetric);
            }
            let tangent = |from: StandardVertex, to: StandardVertex| {
                let delta = uv[to.pos] - uv[from.pos];
                du * delta.x + dv * delta.y
            };
            let angle =
                |a: Vector3, b: Vector3| -> std::result::Result<f64, MetricRefinementError> {
                    let lengths = a.magnitude() * b.magnitude();
                    if !lengths.is_finite() || lengths <= 0.0 {
                        return Err(MetricRefinementError::InvalidMetric);
                    }
                    let cosine = a.dot(b) / lengths;
                    if !cosine.is_finite() {
                        return Err(MetricRefinementError::InvalidMetric);
                    }
                    Ok(cosine.clamp(-1.0, 1.0).acos())
                };
            let opposite =
                angle(tangent(c, a), tangent(c, b))? + angle(tangent(d, a), tangent(d, b))?;
            // This numerical tie guard prevents cocircular roundoff oscillation;
            // it is not a geometric acceptance tolerance.
            if opposite > std::f64::consts::PI + 1e-10 {
                triangles[i] = proposed[0];
                triangles[j] = proposed[1];
                used[i] = true;
                used[j] = true;
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
    }
    Err(MetricRefinementError::NonConvergent)
}

#[cfg(test)]
mod cdt_parameter_winding_tests {
    use super::*;
    use crate::tessellation::triangulation::boundary::SurfacePoint;
    use monstertruck_geometry::prelude::Sphere;

    fn endpoint_triangle(
        surface: &impl ParametricSurface3D,
        uv: [Point2; 3],
        points: [Point3; 3],
        ids: [usize; 3],
    ) -> PolygonMesh {
        let mut boundary = PolyBoundary {
            loops: vec![uv.into_iter().zip(points).map(SurfacePoint::from).collect()],
            uv_min: Point2::new(uv.iter().map(|p|p.x).fold(f64::INFINITY,f64::min),uv.iter().map(|p|p.y).fold(f64::INFINITY,f64::min)),
            uv_max: Point2::new(uv.iter().map(|p|p.x).fold(f64::NEG_INFINITY,f64::max),uv.iter().map(|p|p.y).fold(f64::NEG_INFINITY,f64::max)),
            valid: true,
        };
        boundary.bind_source_vertices(points.into_iter().zip(ids));
        let mut cdt = Cdt::new();
        let mut boundary_map = HashMap::default();
        boundary.insert_to(&mut cdt, &mut boundary_map);
        triangulation_into_polymesh(cdt.vertices(), cdt.inner_faces(), surface, &boundary, &boundary_map).unwrap()
    }

    #[test]
    fn source_endpoint_identity_preserves_distinct_one_ulp_vertices_at_huge_origin() {
        use monstertruck_geometry::prelude::Plane;
        let base = 1e12_f64;
        let next = f64::from_bits(base.to_bits() + 1);
        let points = [Point3::new(base,base,base), Point3::new(next,base,base), Point3::new(base,next,base)];
        let surface = Plane::new(points[0],points[1],points[2]);
        let uv = [Point2::new(0.0,0.0),Point2::new(1.0,0.0),Point2::new(0.0,1.0)];
        let mesh = endpoint_triangle(&surface,uv,points,[0,1,2]);
        assert_eq!(mesh.faces().triangle_iter().count(),1);
        assert!(points.iter().all(|point| mesh.positions().contains(point)));
        assert!((points[1]-points[0]).cross(points[2]-points[0]).magnitude2()>0.0);
    }

    #[test]
    fn source_endpoint_identity_keeps_thin_tiny_and_distinct_near_pole_facets() {
        use monstertruck_geometry::prelude::Plane;
        let points = [Point3::new(100.0,-200.0,300.0),Point3::new(100.001,-200.0,300.0),Point3::new(100.0,-200.0+1e-10,300.0)];
        let surface = Plane::new(points[0],points[1],points[2]);
        let uv = [Point2::new(0.0,0.0),Point2::new(1.0,0.0),Point2::new(0.0,1.0)];
        assert_eq!(endpoint_triangle(&surface,uv,points,[0,1,2]).faces().triangle_iter().count(),1);
        for radius in [1e-200,1.0,1000.0] {
            let sphere = Sphere::new(Point3::origin(),radius);
            let uv = [Point2::new(0.2,0.2),Point2::new(0.4,0.2),Point2::new(0.3,0.4)];
            let points = uv.map(|point| sphere.subs(point.x,point.y));
            assert_eq!(endpoint_triangle(&sphere,uv,points,[0,1,2]).faces().triangle_iter().count(),1);
        }
        let sphere = Sphere::new(Point3::origin(),1000.0);
        let uv = [Point2::new(5e-7,1.0),Point2::new(8e-7,1.0),Point2::new(5e-7,2.0)];
        let points = uv.map(|point| sphere.subs(point.x,point.y));
        assert_eq!(endpoint_triangle(&sphere,uv,points,[0,1,2]).faces().triangle_iter().count(),1);
    }

    #[derive(Clone)]
    struct BentSheet;
    impl ParametricSurface for BentSheet {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 { Point3::new(100.0 * u, v, 2000.0 * u * u) }
        fn derivative_u(&self, u: f64, _: f64) -> Vector3 { Vector3::new(100.0, 0.0, 4000.0 * u) }
        fn derivative_v(&self, _: f64, _: f64) -> Vector3 { Vector3::unit_y() }
        fn derivative_uu(&self, _: f64, _: f64) -> Vector3 { Vector3::new(0.0, 0.0, 4000.0) }
        fn derivative_uv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
        fn derivative_vv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (0, 0) => self.evaluate(u, v).to_vec(),
                (1, 0) => self.derivative_u(u, v),
                (0, 1) => self.derivative_v(u, v),
                (2, 0) => self.derivative_uu(u, v),
                _ => Vector3::zero(),
            }
        }
    }
    impl ParametricSurface3D for BentSheet {}

    fn bent_sheet_mesh(lock_diagonal: bool) -> PolygonMesh {
        let surface = BentSheet;
        let uv = [
            Point2::new(-0.01, 0.0),
            Point2::new(-0.002, -0.5),
            Point2::new(0.01, 0.0),
            Point2::new(0.002, 0.5),
        ];
        let boundary = PolyBoundary {
            loops: vec![
                uv.iter()
                    .map(|p| SurfacePoint::from((*p, surface.evaluate(p.x, p.y))))
                    .collect(),
            ],
            uv_min: Point2::new(-0.01, -0.5),
            uv_max: Point2::new(0.01, 0.5),
            valid: true,
        };
        let mut cdt = Cdt::new();
        let handles: Vec<_> = uv
            .iter()
            .map(|p| {
                let q = boundary.uv_to_cdt(*p);
                cdt.insert(SPoint2::new(q.x, q.y)).unwrap()
            })
            .collect();
        for i in 0..4 {
            cdt.add_constraint(handles[i], handles[(i + 1) % 4]);
        }
        if lock_diagonal {
            cdt.add_constraint(handles[0], handles[2]);
        }
        triangulation_into_polymesh(
            cdt.vertices(),
            cdt.inner_faces(),
            &surface,
            &boundary,
            &HashMap::default(),
        )
        .unwrap()
    }

    fn metric_test_pair() -> (Vec<[StandardVertex; 3]>, Vec<Vector2>) {
        let triangles = [[0, 1, 2], [0, 2, 3]]
            .map(|t| t.map(|v| [v, v, v].into()))
            .to_vec();
        let uv = vec![
            Vector2::new(-0.01, 0.0),
            Vector2::new(-0.002, -0.5),
            Vector2::new(0.01, 0.0),
            Vector2::new(0.002, 0.5),
        ];
        (triangles, uv)
    }

    #[test]
    fn tangent_metric_reports_exhausted_budget() {
        let (mut triangles, uv) = metric_test_pair();
        assert_eq!(
            refine_tangent_metric(&mut triangles, &uv, &Default::default(), &BentSheet, 0),
            Err(MetricRefinementError::NonConvergent)
        );
        assert_eq!(
            refine_tangent_metric(&mut triangles, &uv, &Default::default(), &BentSheet, 10),
            Ok(())
        );
    }

    #[test]
    fn tangent_metric_rejects_singular_or_nonfinite_derivatives() {
        use monstertruck_geometry::prelude::Plane;
        for coordinate in [0.0, f64::NAN, f64::INFINITY] {
            let surface = Plane::new(
                Point3::origin(),
                Point3::new(coordinate, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            );
            let (mut triangles, uv) = metric_test_pair();
            assert_eq!(
                refine_tangent_metric(&mut triangles, &uv, &Default::default(), &surface, 10),
                Err(MetricRefinementError::InvalidMetric)
            );
        }
    }

    #[test]
    fn tangent_metric_fixes_anisotropic_curved_sheet_chord() {
        let mesh = bent_sheet_mesh(false);
        assert_eq!(mesh.faces().tri_faces().len(), 2);
        let distance = mesh
            .faces()
            .tri_faces()
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| mesh.positions()[v.pos]);
                let normal = (b - a).cross(c - a).normalize();
                a.to_vec().dot(normal).abs()
            })
            .fold(f64::INFINITY, f64::min);
        assert!(
            distance < 0.1,
            "source origin lies {distance} mm from the closest triangle plane"
        );
    }

    #[test]
    fn tangent_metric_keeps_vertex_attributes_and_perimeter() {
        let (mut triangles, uv) = metric_test_pair();
        let before = triangles.clone();
        refine_tangent_metric(&mut triangles, &uv, &Default::default(), &BentSheet, 10).unwrap();
        let edge_counts = |faces: &[[StandardVertex; 3]]| {
            let mut edges = std::collections::BTreeMap::new();
            for face in faces {
                for k in 0..3 {
                    *edges
                        .entry(ordered_edge(face[k].pos, face[(k + 1) % 3].pos))
                        .or_insert(0) += 1;
                }
            }
            edges
                .into_iter()
                .filter(|(_, count)| *count == 1)
                .collect::<Vec<_>>()
        };
        assert_eq!(edge_counts(&before), edge_counts(&triangles));
        assert_eq!(triangles.len(), before.len());
        for face in &triangles {
            for vertex in face {
                assert_eq!(
                    *vertex,
                    before
                        .iter()
                        .flatten()
                        .find(|v| v.pos == vertex.pos)
                        .copied()
                        .unwrap()
                );
            }
            let [a, b, c] = face.map(|v| uv[v.pos]);
            assert!((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) > 0.0);
        }
    }

    #[test]
    fn tangent_metric_preserves_internal_source_constraint() {
        let mesh = bent_sheet_mesh(true);
        let usage = mesh
            .faces()
            .tri_faces()
            .iter()
            .filter(|t| t.iter().any(|v| v.pos == 0) && t.iter().any(|v| v.pos == 2))
            .count();
        assert_eq!(
            usage, 2,
            "the constrained diagonal belongs to both triangles"
        );
    }

    #[test]
    fn cdt_uv_order_maps_to_jacobian_then_false_face_orientation_inverts_once() {
        let surface = Sphere::new(Point3::origin(), 2.0);
        let uv = [
            Point2::new(1.2, 0.2),
            Point2::new(1.4, 0.2),
            Point2::new(1.3, 0.4),
        ];
        let boundary = PolyBoundary {
            loops: vec![
                uv.iter()
                    .map(|point| SurfacePoint::from((*point, surface.subs(point.x, point.y))))
                    .collect(),
            ],
            uv_min: Point2::new(1.2, 0.2),
            uv_max: Point2::new(1.4, 0.4),
            valid: true,
        };
        let mut cdt = Cdt::new();
        for point in uv {
            assert!(cdt.insert(SPoint2::new(point.x, point.y)).is_ok());
        }

        let mut mesh = triangulation_into_polymesh(
            cdt.vertices(),
            cdt.inner_faces(),
            &surface,
            &boundary,
            &HashMap::default(),
        )
        .unwrap();
        let face = mesh.faces().face_iter().next().expect("one CDT triangle");
        assert_eq!(mesh.faces().tri_faces().len(), 1);
        let points: [Point3; 3] = std::array::from_fn(|index| mesh.positions()[face[index].pos]);
        let parameters: [Vector2; 3] =
            std::array::from_fn(|index| mesh.uv_coords()[face[index].pos]);
        let uv_a = parameters[1] - parameters[0];
        let uv_b = parameters[2] - parameters[0];
        let uv_area = uv_a.x * uv_b.y - uv_a.y * uv_b.x;
        assert!(
            uv_area > 0.0,
            "Spade face must retain counterclockwise UV order"
        );

        let raw_cross = (points[1] - points[0]).cross(points[2] - points[0]);
        let center = Point2::new(
            (parameters[0].x + parameters[1].x + parameters[2].x) / 3.0,
            (parameters[0].y + parameters[1].y + parameters[2].y) / 3.0,
        );
        let parameter_jacobian = surface
            .derivative_u(center.x, center.y)
            .cross(surface.derivative_v(center.x, center.y));
        let parameter_normal = surface.normal(center.x, center.y);
        assert!(parameter_jacobian.dot(parameter_normal) > 0.0);
        assert!(
            raw_cross.dot(parameter_jacobian) > 0.0,
            "CCW CDT triangle should map with du x dv on an analytic sphere patch"
        );

        // STEP face orientation=false is applied once after the raw CDT mesh is built.
        mesh.invert();
        let face = mesh
            .faces()
            .face_iter()
            .next()
            .expect("one oriented triangle");
        let points: [Point3; 3] = std::array::from_fn(|index| mesh.positions()[face[index].pos]);
        let oriented_cross = (points[1] - points[0]).cross(points[2] - points[0]);
        assert!(
            oriented_cross.dot(-parameter_normal) > 0.0,
            "false face orientation should reverse the raw CDT winding exactly once"
        );
        assert!(oriented_cross.dot(parameter_normal) < 0.0);
    }

    #[test]
    fn trimmed_sphere_cdt_keeps_each_uv_face_counterclockwise() {
        let surface = Sphere::new(Point3::origin(), 2.0);
        let uv = [
            Point2::new(0.15, 0.1),
            Point2::new(2.95, 0.1),
            Point2::new(1.55, 6.1),
        ];
        let boundary = PolyBoundary {
            loops: vec![
                uv.iter()
                    .map(|point| SurfacePoint::from((*point, surface.subs(point.x, point.y))))
                    .collect(),
            ],
            uv_min: Point2::new(0.15, 0.1),
            uv_max: Point2::new(2.95, 6.1),
            valid: true,
        };
        let mesh = trimming_tessellation(
            &surface,
            &boundary,
            100.0,
            TessellationPrimitiveOptions::default(),
            usize::MAX,
        )
        .unwrap();
        assert!(!mesh.faces().tri_faces().is_empty());
        for face in mesh.faces().face_iter() {
            let uv: [Vector2; 3] = std::array::from_fn(|index| mesh.uv_coords()[face[index].pos]);
            let a = uv[1] - uv[0];
            let b = uv[2] - uv[0];
            assert!(
                a.x * b.y - a.y * b.x > 0.0,
                "trimmed CDT must preserve each counterclockwise UV face"
            );
        }
    }
}
#[cfg(test)]
mod v5_sampled_surface_tests {
    use super::*;
    use monstertruck_geometry::prelude::{
        AnalyticParameterSingularity, ParameterAxis, ParameterDivision2D, ParametricSurface,
        ParametricSurface3D,
    };
    use std::f64::consts::PI;

    #[derive(Clone, Copy, Debug)]
    struct StepSphere {
        center: Point3,
        radius: f64,
    }

    impl StepSphere {
        fn point(&self, u: f64, v: f64) -> Point3 {
            Point3::from_vec(
                self.center.to_vec()
                    + Vector3::new(
                        self.radius * v.cos() * u.cos(),
                        self.radius * v.cos() * u.sin(),
                        self.radius * v.sin(),
                    ),
            )
        }
    }

    impl ParametricSurface for StepSphere {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 { self.point(u, v) }
        fn derivative_u(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                -self.radius * v.cos() * u.sin(),
                self.radius * v.cos() * u.cos(),
                0.0,
            )
        }
        fn derivative_v(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                -self.radius * v.sin() * u.cos(),
                -self.radius * v.sin() * u.sin(),
                self.radius * v.cos(),
            )
        }
        fn derivative_uu(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                -self.radius * v.cos() * u.cos(),
                -self.radius * v.cos() * u.sin(),
                0.0,
            )
        }
        fn derivative_uv(&self, u: f64, v: f64) -> Vector3 {
            Vector3::new(
                self.radius * v.sin() * u.sin(),
                -self.radius * v.sin() * u.cos(),
                0.0,
            )
        }
        fn derivative_vv(&self, u: f64, v: f64) -> Vector3 {
            -self.point(u, v).to_vec() + self.center.to_vec()
        }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (1, 0) => self.derivative_u(u, v),
                (0, 1) => self.derivative_v(u, v),
                (2, 0) => self.derivative_uu(u, v),
                (1, 1) => self.derivative_uv(u, v),
                (0, 2) => self.derivative_vv(u, v),
                _ => Vector3::zero(),
            }
        }
        fn analytic_singularities(&self) -> Vec<AnalyticParameterSingularity> {
            vec![
                AnalyticParameterSingularity {
                    axis: ParameterAxis::V,
                    parameter: -PI / 2.0,
                },
                AnalyticParameterSingularity {
                    axis: ParameterAxis::V,
                    parameter: PI / 2.0,
                },
            ]
        }
    }
    impl ParametricSurface3D for StepSphere {
        fn normal(&self, u: f64, v: f64) -> Vector3 {
            -(self.point(u, v).to_vec() - self.center.to_vec()).normalize()
        }
    }
    impl ParameterDivision2D for StepSphere {
        fn parameter_division(&self, _: ((f64, f64), (f64, f64)), _: f64) -> (Vec<f64>, Vec<f64>) {
            (
                linspace(0.0, 2.0 * PI, 64),
                linspace(-PI / 2.0, PI / 2.0, 33),
            )
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct ThinCylinder {
        center: Point3,
        radius: f64,
    }
    impl ParametricSurface for ThinCylinder {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            Point3::from_vec(
                self.center.to_vec()
                    + Vector3::new(self.radius * u.cos(), self.radius * u.sin(), v),
            )
        }
        fn derivative_u(&self, u: f64, _: f64) -> Vector3 {
            Vector3::new(-self.radius * u.sin(), self.radius * u.cos(), 0.0)
        }
        fn derivative_v(&self, _: f64, _: f64) -> Vector3 { Vector3::unit_z() }
        fn derivative_uu(&self, u: f64, _: f64) -> Vector3 {
            Vector3::new(-self.radius * u.cos(), -self.radius * u.sin(), 0.0)
        }
        fn derivative_uv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
        fn derivative_vv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (1, 0) => self.derivative_u(u, v),
                (0, 1) => self.derivative_v(u, v),
                (2, 0) => self.derivative_uu(u, v),
                _ => Vector3::zero(),
            }
        }
    }
    impl ParametricSurface3D for ThinCylinder {}
    impl ParameterDivision2D for ThinCylinder {
        fn parameter_division(&self, _: ((f64, f64), (f64, f64)), _: f64) -> (Vec<f64>, Vec<f64>) {
            (linspace(0.0, 2.0 * PI, 64), linspace(0.0, 1.0, 5))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct AliasedOscillation;
    impl ParametricSurface for AliasedOscillation {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            Point3::new(0.0, 0.001 * (1.0 - (64.0 * u).cos()), v)
        }
        fn derivative_u(&self, u: f64, _: f64) -> Vector3 {
            Vector3::new(0.0, 0.064 * (64.0 * u).sin(), 0.0)
        }
        fn derivative_v(&self, _: f64, _: f64) -> Vector3 { Vector3::unit_z() }
        fn derivative_uu(&self, u: f64, _: f64) -> Vector3 {
            Vector3::new(0.0, 4.096 * (64.0 * u).cos(), 0.0)
        }
        fn derivative_uv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
        fn derivative_vv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (1, 0) => self.derivative_u(u, v),
                (0, 1) => self.derivative_v(u, v),
                (2, 0) => self.derivative_uu(u, v),
                _ => Vector3::zero(),
            }
        }
    }
    impl ParametricSurface3D for AliasedOscillation {}
    impl ParameterDivision2D for AliasedOscillation {
        fn parameter_division(&self, _: ((f64, f64), (f64, f64)), _: f64) -> (Vec<f64>, Vec<f64>) {
            (linspace(0.0, 2.0 * PI, 65), linspace(0.0, 1.0, 5))
        }
    }

    fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
            .collect()
    }

    fn strict_edge_failures(mesh: &PolygonMesh, center: Point3) -> (usize, usize) {
        let quantize = |p: Point3| {
            let p = p.to_vec() - center.to_vec();
            (
                (p.x * 1.0e7).round() as i64,
                (p.y * 1.0e7).round() as i64,
                (p.z * 1.0e7).round() as i64,
            )
        };
        let mut edges =
            std::collections::HashMap::<((i64, i64, i64), (i64, i64, i64)), (usize, i64)>::new();
        let mut degenerate = 0;
        for tri in mesh.faces().triangle_iter() {
            let points = tri.map(|v| mesh.positions()[v.pos]);
            for i in 0..3 {
                let a = quantize(points[i]);
                let b = quantize(points[(i + 1) % 3]);
                if a == b {
                    degenerate += 1;
                    continue;
                }
                let (key, dir) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let entry = edges.entry(key).or_default();
                entry.0 += 1;
                entry.1 += dir;
            }
        }
        (
            edges
                .values()
                .filter(|(uses, dir)| *uses != 2 || *dir != 0)
                .count(),
            degenerate,
        )
    }

    fn signed_volume(mesh: &PolygonMesh, center: Point3) -> f64 {
        mesh.faces()
            .triangle_iter()
            .map(|tri| {
                let p = tri.map(|v| mesh.positions()[v.pos].to_vec() - center.to_vec());
                p[0].dot(p[1].cross(p[2])) / 6.0
            })
            .sum()
    }

    #[test]
    fn translated_and_scaled_spheres_keep_full_closed_mesh_and_outward_normals() {
        for (center, radius) in [
            (Point3::origin(), 5.0),
            (Point3::new(1.0e6, -2.0e6, 3.0e6), 0.25),
            (Point3::new(1.0e8, -3.0e8, 5.0e8), 5.0),
        ] {
            let surface = StepSphere { center, radius };
            let mut mesh = untrimmed_tessellation(
                &surface,
                ((0.0, 2.0 * PI), (-PI / 2.0, PI / 2.0)),
                0.02,
                TessellationPrimitiveMode::Triangles,
            );
            mesh.invert(); // This fixture's STEP face has orientation=false.
            assert!(mesh.faces().triangle_iter().count() > 100);
            assert_eq!(strict_edge_failures(&mesh, center), (0, 0));
            let mut inward = 0;
            for tri in mesh.faces().triangle_iter() {
                let p = tri.map(|v| mesh.positions()[v.pos]);
                let normal = (p[1] - p[0]).cross(p[2] - p[0]);
                let centroid =
                    (p[0].to_vec() + p[1].to_vec() + p[2].to_vec()) / 3.0 - center.to_vec();
                if normal.dot(centroid) <= 0.0 {
                    inward += 1;
                }
                for vertex in tri {
                    let n = mesh.normals()[vertex.nor.unwrap()];
                    let radial = mesh.positions()[vertex.pos].to_vec() - center.to_vec();
                    assert!(
                        n.x.is_finite()
                            && n.y.is_finite()
                            && n.z.is_finite()
                            && n.dot(radial) > 0.0
                    );
                }
            }
            assert_eq!(inward, 0);
            let volume = signed_volume(&mesh, center);
            let expected = 4.0 * PI * radius.powi(3) / 3.0;
            println!(
                "v5_sphere center={center:?} radius={radius} triangles={} invalid={} degenerate={} signed_volume={volume:.12} expected={expected:.12} rel_error={:.6e}",
                mesh.faces().triangle_iter().count(),
                strict_edge_failures(&mesh, center).0,
                strict_edge_failures(&mesh, center).1,
                (volume - expected).abs() / expected
            );
            assert!(
                (volume - expected).abs() / expected < 0.02,
                "r={radius} volume={volume}"
            );
        }
    }

    #[test]
    fn declared_sphere_poles_preserve_interior_quads() {
        let surface = StepSphere {
            center: Point3::origin(),
            radius: 5.0,
        };
        let mesh = untrimmed_tessellation(
            &surface,
            ((0.0, 2.0 * PI), (-PI / 2.0, PI / 2.0)),
            0.02,
            TessellationPrimitiveMode::IsoQuads,
        );
        let triangles = mesh.faces().tri_faces().len();
        let quads = mesh.faces().quad_faces().len();
        println!("v6_sphere_isoquads triangles={triangles} quads={quads}");
        assert_eq!(triangles, 2 * 63);
        assert_eq!(quads, 63 * 30);
        assert!(
            mesh.faces()
                .face_iter()
                .all(|face| face.len() == 3 || face.len() == 4)
        );
        assert!(mesh.faces().face_iter().all(|face| {
            let points = face
                .iter()
                .map(|vertex| mesh.positions()[vertex.pos])
                .collect::<Vec<_>>();
            if face.len() == 3 {
                (points[1] - points[0])
                    .cross(points[2] - points[0])
                    .magnitude2()
                    > 0.0
            } else {
                true
            }
        }));
    }

    #[test]
    fn translated_thin_cylinder_keeps_nonzero_u_rows() {
        let center = Point3::new(1.0e12, -1.0e12, 1.0e12);
        let surface = ThinCylinder {
            center,
            radius: 0.005,
        };
        let mesh = untrimmed_tessellation(
            &surface,
            ((0.0, 2.0 * PI), (0.0, 1.0)),
            0.02,
            TessellationPrimitiveMode::Triangles,
        );
        let nv = 5;
        let row = (0..64)
            .map(|i| mesh.positions()[i * nv].to_vec())
            .collect::<Vec<_>>();
        let distinct = row
            .iter()
            .map(|p| ((p.x - center.x) * 1.0e7).round() as i64)
            .collect::<std::collections::HashSet<_>>()
            .len();
        println!(
            "v5_thin_cylinder distinct_quantized_x={distinct} triangles={}",
            mesh.faces().triangle_iter().count()
        );
        assert!(
            distinct > 20,
            "thin nonzero row was canonicalized: distinct_x={distinct}"
        );
        assert_eq!(mesh.faces().triangle_iter().count(), 63 * 4 * 2);
        assert!(mesh.faces().triangle_iter().all(|tri| {
            let p = tri.map(|v| mesh.positions()[v.pos]);
            (p[1] - p[0]).cross(p[2] - p[0]).magnitude2() > 0.0
        }));
    }

    #[test]
    fn oscillation_aliasing_first_derivative_does_not_erase_sampled_cells() {
        let surface = AliasedOscillation;
        let mesh = untrimmed_tessellation(
            &surface,
            ((0.0, 2.0 * PI), (0.0, 1.0)),
            0.02,
            TessellationPrimitiveMode::Triangles,
        );
        assert_eq!(mesh.faces().triangle_iter().count(), 64 * 4 * 2);
        assert!(surface.derivative_uu(0.0, 0.5).magnitude() > 1.0);
        let (invalid, degenerate) = strict_edge_failures(&mesh, Point3::origin());
        println!(
            "v6_aliased_oscillation triangles={} invalid={invalid} degenerate={degenerate}",
            mesh.faces().triangle_iter().count()
        );
        assert!(surface.analytic_singularities().is_empty());
        assert!(
            invalid > 0 && degenerate > 0,
            "aliased feature must remain a visible strict-mesh failure"
        );
        let nu = 65;
        let nv = 5;
        let udiv = linspace(0.0, 2.0 * PI, nu);
        let vdiv = linspace(0.0, 1.0, nv);
        let mut raw_positions = udiv
            .iter()
            .flat_map(|u| vdiv.iter().map(|v| surface.evaluate(*u, *v)))
            .collect::<Vec<_>>();
        let original_positions = raw_positions.clone();
        let (collapsed_u, collapsed_v) =
            canonicalize_declared_singularities(&surface, &udiv, &vdiv, &mut raw_positions, nv);
        assert!(!collapsed_u.iter().any(|collapsed| *collapsed));
        assert!(!collapsed_v.iter().any(|collapsed| *collapsed));
        assert_eq!(
            raw_positions, original_positions,
            "default-none capability must leave generic samples unchanged"
        );
        let mut actual_faces = mesh
            .faces()
            .triangle_iter()
            .map(|face| {
                let mut ids = face.map(|vertex| vertex.pos);
                ids.sort_unstable();
                ids
            })
            .collect::<Vec<_>>();
        let mut expected_faces = Vec::new();
        for i in 0..nu - 1 {
            for j in 0..nv - 1 {
                let a = i * nv + j;
                let b = a + 1;
                let d = (i + 1) * nv + j;
                let c = d + 1;
                let mut first = [a, b, c];
                let mut second = [a, c, d];
                first.sort_unstable();
                second.sort_unstable();
                expected_faces.push(first);
                expected_faces.push(second);
            }
        }
        actual_faces.sort_unstable();
        expected_faces.sort_unstable();
        assert_eq!(
            actual_faces, expected_faces,
            "a generic surface with no declared singularity must keep the ordinary sampled grid"
        );
    }
}

#[cfg(test)]
mod source_wedge_regression {
    use super::*;
    use boundary::SurfacePoint;
    use monstertruck_geometry::prelude::Plane;

    fn wedge_mesh_on(surface: &impl ParametricSurface3D, uv: [Point2; 5]) -> PolygonMesh {
        let uv_min = uv
            .iter()
            .fold(uv[0], |p, q| Point2::new(p.x.min(q.x), p.y.min(q.y)));
        let uv_max = uv
            .iter()
            .fold(uv[0], |p, q| Point2::new(p.x.max(q.x), p.y.max(q.y)));
        let boundary = PolyBoundary {
            loops: vec![
                uv.iter()
                    .map(|p| SurfacePoint::from((*p, surface.evaluate(p.x, p.y))))
                    .collect(),
            ],
            uv_min,
            uv_max,
            valid: true,
        };
        let mut cdt = Cdt::new();
        let mut map = HashMap::default();
        let handles: Vec<_> = uv
            .iter()
            .map(|p| {
                let q = boundary.uv_to_cdt(*p);
                let h = cdt.insert(SPoint2::new(q.x, q.y)).unwrap();
                map.insert(h, SurfacePoint::from((*p, surface.evaluate(p.x, p.y))));
                h
            })
            .collect();
        for i in 0..handles.len() {
            cdt.add_constraint(handles[i], handles[(i + 1) % handles.len()]);
        }
        assert_eq!(
            cdt.inner_faces().count(),
            4,
            "fixture includes three interior triangles and one outside its concave boundary"
        );
        triangulation_into_polymesh(cdt.vertices(), cdt.inner_faces(), surface, &boundary, &map)
            .unwrap()
    }

    fn wedge_uv(height: f64) -> [Point2; 5] {
        [
            Point2::new(0.0, 0.0),
            Point2::new(0.01, 0.0),
            Point2::new(0.01, height),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ]
    }

    fn wedge_mesh(height: f64) -> PolygonMesh {
        let surface = Plane::new(
            Point3::origin(),
            Point3::new(1e7, 0.0, 0.0),
            Point3::new(0.0, 1e7, 0.0),
        );
        wedge_mesh_on(&surface, wedge_uv(height))
    }

    #[test]
    fn resolved_wedge_survives_chart_rescaling() {
        for scale in [1e-8, 1.0, 1e8] {
            let surface = Plane::new(
                Point3::origin(),
                Point3::new(1e7 / scale, 0.0, 0.0),
                Point3::new(0.0, 1e7 / scale, 0.0),
            );
            let uv = wedge_uv(1e-12).map(|p| Point2::from_vec(p.to_vec() * scale));
            let mesh = wedge_mesh_on(&surface, uv);
            assert_eq!(
                mesh.faces().triangle_iter().count(),
                3,
                "chart scale {scale}"
            );
        }
    }

    #[test]
    fn resolved_narrow_trim_survives_on_nonlinear_sphere() {
        let surface = monstertruck_geometry::prelude::Sphere::new(Point3::origin(), 1e9);
        let uv = wedge_uv(1e-12).map(|p| p + Vector2::new(0.5, 0.0));
        let mesh = wedge_mesh_on(&surface, uv);
        assert_eq!(mesh.faces().triangle_iter().count(), 3);
    }

    #[test]
    fn a_collapsed_physical_boundary_stays_filtered() {
        let surface = monstertruck_geometry::prelude::Sphere::new(Point3::origin(), 1.0);
        let uv = [
            Point2::new(0.0, 0.0),
            Point2::new(1e-8, 0.0),
            Point2::new(0.0, 1e-5),
        ];
        let boundary = PolyBoundary {
            loops: vec![
                uv.iter()
                    .map(|p| SurfacePoint::from((*p, surface.evaluate(p.x, p.y))))
                    .collect(),
            ],
            uv_min: Point2::new(0.0, 0.0),
            uv_max: Point2::new(1.0, 1.0),
            valid: true,
        };
        let mut cdt = Cdt::new();
        let mut map = HashMap::default();
        let handles: Vec<_> = uv
            .iter()
            .map(|p| {
                let h = cdt.insert(SPoint2::new(p.x, p.y)).unwrap();
                map.insert(h, SurfacePoint::from((*p, surface.evaluate(p.x, p.y))));
                h
            })
            .collect();
        for i in 0..3 {
            cdt.add_constraint(handles[i], handles[(i + 1) % 3]);
        }
        assert_eq!(cdt.inner_faces().count(), 1);
        let mesh = triangulation_into_polymesh(
            cdt.vertices(),
            cdt.inner_faces(),
            &surface,
            &boundary,
            &map,
        )
        .unwrap();
        assert_eq!(
            mesh.faces().triangle_iter().count(),
            0,
            "the positive UV triangle has a collapsed source edge at the pole"
        );
    }
    #[test]
    fn retains_source_wedge_with_centroid_near_its_boundary() {
        let mesh = wedge_mesh(1e-7);
        assert_eq!(
            mesh.faces().triangle_iter().count(),
            3,
            "a near-boundary centroid is still inside its source polygon"
        );
    }
    #[test]
    fn retains_nonzero_physical_boundary_wedge_with_small_uv_area() {
        let mesh = wedge_mesh(1e-12);
        assert_eq!(
            mesh.faces().triangle_iter().count(),
            3,
            "UV area is not a bound on physical source area"
        );
    }
}
