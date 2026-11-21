use super::*;
use crate::model::Vec2;

#[test]
fn mirror_ids_substitute_whole_board_tokens_only() {
    let mirror = |id: &str| mirror_id("left", "right", id);
    assert_eq!(mirror("left/U1"), "right/U1");
    assert_eq!(mirror("left-U1"), "right-U1");
    assert_eq!(mirror("left-outline"), "right-outline");
    assert_eq!(mirror("matrix/left-keys"), "matrix/right-keys");
    assert_eq!(
        mirror("matrix/left-keys/r0c0/diode"),
        "matrix/right-keys/r0c0/diode"
    );
    // A token inside a word is not the board.
    assert_eq!(mirror("leftover/U1"), "right/leftover/U1");
    // An id that names no board is prefixed with the mirror board.
    assert_eq!(mirror("outline-1"), "right/outline-1");
}

#[test]
fn a_mirrored_pose_takes_the_pair_heading_rule() {
    let mapping = Pose2 {
        at: Vec2 { x: 10.0, y: 0.0 },
        rotation: 30.0,
    };
    let pose = Pose2 {
        at: Vec2 { x: 0.0, y: 5.0 },
        rotation: 20.0,
    };
    let mirrored = mirror_pose(mapping, pose);
    // The forward_angle (30 - 20 = 10) turned by 180 degrees: 190.
    assert_eq!(mirrored.rotation, 190.0);
    let expected = pair::forward(mapping, pose.at);
    assert_eq!(mirrored.at, expected);
}

fn heading_of(mapping: Pose2, rotation: f64) -> f64 {
    mirror_pose(
        mapping,
        Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation,
        },
    )
    .rotation
}

fn same_heading(actual: f64, expected: f64) -> bool {
    let turn = (actual - expected).rem_euclid(360.0);
    turn.min(360.0 - turn) < 1e-9
}

/// The heading of the mapped direction, read from the mapped vector itself. It does not use
/// `forward_angle`, so it checks the rule against the geometry rather than against itself.
fn image_heading(mapping: Pose2, heading: f64) -> f64 {
    let (sin, cos) = heading.to_radians().sin_cos();
    let image = pair::forward_vector(mapping, Vec2 { x: cos, y: sin });
    image.y.atan2(image.x).to_degrees().rem_euclid(360.0)
}

#[test]
fn a_turned_part_faces_outward_on_the_mirror_over_the_full_circle() {
    let axis = Pose2 {
        at: Vec2 { x: 300.0, y: 0.0 },
        rotation: 0.0,
    };
    // A TRRS jack facing left (180) faces right on the mirror, and the reverse. Modulo 180
    // (or `-r`) would leave the left-facing jack at 180.
    assert_eq!(heading_of(axis, 180.0), 0.0);
    assert_eq!(heading_of(axis, 0.0), 180.0);
    // A part facing up or down needs no turn, only the mirrored position.
    assert_eq!(heading_of(axis, 90.0), 90.0);
    assert_eq!(heading_of(axis, 270.0), 270.0);
    // An off-axis part (an LED at 135 degrees) takes 180 - r as well.
    assert_eq!(heading_of(axis, 135.0), 45.0);
}

#[test]
fn every_heading_mirrors_to_the_mapped_direction_under_axis_and_rotated_maps() {
    let maps = [
        Pose2 {
            at: Vec2 { x: 300.0, y: 0.0 },
            rotation: 0.0,
        },
        Pose2 {
            at: Vec2 { x: 310.0, y: 42.0 },
            rotation: 27.0,
        },
    ];
    for map in maps {
        for heading in [0.0, 30.0, 45.0, 90.0, 135.0, 200.0, 270.0, 315.0] {
            let actual = heading_of(map, heading);
            let expected = image_heading(map, heading);
            assert!(
                same_heading(actual, expected),
                "map rotation {} heading {heading}: {actual} from the mapped direction {expected}",
                map.rotation
            );
        }
    }
    // Under the rotated map the rule is 207 - r, so the off-axis LED at 135 goes to 72.
    assert_eq!(heading_of(maps[1], 135.0), 72.0);
}

#[test]
fn the_four_edge_headings_and_an_off_axis_led_are_exact_under_a_rotated_map() {
    // The rule is `rotation + 180 - r`, read independently from the reflection about the
    // axis at 90 + rotation / 2: a direction at r goes to rotation + 180 - r.
    let rotated = Pose2 {
        at: Vec2 { x: 310.0, y: 42.0 },
        rotation: 27.0,
    };
    assert_eq!(heading_of(rotated, 180.0), 27.0);
    assert_eq!(heading_of(rotated, 0.0), 207.0);
    assert_eq!(heading_of(rotated, 90.0), 117.0);
    assert_eq!(heading_of(rotated, 270.0), 297.0);
    assert_eq!(heading_of(rotated, 135.0), 72.0);
}

