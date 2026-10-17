use super::{
    Feature,
    contours::{self, Profile, Ring},
};
use cavalier_contours::polyline::*;
use monstertruck_modeling::*;
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, String>;
pub(super) struct Band {
    pub low: f64,
    pub high: f64,
    pub initial: bool,
    pub active: Vec<bool>,
    pub lower: Profile,
    pub upper: Profile,
}
#[derive(Clone)]
struct Cap {
    z: f64,
    profile: Profile,
    up: bool,
}
#[derive(Clone)]
struct Patch {
    wire: Wire,
    surface: Surface,
    reversed: bool,
}
#[derive(Default)]
struct Cache {
    vertices: BTreeMap<[u64; 3], Vertex>,
    edges: BTreeMap<([u64; 3], [u64; 3], u64), Edge>,
}
fn bits(x: f64) -> u64 {
    if x == 0. { 0 } else { x.to_bits() }
}
fn key(p: Point3) -> [u64; 3] {
    [bits(p.x), bits(p.y), bits(p.z)]
}
impl Cache {
    fn vertex(&mut self, p: Point3) -> Vertex {
        self.vertices
            .entry(key(p))
            .or_insert_with(|| builder::vertex(p))
            .clone()
    }
    fn edge(&mut self, a: Point3, b: Point3, bulge: f64) -> Result<Edge> {
        let forward = key(a) < key(b);
        let (first, last, bulge) = if forward {
            (a, b, bulge)
        } else {
            (b, a, -bulge)
        };
        let k = (key(first), key(last), bits(bulge));
        if let Some(e) = self.edges.get(&k) {
            return Ok(if forward { e.clone() } else { e.inverse() });
        }
        let va = self.vertex(first);
        let vb = self.vertex(last);
        let edge = if bulge == 0. {
            builder::line(&va, &vb)
        } else {
            let p = PlineVertex::new(first.x, first.y, bulge);
            let q = PlineVertex::new(last.x, last.y, 0.);
            let (_, center) = seg_arc_radius_and_center(p, q);
            let radial = Vector3::new(first.x - center.x, first.y - center.y, 0.);
            let tangent = Vector3::new(-radial.y, radial.x, 0.) * bulge.signum();
            builder::try_circle_arc(&va, &vb, tangent)
                .map_err(|e| format!("exact circle edge: {e:?}"))?
        };
        self.edges.insert(k, edge.clone());
        Ok(if forward { edge } else { edge.inverse() })
    }
}
fn rings(p: &Profile) -> Vec<Ring> {
    p.iter()
        .flat_map(|r| {
            let mut rings = vec![r.outer.clone()];
            for h in &r.holes {
                let mut h = h.clone();
                h.invert_direction_mut();
                rings.push(h);
            }
            rings
        })
        .collect()
}
#[derive(Clone)]
struct PartitionPoint {
    xy: [f64; 2],
    lines: Vec<([f64; 2], [f64; 2])>,
}
fn ring_points(ring: &Ring) -> Vec<PartitionPoint> {
    ring.iter_vertexes()
        .enumerate()
        .map(|(index, vertex)| {
            let previous = (index + ring.vertex_count() - 1) % ring.vertex_count();
            let lines = [index, previous]
                .into_iter()
                .filter_map(|i| match &ring.sources[i] {
                    Some(contours::Source::Line(a, b)) => Some((*a, *b)),
                    _ => None,
                })
                .collect();
            PartitionPoint {
                xy: [vertex.x, vertex.y],
                lines,
            }
        })
        .collect()
}
fn points_at_z(caps: &[Cap], bands: &[Band]) -> BTreeMap<u64, Vec<PartitionPoint>> {
    let mut result: BTreeMap<u64, Vec<PartitionPoint>> = BTreeMap::new();
    for cap in caps {
        for ring in rings(&cap.profile) {
            result
                .entry(bits(cap.z))
                .or_default()
                .extend(ring_points(&ring));
        }
    }
    for band in bands {
        for (z, profile) in [(band.low, &band.lower), (band.high, &band.upper)] {
            for ring in rings(profile) {
                result
                    .entry(bits(z))
                    .or_default()
                    .extend(ring_points(&ring));
            }
        }
    }
    // Propagate source-proven partitions through adjacent bands before edge creation.
    let all: Vec<_> = result.values().flatten().cloned().collect();
    for points in result.values_mut() {
        *points = all.clone();
    }
    result
}
fn partitions(
    a: PlineVertex,
    b: PlineVertex,
    source: Option<&contours::Source>,
    points: &[PartitionPoint],
) -> Vec<(f64, [f64; 2])> {
    let mut values = vec![(0., [a.x, a.y]), (1., [b.x, b.y])];
    if a.bulge == 0. {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx * dx + dy * dy;
        for p in points {
            let x = p.xy[0] - a.x;
            let y = p.xy[1] - a.y;
            let same_source = matches!(source,Some(contours::Source::Line(first,last)) if p.lines.iter().any(|(a,b)| (a==first && b==last)||(b==first && a==last)));
            if same_source || dx * y - dy * x == 0. {
                let t = (x * dx + y * dy) / length;
                if t > 0. && t < 1. {
                    values.push((t, p.xy));
                }
            }
        }
    }
    values.sort_by(|a, b| a.0.total_cmp(&b.0));
    values.dedup_by(|a, b| a.1 == b.1);
    values
}
fn segment_point(a: PlineVertex, b: PlineVertex, t: f64, z: f64) -> Point3 {
    if t == 0. {
        return Point3::new(a.x, a.y, z);
    }
    if t == 1. {
        return Point3::new(b.x, b.y, z);
    }
    if a.bulge == 0. {
        return Point3::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, z);
    }
    let (radius, center) = seg_arc_radius_and_center(a, b);
    let start = (a.y - center.y).atan2(a.x - center.x);
    let angle = start + 4. * a.bulge.atan() * t;
    Point3::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
        z,
    )
}
fn ring_wire(cache: &mut Cache, ring: &Ring, z: f64, points: &[PartitionPoint]) -> Result<Wire> {
    let mut edges = vec![];
    for (index, (a, b)) in ring.iter_segments().enumerate() {
        let ts = partitions(a, b, ring.sources[index].as_ref(), points);
        for pair in ts.windows(2) {
            let bulge = if pair[1].0 - pair[0].0 == 1. {
                a.bulge
            } else {
                (a.bulge.atan() * (pair[1].0 - pair[0].0)).tan()
            };
            edges.push(cache.edge(
                Point3::new(pair[0].1[0], pair[0].1[1], z),
                Point3::new(pair[1].1[0], pair[1].1[1], z),
                bulge,
            )?);
        }
    }
    Ok(edges.into())
}
fn caps(bands: &[Band], features: &[Feature], frame: &Ring) -> Result<Vec<Cap>> {
    let mut caps = vec![];
    for i in 0..=bands.len() {
        let below = if i == 0 {
            vec![]
        } else {
            bands[i - 1].upper.clone()
        };
        let above = if i == bands.len() {
            vec![]
        } else {
            bands[i].lower.clone()
        };
        let z = if i == bands.len() {
            bands[i - 1].high
        } else {
            bands[i].low
        };
        let previous = if i == 0 { None } else { Some(&bands[i - 1]) };
        let next = if i == bands.len() {
            None
        } else {
            Some(&bands[i])
        };
        let mut grows = false;
        let mut shrinks = false;
        let was = previous.is_some_and(|b| b.initial);
        let now = next.is_some_and(|b| b.initial);
        grows |= !was && now;
        shrinks |= was && !now;
        for (n, f) in features.iter().enumerate() {
            let was = previous.is_some_and(|b| b.active[n]);
            let now = next.is_some_and(|b| b.active[n]);
            if was != now {
                if now == f.add {
                    grows = true;
                } else {
                    shrinks = true;
                }
            }
        }
        // Every ordered union/difference is monotone in its material input.
        if shrinks {
            caps.push(Cap {
                z,
                profile: contours::subtract_profile(&below, &above, frame)
                    .map_err(|e| format!("upper cap at z={z}: {e}"))?,
                up: true,
            });
        }
        if grows {
            caps.push(Cap {
                z,
                profile: contours::subtract_profile(&above, &below, frame)
                    .map_err(|e| format!("lower cap at z={z}: {e}"))?,
                up: false,
            });
        }
    }
    Ok(caps)
}
fn surface_key(
    a: Point3,
    b: Point3,
    c: Point3,
    d: Point3,
    bulge: f64,
    authored: Option<([f64; 2], f64, f64)>,
    authored_line: Option<([f64; 2], [f64; 2])>,
) -> Result<(Vec<u64>, Surface, bool)> {
    if bulge == 0. {
        let (first, last) = authored_line.ok_or("planar wall has no structural authored line")?;
        let mut first = Point3::new(first[0], first[1], 0.);
        let mut last = Point3::new(last[0], last[1], 0.);
        if (last - first).dot(b - a) < 0. {
            std::mem::swap(&mut first, &mut last);
        }
        let direction = last - first;
        if direction.magnitude2() == 0. {
            return Err("planar wall has a zero-length authored source".into());
        }
        let normal = Vector3::new(direction.y, -direction.x, 0.).normalize();
        let d = normal.dot(first.to_vec());
        // Every split patch uses its authored carrier, independent of split-point roundoff.
        let surface = Surface::Plane(Plane::new(first, last, first + Vector3::unit_z()));
        Ok((
            vec![0, bits(normal.x), bits(normal.y), bits(d)],
            surface,
            false,
        ))
    } else {
        let positive = bulge > 0.;
        let (mut r, mut center) = seg_arc_radius_and_center(
            PlineVertex::new(a.x, a.y, bulge),
            PlineVertex::new(b.x, b.y, 0.),
        );
        let (mut top_radius, mut top_center) = seg_arc_radius_and_center(
            PlineVertex::new(c.x, c.y, bulge),
            PlineVertex::new(d.x, d.y, 0.),
        );
        if let Some((axis, radius, upper_radius)) = authored {
            center = cavalier_contours::core::math::Vector2::new(axis[0], axis[1]);
            top_center = center;
            r = radius;
            top_radius = upper_radius;
        }
        if center != top_center {
            return Err("cone contour changed its authored axis".into());
        }
        // STEP cylinders have an outward natural normal; hole orientation belongs on the face.
        let reversed = top_radius == r && !positive;
        // Keep the generator's radius in local coordinates, before translating its authored axis.
        // Measuring an arbitrary intersection endpoint would round the nominal radius differently.
        let lower = Point3::new(r, 0., a.z);
        let upper = Point3::new(top_radius, 0., c.z);
        let (start, end) = if positive || reversed {
            (upper, lower)
        } else {
            (lower, upper)
        };
        let generator = Curve::Line(Line(start, end));
        let surface = Surface::RevolutionSurface(Processor::with_transform(
            RevolutionSurface::by_revolution(generator, Point3::origin(), Vector3::unit_z()),
            Matrix4::from_translation(Vector3::new(center.x, center.y, 0.)),
        ));
        let slope = (top_radius - r) / (c.z - a.z);
        let intercept = r - slope * a.z;
        Ok((
            vec![
                1,
                bits(center.x),
                bits(center.y),
                bits(intercept),
                bits(slope),
                positive as u64,
            ],
            surface,
            reversed,
        ))
    }
}
fn wall_patches(
    bands: &[Band],
    points: &BTreeMap<u64, Vec<PartitionPoint>>,
    cache: &mut Cache,
) -> Result<BTreeMap<Vec<u64>, Vec<Patch>>> {
    let mut groups: BTreeMap<Vec<u64>, Vec<Patch>> = BTreeMap::new();
    for band in bands {
        let lower = rings(&band.lower);
        let upper = rings(&band.upper);
        if lower.len() != upper.len() {
            return Err("varying profile changes contour ownership within a height band".into());
        }
        for (low, high) in lower.iter().zip(&upper) {
            if low.vertex_count() != high.vertex_count() {
                return Err("varying profile changes edge count within a height band".into());
            }

            for (index, ((a, b), (c, d))) in
                low.iter_segments().zip(high.iter_segments()).enumerate()
            {
                if a.bulge != c.bulge || b.bulge != d.bulge {
                    return Err("varying profile changes arc spans within a height band".into());
                }
                let mut ts =
                    partitions(a, b, low.sources[index].as_ref(), &points[&bits(band.low)]);
                ts.extend(partitions(
                    c,
                    d,
                    high.sources[index].as_ref(),
                    &points[&bits(band.high)],
                ));
                ts.sort_by(|a, b| a.0.total_cmp(&b.0));
                ts.dedup_by(|a, b| a.0 == b.0);
                for pair in ts.windows(2) {
                    // Constant vertical profiles reuse the exact Boolean/authored split point.
                    // Reconstructing it from t can change one ULP and disconnect its cap edge.
                    let constant = a.x == c.x && a.y == c.y && b.x == d.x && b.y == d.y;
                    let point = |end: (f64, [f64; 2]), z, first, last| {
                        if constant {
                            Point3::new(end.1[0], end.1[1], z)
                        } else {
                            segment_point(first, last, end.0, z)
                        }
                    };
                    let p = point(pair[0], band.low, a, b);
                    let q = point(pair[1], band.low, a, b);
                    let r = point(pair[0], band.high, c, d);
                    let s = point(pair[1], band.high, c, d);
                    let bulge = if pair[1].0 - pair[0].0 == 1. {
                        a.bulge
                    } else {
                        (a.bulge.atan() * (pair[1].0 - pair[0].0)).tan()
                    };
                    let lower = cache.edge(p, q, bulge)?;
                    let right = cache.edge(q, s, 0.)?;
                    let upper = cache.edge(r, s, bulge)?;
                    let left = cache.edge(p, r, 0.)?;
                    let wire: Wire = vec![lower, right, upper.inverse(), left.inverse()].into();
                    let authored = if bulge == 0. {
                        None
                    } else {
                        let source = low.sources[index]
                            .as_ref()
                            .and_then(|source| {
                                if let contours::Source::Circle(circle) = source {
                                    Some(circle)
                                } else {
                                    None
                                }
                            })
                            .ok_or("circular wall has no structural authored source")?;
                        let top = high.sources[index]
                            .as_ref()
                            .and_then(|source| {
                                if let contours::Source::Circle(circle) = source {
                                    Some(circle)
                                } else {
                                    None
                                }
                            })
                            .ok_or("upper circular wall has no structural authored source")?;
                        if source.id != top.id || source.center != top.center {
                            return Err("circular wall changed authored source".into());
                        }
                        // Lineage selects the source. Residuals only reject inconsistent output.
                        for (first, last, source) in [(a, b, source), (c, d, top)] {
                            let (radius, center) = seg_arc_radius_and_center(first, last);
                            if (center.x - source.center[0]).abs() > 1e-10
                                || (center.y - source.center[1]).abs() > 1e-10
                                || (radius - source.radius).abs() > 1e-10
                            {
                                return Err(format!(
                                    "circular wall '{}' exceeds the 2D source residual budget",
                                    source.id
                                ));
                            }
                        }
                        Some((source.center, source.radius, top.radius))
                    };
                    let authored_line = if bulge == 0. {
                        if low.sources[index] != high.sources[index] {
                            return Err("planar wall changed authored source".into());
                        }
                        match low.sources[index] {
                            Some(contours::Source::Line(a, b)) => Some((a, b)),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let (key, surface, reversed) =
                        surface_key(p, q, r, s, bulge, authored, authored_line)?;
                    groups.entry(key).or_default().push(Patch {
                        wire,
                        surface,
                        reversed,
                    });
                }
            }
        }
    }
    Ok(groups)
}
fn merge(mut patches: Vec<Patch>) -> Result<Vec<Face>> {
    let mut faces = vec![];
    while let Some(seed) = patches.pop() {
        let surface = seed.surface;
        let reversed = seed.reversed;
        let mut boundary: Vec<Edge> = seed.wire.into_iter().collect();
        loop {
            let index = patches.iter().position(|p| {
                p.wire
                    .iter()
                    .any(|edge| boundary.iter().any(|e| e.id() == edge.id()))
            });
            let Some(index) = index else {
                break;
            };
            for edge in patches.remove(index).wire {
                if let Some(i) = boundary.iter().position(|e| e.id() == edge.id()) {
                    if boundary[i].orientation() == edge.orientation() {
                        return Err("coincident wall patches have inconsistent orientation".into());
                    }
                    boundary.remove(i);
                } else {
                    boundary.push(edge);
                }
            }
        }
        let mut wires = vec![];
        while let Some(first) = boundary.pop() {
            let start = first.front().id();
            let mut end = first.back().id();
            let mut edges = vec![first];
            while end != start {
                let matches: Vec<_> = boundary
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.front().id() == end)
                    .map(|(i, _)| i)
                    .collect();
                if matches.len() != 1 {
                    return Err("wall merge leaves an open or branching boundary".into());
                }
                let edge = boundary.remove(matches[0]);
                end = edge.back().id();
                edges.push(edge);
            }
            wires.push(edges.into());
        }
        let absolute = if reversed {
            wires.into_iter().map(|wire: Wire| wire.inverse()).collect()
        } else {
            wires
        };
        let face =
            Face::try_new(absolute, surface).map_err(|e| format!("merged exact face: {e:?}"))?;
        faces.push(if reversed { face.inverse() } else { face });
    }
    Ok(faces)
}
pub(super) fn assemble(bands: &[Band], features: &[Feature], frame: &Ring) -> Result<Vec<Solid>> {
    let caps = caps(bands, features, frame)?;
    let points = points_at_z(&caps, bands);
    let mut cache = Cache::default();
    let mut faces = vec![];
    for cap in caps {
        for region in cap.profile {
            let mut wires = vec![ring_wire(
                &mut cache,
                &region.outer,
                cap.z,
                &points[&bits(cap.z)],
            )?];
            for mut hole in region.holes {
                hole.invert_direction_mut();
                wires.push(ring_wire(&mut cache, &hole, cap.z, &points[&bits(cap.z)])?);
            }
            let face: Face = builder::try_attach_plane(wires)
                .map_err(|e| format!("horizontal profile difference: {e:?}"))?;
            faces.push(if cap.up { face } else { face.inverse() });
        }
    }
    for patches in wall_patches(bands, &points, &mut cache)?.into_values() {
        faces.extend(merge(patches)?);
    }
    let shell: Shell = faces.into();

    shell
        .connected_components()
        .into_iter()
        .map(|shell| {
            let top = shell
                .vertex_iter()
                .map(|vertex| vertex.point().z)
                .fold(f64::NEG_INFINITY, f64::max);
            // An outward Z-feature shell has an upward face at its highest authored Z.
            let outward = shell.iter().any(|face| {
                matches!(face.oriented_surface(), Surface::Plane(plane) if plane.normal().z > 0.0)
                    && face
                        .boundaries()
                        .iter()
                        .all(|wire| wire.vertex_iter().all(|vertex| vertex.point().z == top))
            });
            if !outward {
                return Err("disconnected sealed void shell has unsupported ownership".into());
            }
            Solid::try_new(vec![shell]).map_err(|e| format!("layered exact solid: {e:?}"))
        })
        .collect()
}
