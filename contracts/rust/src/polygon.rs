//! Point-in-polygon classification shared by Core, the CAD worker and the viewers.
use crate::Vec2;

/// True when a ray from `p` toward +x crosses the edge `a`-`b` (the half-open rule).
fn crosses(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
}

/// Even-odd ray-cast containment of `p` in one ring, whatever its winding.
///
/// Floating-point, with no tolerance: a point exactly on an edge counts as inside on the
/// left and bottom edges and outside on the right and top edges. Rings with fewer than
/// three points contain nothing.
pub fn path_contains(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    (0..ring.len()).fold(false, |inside, i| {
        inside ^ crosses(ring[i], ring[(i + 1) % ring.len()], p)
    })
}

/// [`path_contains`] for a ring of [`Vec2`].
pub fn polygon_contains(ring: &[Vec2], p: Vec2) -> bool {
    (0..ring.len()).fold(false, |inside, i| {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        inside ^ crosses([a.x, a.y], [b.x, b.y], [p.x, p.y])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [[f64; 2]; 4] = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    // Concave "C" opening to the right.
    const C_SHAPE: [[f64; 2]; 8] = [
        [0., 0.],
        [3., 0.],
        [3., 1.],
        [1., 1.],
        [1., 2.],
        [3., 2.],
        [3., 3.],
        [0., 3.],
    ];

    fn vec2(ring: &[[f64; 2]]) -> Vec<Vec2> {
        ring.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect()
    }

    #[test]
    fn classifies_interior_exterior_and_concave_notch() {
        assert!(path_contains(&SQUARE, [0.5, 0.5]));
        assert!(!path_contains(&SQUARE, [1.5, 0.5]));
        assert!(!path_contains(&SQUARE, [0.5, -0.5]));
        assert!(path_contains(&C_SHAPE, [0.5, 1.5]), "spine of the C");
        assert!(!path_contains(&C_SHAPE, [2., 1.5]), "inside the notch");
    }

    #[test]
    fn boundary_points_follow_the_half_open_rule() {
        // Left and bottom edges count as inside; right and top edges as outside.
        assert!(path_contains(&SQUARE, [0., 0.5]), "left edge");
        assert!(path_contains(&SQUARE, [0.5, 0.]), "bottom edge");
        assert!(!path_contains(&SQUARE, [1., 0.5]), "right edge");
        assert!(!path_contains(&SQUARE, [0.5, 1.]), "top edge");
    }

    #[test]
    fn winding_direction_does_not_matter() {
        let reversed: Vec<_> = SQUARE.iter().rev().copied().collect();
        assert!(path_contains(&reversed, [0.5, 0.5]));
        assert!(!path_contains(&reversed, [1.5, 0.5]));
    }

    #[test]
    fn degenerate_rings_contain_nothing() {
        assert!(!path_contains(&[], [0., 0.]));
        assert!(!path_contains(&[[0., 0.]], [0., 0.]));
        assert!(!path_contains(&[[0., 0.], [1., 1.]], [0.5, 0.5]));
    }

    #[test]
    fn vec2_form_agrees_with_the_path_form() {
        for ring in [&SQUARE[..], &C_SHAPE[..]] {
            let points = vec2(ring);
            for x in -1..=4 {
                for y in -1..=4 {
                    let p = [f64::from(x) * 0.5, f64::from(y) * 0.5];
                    assert_eq!(
                        polygon_contains(&points, Vec2 { x: p[0], y: p[1] }),
                        path_contains(ring, p),
                        "{p:?}"
                    );
                }
            }
        }
    }
}
