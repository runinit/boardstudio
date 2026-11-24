//! Keycap-part projection helpers shared by the Keycaps and Keymap workspaces.
//!
//! Both workspace projections filter board parts down to switch-backed keys
//! and size them from saved keycap units; the rules live here so the two
//! views cannot drift apart.
use boardstudio_model::model::{
    KeycapKeySettings, Part, PartDefinition, PartKind, ProjectDoc, Vec2,
};

/// Whether `part` is a key the keycap views show: a switch definition member
/// or a matrix-generated key address.
pub(crate) fn is_supported_key(doc: &ProjectDoc, part: &Part) -> bool {
    doc.definitions
        .iter()
        .find(|d| d.id == part.definition_id)
        .is_some_and(|d| d.kind == PartKind::Switch)
        || doc.matrices.iter().any(|m| {
            m.part_ids.contains(&part.id)
                && part
                    .id
                    .strip_prefix(&format!("matrix/{}/", m.id))
                    .is_some_and(|s| !s.contains('/'))
        })
}

/// The display size of `part`'s keycap: saved units scaled to millimetres,
/// else the part or definition envelope, else the default keycap.
pub(crate) fn keycap_size(
    part: &Part,
    def: Option<&PartDefinition>,
    s: Option<&KeycapKeySettings>,
) -> Vec2 {
    s.and_then(|s| s.units).map_or_else(
        || {
            part.keycap
                .or_else(|| def.and_then(|d| d.keycap))
                .unwrap_or(Vec2 { x: 18.2, y: 18.2 })
        },
        |u| Vec2 {
            x: u.x * 19.05 - 0.85,
            y: u.y * 19.05 - 0.85,
        },
    )
}
