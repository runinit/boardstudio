//! Host-compatible Case viewer gesture math shared by the viewer and native tests.
//!
//! The `case_viewer` gesture hook owns the mounted presentation; this module is the
//! single production implementation of the pure placement math used on both targets
//! and in native tests. No runtime, signals, or DOM here.
use boardstudio_model::model::{
    MechanicalGasketSupport, MechanicalGasketTrack, Mount, MountKind, Vec2, polygon_contains,
};
use std::collections::BTreeMap;

/// Closed outer contours plus interior holes a mount must respect.
#[derive(Clone, Debug)]
pub struct MountMoveConstraints {
    pub outer: Vec<Vec<Vec2>>,
    pub holes: Vec<Vec<Vec2>>,
}

/// Whether two gasket support sets place every support identically.
pub fn supports_equal_positions(
    a: &[MechanicalGasketSupport],
    b: &[MechanicalGasketSupport],
) -> bool {
    a.len() == b.len()
        && a.iter().all(|support| {
            b.iter()
                .find(|candidate| candidate.id == support.id)
                .is_some_and(|candidate| (support.anchor - candidate.anchor).abs() <= 1e-7)
        })
}

/// Move one gasket support along its track, mirroring its pair when linked.
pub fn move_gasket_support(
    point: Vec2,
    id: &str,
    supports: &[MechanicalGasketSupport],
    tracks: &[MechanicalGasketTrack],
) -> Option<Vec<MechanicalGasketSupport>> {
    let original = supports.iter().find(|support| support.id == id)?;
    let moved = project_gasket(point, original, tracks)?;
    let mut replacements = BTreeMap::from([(id.to_owned(), moved)]);
    if let Some(pair_id) = original.pair_id.as_ref()
        && let Some(axis) = original.mirror_axis
        && !original.unlinked
    {
        let pair = supports.iter().find(|support| support.id == *pair_id)?;
        if !pair.unlinked {
            let moved = replacements.get(id)?;
            let reflected = project_gasket(
                Vec2 {
                    x: 2.0 * axis - moved.at.x,
                    y: moved.at.y,
                },
                pair,
                tracks,
            )?;
            if (reflected.at.x - (2.0 * axis - moved.at.x)).hypot(reflected.at.y - moved.at.y)
                > 0.001
            {
                return None;
            }
            replacements.insert(pair.id.clone(), reflected);
        }
    }
    Some(
        supports
            .iter()
            .map(|support| replacements.get(&support.id).unwrap_or(support).clone())
            .collect(),
    )
}

/// Project one support onto its closest track segment.
pub fn project_gasket(
    point: Vec2,
    support: &MechanicalGasketSupport,
    tracks: &[MechanicalGasketTrack],
) -> Option<MechanicalGasketSupport> {
    let mut best = None;
    let mut best_distance = f64::INFINITY;
    for track in tracks
        .iter()
        .filter(|track| track.region_id == support.region_id)
    {
        let dx = track.end.x - track.start.x;
        let dy = track.end.y - track.start.y;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= f64::EPSILON {
            continue;
        }
        let length = length_squared.sqrt();
        let t = (((point.x - track.start.x) * dx + (point.y - track.start.y) * dy)
            / length_squared)
            .clamp(0.0, 1.0);
        let at = Vec2 {
            x: track.start.x + t * dx,
            y: track.start.y + t * dy,
        };
        let distance = (point.x - at.x).hypot(point.y - at.y);
        if distance >= best_distance {
            continue;
        }
        best_distance = distance;
        let mut next = support.clone();
        next.at = at;
        next.anchor = track.start_anchor + t * (track.end_anchor - track.start_anchor);
        next.tangent = Vec2 {
            x: dx / length,
            y: dy / length,
        };
        next.normal = Vec2 {
            x: dy / length,
            y: -dx / length,
        };
        best = Some(next);
    }
    best
}

/// Move one mount when the point respects the case edge, openings, and peers.
pub fn move_case_mount(
    point: Vec2,
    id: &str,
    mounts: &[Mount],
    constraints: &MountMoveConstraints,
) -> Option<Vec<Mount>> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let original = mounts.iter().find(|mount| mount.id == id)?;
    let radius = mount_radius(original);
    let margin = radius + 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    if !constraints
        .outer
        .iter()
        .any(|outer| polygon_contains(outer, point) && edge_distance(point, outer) >= margin)
    {
        return None;
    }
    if constraints
        .holes
        .iter()
        .any(|hole| polygon_contains(hole, point) || edge_distance(point, hole) < margin)
    {
        return None;
    }
    if mounts.iter().any(|other| {
        if other.id == id {
            return false;
        }
        let other_radius = mount_radius(other);
        !other_radius.is_finite()
            || other_radius <= 0.0
            || (point.x - other.at.x).hypot(point.y - other.at.y) < radius + other_radius + 0.5
    }) {
        return None;
    }
    Some(
        mounts
            .iter()
            .map(|mount| {
                if mount.id == id {
                    let mut moved = mount.clone();
                    moved.at = point;
                    moved
                } else {
                    mount.clone()
                }
            })
            .collect(),
    )
}

