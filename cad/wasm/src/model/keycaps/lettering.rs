use super::{
    empty_mesh,
    mesh_template::{add_triangle, dish_sag},
    MeshData,
};
use boardstudio_contracts::KeycapSpec;
use lyon_path::{math::point, Path};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers,
};
use ttf_parser::{Face, OutlineBuilder};

const LEGEND_RAISE_MM: f64 = 0.03;
const LEGEND_EMBED_MM: f64 = 0.15;

#[derive(Default)]
struct Outline {
    paths: Vec<Vec<(f64, f64)>>,
    current: Vec<(f64, f64)>,
}

impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.current.push((f64::from(x), f64::from(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push((f64::from(x), f64::from(y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let Some(&(start_x, start_y)) = self.current.last() else {
            return;
        };
        for step in 1..=12 {
            let t = f64::from(step) / 12.0;
            let u = 1.0 - t;
            self.current.push((
                u * u * start_x + 2.0 * u * t * f64::from(x1) + t * t * f64::from(x),
                u * u * start_y + 2.0 * u * t * f64::from(y1) + t * t * f64::from(y),
            ));
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let Some(&(start_x, start_y)) = self.current.last() else {
            return;
        };
        for step in 1..=16 {
            let t = f64::from(step) / 16.0;
            let u = 1.0 - t;
            self.current.push((
                u * u * u * start_x
                    + 3.0 * u * u * t * f64::from(x1)
                    + 3.0 * u * t * t * f64::from(x2)
                    + t * t * t * f64::from(x),
                u * u * u * start_y
                    + 3.0 * u * u * t * f64::from(y1)
                    + 3.0 * u * t * t * f64::from(y2)
                    + t * t * t * f64::from(y),
            ));
        }
    }

    fn close(&mut self) {
        if self.current.len() > 2 {
            let mut path = std::mem::take(&mut self.current);
            if path.first() == path.last() {
                path.pop();
            }
            self.paths.push(path);
        } else {
            self.current.clear();
        }
    }
}

pub(super) fn mesh(spec: &KeycapSpec) -> Result<Option<MeshData>, String> {
    if spec.legend.trim().is_empty() {
        return Ok(None);
    }
    let face = Face::parse(include_bytes!("../../../assets/DejaVuSans.ttf"), 0)
        .map_err(|_| "Bundled legend font is invalid")?;
    let mut paths = Vec::new();
    let mut advance = 0.0;
    for character in spec.legend.chars() {
        let glyph = face
            .glyph_index(character)
            .ok_or_else(|| format!("Legend font has no glyph for {character}"))?;
        let mut outline = Outline::default();
        face.outline_glyph(glyph, &mut outline);
        outline.close();
        paths.extend(outline.paths.into_iter().map(|path| {
            path.into_iter()
                .map(|(x, y)| (x + advance, y))
                .collect::<Vec<_>>()
        }));
        advance += f64::from(face.glyph_hor_advance(glyph).unwrap_or(face.units_per_em()));
    }
    if paths.is_empty() {
        return Ok(None);
    }
    let bounds = paths.iter().flatten().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), (x, y)| {
            (min_x.min(*x), max_x.max(*x), min_y.min(*y), max_y.max(*y))
        },
    );
    let span_x = bounds.1 - bounds.0;
    let span_y = bounds.3 - bounds.2;
    if !span_x.is_finite() || !span_y.is_finite() || span_x <= 0.0 || span_y <= 0.0 {
        return Err("Legend font produced invalid outline bounds".into());
    }
    let scale = ((spec.top_size.x - 2.0) / span_x)
        .min((spec.top_size.y - 2.0) / span_y)
        .min(4.0 / span_y);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("Keycap legend does not fit on the roof".into());
    }
    let x_mid = (bounds.0 + bounds.1) / 2.0;
    let y_mid = (bounds.2 + bounds.3) / 2.0;
    let paths = paths
        .into_iter()
        .map(|path| {
            path.into_iter()
                // Cap tessellation and walls must displace the same f32 XY coordinates.
                .map(|(x, y)| {
                    [(x - x_mid) * scale, (y - y_mid) * scale]
                        .map(|coordinate| f64::from(coordinate as f32))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut path = Path::builder();
    for contour in paths.iter().filter(|contour| contour.len() >= 3) {
        path.begin(point(contour[0][0] as f32, contour[0][1] as f32));
        for vertex in contour.iter().skip(1) {
            path.line_to(point(vertex[0] as f32, vertex[1] as f32));
        }
        path.close();
    }
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            &path.build(),
            &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
                [vertex.position().x, vertex.position().y]
            }),
        )
        .map_err(|error| format!("Keycap legend tessellation failed: {error}"))?;
    if buffers.vertices.is_empty() || buffers.indices.is_empty() {
        return Err("Legend outline did not produce filled triangles".into());
    }

    let mut mesh = empty_mesh();
    for triangle in buffers.indices.as_chunks::<3>().0.iter() {
        let mut points = [0, 1, 2].map(|index| buffers.vertices[triangle[index] as usize]);
        if signed_area_f32(&points) < 0.0 {
            points.swap(1, 2);
        }
        let top = points.map(|[x, y]| {
            [
                f64::from(x),
                f64::from(y),
                surface_z(spec, f64::from(x), f64::from(y)) + LEGEND_RAISE_MM,
            ]
        });
        let bottom = points.map(|[x, y]| {
            [
                f64::from(x),
                f64::from(y),
                surface_z(spec, f64::from(x), f64::from(y)) - LEGEND_EMBED_MM,
            ]
        });
        add_triangle_if_nondegenerate(&mut mesh, top)?;
        add_triangle_if_nondegenerate(&mut mesh, [bottom[0], bottom[2], bottom[1]])?;
    }

    for (index, contour) in paths
        .iter()
        .enumerate()
        .filter(|(_, contour)| contour.len() >= 3)
    {
        let nesting = paths
            .iter()
            .enumerate()
            .filter(|(other_index, other)| {
                index != *other_index && other.len() >= 3 && inside(contour[0], other)
            })
            .count();
        let hole = nesting % 2 == 1;
        let mut oriented = contour.clone();
        let area = signed_area_f64(&oriented);
        if (!hole && area < 0.0) || (hole && area > 0.0) {
            oriented.reverse();
        }
        for edge in 0..oriented.len() {
            let a = oriented[edge];
            let b = oriented[(edge + 1) % oriented.len()];
            let a_top = [a[0], a[1], surface_z(spec, a[0], a[1]) + LEGEND_RAISE_MM];
            let b_top = [b[0], b[1], surface_z(spec, b[0], b[1]) + LEGEND_RAISE_MM];
            let a_bottom = [a[0], a[1], surface_z(spec, a[0], a[1]) - LEGEND_EMBED_MM];
            let b_bottom = [b[0], b[1], surface_z(spec, b[0], b[1]) - LEGEND_EMBED_MM];
            add_triangle_if_nondegenerate(&mut mesh, [a_bottom, b_bottom, b_top])?;
            add_triangle_if_nondegenerate(&mut mesh, [a_bottom, b_top, a_top])?;
        }
    }
    Ok(Some(mesh))
}

fn add_triangle_if_nondegenerate(mesh: &mut MeshData, points: [[f64; 3]; 3]) -> Result<(), String> {
    let ab = [
        points[1][0] - points[0][0],
        points[1][1] - points[0][1],
        points[1][2] - points[0][2],
    ];
    let ac = [
        points[2][0] - points[0][0],
        points[2][1] - points[0][1],
        points[2][2] - points[0][2],
    ];
    let normal = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    if normal.iter().all(|coordinate| *coordinate == 0.0) {
        // A1 RED baseline (`multi-letter-red.log`) showed Lyon emits zero-area text triangles.
        return Ok(());
    }
    add_triangle(mesh, points)
}

fn surface_z(spec: &KeycapSpec, x: f64, y: f64) -> f64 {
    spec.height + y * spec.tilt.to_radians().tan() - dish_sag(spec, x, y)
}

fn signed_area_f32(points: &[[f32; 2]]) -> f64 {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            f64::from(point[0] * next[1] - next[0] * point[1])
        })
        .sum::<f64>()
        * 0.5
}

fn signed_area_f64(points: &[[f64; 2]]) -> f64 {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point[0] * next[1] - next[0] * point[1]
        })
        .sum::<f64>()
        * 0.5
}

fn inside(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}
