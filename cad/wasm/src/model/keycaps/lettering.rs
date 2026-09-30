use super::*;
use ttf_parser::{Face, OutlineBuilder};

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
        let (a, b) = *self.current.last().unwrap();
        for i in 1..=12 {
            let t = f64::from(i) / 12.0;
            let u = 1.0 - t;
            self.current.push((
                u * u * a + 2.0 * u * t * f64::from(x1) + t * t * f64::from(x),
                u * u * b + 2.0 * u * t * f64::from(y1) + t * t * f64::from(y),
            ));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b) = *self.current.last().unwrap();
        for i in 1..=16 {
            let t = f64::from(i) / 16.0;
            let u = 1.0 - t;
            self.current.push((
                u * u * u * a
                    + 3.0 * u * u * t * f64::from(x1)
                    + 3.0 * u * t * t * f64::from(x2)
                    + t * t * t * f64::from(x),
                u * u * u * b
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
fn inside(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let mut yes = false;
    for (a, b) in poly
        .iter()
        .zip(poly.iter().cycle().skip(1))
        .take(poly.len())
    {
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            yes = !yes
        }
    }
    yes
}
pub(super) fn solids(
    text: &str,
    width: f64,
    depth: f64,
    z: f64,
    height: f64,
) -> Result<Vec<Solid>, String> {
    if text.trim().is_empty() {
        return Ok(vec![]);
    }
    let face = Face::parse(include_bytes!("../../../assets/DejaVuSans.ttf"), 0)
        .map_err(|_| "Bundled legend font is invalid")?;
    let mut paths = Vec::new();
    let mut advance = 0.0;
    for ch in text.chars() {
        let glyph = face
            .glyph_index(ch)
            .ok_or_else(|| format!("Legend font has no glyph for {ch}"))?;
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
        return Ok(vec![]);
    }
    let bounds = paths.iter().flatten().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(a, b, c, d), (x, y)| (a.min(*x), b.max(*x), c.min(*y), d.max(*y)),
    );
    let scale = (width / (bounds.1 - bounds.0))
        .min(depth / (bounds.3 - bounds.2))
        .min(4.0 / (bounds.3 - bounds.2));
    let xmid = (bounds.0 + bounds.1) / 2.0;
    let ymid = (bounds.2 + bounds.3) / 2.0;
    let nesting: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            paths
                .iter()
                .enumerate()
                .filter(|(j, other)| i != *j && inside(path[0], other))
                .count()
        })
        .collect();
    let mut output = Vec::new();
    for (i, path) in paths
        .iter()
        .enumerate()
        .filter(|(i, _)| nesting[*i] % 2 == 0)
    {
        let make_wire = |path: &[(f64, f64)]| {
            Edge::polygon(
                &path
                    .iter()
                    .map(|(x, y)| DVec3::new((x - xmid) * scale, (y - ymid) * scale, z))
                    .collect::<Vec<_>>(),
            )
            .map_err(cadrum_error)
        };
        let mut edges = make_wire(path)?;
        for (j, hole) in paths
            .iter()
            .enumerate()
            .filter(|(j, hole)| nesting[*j] == nesting[i] + 1 && inside(hole[0], path))
        {
            let _ = j;
            edges.extend(make_wire(hole)?);
        }
        output.push(Solid::extrude(&edges, DVec3::new(0.0, 0.0, height)).map_err(cadrum_error)?);
    }
    Ok(output)
}
