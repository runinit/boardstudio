use lyon_path::{Path, math::point};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub colors: Option<Vec<[f32; 3]>>,
}

pub(crate) struct BoardContour<'a> {
    pub points: &'a [[f32; 2]],
    pub hole: bool,
}

pub(crate) fn polygon_area(points: &[[f32; 2]]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point[0] * next[1] - next[0] * point[1]
        })
        .sum::<f32>()
        .abs()
        * 0.5
}

struct ClipRing {
    points: Vec<[f32; 2]>,
    bounds: [f32; 4],
}

fn bounds(points: &[[f32; 2]]) -> [f32; 4] {
    points.iter().fold(
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ],
        |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        },
    )
}
fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

pub(crate) struct SurfaceClipper {
    outlines: Vec<ClipRing>,
    holes: Vec<ClipRing>,
}
impl SurfaceClipper {
    pub(crate) fn new(contours: &[BoardContour<'_>], holes: &[Vec<[f32; 2]>]) -> Self {
        let ring = |points: &[[f32; 2]]| ClipRing {
            points: points.to_vec(),
            bounds: bounds(points),
        };
        let outlines = contours
            .iter()
            .filter(|c| !c.hole)
            .map(|c| ring(c.points))
            .collect();
        let mut voids = contours
            .iter()
            .filter(|c| c.hole)
            .map(|c| c.points.to_vec())
            .collect::<Vec<_>>();
        for hole in holes {
            if !voids.contains(hole) {
                voids.push(hole.clone());
            }
        }
        Self {
            outlines,
            holes: voids.iter().map(|p| ring(p)).collect(),
        }
    }
    pub(crate) fn clip(&self, mesh: &mut MeshData) -> Result<(), String> {
        use i_overlay::{
            core::{fill_rule::FillRule, overlay_rule::OverlayRule},
            float::single::SingleFloatOverlay,
        };
        if self.outlines.is_empty() {
            return Ok(());
        }
        let mut result = MeshData::default();
        for triangle in mesh.indices.as_chunks::<3>().0.iter() {
            let positions = triangle
                .iter()
                .map(|i| mesh.positions[*i as usize])
                .collect::<Vec<_>>();
            let polygon = positions.iter().map(|p| [p[0], p[1]]).collect::<Vec<_>>();
            let bbox = bounds(&polygon);
            let outlines = self
                .outlines
                .iter()
                .filter(|r| overlap(bbox, r.bounds))
                .collect::<Vec<_>>();
            if outlines.is_empty() {
                continue;
            }
            let holes = self
                .holes
                .iter()
                .filter(|r| overlap(bbox, r.bounds))
                .map(|r| r.points.clone())
                .collect::<Vec<_>>();
            let fully_inside = outlines.iter().any(|r| {
                polygon.iter().all(|p| {
                    polygon_contains(*p, &r.points) || polygon_near_edge(*p, &r.points, 0.00001)
                }) && !r.points.iter().enumerate().any(|(i, a)| {
                    (0..3).any(|j| {
                        segments_cross(
                            *a,
                            r.points[(i + 1) % r.points.len()],
                            polygon[j],
                            polygon[(j + 1) % 3],
                        )
                    })
                })
            });
            if fully_inside && holes.is_empty() {
                let start = result.positions.len() as u32;
                for index in triangle {
                    result.positions.push(mesh.positions[*index as usize]);
                    result.normals.push(mesh.normals[*index as usize]);
                }
                result.indices.extend([start, start + 1, start + 2]);
                continue;
            }
            let mut shapes = if fully_inside {
                vec![vec![polygon.clone()]]
            } else {
                polygon.overlay(
                    &outlines
                        .iter()
                        .map(|r| r.points.clone())
                        .collect::<Vec<_>>(),
                    OverlayRule::Intersect,
                    FillRule::EvenOdd,
                )
            };
            if !holes.is_empty() {
                shapes = shapes.overlay(&holes, OverlayRule::Difference, FillRule::EvenOdd);
            }
            let normal = mesh.normals[triangle[0] as usize];
            for shape in shapes {
                let contours = shape
                    .iter()
                    .enumerate()
                    .map(|(i, p)| BoardContour {
                        points: p,
                        hole: i > 0,
                    })
                    .collect::<Vec<_>>();
                let mut path = Path::builder();
                for contour in &contours {
                    if contour.points.len() < 3 {
                        continue;
                    }
                    path.begin(point(contour.points[0][0], contour.points[0][1]));
                    for p in &contour.points[1..] {
                        path.line_to(point(p[0], p[1]));
                    }
                    path.close();
                }
                let mut buffers = VertexBuffers::new();
                FillTessellator::new()
                    .tessellate_path(
                        &path.build(),
                        &FillOptions::default().with_fill_rule(FillRuleLyon::EvenOdd),
                        &mut BuffersBuilder::new(&mut buffers, |v: FillVertex<'_>| {
                            [v.position().x, v.position().y]
                        }),
                    )
                    .map_err(|error| format!("PCB surface clipping failed: {error}"))?;
                append_layer(&mut result, &buffers, positions[0][2], normal);
            }
        }
        *mesh = result;
        Ok(())
    }
}
use lyon_tessellation::FillRule as FillRuleLyon;

