//! Concrete planar PCurve proof. Source selection and raw conversion stay unknown.
use super::*;
use monstertruck_io::step::load::step_geometry::{
    Curve2D, ElementarySurface, Plane, Surface, SurfaceCurve3D, SurfaceCurveAssociatedGeometry,
    SurfaceCurveRepresentation,
};

fn master(curve: &SurfaceCurve3D) -> Result<(&Curve2D, &Plane), &'static str> {
    let slot = match curve.master_representation() {
        SurfaceCurveRepresentation::ParameterCurve0 => 0,
        SurfaceCurveRepresentation::ParameterCurve1 => 1,
        _ => return Err("planar seam requires the same selected PCurve"),
    };
    let Some(SurfaceCurveAssociatedGeometry::ParameterCurve(master)) =
        curve.associated_geometry().get(slot)
    else {
        return Err("missing selected planar PCurve");
    };
    let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = master.surface().as_ref()
    else {
        return Err("nonplanar selected PCurve remains unsupported");
    };
    Ok((master.curve(), plane))
}
fn spline(curve: &SurfaceCurve3D, seam: Option<f64>) -> Result<Spline, &'static str> {
    let (uv, plane) = master(curve)?;
    let (degree, count) = match uv {
        Curve2D::BsplineCurve(c) => (c.degree(), c.control_points().len()),
        Curve2D::NurbsCurve(c) => (c.degree(), c.control_points().len()),
        _ => return Err("selected planar carrier is not a spline"),
    };
    if degree == 0 || degree > 16 || count > 8192 || count <= degree {
        return Err("planar seam control proof resource cap");
    }
    let (knots, uv) = match uv {
        Curve2D::BsplineCurve(c) => (
            c.knot_vector(),
            c.control_points()
                .iter()
                .map(|p| Ok([scalar(p.x)?, scalar(p.y)?, Q::one()]))
                .collect::<Result<Vec<_>, &'static str>>()?,
        ),
        Curve2D::NurbsCurve(c) => (
            c.knot_vector(),
            c.control_points()
                .iter()
                .map(|p| Ok([scalar(p.x)?, scalar(p.y)?, scalar(p.z)?]))
                .collect::<Result<Vec<_>, &'static str>>()?,
        ),
        _ => unreachable!(),
    };
    if knots.len() != count + degree + 1 {
        return Err("invalid planar seam knot coverage");
    }
    let knots = knots
        .iter()
        .map(|k| scalar(*k))
        .collect::<Result<Vec<_>, _>>()?;
    let seam = seam.map(scalar).transpose()?;
    if knots.windows(2).any(|k| k[0] > k[1])
        || uv.iter().any(|p| !p[2].is_positive())
        || knots.windows(degree + 1).any(|run| {
            run[0] == run[degree]
                && run[0] > knots[degree]
                && run[0] < knots[count]
                && seam.as_ref() != Some(&run[0])
        })
    {
        return Err("ambiguous planar spline piece or denominator");
    }
    // Public constructor equality authenticates the retained raw plane points
    // and identity transform; no evaluation samples or rounded axis inference.
    let o = plane.origin();
    let p = o + plane.axis_u();
    let q = o + plane.axis_v();
    if *plane != Plane::new(o, p, q) {
        return Err("planar seam raw affine coefficients are unavailable");
    }
    let o = [scalar(o.x)?, scalar(o.y)?, scalar(o.z)?];
    let p = [scalar(p.x)?, scalar(p.y)?, scalar(p.z)?];
    let q = [scalar(q.x)?, scalar(q.y)?, scalar(q.z)?];
    let controls = uv
        .iter()
        .map(|uv| {
            std::array::from_fn(|axis| {
                if axis == 3 {
                    uv[2].clone()
                } else {
                    &o[axis] * &uv[2]
                        + (&p[axis] - &o[axis]) * &uv[0]
                        + (&q[axis] - &o[axis]) * &uv[1]
                }
            })
        })
        .collect();
    Ok(Spline {
        degree,
        knots,
        controls,
    })
}

