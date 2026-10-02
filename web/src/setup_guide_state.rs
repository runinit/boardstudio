//! Private guide presentation transitions, independent of browser storage.

/// Reconcile only accepted name changes; unrelated renders must preserve typing.
pub(crate) fn accepted_name_change(
    previous: Option<&(String, String)>,
    current: &(String, String),
) -> Option<(String, String)> {
    previous
        .is_none_or(|previous| previous != current)
        .then(|| current.clone())
}

#[derive(Clone, Copy)]
pub(crate) enum GuideReveal {
    Guide,
    Settings,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PanelReveal {
    pub(crate) objects_open: bool,
    pub(crate) inspector_open: bool,
    pub(crate) pin_objects: bool,
    pub(crate) pin_inspector: bool,
}

pub(crate) fn panel_reveal(intent: GuideReveal, compact: bool) -> PanelReveal {
    match intent {
        GuideReveal::Guide => PanelReveal {
            objects_open: true,
            inspector_open: false,
            pin_objects: !compact,
            pin_inspector: false,
        },
        GuideReveal::Settings => PanelReveal {
            objects_open: !compact,
            inspector_open: true,
            pin_objects: false,
            pin_inspector: !compact,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted(name: &str) -> (String, String) {
        ("keyboard-a".into(), name.into())
    }

    #[test]
    fn accepted_rename_and_undo_replace_the_visible_name() {
        let initial = accepted("Original");
        let renamed = accepted("Renamed");
        assert_eq!(
            accepted_name_change(Some(&initial), &renamed),
            Some(renamed.clone())
        );
        assert_eq!(
            accepted_name_change(Some(&renamed), &initial),
            Some(initial)
        );
    }

    #[test]
    fn unrelated_accepted_edits_preserve_typing_but_project_switch_resets_it() {
        let current = accepted("Original");
        assert_eq!(accepted_name_change(Some(&current), &current), None);
        let switched = ("keyboard-b".into(), "Original".into());
        assert_eq!(
            accepted_name_change(Some(&current), &switched),
            Some(switched)
        );
    }

    #[test]
    fn compact_guide_and_settings_reveal_the_requested_surface() {
        assert_eq!(
            panel_reveal(GuideReveal::Guide, true),
            PanelReveal {
                objects_open: true,
                inspector_open: false,
                pin_objects: false,
                pin_inspector: false,
            }
        );
        assert_eq!(
            panel_reveal(GuideReveal::Settings, true),
            PanelReveal {
                objects_open: false,
                inspector_open: true,
                pin_objects: false,
                pin_inspector: false,
            }
        );
    }

    #[test]
    fn desktop_reveal_pins_only_the_requested_panel() {
        assert!(panel_reveal(GuideReveal::Guide, false).pin_objects);
        assert!(!panel_reveal(GuideReveal::Guide, false).pin_inspector);
        assert!(panel_reveal(GuideReveal::Settings, false).pin_inspector);
        assert!(!panel_reveal(GuideReveal::Settings, false).pin_objects);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CreationSettlement {
    Wait,
    Reveal,
    Retire,
}

/// Called only after this creation's exact Completed outcome has arrived.
pub(crate) fn creation_settlement(
    project_id: &str,
    accepted_project_id: Option<&str>,
    saved_and_ready: bool,
) -> CreationSettlement {
    let Some(accepted_project_id) = accepted_project_id else {
        return CreationSettlement::Retire;
    };
    if accepted_project_id != project_id {
        CreationSettlement::Retire
    } else if saved_and_ready {
        CreationSettlement::Reveal
    } else {
        CreationSettlement::Wait
    }
}

#[cfg(test)]
mod creation_tests {
    use super::*;

    #[test]
    fn closed_or_superseded_completed_creation_releases_the_pending_slot() {
        assert_eq!(
            creation_settlement("new", None, false),
            CreationSettlement::Retire
        );
        assert_eq!(
            creation_settlement("new", Some("other"), true),
            CreationSettlement::Retire
        );
        assert_eq!(
            creation_settlement("new", Some("new"), false),
            CreationSettlement::Wait
        );
        assert_eq!(
            creation_settlement("new", Some("new"), true),
            CreationSettlement::Reveal
        );
    }
}