/// The world point of a part-frame point: `at + R(heading) * local`.
fn world(pose: Pose2, local: Vec2) -> Vec2 {
    let (sin, cos) = pose.rotation.to_radians().sin_cos();
    Vec2 {
        x: pose.at.x + local.x * cos - local.y * sin,
        y: pose.at.y + local.x * sin + local.y * cos,
    }
}

#[test]
fn anchored_points_land_on_the_mapped_world_point_at_every_heading() {
    let maps = [
        Pose2 {
            at: Vec2 { x: 300.0, y: 0.0 },
            rotation: 0.0,
        },
        Pose2 {
            at: Vec2 { x: 310.0, y: 42.0 },
            rotation: 27.0,
        },
    ];
    let local = Vec2 { x: 4.0, y: 2.5 };
    for map in maps {
        for heading in [0.0, 90.0, 180.0, 270.0, 37.0] {
            let source = Pose2 {
                at: Vec2 { x: 20.0, y: -15.0 },
                rotation: heading,
            };
            let expected = pair::forward(map, world(source, local));
            // An independent part: the full rule and the (x, -y) frame.
            let part = mirror_pose(map, source);
            let actual = world(part, anchored_local(local, true));
            assert!(
                (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
                "independent part, map {} heading {heading}: {actual:?} vs {expected:?}",
                map.rotation
            );
            // A key or companion: the switch rule `rotation - r` and the (-x, y) frame.
            let key = Pose2 {
                at: part.at,
                rotation: pair::forward_angle(map, heading),
            };
            let actual = world(key, anchored_local(local, false));
            assert!(
                (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
                "switch, map {} heading {heading}: {actual:?} vs {expected:?}",
                map.rotation
            );
        }
    }
}

/// The world point of a pad at `local` on a part (a back-side part mirrors its local x, as the
/// KiCad export does).
fn pad_world(pose: Pose2, back: bool, local: Vec2) -> Vec2 {
    let x = if back { -local.x } else { local.x };
    world(pose, Vec2 { x, y: local.y })
}

/// Ticket 27 verification. The heading rule turns an independent part by `rotation + 180 - r`
/// and does not flip it, so its pad layout is rotated, not mirrored. A pad lands on the true
/// mirror of the source pad only when its local `y` is zero (on the heading axis); every other
/// pad is off by exactly `2 |y|`, with its local `y` reversed. This holds at every 90-degree
/// heading, so the on-grid case is not certain for a footprint with pads off its heading axis.
#[test]
fn a_turned_part_puts_a_pad_on_the_true_mirror_only_when_it_sits_on_the_heading_axis() {
    let maps = [
        Pose2 {
            at: Vec2 { x: 300.0, y: 0.0 },
            rotation: 0.0,
        },
        Pose2 {
            at: Vec2 { x: 310.0, y: 42.0 },
            rotation: 27.0,
        },
    ];
    let offsets = [
        (1.0, 0.0),
        (-2.0, 0.0),
        (0.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (1.0, 0.5),
        (2.0, -3.0),
    ];
    for map in maps {
        for back in [false, true] {
            for heading in [0.0, 90.0, 180.0, 270.0] {
                let source = Pose2 {
                    at: Vec2 { x: 20.0, y: -15.0 },
                    rotation: heading,
                };
                let part = mirror_pose(map, source);
                for (x, y) in offsets {
                    let local = Vec2 { x, y };
                    let true_mirror = pair::forward(map, pad_world(source, back, local));
                    let placed = pad_world(part, back, local);
                    let error = Vec2 {
                        x: placed.x - true_mirror.x,
                        y: placed.y - true_mirror.y,
                    };
                    // The error vector is the local y reversed, in the mirror part's frame
                    // taken without the half turn: `R(rotation - r) * (0, -2y)`.
                    let (sin, cos) = (map.rotation - heading).to_radians().sin_cos();
                    let expected = Vec2 {
                        x: 2.0 * y * sin,
                        y: -2.0 * y * cos,
                    };
                    let label = format!(
                        "map {} back {back} heading {heading} pad ({x}, {y})",
                        map.rotation
                    );
                    assert!(
                        (error.x - expected.x).abs() < 1e-9 && (error.y - expected.y).abs() < 1e-9,
                        "{label}: error {error:?} expected {expected:?}"
                    );
                    let off = error.x.hypot(error.y);
                    assert_eq!(off < 1e-9, y == 0.0, "{label}: off by {off}");
                }
            }
        }
    }
}

#[test]
fn a_non_finite_mapping_is_refused_before_anything_is_written() {
    let mut doc = ProjectDoc::empty("doc", "doc");
    let before = doc.clone();
    let mapping = Pose2 {
        at: Vec2 {
            x: f64::NAN,
            y: 0.0,
        },
        rotation: 0.0,
    };
    let error = create(
        &mut doc,
        "pair-right",
        "left",
        &MirroredHalfMode::ReuseBoard {
            instance_id: "right".into(),
        },
        mapping,
        MirroredIdRule::BoardPrefix,
        None,
        MirroredCompanionMode::LiteralMirror,
        None,
        None,
    )
    .unwrap_err();
    assert!(error.contains("finite"), "{error}");
    assert_eq!(doc, before);
}