#[cfg(test)]
fn clip_surface_to_board(
    mesh: &mut MeshData,
    contours: &[BoardContour<'_>],
    holes: &[Vec<[f32; 2]>],
) {
    SurfaceClipper::new(contours, holes).clip(mesh).unwrap();
}

fn polygon_contains(point: [f32; 2], polygon: &[[f32; 2]]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    for (index, a) in polygon.iter().enumerate() {
        let b = polygon[(index + 1) % polygon.len()];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

fn polygon_near_edge(point: [f32; 2], polygon: &[[f32; 2]], tolerance: f32) -> bool {
    polygon.iter().enumerate().any(|(index, a)| {
        let b = polygon[(index + 1) % polygon.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length_squared = dx * dx + dy * dy;
        if length_squared <= f32::EPSILON {
            return (point[0] - a[0]).hypot(point[1] - a[1]) <= tolerance;
        }
        let t =
            (((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / length_squared).clamp(0.0, 1.0);
        let nearest = [a[0] + t * dx, a[1] + t * dy];
        (point[0] - nearest[0]).hypot(point[1] - nearest[1]) <= tolerance
    })
}

fn segments_cross(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    let cross = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let ab_c = cross(a, b, c);
    let ab_d = cross(a, b, d);
    let cd_a = cross(c, d, a);
    let cd_b = cross(c, d, b);
    ab_c * ab_d < 0.0 && cd_a * cd_b < 0.0
}

#[cfg(test)]
fn segments_intersect(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    if segments_cross(a, b, c, d) {
        return true;
    }
    let on_segment = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
        let cross = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
        cross.abs() <= 1e-6
            && r[0] >= p[0].min(q[0]) - 1e-6
            && r[0] <= p[0].max(q[0]) + 1e-6
            && r[1] >= p[1].min(q[1]) - 1e-6
            && r[1] <= p[1].max(q[1]) + 1e-6
    };
    on_segment(a, b, c) || on_segment(a, b, d) || on_segment(c, d, a) || on_segment(c, d, b)
}

pub(crate) fn board_mesh(
    contours: &[BoardContour<'_>],
    thickness: f32,
) -> Result<MeshData, String> {
    let mut path = Path::builder();
    for contour in contours.iter().filter(|contour| contour.points.len() >= 3) {
        path.begin(point(contour.points[0][0], contour.points[0][1]));
        for point2 in contour.points.iter().skip(1) {
            path.line_to(point(point2[0], point2[1]));
        }
        path.close();
    }
    let path = path.build();
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            &path,
            &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
                [vertex.position().x, vertex.position().y]
            }),
        )
        .map_err(|error| format!("PCB outline tessellation failed: {error}"))?;
    let mut mesh = MeshData::default();
    append_layer(&mut mesh, &buffers, thickness, [0.0, 0.0, 1.0]);
    append_layer(&mut mesh, &buffers, 0.0, [0.0, 0.0, -1.0]);
    for contour in contours.iter().filter(|contour| contour.points.len() >= 2) {
        let orientation = winding(contour.points);
        for index in 0..contour.points.len() {
            let a = contour.points[index];
            let b = contour.points[(index + 1) % contour.points.len()];
            let direction = if contour.hole { -1.0 } else { 1.0 } * orientation;
            let normal_length = (b[0] - a[0]).hypot(b[1] - a[1]).max(f32::EPSILON);
            append_quad(
                &mut mesh,
                [a[0], a[1], 0.0],
                [b[0], b[1], 0.0],
                [b[0], b[1], thickness],
                [a[0], a[1], thickness],
                [
                    (b[1] - a[1]) * direction / normal_length,
                    (a[0] - b[0]) * direction / normal_length,
                    0.0,
                ],
            );
        }
    }
    Ok(mesh)
}

pub(crate) fn surface_mesh(
    points: &[[f32; 2]],
    z: f32,
    normal: [f32; 3],
) -> Result<MeshData, String> {
    if points.len() < 3 {
        return Ok(MeshData::default());
    }
    let mut path = Path::builder();
    path.begin(point(points[0][0], points[0][1]));
    for point2 in points.iter().skip(1) {
        path.line_to(point(point2[0], point2[1]));
    }
    path.close();
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            &path.build(),
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
                [vertex.position().x, vertex.position().y]
            }),
        )
        .map_err(|error| format!("PCB surface tessellation failed: {error}"))?;
    let mut mesh = MeshData::default();
    append_layer(&mut mesh, &buffers, z, normal);
    Ok(mesh)
}