/// Geometric repair and numerical carrier deviation, separately. Only sealed
/// same-call owner stages may call this; a captured pair alone is not authority.
pub(in crate::step::repair) fn bound(
    stage: &str,
    change: StepRepairCurveChange<'_>,
    factor: f64,
    placement: Matrix4,
) -> Result<(f64, f64), &'static str> {
    let (Curve3D::SurfaceCurve(a), Curve3D::SurfaceCurve(b)) = change.curves() else {
        return Err("missing same-master planar pair");
    };
    proof(stage, a, b, change.domains(), factor, placement)
}
fn proof(
    stage: &str,
    a: &SurfaceCurve3D,
    b: &SurfaceCurve3D,
    domains: ((f64, f64), (f64, f64)),
    factor: f64,
    placement: Matrix4,
) -> Result<(f64, f64), &'static str> {
    if a.master_representation() != b.master_representation() || master(a)?.1 != master(b)?.1 {
        return Err("selected planar master/surface mismatch");
    }
    if placement.x.w != 0.
        || placement.y.w != 0.
        || placement.z.w != 0.
        || placement.w.w != 1.
        || !(0..4).all(|i| (0..4).all(|j| placement[i][j].is_finite()))
        || !factor.is_finite()
        || factor <= 0.
    {
        return Err("invalid planar seam occurrence frame");
    }
    let scale = scalar(factor)?;
    match stage {
        "same-master-planar-pcurve-seam-extension-v1" => {
            let before = spline(a, None)?;
            let da = before.domain(domains.0)?;
            let (s, e) = domains.0;
            let span = e - s;
            let join = s + span;
            if !span.is_finite()
                || span <= 0.
                || domains.1 != (s, e + span)
                || scalar(s)? != before.knots[before.degree]
                || scalar(e)? != before.knots[before.controls.len()]
                || before.knots.first() != Some(&scalar(s)?)
                || before.knots.last() != Some(&scalar(e)?)
                || a.active_trim().is_some()
                || b.active_trim().is_some()
            {
                return Err("missing complete planar seam source/copy domains");
            }
            let after = spline(b, Some(join))?;
            let join_q = scalar(join)?;
            if after.degree != before.degree
                || after.knots.iter().filter(|k| *k == &join_q).count() != after.degree + 1
            {
                return Err("missing actual two-piece planar seam boundary");
            }
            let d0 = after.domain((s, join))?;
            let d1 = after.domain((join, e + span))?;
            // BOTH full finite copies, not one affine map across a doubled domain.
            // At the join evaluation selects the right copy's front. Original
            // back remains covered at the doubled carrier's final endpoint.
            let numeric = polynomial_bound(&before, &da, &after, &d0, &scale, placement)?.max(
                polynomial_bound(&before, &da, &after, &d1, &scale, placement)?,
            );
            Ok((0., numeric))
        }
        "same-master-planar-pcurve-trim-v1" => {
            let source = spline(a, join(a)?)?;
            source.domain(domains.0)?;
            let full = master(a)?.0.range_tuple();
            if !domains.0.0.is_finite()
                || !domains.0.1.is_finite()
                || domains.0.0 >= domains.0.1
                || domains.0.0 < full.0
                || domains.0.1 > full.1
                || domains.0 != domains.1
                || b.active_trim() != Some(domains.1)
                || a.kind() != b.kind()
                || a.leader() != b.leader()
                || a.associated_geometry() != b.associated_geometry()
            {
                return Err("finite trim installation changed its retained carrier/domain");
            }
            // Its SAME finite view is unchanged; choosing that region remains
            // independently Unsupported and is never certified by this zero.
            Ok((0., 0.))
        }
        "same-master-planar-pcurve-sense-inversion-v1" => {
            if a.inverse() != *b {
                return Err("planar inversion is not the actual inverse");
            }
            let before = spline(a, join(a)?)?;
            let after = spline(b, join(b)?)?;
            for (curve, source) in [(&before, a), (&after, b)] {
                if let Some(join) = join(source)? {
                    continuous_join(curve, join)?;
                }
            }
            let da = before.domain(domains.0)?;
            let db = after.domain(domains.1)?;
            Ok((
                0.,
                polynomial_bound(&before, &da, &after, &db, &scale, placement)?,
            ))
        }
        _ => Err("unqualified planar seam stage"),
    }
}

