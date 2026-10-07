//! Native-testable state transitions shared by the mounted mirrored-pair owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairFormStage {
    Setup,
    Preparing,
    Placement,
    Pending,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairFormState<T> {
    pub stage: PairFormStage,
    pub values: T,
}

impl<T> PairFormState<T> {
    pub fn new(values: T) -> Self {
        Self {
            stage: PairFormStage::Setup,
            values,
        }
    }

    pub fn setup_is_visible(&self) -> bool {
        self.stage != PairFormStage::Placement
    }

    pub fn setup_is_editable(&self) -> bool {
        self.stage == PairFormStage::Setup
    }
}

pub fn pair_cancel_is_allowed(active_owner: bool, save_pending: bool) -> bool {
    active_owner && !save_pending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_can_cancel_active_placement_but_not_a_pending_save() {
        let mut state = PairFormState::new("submitted draft");
        state.stage = PairFormStage::Placement;
        assert!(pair_cancel_is_allowed(true, false));
        assert!(!pair_cancel_is_allowed(true, true));
        assert!(!pair_cancel_is_allowed(false, false));
        assert_eq!(state.stage, PairFormStage::Placement);
    }

    #[test]
    fn setup_is_not_visible_as_an_editable_form_while_placement_owns_the_canvas() {
        let mut state = PairFormState::new("retained draft");
        assert!(state.setup_is_visible());
        assert!(state.setup_is_editable());
        state.stage = PairFormStage::Placement;
        assert!(!state.setup_is_visible());
        assert!(!state.setup_is_editable());
    }
}
