//! Wire types and lightweight input-topology predicates shared by the document engine and CAD adapter.
//! No geometry engine, worker, or platform dependencies belong in this crate.
use num_rational::BigRational;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod case;
mod collision;
mod polygon;
mod support_owner;

pub use polygon::{path_contains, path_signed_area, polygon_contains, polygon_signed_area};

pub use collision::{
    AffineTransformError, CadStepImportOptions, is_sha256_hex, validate_affine_row_transform,
};

pub use case::{
    CaseAssemblyIR, CaseBody, CaseFeature, CaseIR, CaseKind, CaseOpening, CasePcbCircularSupport,
    CasePcbSlot, CasePcbSource, CasePcbSourceContour, CasePcbSourcePrimitive, Contour, Gasket,
    Mount, MountKind, PreparedCaseAssemblyIR, PreparedCaseIR, PreparedCaseRegion,
    PreparedGasketRegion,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

/// Largest coordinate magnitude, in mm, that Core produces and the CAD kernel accepts.
pub const MAX_COORDINATE_MM: f64 = 1_000_000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalDrillShape {
    Circle,
    Oval,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum MechanicalShape {
    Line {
        start: Vec2,
        end: Vec2,
        width: Option<f64>,
    },
    Arc {
        start: Vec2,
        mid: Option<Vec2>,
        end: Vec2,
        angle_degrees: Option<f64>,
        width: Option<f64>,
    },
    Circle {
        center: Vec2,
        end: Vec2,
        width: Option<f64>,
    },
    Rectangle {
        start: Vec2,
        end: Vec2,
        width: Option<f64>,
    },
    Polygon {
        points: Vec<Vec2>,
        width: Option<f64>,
    },
    Drill {
        at: Vec2,
        size: Vec2,
        offset: Vec2,
        rotation_degrees: f64,
        shape: MechanicalDrillShape,
        plated: Option<bool>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose2 {
    pub at: Vec2,
    pub rotation: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Front,
    Back,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeycapProfile {
    Cherry,
    Oem,
    Dcs,
    Dsa,
    Sa,
    HiPro,
    G20,
    Choc,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeycapMount {
    Mx,
    ChocV1,
    ChocV2,
    Alps,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapConfiguration {
    pub boards: BTreeMap<String, KeycapBoardSettings>,
    pub matrices: BTreeMap<String, KeycapMatrixSettings>,
    pub keys: BTreeMap<String, KeycapKeySettings>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapBoardSettings {
    pub color: String,
    pub legend_color: String,
    pub clearance: f64,
}
impl Default for KeycapBoardSettings {
    fn default() -> Self {
        Self {
            color: "#e8e4dc".into(),
            legend_color: "#202630".into(),
            clearance: 0.5,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapMatrixSettings {
    pub profile: Option<KeycapProfile>,
    pub mount: Option<KeycapMount>,
    pub first_row: u8,
    pub wall_thickness: f64,
}
impl Default for KeycapMatrixSettings {
    fn default() -> Self {
        Self {
            profile: None,
            mount: None,
            first_row: 1,
            wall_thickness: 1.2,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapKeySettings {
    pub mount: Option<KeycapMount>,
    pub legend: Option<String>,
    pub color: Option<String>,
    pub row: Option<u8>,
    pub units: Option<Vec2>,
    pub profile: Option<KeycapProfile>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycapSpec {
    pub id: String,
    pub reference: String,
    pub profile: KeycapProfile,
    pub mount: KeycapMount,
    pub row: u8,
    pub size: Vec2,
    pub top_size: Vec2,
    pub height: f64,
    pub tilt: f64,
    pub dish_depth: f64,
    pub spherical: bool,
    pub wall_thickness: f64,
    pub pose: Pose2,
    pub side: Side,
    pub z: f64,
    pub travel: f64,
    pub legend: String,
    pub color: String,
    pub legend_color: String,
}

/// Reports whether a support-prism ring crosses or touches itself.
///
/// Exact consecutive duplicates and one repeated closing point are ignored in
/// the local predicate input. The caller's points are never changed. Predicate
/// signs are exact for the represented f64 coordinates; no grid or tolerance
/// is applied. `false` means no self-intersection was found; callers remain
/// responsible for finite-coordinate, vertex-count, area, and height checks.
pub fn support_polygon_has_self_intersection(points: &[[f64; 2]]) -> bool {
    let mut ring = Vec::with_capacity(points.len());
    for point in points {
        if !point[0].is_finite() || !point[1].is_finite() {
            return false;
        }
        if ring
            .last()
            .is_none_or(|last: &[f64; 2]| last[0] != point[0] || last[1] != point[1])
        {
            ring.push(*point);
        }
    }
    if ring.len() > 1
        && ring.first().is_some_and(|first| {
            ring.last()
                .is_some_and(|last| first[0] == last[0] && first[1] == last[1])
        })
    {
        ring.pop();
    }
    let n = ring.len();
    if n < 3 {
        return false;
    }

    // Adjacent edges may meet at their shared endpoint, but a collinear
    // reversal overlaps beyond it and is not a valid support boundary.
    for i in 0..n {
        let previous = ring[(i + n - 1) % n];
        let shared = ring[i];
        let next = ring[(i + 1) % n];
        if orientation(previous, shared, next) == 0
            && ((previous[0] < shared[0] && next[0] < shared[0])
                || (previous[0] > shared[0] && next[0] > shared[0])
                || (previous[1] < shared[1] && next[1] < shared[1])
                || (previous[1] > shared[1] && next[1] > shared[1]))
        {
            return true;
        }
    }

    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let c = ring[j];
            let d = ring[(j + 1) % n];
            if segments_intersect(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> i8 {
    let exact = |value: f64| BigRational::from_float(value).expect("finite support coordinate");
    let ax = exact(a[0]);
    let ay = exact(a[1]);
    let bx = exact(b[0]);
    let by = exact(b[1]);
    let cx = exact(c[0]);
    let cy = exact(c[1]);
    let determinant = (&bx - &ax) * (&cy - &ay) - (&by - &ay) * (&cx - &ax);
    if determinant > BigRational::zero() {
        1
    } else if determinant < BigRational::zero() {
        -1
    } else {
        0
    }
}

fn between(a: f64, b: f64, p: f64) -> bool {
    a.min(b) <= p && p <= a.max(b)
}

fn point_on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    between(a[0], b[0], p[0]) && between(a[1], b[1], p[1]) && orientation(a, b, p) == 0
}

fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    if a[0].max(b[0]) < c[0].min(d[0])
        || c[0].max(d[0]) < a[0].min(b[0])
        || a[1].max(b[1]) < c[1].min(d[1])
        || c[1].max(d[1]) < a[1].min(b[1])
    {
        return false;
    }
    let ab_c = orientation(a, b, c);
    let ab_d = orientation(a, b, d);
    let cd_a = orientation(c, d, a);
    let cd_b = orientation(c, d, b);
    if ab_c == 0 && point_on_segment(a, b, c)
        || ab_d == 0 && point_on_segment(a, b, d)
        || cd_a == 0 && point_on_segment(c, d, a)
        || cd_b == 0 && point_on_segment(c, d, b)
    {
        return true;
    }
    (ab_c < 0 && ab_d > 0 || ab_c > 0 && ab_d < 0) && (cd_a < 0 && cd_b > 0 || cd_a > 0 && cd_b < 0)
}

/// A normalized profile region: counter-clockwise exterior and clockwise holes.
pub type NormalizedProfileRegion = (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>);

/// Validates and normalizes one prepared case profile region.
///
/// The returned exterior is counter-clockwise and holes are clockwise. Exact
/// consecutive duplicates and one closing duplicate are removed in the
/// returned coordinates. Rings must be finite, simple, nonzero-area, and each
/// hole must be strictly contained by the exterior without touching or
/// nesting another hole. Inputs are never changed.
pub fn normalize_case_profile_region(
    exterior: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
) -> Result<NormalizedProfileRegion, String> {
    support_owner::normalize_case_profile_region(exterior, holes)
}

/// Tests whether the exact planar area outside a support prism and one exterior
/// is strictly below the existing unit-height owner cutoff of `1e-8 mm³`.
///
/// Coordinates are interpreted as their exact represented f64 values. The
/// routine accepts only finite, nonzero-area simple rings (exact consecutive
/// and closing duplicates are normalized in a local copy). It returns a
/// validation error for unsupported operands and never changes either input.
pub fn support_polygon_outside_area_below_owner_cutoff(
    support: &[[f64; 2]],
    exterior: &[[f64; 2]],
) -> Result<bool, String> {
    support_owner::outside_area_below_owner_cutoff(support, exterior)
}

#[cfg(test)]
mod support_polygon_tests {
    use super::support_polygon_has_self_intersection as self_intersects;

    #[test]
    fn detects_crossings_touches_and_nonadjacent_overlap() {
        let star = (0..5)
            .map(|i| {
                let a = std::f64::consts::TAU * ((i * 2) % 5) as f64 / 5.0
                    - std::f64::consts::FRAC_PI_2;
                [10.0 + 8.0 * a.cos(), 10.0 + 8.0 * a.sin()]
            })
            .collect::<Vec<_>>();
        assert!(self_intersects(&star));
        assert!(self_intersects(&[
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 4.0],
            [2.0, 0.0],
            [0.0, 4.0],
        ]));
        assert!(self_intersects(&[
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 4.0],
            [2.0, 0.0],
            [1.0, 0.0],
            [0.0, 4.0],
        ]));
        assert!(
            self_intersects(&[[0.0, 0.0], [4.0, 0.0], [2.0, 0.0], [2.0, 4.0],]),
            "adjacent collinear reversal overlaps itself"
        );
        assert!(
            !self_intersects(&[[0.0, 0.0], [2.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0],]),
            "straight collinear continuation remains valid"
        );
    }

    #[test]
    fn exact_orientation_covers_subnormal_products_and_large_finite_inputs() {
        use super::orientation;
        assert_eq!(orientation([0.0, 0.0], [1e-200, 0.0], [0.0, 1e-200]), 1);
        assert_eq!(orientation([0.0, 0.0], [1e200, 0.0], [0.0, 1e200]), 1);
        assert_eq!(
            orientation(
                [1e16, 1e16],
                [1e16 + 2.0, 1e16 + 2.0],
                [1e16 + 4.0, 1e16 + 6.0]
            ),
            1
        );
    }

    #[test]
    fn exact_duplicates_and_nearby_distinct_edges_are_preserved() {
        let ring = [
            [0.0, 0.0],
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [1e-10, 10.0],
            [1e-10, 5.0],
            [0.0, 5.0],
            [0.0, 0.0],
        ];
        let input_copy = ring;
        assert!(!self_intersects(&ring));
        assert_eq!(ring, input_copy, "predicate must not mutate caller points");
    }
}
