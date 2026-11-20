use super::{KnotVector, Point3};

/// Exact endpoint basis selectors and equal transverse coefficients prove the
/// entire finite boundary curves equal, without declaring an evaluator period.
pub(super) fn finite_boundary_equal_axes<P: PartialEq>(
    knots: &(KnotVector, KnotVector),
    controls: &[Vec<P>],
    valid_coefficient: impl Fn(&P) -> bool,
) -> Option<[bool; 2]> {
    if controls.is_empty()
        || controls[0].is_empty()
        || controls.iter().any(|row| row.len() != controls[0].len())
        || controls
            .iter()
            .flatten()
            .any(|point| !valid_coefficient(point))
    {
        return None;
    }
    let degree_u = knots.0.len().checked_sub(controls.len().checked_add(1)?)?;
    let degree_v = knots
        .1
        .len()
        .checked_sub(controls[0].len().checked_add(1)?)?;

    if !endpoint_interpolating(&knots.0, degree_u, controls.len())
        || !endpoint_interpolating(&knots.1, degree_v, controls[0].len())
    {
        return Some([false, false]);
    }
    Some([
        controls.first() == controls.last(),
        controls.iter().all(|row| row.first() == row.last()),
    ])
}

fn endpoint_interpolating(knots: &KnotVector, degree: usize, count: usize) -> bool {
    if count < degree + 1 || knots.len() != count + degree + 1 {
        return false;
    }
    let (first, last) = (knots[degree], knots[count]);
    first < last
        && knots.iter().all(|value| value.is_finite())
        && knots.range_length().is_finite()
        && knots.windows(2).all(|pair| {
            let span = pair[1] - pair[0];
            span == 0.0 || (span > 0.0 && span.recip().is_finite())
        })
        && knots
            .windows(degree + 2)
            .all(|window| window.first() != window.last())
        && first < knots[degree + 1]
        && knots[count - 1] < last
        && knots[1..degree + 1].iter().all(|value| *value == first)
        && knots[count..count + degree]
            .iter()
            .all(|value| *value == last)
}

/// A clamped point boundary and a strictly separated coefficient cone prove
/// its point has no other preimage in the finite source carrier.
pub(super) fn finite_collapsed_boundaries(
    knots: &(KnotVector, KnotVector),
    controls: &[Vec<Point3>],
    valid_coefficient: impl Fn(&Point3) -> bool,
) -> Option<[[bool; 2]; 2]> {
    // Reuse all representation and machine-arithmetic checks of the existing
    // finite-boundary proof before indexing or computing endpoint selectors.
    finite_boundary_equal_axes(knots, controls, valid_coefficient)?;
    let rows = controls.len();
    let columns = controls[0].len();
    let degree_u = knots.0.len().checked_sub(rows.checked_add(1)?)?;
    let degree_v = knots.1.len().checked_sub(columns.checked_add(1)?)?;
    if !endpoint_interpolating(&knots.0, degree_u, rows)
        || !endpoint_interpolating(&knots.1, degree_v, columns)
        // Unlike the existing finite-equality proof's active endpoint selectors,
        // this conservative capability supports only full p+1 clamping.
        || knots.0[0] != knots.0[degree_u]
        || knots.0.last() != Some(&knots.0[rows])
        || knots.1[0] != knots.1[degree_v]
        || knots.1.last() != Some(&knots.1[columns])
    {
        return None;
    }
    let exclusive = |axis: usize, upper: bool| {
        let (count, transverse_count, degree) = if axis == 0 {
            (rows, columns, degree_u)
        } else {
            (columns, rows, degree_v)
        };
        let coefficient = |slice: usize, transverse: usize| {
            let index = if upper { count - 1 - slice } else { slice };
            if axis == 0 {
                controls[index][transverse]
            } else {
                controls[transverse][index]
            }
        };
        let pole = coefficient(0, 0);
        let prefix = (0..count)
            .take_while(|&slice| (0..transverse_count).all(|j| coefficient(slice, j) == pole))
            .count();
        if prefix == 0 || prefix > degree {
            return false;
        }
        // With p+1 clamping and k<=p zero slices, every off-endpoint
        // normal basis blend gives some weight to a remaining slice. One
        // fixed strict coordinate/sign across ALL its transverse controls
        // then excludes the pole, including at interior knots. A folded or
        // flat-band carrier receives no certificate.
        (0..3).any(|coordinate| {
            [false, true].into_iter().any(|positive| {
                (prefix..count).all(|slice| {
                    (0..transverse_count).all(|j| {
                        let value = coefficient(slice, j)[coordinate];
                        if positive {
                            value > pole[coordinate]
                        } else {
                            value < pole[coordinate]
                        }
                    })
                })
            })
        })
    };
    Some([
        [exclusive(0, false), exclusive(0, true)],
        [exclusive(1, false), exclusive(1, true)],
    ])
}
