use super::brep::{Point3, Vector3};
use super::{
    Feature,
    brep::{self, Circle, Edge, EdgeUse, Face, Region, Ring as EdgeRing, Strip, Support, Topology},
    contours::{self, Profile, Ring},
};
use cavalier_contours::polyline::*;
use cgmath::{EuclideanSpace, InnerSpace};
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
struct Patch {
    wire: [EdgeUse; 4],
    support: Support,
    reversed: bool,
    strip: Strip,
}
#[derive(Default)]
struct Cache {
    vertices: BTreeMap<[u64; 3], usize>,
    edges: BTreeMap<EdgeKey, usize>,
    topology: Topology,
}
type EdgeKey = ([u64; 3], [u64; 3], [u64; 4]);

fn bits(x: f64) -> u64 {
    if x == 0. { 0 } else { x.to_bits() }
}
fn key(p: Point3) -> [u64; 3] {
    [bits(p.x), bits(p.y), bits(p.z)]
}
impl Cache {
    fn vertex(&mut self, point: Point3) -> usize {
        *self.vertices.entry(key(point)).or_insert_with(|| {
            let index = self.topology.vertices.len();
            self.topology.vertices.push(point);
            index
        })
    }
    fn edge(
        &mut self,
        a: Point3,
        b: Point3,
        bulge: f64,
        source: Option<&contours::Source>,
    ) -> Result<EdgeUse> {
        let forward = key(a) < key(b);
        let (first, last, bulge) = if forward {
            (a, b, bulge)
        } else {
            (b, a, -bulge)
        };
        let circle = match source {
            Some(contours::Source::Circle(circle)) if bulge != 0. => Some(circle),
            Some(contours::Source::Line(_, _)) | None if bulge == 0. => None,
            _ => return Err("edge has no matching structural line or circular source".into()),
        };
        let support = circle.map_or([0; 4], |circle| {
            let [x, y] = circle.center;
            [bits(x), bits(y), bits(circle.radius), bits(bulge.signum())]
        });
        let k = (key(first), key(last), support);
        if key(first) == key(last) {
            return Err("layered edge has coincident vertices".into());
        }
        if let Some(&index) = self.edges.get(&k) {
            return Ok(EdgeUse { index, forward });
        }
        let circle = if let Some(circle) = circle {
            let [x, y] = circle.center;
            // Measure the signed trim on the authored carrier in the shared edge direction.
            let sweep = arc_distance(
                circle.center,
                [first.x, first.y],
                [last.x, last.y],
                bulge.signum(),
            );
            if !(sweep > 0. && sweep <= std::f64::consts::PI + 1e-10) {
                return Err("circular edge has an invalid signed trim".into());
            }
            Some(Circle {
                center: Point3::new(x, y, first.z),
                radius: circle.radius,
                sweep: bulge.signum() * sweep,
            })
        } else {
            None
        };
        let vertices = [self.vertex(first), self.vertex(last)];
        let index = self.topology.edges.len();
        self.topology.edges.push(Edge { vertices, circle });
        self.edges.insert(k, index);
        Ok(EdgeUse { index, forward })
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
    circles: Vec<([f64; 2], f64)>,
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
                circles: [index, previous]
                    .into_iter()
                    .filter_map(|i| match &ring.sources[i] {
                        Some(contours::Source::Circle(circle)) => {
                            Some((circle.center, circle.radius))
                        }
                        _ => None,
                    })
                    .collect(),
            }
        })
        .collect()
}
fn partition_points(caps: &[Cap], bands: &[Band]) -> Vec<PartitionPoint> {
    let mut by_z: BTreeMap<u64, Vec<PartitionPoint>> = BTreeMap::new();
    let profiles = caps.iter().map(|cap| (cap.z, &cap.profile)).chain(
        bands
            .iter()
            .flat_map(|band| [(band.low, &band.lower), (band.high, &band.upper)]),
    );
    for (z, profile) in profiles {
        by_z.entry(bits(z))
            .or_default()
            .extend(rings(profile).iter().flat_map(ring_points));
    }
    // All bands share source-proven partitions, in the same height order.
    by_z.into_values().flatten().collect()
}
fn arc_distance(center: [f64; 2], from: [f64; 2], to: [f64; 2], sign: f64) -> f64 {
    let [x, y] = [from[0] - center[0], from[1] - center[1]];
    let [u, v] = [to[0] - center[0], to[1] - center[1]];
    (sign * (x * v - y * u))
        .atan2(x * u + y * v)
        .rem_euclid(std::f64::consts::TAU)
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
    } else if let Some(contours::Source::Circle(circle)) = source {
        let first = [a.x, a.y];
        let last = [b.x, b.y];
        let sign = a.bulge.signum();
        let span = arc_distance(circle.center, first, last, sign);
        for p in points {
            if p.xy == first || p.xy == last || !p.circles.contains(&(circle.center, circle.radius))
            {
                continue;
            }
            let distance = arc_distance(circle.center, first, p.xy, sign);
            if distance > 0. && distance < span {
                values.push((distance / span, p.xy));
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
fn ring_wire(
    cache: &mut Cache,
    ring: &Ring,
    z: f64,
    points: &[PartitionPoint],
) -> Result<EdgeRing> {
    let mut edges = vec![];
    for (index, (a, b)) in ring.iter_segments().enumerate() {
        let ts = partitions(a, b, ring.sources[index].as_ref(), points);
        for pair in ts.windows(2) {
            edges.push(cache.edge(
                Point3::new(pair[0].1[0], pair[0].1[1], z),
                Point3::new(pair[1].1[0], pair[1].1[1], z),
                a.bulge,
                ring.sources[index].as_ref(),
            )?);
        }
    }
    Ok(edges)
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
    bulge: f64,
    authored: Option<([f64; 2], f64, f64)>,
    authored_line: Option<([f64; 2], [f64; 2])>,
) -> Result<(Vec<u64>, Support, bool)> {
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
        let support = Support::Plane {
            origin: first,
            u: direction,
            v: Vector3::unit_z(),
        };
        Ok((
            vec![0, bits(normal.x), bits(normal.y), bits(d)],
            support,
            false,
        ))
    } else {
        let positive = bulge > 0.;
        let ([x, y], r, top_radius) =
            authored.ok_or("circular wall has no structural authored source")?;
        // STEP cylinders have an outward natural normal; hole orientation belongs on the face.
        let reversed = top_radius == r && !positive;
        // Keep the generator's radius in local coordinates, before translating its authored axis.
        // Measuring an arbitrary intersection endpoint would round the nominal radius differently.
        let support = Support::Round {
            center: [x, y],
            lower: [a.z, r],
            upper: [c.z, top_radius],
            outward: positive || reversed,
        };
        let slope = (top_radius - r) / (c.z - a.z);
        let intercept = r - slope * a.z;
        Ok((
            vec![
                1,
                bits(x),
                bits(y),
                bits(intercept),
                bits(slope),
                positive as u64,
            ],
            support,
            reversed,
        ))
    }
}
fn wall_patches(
    bands: &[Band],
    points: &[PartitionPoint],
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
                let mut ts = partitions(a, b, low.sources[index].as_ref(), points);
                ts.extend(partitions(c, d, high.sources[index].as_ref(), points));
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
                    let bulge = a.bulge;
                    let lower = cache.edge(p, q, bulge, low.sources[index].as_ref())?;
                    let right = cache.edge(q, s, 0., None)?;
                    let upper = cache.edge(r, s, bulge, high.sources[index].as_ref())?;
                    let left = cache.edge(p, r, 0., None)?;
                    let wire = [lower, right, upper.inverse(), left.inverse()];
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
                    let (key, support, reversed) =
                        surface_key(p, q, r, bulge, authored, authored_line)?;
                    groups.entry(key).or_default().push(Patch {
                        wire,
                        support,
                        reversed,
                        strip: Strip { lower, upper },
                    });
                }
            }
        }
    }
    Ok(groups)
}
fn merge(mut patches: Vec<Patch>, topology: &Topology) -> Result<Vec<Face>> {
    let mut faces = vec![];
    while let Some(seed) = patches.pop() {
        let support = seed.support;
        let reversed = seed.reversed;
        let mut boundary = seed.wire.to_vec();
        let mut strips = vec![seed.strip];
        loop {
            let index = patches.iter().position(|p| {
                p.wire
                    .iter()
                    .any(|edge| boundary.iter().any(|e| e.index == edge.index))
            });
            let Some(index) = index else {
                break;
            };
            let patch = patches.remove(index);
            strips.push(patch.strip);
            for edge in patch.wire {
                if let Some(i) = boundary.iter().position(|e| e.index == edge.index) {
                    if boundary[i].forward == edge.forward {
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
            let [start, mut end] = first.vertices(topology);
            let mut edges = vec![first];
            while end != start {
                let matches: Vec<_> = boundary
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.vertices(topology)[0] == end)
                    .map(|(i, _)| i)
                    .collect();
                if matches.len() != 1 {
                    return Err("wall merge leaves an open or branching boundary".into());
                }
                let edge = boundary.remove(matches[0]);
                end = edge.vertices(topology)[1];
                edges.push(edge);
            }
            wires.push(edges);
        }
        if reversed {
            for ring in &mut wires {
                brep::reverse(ring);
            }
        }
        faces.push(Face {
            rings: wires,
            support,
            forward: !reversed,
            strips,
        });
    }
    Ok(faces)
}
pub(super) fn assemble(bands: &[Band], features: &[Feature], frame: &Ring) -> Result<Region> {
    let caps = caps(bands, features, frame)?;
    let points = partition_points(&caps, bands);
    let mut cache = Cache::default();
    let mut faces = vec![];
    for cap in caps {
        for region in cap.profile {
            let mut wires = vec![ring_wire(&mut cache, &region.outer, cap.z, &points)?];
            for mut hole in region.holes {
                hole.invert_direction_mut();
                wires.push(ring_wire(&mut cache, &hole, cap.z, &points)?);
            }
            faces.push(Face {
                rings: wires,
                support: Support::Plane {
                    origin: Point3::new(0., 0., cap.z),
                    u: Vector3::unit_x(),
                    v: Vector3::unit_y(),
                },
                forward: cap.up,
                strips: vec![],
            });
        }
    }
    for patches in wall_patches(bands, &points, &mut cache)?.into_values() {
        faces.extend(merge(patches, &cache.topology)?);
    }
    Region::new(cache.topology, faces)
}
