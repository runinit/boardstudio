//! Private exact verifier; production access is owned by the qualification facade.

mod exact;
mod query;
mod source;
#[cfg(test)]
mod tests;

use crate::ImportedStep;
use exact::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use std::collections::{BTreeMap, BTreeSet};

const IDENTITY: [f64; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

#[derive(Debug)]
struct SurfaceBound {
    direct_mm: f64,
    precision_mm: f64,
    total_mm: f64,
}
struct QualifiedSurface {
    triangles: Vec<FTriangle>,
    #[cfg(test)]
    tags: Vec<usize>,
    bound: SurfaceBound,
    source_hull: [Point; 2],
    #[cfg(test)]
    transform_rounding_mm: f64,
    #[cfg(test)]
    repair_displacement_mm: Option<f64>,
}
fn transform(p: &Point, m: &[Q; 16]) -> Point {
    std::array::from_fn(|i| &m[i] * &p[0] + &m[i + 4] * &p[1] + &m[i + 8] * &p[2] + &m[i + 12])
}
fn float_transform(p: FPoint, m: &[f64; 16]) -> FPoint {
    std::array::from_fn(|i| m[i] * p[0] + m[i + 4] * p[1] + m[i + 8] * p[2] + m[i + 12])
}
fn affine(m: [f64; 16]) -> Check<([Q; 16], Q)> {
    if [m[3], m[7], m[11], m[15]] != [0., 0., 0., 1.] {
        return Err("invalid affine row");
    }
    let matrix = m
        .into_iter()
        .map(|x| point([x, 0., 0.]).map(|p| p[0].clone()))
        .collect::<Check<Vec<_>>>()?;
    let matrix: [Q; 16] = matrix.try_into().map_err(|_| "affine size")?;
    let cols: [Point; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| matrix[i * 4 + j].clone()));
    if dot(&cols[0], &cross(&cols[1], &cols[2])).is_zero() {
        return Err("singular affine row");
    }
    let one = (0..3)
        .map(|i| (0..3).map(|j| matrix[i * 4 + j].abs()).sum::<Q>())
        .max()
        .unwrap();
    let infinity = (0..3)
        .map(|i| (0..3).map(|j| matrix[j * 4 + i].abs()).sum::<Q>())
        .max()
        .unwrap();
    Ok((matrix, one * infinity))
}
fn qualify(
    model: &ImportedStep,
    positions: &[FPoint],
    tags: &[usize],
    row: [f64; 16],
) -> Check<QualifiedSurface> {
    if !model.accuracy().mesh_closed_oriented {
        return Err("unsupported nonmanifold or unoriented source surface");
    }
    if positions.is_empty() || positions.len() != tags.len() * 3 || tags.len() > 4096 {
        return Err("final buffer shape or cap");
    }
    if model.accuracy().fallback_meshed_faces != 0
        || model.accuracy().relaxed_beyond_declared_precision
    {
        return Err("unsupported preview repair or fallback");
    }
    if model.line_presentation().occurrences().len() != 0 {
        return Err("unsupported source wire occurrence");
    }
    let source = source::read(model.original_bytes())?;
    if model.solid_count() != source.solids
        || model.occurrence_count() != source.solids
        || model.source_faces().len() != source.faces.len()
        || model.face_occurrences().len() != source.faces.len()
    {
        return Err("incomplete face or occurrence census");
    }
    let (matrix, norm_squared) = affine(row)?;
    let norm = Q::from_float(sqrt_bounds(&norm_squared)?[1]).unwrap();
    let precision = upper(&(&source.precision * &norm))?;
    let mut transformed = Vec::with_capacity(positions.len());
    let mut rounding = Q::zero();
    for &p in positions {
        let exact = transform(&point(p)?, &matrix);
        let actual = float_transform(p, &row);
        rounding = rounding.max(norm2(&sub(&point(actual)?, &exact)));
        transformed.push(actual);
    }
    let mut by_face = BTreeMap::<(u64, usize, u64), Vec<Triangle>>::new();
    let mut occurrences = BTreeSet::new();
    for (t, &tag) in transformed.as_chunks::<3>().0.iter().zip(tags) {
        let occurrence = model
            .face_occurrences()
            .get(tag)
            .ok_or("invalid final triangle tag")?;
        let face = model
            .source_faces()
            .get(occurrence.source_face)
            .ok_or("invalid final source face")?;
        if occurrence.occurrence >= model.occurrence_count() {
            return Err("invalid final occurrence");
        }
        occurrences.insert(tag);
        by_face
            .entry((face.solid_id, face.shell_index, face.face_id))
            .or_default()
            .push([point(t[0])?, point(t[1])?, point(t[2])?]);
    }
    if occurrences.len() != model.face_occurrences().len() {
        return Err("missing final face occurrence");
    }
    let mut maximum = Q::zero();
    let mut radius = Q::zero();
    let mut hull: Option<[Point; 2]> = None;
    for face in source.faces {
        let key = (face.solid, face.shell, face.id);
        let catalogue = model
            .source_faces()
            .iter()
            .filter(|f| (f.solid_id, f.shell_index, f.face_id) == key)
            .collect::<Vec<_>>();
        if catalogue.len() != 1
            || catalogue[0].support_id != face.support
            || catalogue[0].support_kind != "PLANE"
        {
            return Err("source face census mismatch");
        }
        for p in face.loops.iter().flatten() {
            radius = radius.max(
                (&p[0] - &source.child_origin[0]).abs() + (&p[1] - &source.child_origin[1]).abs(),
            );
        }
        let loops = face
            .loops
            .into_iter()
            .map(|r| {
                r.iter()
                    .map(|p| transform(&transform(p, &source.placement), &matrix))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for p in loops.iter().flatten() {
            if let Some(h) = &mut hull {
                for i in 0..3 {
                    h[0][i] = h[0][i].clone().min(p[i].clone());
                    h[1][i] = h[1][i].clone().max(p[i].clone());
                }
            } else {
                hull = Some([p.clone(), p.clone()]);
            }
        }
        maximum = maximum.max(source::face_bound(
            &loops,
            &by_face.remove(&key).ok_or("missing final source face")?,
        )?);
    }
    if !by_face.is_empty() {
        return Err("extra final source face");
    }
    let placement_error = source.placement_error_per_radius * radius * norm;
    let direct = upper(&(Q::from_float(sqrt_bounds(&maximum)?[1]).unwrap() + &placement_error))?;
    if direct > 1e-8 {
        return Err("private direct-bound cap");
    }
    let mut hull = hull.ok_or("source hull")?;
    let [minimum, maximum] = &mut hull;
    for (lo, hi) in minimum.iter_mut().zip(maximum) {
        *lo -= &placement_error;
        *hi += &placement_error;
    }
    Ok(QualifiedSurface {
        triangles: transformed.as_chunks::<3>().0.to_vec(),
        #[cfg(test)]
        tags: tags.to_vec(),
        bound: SurfaceBound {
            direct_mm: direct,
            precision_mm: precision,
            total_mm: upper(&(Q::from_float(direct).unwrap() + Q::from_float(precision).unwrap()))?,
        },
        source_hull: hull,
        #[cfg(test)]
        transform_rounding_mm: sqrt_bounds(&rounding)?[1],
        #[cfg(test)]
        repair_displacement_mm: None,
    })
}

// New narrow owner-side functions keep the exact verifier and source AST private.
pub(crate) fn asset_bound(model: &ImportedStep) -> Result<[f64; 3], Vec<String>> {
    qualify(
        model,
        model.preview_positions_f64(),
        model.triangle_faces(),
        IDENTITY,
    )
    .map(|s| [s.bound.direct_mm, s.bound.precision_mm, s.bound.total_mm])
    .map_err(|reason| vec![reason.into()])
}

fn local_positions(owner: &crate::qualification::PreparedAnalysisMesh) -> &[FPoint] {
    use crate::qualification::PreparedAnalysisView;
    match owner.export_view() {
        PreparedAnalysisView::Generated(view) => view.emission.positions_mm_f64.as_chunks::<3>().0,
        PreparedAnalysisView::Vendor(view) => view.positions_mm_f64,
    }
}
fn final_triangle_count(owner: &crate::qualification::PreparedAnalysisMesh) -> Option<usize> {
    use crate::qualification::PreparedAnalysisView;
    let (positions, tags) = match owner.export_view() {
        PreparedAnalysisView::Generated(view) => (
            view.emission.positions_mm_f64.len(),
            view.emission.triangle_face_occurrence_indices.len(),
        ),
        PreparedAnalysisView::Vendor(view) => (
            view.positions_mm_f64.len().checked_mul(3)?,
            view.triangle_face_occurrence_indices.len(),
        ),
    };
    (positions != 0 && positions.is_multiple_of(9) && tags == positions / 9).then_some(tags)
}
fn world_surface(
    owner: &crate::qualification::PreparedAnalysisMesh,
    row: [f64; 16],
    world: &[FPoint],
) -> Check<(
    crate::qualification::WorldSourceBound,
    Option<QualifiedSurface>,
)> {
    use crate::qualification::{SourceQualification, WorldSourceBound};
    let (matrix, norm_squared) = affine(row)?;
    let norm_upper = sqrt_bounds(&norm_squared)?[1];
    let norm = Q::from_float(norm_upper).ok_or("row norm overflow")?;
    let local = local_positions(owner);
    let mut rounding = Q::zero();
    for (p, actual) in local.iter().zip(world) {
        let exact = transform(&point(*p)?, &matrix);
        rounding = rounding.max(norm2(&sub(&point(*actual)?, &exact)));
    }
    let triangles = world.as_chunks::<3>().0.to_vec();
    let transform_rounding_mm = sqrt_bounds(&rounding)?[1];
    let SourceQualification::Bounded {
        direct_bound_mm,
        source_precision_mm,
        total_bound_mm,
        ..
    } = owner.source_qualification()
    else {
        return Ok((
            WorldSourceBound::Unknown {
                reasons: vec!["source-proof-unavailable".into()],
            },
            None,
        ));
    };
    let direct = upper(&(Q::from_float(*direct_bound_mm).unwrap() * &norm))?;
    let precision = upper(&(Q::from_float(*source_precision_mm).unwrap() * &norm))?;
    let round = Q::from_float(transform_rounding_mm).unwrap();
    let total = upper(
        &((Q::from_float(direct).unwrap() + Q::from_float(precision).unwrap() + &round)
            .max(Q::from_float(*total_bound_mm).unwrap() * &norm + round)),
    )?;
    // A symmetric bound contains the complete nominal source surface in the
    // emitted surface's expanded AABB. Every bounded solid lies in its surface
    // AABB; this supports slabs, never parity/global correspondence.
    let mut hull: [Point; 2] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for axis in 0..3 {
        let lo = world.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
        let hi = world
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        hull[0][axis] = Q::from_float(lo).ok_or("empty final surface")?;
        hull[1][axis] = Q::from_float(hi).ok_or("empty final surface")?;
    }
    let source = QualifiedSurface {
        triangles: triangles.clone(),
        #[cfg(test)]
        tags: Vec::new(),
        bound: SurfaceBound {
            direct_mm: direct,
            precision_mm: precision,
            total_mm: total,
        },
        source_hull: hull,
        #[cfg(test)]
        transform_rounding_mm,
        #[cfg(test)]
        repair_displacement_mm: None,
    };
    let report = WorldSourceBound::Bounded {
        mesh_identity: owner.analysis_mesh().mesh_identity.clone(),
        method: "scaled-local-plus-row-rounding".into(),
        norm_upper,
        direct_bound_mm: direct,
        source_precision_mm: precision,
        transform_rounding_mm,
        total_bound_mm: total,
    };
    Ok((report, Some(source)))
}
pub(crate) fn prepared_pair(
    owners: [&crate::qualification::PreparedAnalysisMesh; 2],
    rows: [[f64; 16]; 2],
    limits: crate::qualification::Limits,
) -> crate::qualification::QueryEvidence {
    use crate::qualification::{
        ContactBoxPair, ContactRegion, Containment, MeshDistance, QueryEvidence, WorldSourceBound,
    };
    let unknown = |reason: &str| QueryEvidence {
        limits,
        mesh_distance: MeshDistance::Unknown {
            reasons: vec![reason.into()],
        },
        source_bounds: std::array::from_fn(|_| WorldSourceBound::Unknown {
            reasons: vec![reason.into()],
        }),
        contact_region: ContactRegion::Unknown {
            reasons: vec![reason.into()],
        },
        containment: Containment::Unknown {
            reasons: vec![reason.into()],
        },
        world_positions: std::array::from_fn(|_| Vec::new()),
        counters: [0, 0],
    };
    // Refuse effective caps before exact row arithmetic, source hulls or proof
    // allocations. The float-only view remains available for raw observations.
    let mut refusal = limits.validate().err();
    if refusal.is_none() {
        for owner in owners {
            match final_triangle_count(owner) {
                None => {
                    refusal = Some("final buffer shape".into());
                    break;
                }
                Some(count) if count > limits.max_triangles as usize => {
                    refusal = Some("triangle cap".into());
                    break;
                }
                Some(_) => (),
            }
        }
    }
    let world: [Vec<FPoint>; 2] = std::array::from_fn(|side| {
        local_positions(owners[side])
            .iter()
            .map(|&p| float_transform(p, &rows[side]))
            .collect()
    });
    if refusal.is_none() {
        let cap_min = 2f64.powi(limits.minimum_exponent);
        let cap_max = 2f64.powi(limits.maximum_exponent);
        let local = owners
            .iter()
            .flat_map(|owner| local_positions(owner).iter().flatten());
        if local
            .chain(rows.iter().flatten())
            .chain(world.iter().flat_map(|points| points.iter().flatten()))
            .any(|x| !x.is_finite() || (*x != 0. && !(cap_min..=cap_max).contains(&x.abs())))
        {
            refusal = Some("exponent cap".into());
        }
    }
    let mut report = unknown(refusal.as_deref().unwrap_or("query-proof-unavailable"));
    report.world_positions = world.each_ref().map(|ps| ps.as_flattened().to_vec());
    if refusal.is_some() {
        return report;
    }
    let [(bound_a, source_a), (bound_b, source_b)] = std::array::from_fn(|side| {
        world_surface(owners[side], rows[side], &world[side]).unwrap_or_else(|reason| {
            (
                WorldSourceBound::Unknown {
                    reasons: vec![reason.into()],
                },
                None,
            )
        })
    });
    let [a, b] = world.map(|p| p.as_chunks::<3>().0.to_vec());
    report.source_bounds = [bound_a, bound_b];
    let sources = source_a
        .as_ref()
        .zip(source_b.as_ref())
        .map(|(a, b)| [a, b]);
    if let Some([a, b]) = sources {
        if let Ok(query::Containment::RuledOutSlab {
            axis,
            left_upper,
            right_lower,
        }) = query::slab(a, b)
        {
            report.containment = Containment::Slab {
                method: "slab".into(),
                axis: axis as u32,
                left_upper_mm: left_upper,
                right_lower_mm: right_lower,
            };
        } else {
            report.containment = Containment::Unknown {
                reasons: vec!["no separating slab; parity is unqualified".into()],
            };
        }
    }
    let effective = query::Limits {
        max_triangles: limits.max_triangles as usize,
        max_node_pairs: limits.max_node_pairs as usize,
        max_exact_pairs: limits.max_exact_pairs as usize,
        max_contact_boxes: limits.max_contact_boxes as usize,
        numerical_width_mm: limits.requested_numerical_width_mm,
    };
    // The exact verifier validates and recomputes this feasible triangle seed;
    // producer order is free and no fixture report participates in the proof.
    match query::distance(&a, &b, [0, 0], sources, effective) {
        Ok(distance) => {
            report.counters = [distance.exact_pairs, distance.node_pairs];
            debug_assert_eq!(distance.limits.max_exact_pairs, effective.max_exact_pairs);
            report.mesh_distance = MeshDistance::Bounded {
                lower_mm: distance.mm[0],
                upper_mm: distance.mm[1],
                witness_triangles: distance.witness.map(|i| i as u32),
            };
            report.contact_region = match distance.contact {
                query::ContactRegion::Unknown(reason) => ContactRegion::Unknown {
                    reasons: vec![reason.into()],
                },
                query::ContactRegion::Boxes(boxes) => ContactRegion::Boxes {
                    source_inflation_mm: sources.unwrap().map(|s| s.bound.total_mm),
                    pairs: boxes
                        .into_iter()
                        .map(|[left_mm, right_mm]| ContactBoxPair { left_mm, right_mm })
                        .collect(),
                },
            };
        }
        Err(reason) => {
            report.mesh_distance = MeshDistance::Unknown {
                reasons: vec![reason.into()],
            }
        }
    }
    report
}
