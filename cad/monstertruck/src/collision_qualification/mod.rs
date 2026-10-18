//! Private proof checkpoint. See CONTRACT.md; no production qualification is published.

mod exact;
mod query;
mod source;
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
    tags: Vec<usize>,
    bound: SurfaceBound,
    source_hull: [Point; 2],
    transform_rounding_mm: f64,
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
        tags: tags.to_vec(),
        bound: SurfaceBound {
            direct_mm: direct,
            precision_mm: precision,
            total_mm: upper(&(Q::from_float(direct).unwrap() + Q::from_float(precision).unwrap()))?,
        },
        source_hull: hull,
        transform_rounding_mm: sqrt_bounds(&rounding)?[1],
        repair_displacement_mm: None,
    })
}