fn join(curve: &SurfaceCurve3D) -> Result<Option<f64>, &'static str> {
    let (degree, knots, count) = match master(curve)?.0 {
        Curve2D::BsplineCurve(c) => (c.degree(), c.knot_vector(), c.control_points().len()),
        Curve2D::NurbsCurve(c) => (c.degree(), c.knot_vector(), c.control_points().len()),
        _ => return Err("planar inversion requires concrete spline controls"),
    };
    if count <= degree || knots.len() != count + degree + 1 {
        return Err("invalid planar spline coverage");
    }
    let mut join = None;
    for run in knots
        .windows(degree + 1)
        .filter(|run| run[0] == run[degree] && run[0] > knots[degree] && run[0] < knots[count])
    {
        if join.is_some_and(|value| value != run[0]) {
            return Err("multiple planar seam joins remain unsupported");
        }
        join = Some(run[0]);
    }
    Ok(join)
}
fn continuous_join(curve: &Spline, join: f64) -> Result<(), &'static str> {
    let join = scalar(join)?;
    let left = curve
        .knots
        .iter()
        .filter(|k| **k < join)
        .max()
        .ok_or("missing left seam span")?;
    let right = curve
        .knots
        .iter()
        .filter(|k| **k > join)
        .min()
        .ok_or("missing right seam span")?;
    let a = curve.piece(&(left.clone(), &join - left), &Q::zero(), &Q::one())?;
    let b = curve.piece(&(join.clone(), right - &join), &Q::zero(), &Q::one())?;
    let a = a.each_ref().map(|p| p.iter().sum::<Q>());
    let b = b.each_ref().map(|p| p[0].clone());
    if (0..3).any(|i| &a[i] * &b[3] != &b[i] * &a[3]) {
        return Err("one-sided planar seam inversion is unsupported");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_io::step::load::convert::StepRepairConversion;
    const SOURCE: &str = include_str!("../../../../tests/fixtures/repair-planar-seam.step");
    fn closed_source() -> String {
        assert_eq!(SOURCE.matches("#10=EDGE_LOOP('',(#9));").count(), 1);
        SOURCE.replace("#13=CLOSED_SHELL('',(#12));", "#13=CLOSED_SHELL('',(#12,#75));")
            .replace("#10=EDGE_LOOP('',(#9));", "#10=EDGE_LOOP('',(#9,#74));")
            .replace("ENDSEC;END-ISO-10303-21;", "#70=DIRECTION('',(0.,-1.,0.));#71=VECTOR('',#70,1.);#72=LINE('',#34,#71);#73=EDGE_CURVE('',#32,#2,#72,.T.);#74=ORIENTED_EDGE('',*,*,#73,.T.);#75=ADVANCED_FACE('',(#11),#6,.F.);ENDSEC;END-ISO-10303-21;")
    }
    fn owner(text: &str) -> StepRepairConversion {
        let mut table = Table::from_step(text).unwrap();
        table.set_preview_floor_mm(0.02);
        table
            .to_source_bound_solid_with_repair_checked(
                14,
                Some(15),
                &StepRepairSource::from_step_bytes(text.as_bytes()).unwrap(),
            )
            .unwrap()
    }
    #[test]
    fn actual_seam_both_copies_trim_and_sense_are_measured_separately() {
        for reversed in [false, true] {
            let text = if reversed {
                SOURCE.replace(
                    "#8=EDGE_CURVE('',#2,#32,#7,.T.);",
                    "#8=EDGE_CURVE('',#32,#2,#7,.F.);",
                )
            } else {
                SOURCE.to_owned()
            };
            let owner = owner(&text);
            let mut count = 0;
            for op in owner
                .operations(0)
                .filter(|op| op.stage().starts_with("same-master-planar-pcurve-"))
            {
                let measured = bound(
                    op.stage(),
                    op.curve_change().unwrap(),
                    1000.,
                    Matrix4::from_scale(-2.),
                )
                .unwrap();
                assert_eq!(measured, (0., 0.), "{}", op.stage());
                count += 1;
            }
            assert_eq!(count, if reversed { 3 } else { 2 });
            let mut ledger = Ledger::default();
            ledger.conversion_operations(
                owner.operations(0),
                owner.complete_bound(0).is_ok(),
                0,
                1000.,
                Matrix4::from_scale(-2.),
                7,
            );
            assert!(ledger.enforce_finite_lines(7, 0.02).is_ok());
            assert!(
                ledger.enforce_finite_lines(7, 0.).is_ok(),
                "exact zero equality"
            );
            assert_eq!(
                ledger.complete_bound_mm(),
                None,
                "original selection/surface remain unknown"
            );
            assert!(owner.complete_bound(0).is_err());
        }
    }
    #[test]
    fn actual_shift_rounding_is_numeric_and_uses_world_units_once() {
        let text = SOURCE.replace("(0.,1.,2.,3.)", "(0.1,0.4,1.1,1.4)");
        assert_ne!(text, SOURCE);
        let owner = owner(&text);
        let op = owner
            .operations(0)
            .find(|op| op.stage() == "same-master-planar-pcurve-seam-extension-v1")
            .unwrap();
        let local = bound(
            op.stage(),
            op.curve_change().unwrap(),
            1.,
            Matrix4::identity(),
        )
        .unwrap();
        assert_eq!(local.0, 0.);
        assert!(local.1 > 0. && local.1 < 1e-12, "{local:?}");
        let world = bound(
            op.stage(),
            op.curve_change().unwrap(),
            1000.,
            Matrix4::from_scale(-2.),
        )
        .unwrap();
        assert_eq!(world.0, 0.);
        assert!(world.1 >= local.1 * 2000. && world.1 <= (local.1 * 2000.).next_up());
        let mut ledger = Ledger::default();
        ledger.conversion_operations(
            owner.operations(0),
            owner.complete_bound(0).is_ok(),
            0,
            1000.,
            Matrix4::identity(),
            3,
        );
        assert!(
            ledger
                .0
                .iter()
                .any(|op| op.stage == "planar-pcurve-numeric-conversion-v1"
                    && op.diagnostic_only
                    && op.bound_mm.is_ok_and(|v| v > 0.))
        );
        assert!(
            ledger.enforce_finite_lines(3, 0.).is_ok(),
            "numeric deviation is separate from geometric repair"
        );
    }
    #[test]
    fn unclamped_candidate_cannot_borrow_the_active_domain_seam_proof() {
        let text = SOURCE.replace(
            "((3,2,2,3),(0.,1.,2.,3.),.UNSPECIFIED.)",
            "((1,1,1,2,2,1,1,1),(-1.,-0.00000001,0.,1.,2.,3.,3.00000001,4.),.UNSPECIFIED.)",
        );
        assert_ne!(text, SOURCE);
        let owner = owner(&text);
        let mut table = Table::from_step(&text).unwrap();
        table.set_preview_floor_mm(0.02);
        let legacy = table
            .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&14], 15)
            .unwrap();
        assert_eq!(
            owner.source().shells()[0].shell(),
            legacy.shells()[0].shell()
        );
        let Curve3D::SurfaceCurve(actual) = &owner.source().shells()[0].shell().edges[0].curve
        else {
            panic!()
        };
        assert_eq!(
            actual.master_representation(),
            SurfaceCurveRepresentation::Curve3D
        );
        assert!(
            owner
                .operations(0)
                .all(|op| !op.stage().starts_with("same-master-planar-pcurve-"))
        );
        assert!(owner.complete_bound(0).is_err());
    }
    #[test]
    fn actual_seam_import_retains_original_bytes_and_unknown_asset_accuracy() {
        assert!(
            import_step(SOURCE.as_bytes().to_vec())
                .err()
                .unwrap()
                .contains("open boundary")
        );
        let text = closed_source();
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert!(model._repair_ledger.0.iter().any(|op| op.stage
            == "same-master-planar-pcurve-seam-extension-v1"
            && op.bound_mm == Ok(0.)));
    }
    fn extension(owner: &StepRepairConversion) -> (SurfaceCurve3D, SurfaceCurve3D) {
        let op = owner
            .operations(0)
            .find(|op| op.stage() == "same-master-planar-pcurve-seam-extension-v1")
            .unwrap();
        let view = op.curve_change().unwrap();
        let (Curve3D::SurfaceCurve(a), Curve3D::SurfaceCurve(b)) = view.curves() else {
            panic!()
        };
        (a.clone(), b.clone())
    }
    #[test]
    fn whole_nets_detect_changed_interior_and_reject_missing_or_mapped_domains() {
        let owner = owner(SOURCE);
        let (a, b) = extension(&owner);
        let mut geometry = b.associated_geometry().to_vec();
        let SurfaceCurveAssociatedGeometry::ParameterCurve(c) = &mut geometry[0] else {
            panic!()
        };
        let mut uv = c.curve().as_ref().clone();
        let Curve2D::NurbsCurve(spline) = &mut uv else {
            panic!()
        };
        spline.control_points_mut().nth(1).unwrap().y += 0.125;
        *c = monstertruck_io::step::load::step_geometry::StepParameterCurve::new(
            Box::new(uv),
            c.surface().clone(),
        );
        let changed = SurfaceCurve3D::new(
            b.kind(),
            Box::new(b.leader().clone()),
            geometry,
            b.master_representation(),
        );
        assert_eq!(b.front(), changed.front());
        assert_eq!(b.back(), changed.back());
        let stage = "same-master-planar-pcurve-seam-extension-v1";
        let measured = proof(
            stage,
            &a,
            &changed,
            ((0., 3.), (0., 6.)),
            1.,
            Matrix4::identity(),
        )
        .unwrap();
        assert!(
            measured.1 >= 0.125,
            "interior displacement cannot be numerical zero: {measured:?}"
        );
        assert_eq!(
            proof(stage, &a, &b, ((0., 3.), (0., 6.)), 1., Matrix4::identity()),
            Ok((0., 0.))
        );
        for domains in [
            ((0., 3.), (0., 3.)),
            ((0., 3.), (3., 6.)),
            ((3., 0.), (0., 6.)),
            ((0., 2.), (0., 6.)),
        ] {
            assert!(proof(stage, &a, &b, domains, 1., Matrix4::identity()).is_err());
        }
        assert!(proof(stage, &b, &a, ((0., 6.), (0., 3.)), 1., Matrix4::identity()).is_err());
        assert!(
            proof(
                "selected-master-v1",
                &a,
                &b,
                ((0., 3.), (0., 6.)),
                1.,
                Matrix4::identity()
            )
            .is_err()
        );
    }
}