pub(crate) fn stroke_mesh(
    points: &[[f32; 2]],
    width: f32,
    z: f32,
    normal: [f32; 3],
) -> Result<MeshData, String> {
    if points.len() < 2 {
        return Ok(MeshData::default());
    }
    let mut path = Path::builder();
    path.begin(point(points[0][0], points[0][1]));
    for point2 in points.iter().skip(1) {
        path.line_to(point(point2[0], point2[1]));
    }
    path.end(false);
    let path = path.build();
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &StrokeOptions::default().with_line_width(width.max(0.01)),
            &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex<'_, '_>| {
                [vertex.position().x, vertex.position().y]
            }),
        )
        .map_err(|error| format!("PCB stroke tessellation failed: {error}"))?;
    let mut mesh = MeshData::default();
    append_layer(&mut mesh, &buffers, z, normal);
    Ok(mesh)
}

fn append_layer(
    mesh: &mut MeshData,
    buffers: &VertexBuffers<[f32; 2], u32>,
    z: f32,
    normal: [f32; 3],
) {
    let offset = mesh.positions.len() as u32;
    mesh.positions.extend(
        buffers
            .vertices
            .iter()
            .map(|position| [position[0], position[1], z]),
    );
    mesh.normals
        .extend(std::iter::repeat_n(normal, buffers.vertices.len()));
    for triangle in buffers.indices.as_chunks::<3>().0.iter() {
        let a = buffers.vertices[triangle[0] as usize];
        let b = buffers.vertices[triangle[1] as usize];
        let c = buffers.vertices[triangle[2] as usize];
        let cross_z = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        let agrees = cross_z * normal[2] > 0.0;
        mesh.indices.extend_from_slice(&[
            offset + triangle[0],
            offset + if agrees { triangle[1] } else { triangle[2] },
            offset + if agrees { triangle[2] } else { triangle[1] },
        ]);
    }
}

fn append_quad(
    mesh: &mut MeshData,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    normal: [f32; 3],
) {
    let offset = mesh.positions.len() as u32;
    mesh.positions.extend_from_slice(&[a, b, c, d]);
    mesh.normals.extend_from_slice(&[normal; 4]);
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let dot = (ab[1] * ac[2] - ab[2] * ac[1]) * normal[0]
        + (ab[2] * ac[0] - ab[0] * ac[2]) * normal[1]
        + (ab[0] * ac[1] - ab[1] * ac[0]) * normal[2];
    if dot > 0.0 {
        mesh.indices.extend_from_slice(&[
            offset,
            offset + 1,
            offset + 2,
            offset,
            offset + 2,
            offset + 3,
        ]);
    } else {
        mesh.indices.extend_from_slice(&[
            offset,
            offset + 2,
            offset + 1,
            offset,
            offset + 3,
            offset + 2,
        ]);
    }
}

