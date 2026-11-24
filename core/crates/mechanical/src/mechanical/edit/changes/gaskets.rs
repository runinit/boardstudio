//! Gasket support placement and reset changes.
/// Applies one gaskets change. Expands in [`super::apply_change`] scope and uses its
/// `configuration`, `change`, `document`, `board_id`, `identity_floor` and
/// `changed` bindings; exactly one arm runs because domains partition variants.
macro_rules! gasket_arms {
    ($configuration:expr, $change:expr, $document:expr, $board_id:expr, $identity_floor:expr, $changed:expr) => {
        match $change {
            MechanicalChange::SetGasketSupportDimension {
                support_id,
                anchors,
                field,
                value,
            } => {
                if !matches!(
                    *field,
                    MechanicalDimension::GasketSupportLength
                        | MechanicalDimension::GasketSupportWidth
                ) {
                    return Err("The selected gasket field is unavailable.".into());
                }
                validate_dimension(*field, *value)?;
                validate_gasket_anchor_selection(support_id, anchors, true)?;
                let is_length = *field == MechanicalDimension::GasketSupportLength;
                upsert_gasket_anchors(
                    $configuration,
                    support_id,
                    anchors,
                    |anchor| {
                        if is_length {
                            anchor.length = Some(*value);
                        } else {
                            anchor.width = Some(*value);
                        }
                    },
                    $changed,
                )?;
            }
            MechanicalChange::SetGasketSupportPlacement {
                support_id,
                anchors,
            } => {
                validate_gasket_anchor_selection(support_id, anchors, false)?;
                validate_gasket_support_anchors(anchors)?;
                upsert_gasket_anchors($configuration, support_id, anchors, |_| {}, $changed)?;
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
                    $configuration,
                    support_id,
                    anchors,
                    |anchor| anchor.unlinked = true,
                    $changed,
                )?;
            }
            MechanicalChange::ResetGasketPlacement => {
                if $configuration.mount != MechanicalMount::Gasket
                    || $configuration.internal_gasket.is_none()
                {
                    return Err(
                        "The current configuration no longer contains gasket supports.".into(),
                    );
                }
                $configuration
                    .gasket_layout
                    .get_or_insert_with(crate::mechanical::default_gasket_layout)
                    .supports
                    .clear();
            }
            _ => {}
        }
    };
}
