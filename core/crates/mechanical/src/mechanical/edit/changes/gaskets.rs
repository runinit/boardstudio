//! Gasket support placement and reset changes.
use super::*;

/// Applies one change in this domain; anything else is a routing error.
fn upsert_gasket_anchors(
    configuration: &mut MechanicalConfiguration,
    support_id: &str,
    anchors: &[MechanicalGasketAnchor],
    mutate: impl Fn(&mut MechanicalGasketAnchor),
    changed: &mut Vec<String>,
) -> Result<(), String> {
    if configuration.mount != MechanicalMount::Gasket {
        return Err("The current configuration no longer contains gasket supports.".into());
    }
    let layout = configuration
        .gasket_layout
        .get_or_insert_with(crate::mechanical::default_gasket_layout);
    for source in anchors {
        let mut anchor = source.clone();
        mutate(&mut anchor);
        anchor.placement = Some(GasketPlacement::User);
        upsert_item(&mut layout.supports, anchor, |entry| &entry.id);
    }
    changed.push(support_id.to_owned());
    Ok(())
}
pub(super) fn apply(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    match change {
        MechanicalChange::SetGasketSupportDimension {
            support_id,
            anchors,
            field,
            value,
        } => {
            if !matches!(
                *field,
                MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth
            ) {
                return Err("The selected gasket field is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            validate_gasket_anchor_selection(support_id, anchors, true)?;
            let is_length = *field == MechanicalDimension::GasketSupportLength;
            upsert_gasket_anchors(
                configuration,
                support_id,
                anchors,
                |anchor| {
                    if is_length {
                        anchor.length = Some(*value);
                    } else {
                        anchor.width = Some(*value);
                    }
                },
                changed,
            )?;
        }
        MechanicalChange::SetGasketSupportPlacement {
            support_id,
            anchors,
        } => {
            validate_gasket_anchor_selection(support_id, anchors, false)?;
            validate_gasket_support_anchors(anchors)?;
            upsert_gasket_anchors(configuration, support_id, anchors, |_| {}, changed)?;
        }
        MechanicalChange::SetGasketSupportUnlinked {
            support_id,
            pair_id,
            anchors,
        } => {
            validate_gasket_anchor_selection(support_id, anchors, true)?;
            let pair_is_present = pair_id
                .as_ref()
                .is_some_and(|pair| anchors.iter().any(|anchor| anchor.id == *pair));
            if pair_id.as_deref() == Some(support_id.as_str())
                || pair_id.is_some() != pair_is_present
                || anchors.len() != if pair_id.is_some() { 2 } else { 1 }
            {
                return Err("The selected gasket support pair is no longer available.".into());
            }
            if anchors
                .iter()
                .find(|anchor| anchor.id == *support_id)
                .is_some_and(|anchor| anchor.unlinked)
            {
                return Err("The selected gasket support is already unlinked.".into());
            }
            upsert_gasket_anchors(
                configuration,
                support_id,
                anchors,
                |anchor| anchor.unlinked = true,
                changed,
            )?;
        }
        MechanicalChange::ResetGasketPlacement => {
            if configuration.mount != MechanicalMount::Gasket
                || configuration.internal_gasket.is_none()
            {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout)
                .supports
                .clear();
        }
        _ => return Err("A change was routed to the wrong edit domain.".into()),
    };
    Ok(())
}
