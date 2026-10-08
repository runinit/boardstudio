//! Geometry helpers for explicit mechanical mirror pairs.
//!
//! A pair map is a `Pose2` composed with a fixed X reflection:
//! `q = R(rotation) * (-p.x, p.y) + at`, with an exact inverse. Planar orientation
//! transforms by `angle_B = rotation - angle_A`; reflection reverses polygon
//! winding, so projected opening vertex sequences reverse while keeping their
//! identities. Filled in by the mirror-pair slice.
