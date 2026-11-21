//! Per-key Omit/Rejoin: native-pair half-exception lookup and its execution-time resolvers.

use super::KeyHalfException;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{
    EditOperation, HalfExceptionState, MirroredExceptionState, MirroredItemKind, ProjectDoc,
};

/// Read one mirror-side key's native-pair exception context, if the member
/// part is the mirror side of a Part correspondence. Pure: unit-tested beside
/// the resolver it feeds.
pub fn key_half_exception(document: &ProjectDoc, member_id: &str) -> Option<KeyHalfException> {
    let hardware = document.hardware.as_ref()?;
    let (pair, halves) = hardware
        .pairs
        .iter()
        .filter_map(|pair| pair.halves.as_ref().map(|halves| (pair, halves)))
        .find(|(_, halves)| {
            halves
                .correspondences
                .iter()
                .any(|link| link.kind == MirroredItemKind::Part && link.mirror_id == member_id)
        })?;
    let omitted = halves.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Part
            && exception.mirror_id == member_id
            && exception.state == MirroredExceptionState::Omitted
    });
    Some(KeyHalfException {
        pair_id: pair.id.clone(),
        mirror_id: member_id.to_owned(),
        omitted,
    })
}

/// Resolve one key's Omit/Rejoin toggle against the accepted document by
/// matrix coordinates. The member and pair re-resolve at execution time, so a
/// queued omit never lands on a reindexed key. An omitted key left the scene:
/// its Omitted overlay names the canonical member.
pub fn key_half_exception_at_resolver(
    matrix_id: String,
    row: u32,
    column: u32,
    omit: bool,
) -> EditResolver {
    EditResolver::new(
        "layout-matrix-key-omit",
        move |accepted: &AcceptedSnapshot| {
            // A live key resolves through its scene cell; an omitted key left
            // the scene, so its Omitted overlay names the canonical member.
            let member = accepted
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix_id)
                .and_then(|scene| {
                    scene
                        .cells
                        .iter()
                        .find(|cell| cell.row == row && cell.column == column)
                })
                .and_then(|cell| cell.member_id.clone());
            let member = match member {
                Some(member) => member,
                None => {
                    match key_half_exception_omitted(&accepted.document, &matrix_id, row, column) {
                        Some(exception) => exception.mirror_id,
                        None => {
                            return Resolution::Retire("The selected key no longer exists.".into());
                        }
                    }
                }
            };
            let Some(exception) = key_half_exception(&accepted.document, &member) else {
                return Resolution::Retire(
                    "The key is not a mirrored key with a linked counterpart.".into(),
                );
            };
            key_half_exception_resolver(exception.pair_id, exception.mirror_id, omit)
                .resolve(accepted)
        },
    )
}

/// Omitted-only half-exception lookup by matrix coordinates, for keys that
/// left the scene. Mirrored matrix members carry derived identities, so the
/// canonical id names the exact member an Omitted overlay suppresses; the
/// overlay itself is the authority, never the constructed id alone.
pub fn key_half_exception_omitted(
    document: &ProjectDoc,
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<KeyHalfException> {
    let candidate = format!("matrix/{matrix_id}/r{row}c{column}");
    key_half_exception(document, &candidate).filter(|exception| exception.omitted)
}

/// Resolve one key's Omit/Rejoin toggle against the accepted document. Only the
/// pair/correspondence identity is captured; the overlay is read at execution
/// time, so omitting one key is a single atomic edit with one Undo entry that
/// composes with other field edits.
pub fn key_half_exception_resolver(pair_id: String, mirror_id: String, omit: bool) -> EditResolver {
    EditResolver::new(
        "layout-matrix-key-omit",
        move |accepted: &AcceptedSnapshot| {
            let Some(halves) = accepted
                .document
                .hardware
                .as_ref()
                .and_then(|hardware| hardware.pairs.iter().find(|pair| pair.id == pair_id))
                .and_then(|pair| pair.halves.as_ref())
            else {
                return Resolution::Retire("The mirrored pair no longer exists.".into());
            };
            if !halves
                .correspondences
                .iter()
                .any(|link| link.kind == MirroredItemKind::Part && link.mirror_id == mirror_id)
            {
                return Resolution::Retire("The linked key no longer exists.".into());
            }
            let omitted = halves
                .exceptions
                .iter()
                .find(|exception| {
                    exception.kind == MirroredItemKind::Part && exception.mirror_id == mirror_id
                })
                .is_some_and(|exception| exception.state == MirroredExceptionState::Omitted);
            if omitted == omit {
                return Resolution::Unchanged;
            }
            Resolution::submit(
                vec![pair_id.clone(), mirror_id.clone()],
                EditOperation::SetHalfException {
                    pair_id: pair_id.clone(),
                    kind: MirroredItemKind::Part,
                    mirror_id: mirror_id.clone(),
                    state: if omit {
                        HalfExceptionState::Omitted
                    } else {
                        HalfExceptionState::Linked
                    },
                },
            )
        },
    )
}
