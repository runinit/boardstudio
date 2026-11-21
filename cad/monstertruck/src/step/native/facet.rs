//! Certify facet normals against the analytic support under radial projection.
use super::*;

pub(super) fn radial_facet_fits(
    triangle: [Point3; 3],
    radial: [Vector3<f64>; 3],
    same_sense: bool,
    world: Matrix4,
) -> bool {
    let Ok(normal) = crate::mesh::triangle_normal(triangle.map(|p| world.transform_point(p)))
    else {
        return false;
    };
    let [x, y, z] = [world.x, world.y, world.z].map(|column| column.truncate());
    // Cofactors carry the support normals into world space. For a mirror,
    // the publication swaps the triangle winding and negates this cofactor
    // orientation as well. The dot product is unaffected by that common sign.
    let [a, b, c] = [y.cross(z), z.cross(x), x.cross(y)];
    let axis_scale = x.magnitude().max(y.magnitude()).max(z.magnitude());
    let sense = if same_sense { 1.0 } else { -1.0 };
    let cos_limit = ANGULAR_TOLERANCE.cos();
    radial.into_iter().all(|p| {
        let support = (a * p.x + b * p.y + c * p.z) * sense;
        let length = support.x.hypot(support.y).hypot(support.z);
        let alignment = normal.dot(support);
        // Keep a roundoff margin tied to the cofactor inputs. A nearly
        // singular transform can cancel the normal's mapped components.
        let roundoff = (64.0 * f64::EPSILON * axis_scale) * axis_scale * p.magnitude();
        // A cone of half-angle < pi/2 is convex. The unnormalized radial
        // field is affine on the whole triangle (with its axial component
        // removed for a cylinder). Checking all three generators therefore
        // certifies the projected support everywhere, including the interior.
        length > 0.0
            && length.is_finite()
            && alignment.is_finite()
            && alignment >= length * cos_limit + roundoff
    })
}
