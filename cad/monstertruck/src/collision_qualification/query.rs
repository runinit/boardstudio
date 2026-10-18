use super::QualifiedSurface;
use super::exact::*;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug)]
pub(super) struct Limits {
    pub max_triangles: usize,
    pub max_node_pairs: usize,
    pub max_exact_pairs: usize,
    pub max_contact_boxes: usize,
    pub numerical_width_mm: f64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_triangles: 4096,
            max_node_pairs: 100_000,
            max_exact_pairs: 2048,
            max_contact_boxes: 8,
            numerical_width_mm: 1e-10,
        }
    }
}
pub(super) type Bounds = [FPoint; 2];
pub(super) type BoxPair = [Bounds; 2];
#[derive(Debug)]
pub(super) enum ContactRegion {
    Boxes(Vec<BoxPair>),
    Unknown(&'static str),
}
#[derive(Debug)]
pub(super) struct Distance {
    pub mm: [f64; 2],
    pub witness: [usize; 2],
    pub contact: ContactRegion,
    pub exact_pairs: usize,
    pub node_pairs: usize,
    pub limits: Limits,
}
#[derive(Debug, PartialEq)]
pub(super) enum Containment {
    RuledOutSlab {
        axis: usize,
        left_upper: f64,
        right_lower: f64,
    },
    Unknown(&'static str),
}

fn bounds(t: &FTriangle) -> Bounds {
    std::array::from_fn(|end| {
        std::array::from_fn(|axis| {
            t.iter().map(|p| p[axis]).fold(
                if end == 0 {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                },
                if end == 0 { f64::min } else { f64::max },
            )
        })
    })
}
fn merge(a: Bounds, b: Bounds) -> Bounds {
    std::array::from_fn(|end| {
        std::array::from_fn(|i| {
            if end == 0 {
                a[end][i].min(b[end][i])
            } else {
                a[end][i].max(b[end][i])
            }
        })
    })
}
fn box_lower(a: Bounds, b: Bounds) -> f64 {
    let mut sum = Interval::point(0.0);
    for i in 0..3 {
        let d = Interval::point(b[0][i])
            .sub(Interval::point(a[1][i]))
            .lo
            .max(Interval::point(a[0][i]).sub(Interval::point(b[1][i])).lo)
            .max(0.0);
        sum = sum.add(Interval::point(d).mul(Interval::point(d)));
    }
    sum.lo.max(0.0)
}
fn triangle_lower(a: &FTriangle, b: &FTriangle) -> f64 {
    let subtract = |p: FPoint, q: FPoint| std::array::from_fn::<_, 3, _>(|i| p[i] - q[i]);
    let cross = |p: FPoint, q: FPoint| {
        [
            p[1] * q[2] - p[2] * q[1],
            p[2] * q[0] - p[0] * q[2],
            p[0] * q[1] - p[1] * q[0],
        ]
    };
    let ea = std::array::from_fn::<_, 3, _>(|i| subtract(a[(i + 1) % 3], a[i]));
    let eb = std::array::from_fn::<_, 3, _>(|i| subtract(b[(i + 1) % 3], b[i]));
    let mut lower = box_lower(bounds(a), bounds(b));
    let directions = [cross(ea[0], ea[1]), cross(eb[0], eb[1])]
        .into_iter()
        .chain(
            ea.into_iter()
                .flat_map(|u| eb.into_iter().map(move |v| cross(u, v))),
        );
    for d in directions {
        if d.iter().any(|x| !x.is_finite()) {
            continue;
        }
        let project = |p: FPoint| {
            (0..3)
                .map(|i| Interval::point(p[i]).mul(Interval::point(d[i])))
                .fold(Interval::point(0.0), Interval::add)
        };
        let pa = a.map(project);
        let pb = b.map(project);
        let range = |p: [Interval; 3]| Interval {
            lo: p.into_iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
            hi: p
                .into_iter()
                .map(|x| x.hi)
                .fold(f64::NEG_INFINITY, f64::max),
        };
        let ra = range(pa);
        let rb = range(pb);
        let gap = rb.sub(ra).lo.max(ra.sub(rb).lo).max(0.0);
        let norm = (0..3)
            .map(|i| Interval::point(d[i]).mul(Interval::point(d[i])))
            .fold(Interval::point(0.0), Interval::add);
        let candidate = Interval::point(gap)
            .mul(Interval::point(gap))
            .div_positive(norm)
            .lo;
        if candidate.is_finite() {
            lower = lower.max(candidate.max(0.0));
        }
    }
    lower
}

struct Node {
    bounds: Bounds,
    children: Option<[usize; 2]>,
    triangle: usize,
}
struct Tree {
    nodes: Vec<Node>,
    root: usize,
}
impl Tree {
    fn new(mesh: &[FTriangle]) -> Self {
        fn build(nodes: &mut Vec<Node>, mesh: &[FTriangle], indices: &mut [usize]) -> usize {
            let b = indices
                .iter()
                .map(|&i| bounds(&mesh[i]))
                .reduce(merge)
                .unwrap();
            let id = nodes.len();
            nodes.push(Node {
                bounds: b,
                children: None,
                triangle: indices[0],
            });
            if indices.len() > 1 {
                let axis = (0..3)
                    .max_by(|&i, &j| (b[1][i] - b[0][i]).total_cmp(&(b[1][j] - b[0][j])))
                    .unwrap();
                indices.sort_unstable_by(|&i, &j| {
                    bounds(&mesh[i])[0][axis].total_cmp(&bounds(&mesh[j])[0][axis])
                });
                let middle = indices.len() / 2;
                let (a, b) = indices.split_at_mut(middle);
                nodes[id].children = Some([build(nodes, mesh, a), build(nodes, mesh, b)]);
            }
            id
        }
        let mut nodes = Vec::new();
        let root = build(&mut nodes, mesh, &mut (0..mesh.len()).collect::<Vec<_>>());
        Self { nodes, root }
    }
}
#[derive(Clone, Copy)]
struct Pending {
    lower: f64,
    pair: [usize; 2],
}
impl PartialEq for Pending {
    fn eq(&self, b: &Self) -> bool {
        self.lower.to_bits() == b.lower.to_bits() && self.pair == b.pair
    }
}
impl Eq for Pending {}
impl PartialOrd for Pending {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Pending {
    fn cmp(&self, b: &Self) -> Ordering {
        b.lower
            .total_cmp(&self.lower)
            .then_with(|| self.pair.cmp(&b.pair))
    }
}
fn split(a: &Tree, b: &Tree, p: [usize; 2]) -> Vec<[usize; 2]> {
    match (a.nodes[p[0]].children, b.nodes[p[1]].children) {
        (Some(x), _) => x.map(|i| [i, p[1]]).to_vec(),
        (None, Some(x)) => x.map(|i| [p[0], i]).to_vec(),
        _ => Vec::new(),
    }
}
fn exact_triangle(t: &FTriangle) -> Check<Triangle> {
    Ok([point(t[0])?, point(t[1])?, point(t[2])?])
}
fn bracket(lower: f64, upper: &Q) -> Check<[f64; 2]> {
    Ok([
        sqrt_bounds(&Q::from_float(lower.max(0.0)).ok_or("numeric lower range")?)?[0],
        sqrt_bounds(upper)?[1],
    ])
}

fn within_width(mm: [f64; 2], width: f64) -> bool {
    Q::from_float(mm[1]).unwrap() - Q::from_float(mm[0]).unwrap() <= Q::from_float(width).unwrap()
}

pub(super) fn distance(
    a: &[FTriangle],
    b: &[FTriangle],
    seed: [usize; 2],
    sources: Option<[&QualifiedSurface; 2]>,
    limits: Limits,
) -> Check<Distance> {
    if limits.max_triangles == 0
        || limits.max_node_pairs == 0
        || limits.max_exact_pairs == 0
        || limits.max_contact_boxes == 0
        || !limits.numerical_width_mm.is_finite()
        || limits.numerical_width_mm <= 0.0
    {
        return Err("invalid query limits");
    }
    if a.is_empty()
        || b.is_empty()
        || a.len() > limits.max_triangles
        || b.len() > limits.max_triangles
    {
        return Err("triangle cap or empty mesh");
    }
    for t in a.iter().chain(b) {
        let exact = exact_triangle(t)?;
        if norm2(&normal(&exact)).is_zero() {
            return Err("degenerate triangle");
        }
    }
    let (x, y) = (
        a.get(seed[0]).ok_or("invalid seed index")?,
        b.get(seed[1]).ok_or("invalid seed index")?,
    );
    let mut upper = distance2(&exact_triangle(x)?, &exact_triangle(y)?)?;
    let mut witness = seed;
    let mut exact_pairs = 1;
    let mut node_pairs = 0;
    let ta = Tree::new(a);
    let tb = Tree::new(b);
    let mut queue = BinaryHeap::new();
    let initial = [ta.root, tb.root];
    queue.push(Pending {
        lower: box_lower(ta.nodes[initial[0]].bounds, tb.nodes[initial[1]].bounds),
        pair: initial,
    });
    let mut finalized_lower = f64::INFINITY;
    let mm = loop {
        let lower = queue
            .peek()
            .map_or(finalized_lower, |p| p.lower.min(finalized_lower));
        if upper.is_zero() {
            break [0.0, 0.0];
        }
        let approximate_upper = upper.to_f64().ok_or("query range")?.sqrt().next_up();
        if lower.is_finite()
            && lower > 0.0
            && approximate_upper - lower.sqrt().next_down() <= limits.numerical_width_mm
        {
            let mm = bracket(lower, &upper)?;
            if within_width(mm, limits.numerical_width_mm) {
                break mm;
            }
        }
        let Some(p) = queue.pop() else {
            break bracket(
                finalized_lower.min(-super::exact::upper(&(-upper.clone()))?),
                &upper,
            )?;
        };
        node_pairs += 1;
        if node_pairs > limits.max_node_pairs {
            return Err("node pair cap");
        }
        if Q::from_float(p.lower).unwrap() > upper {
            continue;
        }
        let children = split(&ta, &tb, p.pair);
        if !children.is_empty() {
            for child in children {
                queue.push(Pending {
                    lower: box_lower(ta.nodes[child[0]].bounds, tb.nodes[child[1]].bounds),
                    pair: child,
                });
            }
            continue;
        }
        let i = ta.nodes[p.pair[0]].triangle;
        let j = tb.nodes[p.pair[1]].triangle;
        let lower = triangle_lower(&a[i], &b[j]);
        if Q::from_float(lower).unwrap() > upper {
            continue;
        }
        if lower > 0.0 && approximate_upper - lower.sqrt().next_down() <= limits.numerical_width_mm
        {
            finalized_lower = finalized_lower.min(lower);
            continue;
        }
        exact_pairs += 1;
        if exact_pairs > limits.max_exact_pairs {
            return Err("exact pair cap");
        }
        let exact = distance2(&exact_triangle(&a[i])?, &exact_triangle(&b[j])?)?;
        let lower = (-super::exact::upper(&(-exact.clone()))?).max(0.0);
        finalized_lower = finalized_lower.min(lower);
        if exact < upper {
            upper = exact;
            witness = [i, j];
        }
    };
    if !within_width(mm, limits.numerical_width_mm) {
        return Err("numerical refinement unavailable");
    }
    let contact = match sources {
        None => ContactRegion::Unknown("source certificates unavailable"),
        Some(sources) => {
            if sources[0].triangles != a || sources[1].triangles != b {
                return Err("source certificate mesh mismatch");
            }
            let e = sources.map(|s| s.bound.total_mm);
            match contact_boxes([a, b], [&ta, &tb], mm[1], e, limits, &mut node_pairs) {
                Ok(x) => ContactRegion::Boxes(x),
                Err(reason) => ContactRegion::Unknown(reason),
            }
        }
    };
    Ok(Distance {
        mm,
        witness,
        contact,
        exact_pairs,
        node_pairs,
        limits,
    })
}
fn contact_boxes(
    [a, b]: [&[FTriangle]; 2],
    [ta, tb]: [&Tree; 2],
    upper: f64,
    e: [f64; 2],
    limits: Limits,
    visited: &mut usize,
) -> Check<Vec<BoxPair>> {
    // A closest source pair maps within eA/eB of the meshes. Triangle candidates
    // therefore need mesh U + 2(eA+eB); retaining every such pair covers ties too.
    let cutoff = Interval::point(upper)
        .add(Interval::point(2.0).mul(Interval::point(e[0]).add(Interval::point(e[1]))))
        .hi;
    let cutoff2 = Interval::point(cutoff).mul(Interval::point(cutoff)).hi;
    if !cutoff2.is_finite() {
        return Err("contact range");
    }
    let mut queue = vec![[ta.root, tb.root]];
    let mut out = Vec::<BoxPair>::new();
    while let Some(p) = queue.pop() {
        *visited += 1;
        if *visited > limits.max_node_pairs {
            return Err("contact node pair cap");
        }
        if box_lower(ta.nodes[p[0]].bounds, tb.nodes[p[1]].bounds) > cutoff2 {
            continue;
        }
        let children = split(ta, tb, p);
        if !children.is_empty() {
            queue.extend(children);
            continue;
        }
        let x = &a[ta.nodes[p[0]].triangle];
        let y = &b[tb.nodes[p[1]].triangle];
        if triangle_lower(x, y) > cutoff2 {
            continue;
        }
        let pair = [bounds(x), bounds(y)];
        let pair = std::array::from_fn(|side| {
            std::array::from_fn(|end| {
                std::array::from_fn(|i| {
                    if end == 0 {
                        (pair[side][end][i] - e[side]).next_down()
                    } else {
                        (pair[side][end][i] + e[side]).next_up()
                    }
                })
            })
        });
        if out.len() < limits.max_contact_boxes {
            out.push(pair);
        } else {
            for side in 0..2 {
                out[0][side] = merge(out[0][side], pair[side]);
            }
        }
    }
    if out.is_empty() {
        return Err("empty contact proof");
    }
    Ok(out)
}
pub(super) fn slab(a: &QualifiedSurface, b: &QualifiedSurface) -> Check<Containment> {
    for axis in 0..3 {
        let left = &a.source_hull[1][axis] + Q::from_float(a.bound.total_mm).unwrap();
        let right = &b.source_hull[0][axis] - Q::from_float(b.bound.total_mm).unwrap();
        if left < right {
            let left_upper = super::exact::upper(&left)?;
            let right_lower = -super::exact::upper(&(-right))?;
            if left_upper >= right_lower {
                continue;
            }
            return Ok(Containment::RuledOutSlab {
                axis,
                left_upper,
                right_lower,
            });
        }
    }
    Ok(Containment::Unknown(
        "no separating slab; parity is unqualified",
    ))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn rounded_interval_width_cannot_meet_a_tighter_exact_cap() {
    let interval = [3.0 * 2f64.powi(-54), 1.0];
    let rounded_width = 1.0 - 2f64.powi(-52);
    assert_eq!(interval[1] - interval[0], rounded_width);
    assert!(!within_width(interval, rounded_width));
    assert!(within_width(interval, 1.0));
}
