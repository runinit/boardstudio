use num_rational::BigRational;
use num_traits::{One, Zero};

type Rat = BigRational;
#[derive(Clone, Debug)]
struct Pt {
    x: Rat,
    y: Rat,
    input_xy: Option<[f64; 2]>,
}
impl PartialEq for Pt {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Eq for Pt {}
impl Pt {
    fn sub(&self, b: &Pt) -> Pt {
        Pt {
            x: &self.x - &b.x,
            y: &self.y - &b.y,
            input_xy: None,
        }
    }
    fn add_scaled(&self, v: &Pt, t: &Rat) -> Pt {
        Pt {
            x: &self.x + &v.x * t,
            y: &self.y + &v.y * t,
            input_xy: None,
        }
    }
}
fn cross(a: &Pt, b: &Pt) -> Rat {
    &a.x * &b.y - &a.y * &b.x
}
fn orient(a: &Pt, b: &Pt, c: &Pt) -> Rat {
    cross(&b.sub(a), &c.sub(a))
}
fn half() -> Rat {
    Rat::new(1.into(), 2.into())
}
fn area2(p: &[Pt]) -> Rat {
    let mut s = Rat::zero();
    for i in 0..p.len() {
        s += cross(&p[i], &p[(i + 1) % p.len()]);
    }
    s
}
fn point_from_f64(p: [f64; 2]) -> Result<Pt, String> {
    let x = Rat::from_float(p[0]).ok_or("nonfinite x")?;
    let y = Rat::from_float(p[1]).ok_or("nonfinite y")?;
    Ok(Pt {
        x,
        y,
        input_xy: Some(p),
    })
}
fn between(a: &Rat, b: &Rat, x: &Rat) -> bool {
    a.min(b) <= x && x <= a.max(b)
}
fn on_segment(a: &Pt, b: &Pt, p: &Pt) -> bool {
    if let (Some([ax, ay]), Some([bx, by]), Some([px, py])) = (a.input_xy, b.input_xy, p.input_xy) {
        if px < ax.min(bx) || px > ax.max(bx) || py < ay.min(by) || py > ay.max(by) {
            return false;
        }
    } else if !between(&a.x, &b.x, &p.x) || !between(&a.y, &b.y, &p.y) {
        return false;
    }
    orient(a, b, p).is_zero()
}
fn input_segment_bounds_disjoint(a: &Pt, b: &Pt, c: &Pt, d: &Pt) -> bool {
    let (Some([ax, ay]), Some([bx, by]), Some([cx, cy]), Some([dx, dy])) =
        (a.input_xy, b.input_xy, c.input_xy, d.input_xy)
    else {
        return false;
    };
    ax.max(bx) < cx.min(dx)
        || cx.max(dx) < ax.min(bx)
        || ay.max(by) < cy.min(dy)
        || cy.max(dy) < ay.min(by)
}
fn segment_hits(a: &Pt, b: &Pt, c: &Pt, d: &Pt) -> Result<Vec<(Rat, Rat, Pt)>, String> {
    // Original finite f64 endpoints are exact dyadic values, so their native
    // total ordering and interval extrema are exact. Strictly disjoint boxes
    // prove segment disjointness; touching boxes continue through exact math.
    if input_segment_bounds_disjoint(a, b, c, d) {
        return Ok(Vec::new());
    }
    let r = b.sub(a);
    let s = d.sub(c);
    let denom = cross(&r, &s);
    let q = c.sub(a);
    if denom.is_zero() {
        if !cross(&q, &r).is_zero() {
            return Ok(Vec::new());
        }
        // Collinear contact is valid input. Split both segments at the exact
        // endpoints of their overlap; integration handles coincident spans.
        let use_x = !r.x.is_zero();
        let (a0, a1, c0, c1) = if use_x {
            (&a.x, &b.x, &c.x, &d.x)
        } else {
            (&a.y, &b.y, &c.y, &d.y)
        };
        let lo = a0.min(a1).max(c0.min(c1)).clone();
        let hi = a0.max(a1).min(c0.max(c1)).clone();
        if lo > hi {
            return Ok(Vec::new());
        }
        let coordinates = if lo == hi { vec![lo] } else { vec![lo, hi] };
        let mut out: Vec<(Rat, Rat, Pt)> = Vec::with_capacity(coordinates.len());
        for coordinate in coordinates {
            let (av, ad, cv, cd) = if use_x {
                (&a.x, &r.x, &c.x, &s.x)
            } else {
                (&a.y, &r.y, &c.y, &s.y)
            };
            let t = (&coordinate - av) / ad;
            let u = (&coordinate - cv) / cd;
            out.push((t.clone(), u, a.add_scaled(&r, &t)));
        }
        return Ok(out);
    }
    let t = &cross(&q, &s) / &denom;
    let u = &cross(&q, &r) / &denom;
    if t >= Rat::zero() && t <= Rat::one() && u >= Rat::zero() && u <= Rat::one() {
        Ok(vec![(t.clone(), u, a.add_scaled(&r, &t))])
    } else {
        Ok(Vec::new())
    }
}
fn validate_poly(p: &[[f64; 2]]) -> Result<Vec<Pt>, String> {
    if p.iter().flatten().any(|x| !x.is_finite()) {
        return Err("nonfinite coordinate".into());
    }
    // Preserve the original caller slice for diagnostics; normalize only this
    // local measurement copy. Core's retained CAD baseline accepts exact
    // adjacent and one closing duplicate for the measured fixtures.
    let mut normalized: Vec<[f64; 2]> = Vec::with_capacity(p.len());
    for point in p.iter().copied() {
        if normalized.last().is_none_or(|last| *last != point) {
            normalized.push(point);
        }
    }
    if normalized.len() > 1 && normalized.first() == normalized.last() {
        normalized.pop();
    }
    if normalized.len() < 3 {
        return Err(
            "ring has fewer than three vertices after exact duplicate normalization".into(),
        );
    }
    let mut q = normalized
        .into_iter()
        .map(point_from_f64)
        .collect::<Result<Vec<_>, _>>()?;
    let signed = area2(&q);
    if signed.is_zero() {
        return Err("zero-area ring".into());
    }
    if signed < Rat::zero() {
        q.reverse();
    }
    // This narrow reference only claims simple rings with no self-touch or self-cross.
    let n = q.len();
    // Adjacent edges may be collinear when they continue straight through a
    // vertex. Reject only a reversal, where the adjacent segments overlap.
    for i in 0..n {
        let prev = &q[(i + n - 1) % n];
        let here = &q[i];
        let next = &q[(i + 1) % n];
        if orient(prev, here, next).is_zero() {
            let back = prev.sub(here);
            let forward = next.sub(here);
            if (&back.x * &forward.x + &back.y * &forward.y) > Rat::zero() {
                return Err("adjacent collinear backtracking unsupported".into());
            }
        }
    }
    for i in 0..n {
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            if !segment_hits(&q[i], &q[(i + 1) % n], &q[j], &q[(j + 1) % n])?.is_empty() {
                return Err("self-intersecting/touching ring unsupported".into());
            }
        }
    }
    Ok(q)
}
/// Validate one operand in unit tests without measuring area.
#[cfg(test)]
fn validate_simple_ring(vertices: &[[f64; 2]]) -> Result<(), String> {
    validate_poly(vertices).map(|_| ())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Location {
    Inside,
    Outside,
    Boundary { same_direction: bool },
}
fn locate(p: &Pt, poly: &[Pt], reference_edge: Option<(&Pt, &Pt)>) -> Location {
    let mut result = false;
    for i in 0..poly.len() {
        let a = &poly[i];
        let b = &poly[(i + 1) % poly.len()];
        if on_segment(a, b, p) {
            let same_direction = reference_edge.is_some_and(|(c, d)| {
                let u = b.sub(a);
                let v = d.sub(c);
                (&u.x * &v.x + &u.y * &v.y) > Rat::zero()
            });
            return Location::Boundary { same_direction };
        }
        let up = a.y <= p.y && p.y < b.y;
        let down = b.y <= p.y && p.y < a.y;
        if up && orient(a, b, p) > Rat::zero() || down && orient(a, b, p) < Rat::zero() {
            result = !result;
        }
    }
    if result {
        Location::Inside
    } else {
        Location::Outside
    }
}
fn add_breaks(a: &[Pt], b: &[Pt], ba: &mut [Vec<Rat>], bb: &mut [Vec<Rat>]) -> Result<(), String> {
    for i in 0..a.len() {
        for j in 0..b.len() {
            for (t, u, _) in
                segment_hits(&a[i], &a[(i + 1) % a.len()], &b[j], &b[(j + 1) % b.len()])?
            {
                ba[i].push(t);
                bb[j].push(u);
            }
        }
    }
    Ok(())
}
fn integrate(
    poly: &[Pt],
    other: &[Pt],
    breaks: &mut [Vec<Rat>],
    keep_outside: bool,
    subject_boundary: bool,
) -> Result<Rat, String> {
    let mut sum = Rat::zero();
    for i in 0..poly.len() {
        let a = &poly[i];
        let b = &poly[(i + 1) % poly.len()];
        let v = b.sub(a);
        let ts = &mut breaks[i];
        ts.sort();
        ts.dedup();
        for w in ts.windows(2) {
            if w[0] == w[1] {
                continue;
            }
            let p = a.add_scaled(&v, &w[0]);
            let q = a.add_scaled(&v, &w[1]);
            let mid = Pt {
                x: (&p.x + &q.x) * &half(),
                y: (&p.y + &q.y) * &half(),
                input_xy: None,
            };
            let location = locate(&mid, other, Some((&p, &q)));
            let include = match location {
                Location::Outside => keep_outside,
                Location::Inside => !keep_outside,
                Location::Boundary { same_direction } if subject_boundary => !same_direction,
                // Coincident clip boundary is not subtracted: either it is
                // the shared edge of two interiors (cancelled) or clip lies
                // outside subject (no clip contribution to the difference).
                Location::Boundary { .. } => false,
            };
            if include {
                sum += cross(&p, &q);
            }
        }
    }
    Ok(sum)
}
/// Exact area of `subject \ clip` for two simple, finite rings.
/// Fails closed for self intersections/touches and adjacent backtracking. Valid
/// cross-ring tangencies and coincident collinear boundary spans are supported.
fn difference_area(subject: &[[f64; 2]], clip: &[[f64; 2]]) -> Result<Rat, String> {
    let a = validate_poly(subject).map_err(|e| format!("support ring: {e}"))?;
    let b = validate_poly(clip).map_err(|e| format!("exterior ring: {e}"))?;
    difference_area_validated(&a, &b)
}

fn difference_area_validated(a: &[Pt], b: &[Pt]) -> Result<Rat, String> {
    let mut ba = vec![vec![Rat::zero(), Rat::one()]; a.len()];
    let mut bb = vec![vec![Rat::zero(), Rat::one()]; b.len()];
    add_breaks(a, b, &mut ba, &mut bb)?;
    let a_sum = integrate(a, b, &mut ba, true, true)?;
    let b_sum = integrate(b, a, &mut bb, false, false)?;
    let result = (a_sum.clone() - b_sum.clone()) * half();
    if result < Rat::zero() {
        return Err(format!(
            "negative difference area: support boundary sum={a_sum}, exterior boundary sum={b_sum}"
        ));
    }
    Ok(result)
}

pub(super) fn outside_area_below_owner_cutoff(
    support: &[[f64; 2]],
    exterior: &[[f64; 2]],
) -> Result<bool, String> {
    let area = difference_area(support, exterior)?;
    let cutoff = Rat::from_float(1e-8).ok_or("owner area cutoff is not finite")?;
    Ok(area < cutoff)
}

fn rings_intersect(a: &[Pt], b: &[Pt]) -> Result<bool, String> {
    for i in 0..a.len() {
        for j in 0..b.len() {
            if !segment_hits(&a[i], &a[(i + 1) % a.len()], &b[j], &b[(j + 1) % b.len()])?.is_empty()
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn input_vertices(ring: &[Pt]) -> Result<Vec<[f64; 2]>, String> {
    ring.iter()
        .map(|point| {
            point
                .input_xy
                .ok_or_else(|| "normalized case profile ring lost an input vertex".into())
        })
        .collect()
}

/// Prepared-case profile validation and winding normalization. Ring predicates
/// and containment use the exact represented-f64 arithmetic above.
pub(super) fn normalize_case_profile_region(
    exterior: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
) -> Result<super::NormalizedProfileRegion, String> {
    let exterior = validate_poly(exterior).map_err(|error| format!("exterior: {error}"))?;
    let mut validated_holes: Vec<Vec<Pt>> = Vec::with_capacity(holes.len());
    for (index, source) in holes.iter().enumerate() {
        let hole = validate_poly(source).map_err(|error| format!("hole {index}: {error}"))?;
        if rings_intersect(&exterior, &hole)? {
            return Err(format!("hole {index} intersects or touches the exterior"));
        }
        if locate(&hole[0], &exterior, None) != Location::Inside {
            return Err(format!("hole {index} must be strictly inside the exterior"));
        }
        for previous in &validated_holes {
            if rings_intersect(previous, &hole)? {
                return Err("case profile holes intersect or touch".into());
            }
            if locate(&hole[0], previous, None) == Location::Inside
                || locate(&previous[0], &hole, None) == Location::Inside
            {
                return Err("case profile holes must not be nested".into());
            }
        }
        validated_holes.push(hole);
    }

    let exterior = input_vertices(&exterior)?;
    let holes = validated_holes
        .into_iter()
        .map(|mut hole| {
            hole.reverse();
            input_vertices(&hole)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((exterior, holes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collinear_partial_overlap_reports_both_split_endpoints() {
        let a = point_from_f64([0.0, 3.0]).unwrap();
        let b = point_from_f64([10.0, 3.0]).unwrap();
        let c = point_from_f64([0.0, 3.0]).unwrap();
        let d = point_from_f64([5.0, 3.0]).unwrap();
        let hits = segment_hits(&a, &b, &c, &d).unwrap();
        assert_eq!(hits.len(), 2, "overlap start and end must both split");
        assert_eq!(hits[0].2, a);
        assert_eq!(hits[1].2, d);
    }

    #[test]
    fn collinear_chain_reports_each_subspan_endpoints() {
        let a = point_from_f64([0.0, 3.0]).unwrap();
        let b = point_from_f64([10.0, 3.0]).unwrap();
        let chain = [0.0, 2.0, 5.0].map(|x| point_from_f64([x, 3.0]).unwrap());
        for pair in chain.windows(2) {
            let hits = segment_hits(&a, &b, &pair[0], &pair[1]).unwrap();
            assert_eq!(hits.len(), 2, "each chain segment needs both endpoints");
            assert_eq!(hits[0].2, pair[0]);
            assert_eq!(hits[1].2, pair[1]);
        }
    }

    #[test]
    fn collinear_single_point_contact_reports_one_split() {
        let a = point_from_f64([0.0, 3.0]).unwrap();
        let b = point_from_f64([10.0, 3.0]).unwrap();
        let c = point_from_f64([10.0, 3.0]).unwrap();
        let d = point_from_f64([15.0, 3.0]).unwrap();
        let hits = segment_hits(&a, &b, &c, &d).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].2, b);
    }

    #[test]
    fn decimal_area_threshold_uses_the_current_f64_literal() {
        let exact_decimal = Rat::new(1.into(), 100_000_000.into());
        let current_literal = Rat::from_float(1e-8).unwrap();
        assert!(
            exact_decimal < current_literal,
            "strict source threshold accepts this exact decimal area because the source f64 literal is slightly greater"
        );
    }

    #[test]
    fn direct_ring_validation_rejects_unsafe_boundary_inputs() {
        assert!(validate_simple_ring(&[[0.0, 0.0], [1.0, 0.0]]).is_err());
        assert!(validate_simple_ring(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]]).is_ok());
        assert!(validate_simple_ring(&[[0.0, 0.0], [f64::INFINITY, 0.0], [0.0, 1.0]]).is_err());
        assert!(validate_simple_ring(&[[0.0, 0.0], [1_000_001.0, 0.0], [0.0, 1.0]]).is_ok());
        assert!(
            validate_simple_ring(&[[0.0, 0.0], [1.0e200, 0.0], [1.0e200, 1.0], [0.0, 1.0]]).is_ok()
        );
        let more_than_v4_limit = regular_test_ring(1025);
        assert!(validate_simple_ring(&more_than_v4_limit).is_ok());
        let distant = [[3.0, 3.0], [4.0, 3.0], [4.0, 4.0], [3.0, 4.0]];
        assert!(difference_area(&more_than_v4_limit, &distant).unwrap() > Rat::zero());
    }

    #[test]
    fn exact_adjacent_and_closing_duplicates_preserve_area_and_threshold() {
        let clip = [[3.0, 3.0], [4.0, 3.0], [4.0, 4.0], [3.0, 4.0]];
        let plain = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let duplicated = [
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 0.0],
            [2.0, 2.0],
            [0.0, 2.0],
            [0.0, 0.0],
            [0.0, 0.0],
        ];
        assert_eq!(
            difference_area(&plain, &clip).unwrap(),
            difference_area(&duplicated, &clip).unwrap()
        );
        assert_eq!(
            difference_area(&plain, &clip).unwrap(),
            Rat::from_integer(4.into())
        );

        let sliver = [[0.0, 0.0], [1e-8, 0.0], [1e-8, 1.0], [0.0, 1.0]];
        let duplicate_sliver = [
            [0.0, 0.0],
            [1e-8, 0.0],
            [1e-8, 0.0],
            [1e-8, 1.0],
            [0.0, 1.0],
            [0.0, 0.0],
        ];
        let base_area = difference_area(&sliver, &clip).unwrap();
        let duplicate_area = difference_area(&duplicate_sliver, &clip).unwrap();
        assert_eq!(base_area, duplicate_area);
        let threshold = Rat::from_float(1e-8).unwrap();
        assert_eq!(base_area < threshold, duplicate_area < threshold);
    }

    fn regular_test_ring(count: usize) -> Vec<[f64; 2]> {
        (0..count)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / count as f64;
                [angle.cos(), angle.sin()]
            })
            .collect()
    }
}