fn winding(points: &[[f32; 2]]) -> f32 {
    let area = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point[0] * next[1] - next[0] * point[1]
        })
        .sum::<f32>();
    if area < 0.0 { -1.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_dot_normal(mesh: &MeshData, triangle: [usize; 3]) -> f32 {
        let a = mesh.positions[triangle[0]];
        let b = mesh.positions[triangle[1]];
        let c = mesh.positions[triangle[2]];
        let normal = mesh.normals[triangle[0]];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        cross[0] * normal[0] + cross[1] * normal[1] + cross[2] * normal[2]
    }

    #[test]
    fn board_caps_face_their_normals() {
        let outline = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let mesh = board_mesh(
            &[BoardContour {
                points: &outline,
                hole: false,
            }],
            1.6,
        )
        .unwrap();

        for triangle in mesh.indices.as_chunks::<3>().0.iter() {
            assert!(
                triangle_dot_normal(
                    &mesh,
                    [
                        triangle[0] as usize,
                        triangle[1] as usize,
                        triangle[2] as usize
                    ]
                ) > 0.0
            );
        }
    }

    #[test]
    fn hole_wall_normals_face_into_the_opening() {
        let outline = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let hole = [[5.0, 5.0], [8.0, 5.0], [8.0, 8.0], [5.0, 8.0]];
        let mesh = board_mesh(
            &[
                BoardContour {
                    points: &outline,
                    hole: false,
                },
                BoardContour {
                    points: &hole,
                    hole: true,
                },
            ],
            1.6,
        )
        .unwrap();
        let lower_hole_edge = mesh.positions.len() - 16;

        assert_eq!(mesh.normals[lower_hole_edge], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn board_mesh_preserves_concave_notch_and_hole() {
        let outline = [
            [0.0, 0.0],
            [6.0, 0.0],
            [6.0, 2.0],
            [2.0, 2.0],
            [2.0, 6.0],
            [0.0, 6.0],
        ];
        let hole = [[0.5, 0.5], [0.5, 1.5], [1.5, 1.5], [1.5, 0.5]];
        let mesh = board_mesh(
            &[
                BoardContour {
                    points: &outline,
                    hole: false,
                },
                BoardContour {
                    points: &hole,
                    hole: true,
                },
            ],
            1.2,
        )
        .unwrap();
        let contains_on_top = |point: [f32; 2]| {
            mesh.indices.chunks_exact(3).any(|triangle| {
                let vertices = [triangle[0], triangle[1], triangle[2]]
                    .map(|index| mesh.positions[index as usize]);
                if !vertices.iter().all(|vertex| (vertex[2] - 1.2).abs() < 1e-5) {
                    return false;
                }
                let cross = |a: [f32; 3], b: [f32; 3]| {
                    (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0])
                };
                let sides = [
                    cross(vertices[0], vertices[1]),
                    cross(vertices[1], vertices[2]),
                    cross(vertices[2], vertices[0]),
                ];
                sides.iter().all(|side| *side >= -1e-5) || sides.iter().all(|side| *side <= 1e-5)
            })
        };
        assert!(contains_on_top([1.0, 4.0]));
        assert!(!contains_on_top([4.0, 4.0]));
        assert!(!contains_on_top([1.0, 1.0]));
    }

    #[test]
    fn external_wall_normals_face_away_from_the_board() {
        let outline = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let mesh = board_mesh(
            &[BoardContour {
                points: &outline,
                hole: false,
            }],
            1.6,
        )
        .unwrap();
        let lower_outer_edge = mesh.positions.len() - 16;

        assert_eq!(mesh.normals[lower_outer_edge], [0.0, -1.0, 0.0]);
        assert!(
            triangle_dot_normal(
                &mesh,
                [
                    mesh.indices[mesh.indices.len() - 6] as usize,
                    mesh.indices[mesh.indices.len() - 5] as usize,
                    mesh.indices[mesh.indices.len() - 4] as usize,
                ]
            ) > 0.0
        );
    }

    #[test]
    fn back_surface_triangles_follow_the_rear_normal() {
        let polygon = [[0.0, 0.0], [5.0, 0.0], [5.0, 4.0], [0.0, 4.0]];
        let mesh = surface_mesh(&polygon, -0.001, [0.0, 0.0, -1.0]).unwrap();

        for triangle in mesh.indices.as_chunks::<3>().0.iter() {
            assert!(
                triangle_dot_normal(
                    &mesh,
                    [
                        triangle[0] as usize,
                        triangle[1] as usize,
                        triangle[2] as usize
                    ]
                ) > 0.0
            );
        }
    }

    #[test]
    fn surface_triangles_are_clipped_at_notches_and_drills() {
        let notched_outline = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [6.0, 10.0],
            [6.0, 8.0],
            [4.0, 8.0],
            [4.0, 10.0],
            [0.0, 10.0],
        ];
        let mut crosses_notch = MeshData {
            positions: vec![[1.0, 9.0, 1.0], [9.0, 9.0, 1.0], [5.0, 1.0, 1.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            colors: None,
        };
        clip_surface_to_board(
            &mut crosses_notch,
            &[BoardContour {
                points: &notched_outline,
                hole: false,
            }],
            &[],
        );
        assert!(!crosses_notch.indices.is_empty());
        assert!(crosses_notch.indices.as_chunks::<3>().0.iter().all(|t| {
            let p = t
                .iter()
                .map(|i| crosses_notch.positions[*i as usize])
                .fold([0., 0.], |sum, p| [sum[0] + p[0] / 3., sum[1] + p[1] / 3.]);
            polygon_contains(p, &notched_outline)
        }));

        let outline = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let drill = vec![[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0]];
        let mut crosses_drill = MeshData {
            positions: vec![[1.0, 1.0, 1.0], [8.0, 1.0, 1.0], [1.0, 8.0, 1.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            colors: None,
        };
        clip_surface_to_board(
            &mut crosses_drill,
            &[BoardContour {
                points: &outline,
                hole: false,
            }],
            &[drill],
        );
        assert!(!crosses_drill.indices.is_empty());
        assert!(crosses_drill.indices.as_chunks::<3>().0.iter().all(|t| {
            let p = t
                .iter()
                .map(|i| crosses_drill.positions[*i as usize])
                .fold([0., 0.], |sum, p| [sum[0] + p[0] / 3., sum[1] + p[1] / 3.]);
            !(p[0] > 4. && p[0] < 6. && p[1] > 4. && p[1] < 6.)
        }));
    }

    #[test]
    fn collinear_segment_checks_require_overlapping_bounds() {
        assert!(!segments_intersect(
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [3.0, 0.0]
        ));
        assert!(segments_intersect(
            [0.0, 0.0],
            [2.0, 0.0],
            [1.0, 0.0],
            [3.0, 0.0]
        ));
    }
}

#[cfg(test)]
mod clipping_regressions {
    use super::*;
    #[test]
    fn clips_crossing_triangles_instead_of_erasing_visible_copper() {
        let outline = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let mut mesh = surface_mesh(&[[-2., 2.], [4., 2.], [4., 8.]], 1.6, [0., 0., 1.]).unwrap();
        clip_surface_to_board(
            &mut mesh,
            &[BoardContour {
                points: &outline,
                hole: false,
            }],
            &[],
        );
        assert!(!mesh.indices.is_empty());
        assert!(
            mesh.positions
                .iter()
                .all(|p| p[0] >= -0.001 && p[0] <= 10.001)
        );
    }
}

/// Unique boundary and crease edges, independent of flat tessellation diagonals.
pub(crate) fn feature_edges(mesh: &MeshData) -> Vec<[[f32; 3]; 2]> {
    use std::collections::BTreeMap;
    type Key = [i64; 3];
    let key = |p: [f32; 3]| p.map(|v| (v as f64 * 10000.0).round() as i64);
    type EdgeValue = ([f32; 3], [f32; 3], [f32; 3], bool, usize);
    let mut edges: BTreeMap<(Key, Key), EdgeValue> = BTreeMap::new();
    for triangle in mesh.indices.as_chunks::<3>().0.iter() {
        let p = [
            mesh.positions[triangle[0] as usize],
            mesh.positions[triangle[1] as usize],
            mesh.positions[triangle[2] as usize],
        ];
        let a = three_d::Vec3::from(p[1]) - three_d::Vec3::from(p[0]);
        let b = three_d::Vec3::from(p[2]) - three_d::Vec3::from(p[0]);
        use three_d::InnerSpace;
        if a.cross(b).magnitude2() < 1e-12 {
            continue;
        }
        let normal: [f32; 3] = a.cross(b).normalize().into();
        for i in 0..3 {
            let (mut a, mut b) = (p[i], p[(i + 1) % 3]);
            if key(a) > key(b) {
                std::mem::swap(&mut a, &mut b);
            }
            let entry = edges
                .entry((key(a), key(b)))
                .or_insert((a, b, normal, false, 0));
            entry.3 |= normal.iter().zip(entry.2).map(|(a, b)| a * b).sum::<f32>() < 0.866;
            entry.4 += 1;
        }
    }
    edges
        .into_values()
        .filter(|(_, _, _, crease, count)| *crease || *count == 1)
        .map(|(a, b, _, _, _)| [a, b])
        .collect()
}

#[cfg(test)]
mod feature_edge_tests {
    use super::*;

    #[test]
    fn coplanar_triangle_diagonals_are_not_cad_edges() {
        let mesh = MeshData {
            positions: vec![[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]],
            indices: vec![0, 1, 2, 0, 2, 3],
            ..MeshData::default()
        };
        assert_eq!(feature_edges(&mesh).len(), 4);
    }

    #[test]
    fn solid_box_has_twelve_feature_edges() {
        let points = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let mesh = board_mesh(
            &[BoardContour {
                points: &points,
                hole: false,
            }],
            2.,
        )
        .unwrap();
        assert_eq!(feature_edges(&mesh).len(), 12);
    }
}