/// Mount keep-out radius from its kind and diameters.
pub fn mount_radius(mount: &Mount) -> f64 {
    if mount.kind == MountKind::Boss {
        mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0
    } else {
        mount.hole_diameter / 2.0
    }
}

/// Shortest distance from a point to a closed polygon.
pub fn edge_distance(point: Vec2, polygon: &[Vec2]) -> f64 {
    let mut nearest = f64::INFINITY;
    for index in 0..polygon.len() {
        let a = &polygon[index];
        let b = &polygon[(index + 1) % polygon.len()];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx * dx + dy * dy;
        let t = if length > 0.0 {
            (((point.x - a.x) * dx + (point.y - a.y) * dy) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        nearest = nearest.min((point.x - a.x - t * dx).hypot(point.y - a.y - t * dy));
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::{Mount, MountKind, Vec2};

    fn outer() -> Vec<Vec2> {
        vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 100.0, y: 0.0 },
            Vec2 { x: 100.0, y: 100.0 },
            Vec2 { x: 0.0, y: 100.0 },
        ]
    }

    fn constraints() -> MountMoveConstraints {
        MountMoveConstraints {
            outer: vec![outer()],
            holes: vec![],
        }
    }

    fn mount(id: &str, x: f64, y: f64) -> Mount {
        Mount {
            id: id.into(),
            at: Vec2 { x, y },
            kind: MountKind::Boss,
            hole_diameter: 2.0,
            boss_diameter: Some(4.0),
            height: Some(5.0),
        }
    }

    #[test]
    fn valid_mount_move_updates_only_the_dragged_mount() {
        let mounts = vec![mount("mount-1", 20.0, 20.0), mount("mount-2", 80.0, 80.0)];
        let moved = move_case_mount(
            Vec2 { x: 30.0, y: 30.0 },
            "mount-1",
            &mounts,
            &constraints(),
        )
        .expect("interior point respects the edge");
        assert_eq!(moved[0].at, Vec2 { x: 30.0, y: 30.0 });
        assert_eq!(moved[1].at, Vec2 { x: 80.0, y: 80.0 });
    }

    #[test]
    fn mount_move_rejects_outside_collisions_and_nonfinite_points() {
        let mounts = vec![mount("mount-1", 20.0, 20.0), mount("mount-2", 30.0, 30.0)];
        // Outside the 100x100 outer contour.
        assert!(
            move_case_mount(
                Vec2 { x: 200.0, y: 200.0 },
                "mount-1",
                &mounts,
                &constraints()
            )
            .is_none()
        );
        // Overlapping the peer mount (radius 2 + 2 + 0.5 keep-out).
        assert!(
            move_case_mount(
                Vec2 { x: 31.0, y: 30.0 },
                "mount-1",
                &mounts,
                &constraints()
            )
            .is_none()
        );
        // Non-finite drag points never move.
        assert!(
            move_case_mount(
                Vec2 {
                    x: f64::NAN,
                    y: 30.0
                },
                "mount-1",
                &mounts,
                &constraints()
            )
            .is_none()
        );
    }

    #[test]
    fn gasket_projection_snaps_to_its_region_track() {
        let support = MechanicalGasketSupport {
            id: "gasket-1".into(),
            region_id: "region-1".into(),
            outline_key: "outline-1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            tangent: Vec2 { x: 1.0, y: 0.0 },
            normal: Vec2 { x: 0.0, y: 1.0 },
            anchor: 0.0,
            length: 10.0,
            width: 3.0,
            z: 0.0,
            thickness: 2.0,
            placement: None,
            fit_error: None,
            pair_id: None,
            mirror_axis: None,
            unlinked: false,
        };
        let tracks = vec![MechanicalGasketTrack {
            region_id: "region-1".into(),
            start: Vec2 { x: 0.0, y: 0.0 },
            end: Vec2 { x: 100.0, y: 0.0 },
            start_anchor: 0.0,
            end_anchor: 1.0,
        }];
        let moved = move_gasket_support(Vec2 { x: 30.0, y: 5.0 }, "gasket-1", &[support], &tracks)
            .expect("nearby point projects onto the track");
        assert!((moved[0].at.x - 30.0).abs() < 1e-9);
        assert!((moved[0].at.y).abs() < 1e-9);
        // Other regions do not attract the support.
        let other = vec![MechanicalGasketTrack {
            region_id: "region-2".into(),
            start: Vec2 { x: 0.0, y: 0.0 },
            end: Vec2 { x: 100.0, y: 0.0 },
            start_anchor: 0.0,
            end_anchor: 1.0,
        }];
        assert!(project_gasket(Vec2 { x: 30.0, y: 5.0 }, &moved[0], &other).is_none());
    }
}
