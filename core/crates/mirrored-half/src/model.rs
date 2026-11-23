pub use boardstudio_model::mirrored_half::{
    MirrorDestination, MirrorOverride, MirroredPair, PartOverride,
};

use crate::model::ProjectDoc;

pub fn validate_pairs(doc: &ProjectDoc) -> Result<(), String> {
    let mut identities = std::collections::BTreeSet::new();
    let mut destinations = Vec::new();
    for pair in &doc.mirrored_pairs {
        if pair.id.is_empty() || !identities.insert(&pair.id) {
            return Err(format!("Invalid or duplicate mirror pair {}", pair.id));
        }
        if !pair.mapping.at.x.is_finite()
            || !pair.mapping.at.y.is_finite()
            || !pair.mapping.rotation.is_finite()
        {
            return Err(format!("Mirror pair {} has a nonfinite mapping", pair.id));
        }
        match (&pair.source, &pair.destination) {
            (MirrorDestination::SecondBoard(left), MirrorDestination::SecondBoard(right)) => {
                if left == right
                    || ![left, right]
                        .into_iter()
                        .all(|id| doc.boards.iter().any(|board| &board.id == id))
                {
                    return Err(format!(
                        "Mirror pair {} requires two distinct boards",
                        pair.id
                    ));
                }
            }
            (
                MirrorDestination::SameBoardLayout(left),
                MirrorDestination::SameBoardLayout(right),
            ) => {
                let board = |id: &str| {
                    doc.layouts
                        .iter()
                        .find(|layout| layout.id == id)
                        .map(|layout| layout.board_id.as_str())
                };
                if left == right || board(left).is_none() || board(left) != board(right) {
                    return Err(format!(
                        "Mirror pair {} requires two layouts on one board",
                        pair.id
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Mirror pair {} has mismatched destinations",
                    pair.id
                ));
            }
        }
        if destinations.contains(&&pair.destination)
            || doc
                .mirrored_pairs
                .iter()
                .any(|other| other.destination == pair.source)
        {
            return Err(format!(
                "Mirror pair {} requires independent source and destination ownership",
                pair.id
            ));
        }
        destinations.push(&pair.destination);
        let mut overrides = std::collections::BTreeSet::new();
        if pair
            .overrides
            .iter()
            .any(|item| !overrides.insert(&item.source_id))
        {
            return Err(format!("Mirror pair {} has a duplicate override", pair.id));
        }
    }
    Ok(())
}
