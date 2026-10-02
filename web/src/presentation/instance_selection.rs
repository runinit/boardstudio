//! Reference UI preference; Session remains the authority for effective scope.
use boardstudio_application::{Durability, Lifecycle, ReadModel, SessionEpoch};
use boardstudio_core::model::ProjectDoc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Preference {
    pub session_epoch: SessionEpoch,
    pub document_id: String,
    pub explicit_id: String,
}

pub(super) fn resolve<'a>(
    document: &'a ProjectDoc,
    epoch: SessionEpoch,
    board_id: &str,
    preference: Option<&Preference>,
) -> Option<&'a str> {
    let explicit_id = preference
        .filter(|preference| {
            preference.session_epoch == epoch && preference.document_id == document.id
        })
        .map(|preference| preference.explicit_id.as_str());
    let instances = &document.hardware.as_ref()?.instances;
    instances
        .iter()
        .find(|instance| instance.board_id == board_id && Some(instance.id.as_str()) == explicit_id)
        .or_else(|| {
            instances
                .iter()
                .find(|instance| instance.board_id == board_id)
        })
        .map(|instance| instance.id.as_str())
}

pub(super) fn is_current(model: &ReadModel, preference: Option<&Preference>) -> bool {
    model.accepted.as_ref().is_some_and(|snapshot| {
        resolve(
            &snapshot.document,
            snapshot.session_epoch,
            &model.active_board_id,
            preference,
        ) == model.active_instance_id.as_deref()
    })
}

pub(super) fn can_reconcile(model: &ReadModel) -> bool {
    model.lifecycle == Lifecycle::Ready
        && model.gesture.is_none()
        && model.display_preview.is_none()
        && model.accepted.as_ref().is_some_and(|snapshot| {
            model.durability
                == (Durability::Saved {
                    revision: snapshot.document.revision,
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Completion, Effect, Event, OperationId, SaveResult, Session};
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{HardwareConfiguration, PhysicalBoardInstance};

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("same-document", "Instance policy");
        document.hardware = Some(HardwareConfiguration {
            instances: [("a-first", "a"), ("b-first", "b"), ("a-second", "a")]
                .into_iter()
                .map(|(id, board_id)| PhysicalBoardInstance {
                    id: id.into(),
                    name: id.into(),
                    board_id: board_id.into(),
                    half: String::new(),
                    role: String::new(),
                    flipped: id == "a-second",
                    controller_part_id: None,
                    mechanical: None,
                    construction_linked: true,
                })
                .collect(),
            ..HardwareConfiguration::default()
        });
        document
    }

    fn preference() -> Preference {
        Preference {
            session_epoch: SessionEpoch(1),
            document_id: "same-document".into(),
            explicit_id: "a-second".into(),
        }
    }

    #[test]
    fn fallback_uses_board_order_without_requiring_mechanical_configuration() {
        let document = document();
        assert_eq!(
            resolve(&document, SessionEpoch(1), "a", None),
            Some("a-first")
        );
        assert_eq!(
            resolve(&document, SessionEpoch(1), "b", None),
            Some("b-first")
        );
        assert_eq!(
            resolve(&document, SessionEpoch(1), "no-instances", None),
            None
        );
    }

    #[test]
    fn board_fallback_does_not_replace_explicit_preference() {
        let document = document();
        let preference = preference();
        for (board, expected) in [("a", "a-second"), ("b", "b-first"), ("a", "a-second")] {
            assert_eq!(
                resolve(&document, SessionEpoch(1), board, Some(&preference)),
                Some(expected)
            );
        }
    }

    #[test]
    fn restored_instance_recovers_preference_and_reopen_resets_it() {
        let mut document = document();
        let preference = preference();
        let removed = document.hardware.as_mut().unwrap().instances.pop().unwrap();
        assert_eq!(
            resolve(&document, SessionEpoch(1), "a", Some(&preference)),
            Some("a-first")
        );
        document.hardware.as_mut().unwrap().instances.push(removed);
        assert_eq!(
            resolve(&document, SessionEpoch(1), "a", Some(&preference)),
            Some("a-second")
        );
        assert_eq!(
            resolve(&document, SessionEpoch(2), "a", Some(&preference)),
            Some("a-first")
        );
        document.id = "different-document".into();
        assert_eq!(
            resolve(&document, SessionEpoch(1), "a", Some(&preference)),
            Some("a-first")
        );
    }

    #[test]
    fn automatic_navigation_waits_for_real_session_open_and_save() {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        assert!(!can_reconcile(session.read_model()));
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document: ProjectDoc::empty("open", "Open"),
        });
        assert!(!can_reconcile(session.read_model()));
        let mut saves = Vec::new();
        for effect in effects {
            if let Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } = effect
            {
                saves.extend(session.complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(core.handle(*request)),
                }));
            }
        }
        assert!(!can_reconcile(session.read_model()));
        for effect in saves {
            if let Effect::Persist {
                save_attempt_id, ..
            } = effect
            {
                session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                });
            }
        }
        assert!(can_reconcile(session.read_model()));
        assert!(is_current(session.read_model(), None));
        let mut previewing = session.read_model().clone();
        previewing.display_preview = previewing
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.scene.clone());
        assert!(!can_reconcile(&previewing));
        let mut recovering = session.read_model().clone();
        recovering.lifecycle = Lifecycle::RecoveryRequired;
        assert!(!can_reconcile(&recovering));
    }
}
