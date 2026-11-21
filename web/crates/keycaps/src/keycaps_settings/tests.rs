use super::*;

fn test_scope() -> Scope {
    Scope {
        session_epoch: boardstudio_application::SessionEpoch(1),
        document_id: "doc".into(),
        board_id: "board".into(),
        instance_id: None,
    }
}

#[test]
fn width_and_depth_intents_merge_with_the_latest_accepted_sibling() {
    let scope = test_scope();
    let document = ProjectDoc {
        keycaps: Some(boardstudio_model::model::KeycapConfiguration {
            keys: [(
                "sw1".into(),
                KeycapKeySettings {
                    units: Some(Vec2 { x: 2.0, y: 1.5 }),
                    ..KeycapKeySettings::default()
                },
            )]
            .into(),
            ..boardstudio_model::model::KeycapConfiguration::default()
        }),
        ..ProjectDoc::empty("keycaps-settings-test", "Keycaps settings test")
    };
    assert_eq!(
        edit_operation(
            &document,
            &scope,
            &KeycapsEditTarget::Key("sw1".into()),
            &KeycapEditChange::UnitsWidth(3.0)
        ),
        Some(EditOperation::SetKeycapKey {
            key_id: "sw1".into(),
            change: KeycapKeyChange::Units {
                value: Some(Vec2 { x: 3.0, y: 1.5 })
            }
        })
    );
    assert_eq!(
        edit_operation(
            &document,
            &scope,
            &KeycapsEditTarget::Key("sw1".into()),
            &KeycapEditChange::UnitsDepth(2.25)
        ),
        Some(EditOperation::SetKeycapKey {
            key_id: "sw1".into(),
            change: KeycapKeyChange::Units {
                value: Some(Vec2 { x: 2.0, y: 2.25 })
            }
        })
    );
    assert_eq!(
        edit_operation(
            &document,
            &scope,
            &KeycapsEditTarget::Key("sw1".into()),
            &KeycapEditChange::ClearUnits(KeycapEditField::UnitsDepth)
        ),
        Some(EditOperation::SetKeycapKey {
            key_id: "sw1".into(),
            change: KeycapKeyChange::Units { value: None }
        })
    );
}

#[test]
fn setting_noop_preserves_null_and_blank_legend() {
    let scope = test_scope();
    let document = ProjectDoc {
        keycaps: Some(boardstudio_model::model::KeycapConfiguration {
            keys: [(
                "sw1".into(),
                KeycapKeySettings {
                    legend: Some(String::new()),
                    ..KeycapKeySettings::default()
                },
            )]
            .into(),
            ..boardstudio_model::model::KeycapConfiguration::default()
        }),
        ..ProjectDoc::empty("keycaps-settings-test", "Keycaps settings test")
    };
    assert!(change_is_noop(
        &document,
        &scope,
        &KeycapsEditTarget::Key("sw1".into()),
        &KeycapEditChange::Legend(Some(String::new()))
    ));
    assert!(!change_is_noop(
        &document,
        &scope,
        &KeycapsEditTarget::Key("sw1".into()),
        &KeycapEditChange::Legend(None)
    ));
}

#[test]
fn selection_owner_generation_changes_when_a_key_is_reselected() {
    let scope = Scope {
        session_epoch: boardstudio_application::SessionEpoch(1),
        document_id: "doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let mut tracker = OwnerTracker::default();
    let first = tracker.observe(Some(&scope), 0, "Keycaps", Some("sw1"));
    let other = tracker.observe(Some(&scope), 0, "Keycaps", Some("sw2"));
    let returned = tracker.observe(Some(&scope), 0, "Keycaps", Some("sw1"));
    assert!(first < other && other < returned);
}

#[test]
fn latest_same_owner_retry_intent_survives_older_operation_failure() {
    let scope = Scope {
        session_epoch: boardstudio_application::SessionEpoch(1),
        document_id: "doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let mut drafts = BTreeMap::new();
    let request = |request_id, width| KeycapsEditRequest {
        scope: scope.clone(),
        scope_generation: 3,
        selection_generation: 8,
        editor_instance_id: 11,
        request_id,
        admission_token: SnapshotToken(9),
        admission_revision: 4,
        target: KeycapsEditTarget::Key("sw1".into()),
        baseline: KeycapsEditBaseline::Key(KeycapKeySettings::default()),
        change: KeycapEditChange::UnitsWidth(width),
    };

    let first = request(1, 2.0);
    let latest = request(2, 3.0);
    store_retry_draft(&mut drafts, &latest, "another edit is pending".into());
    store_retry_draft(&mut drafts, &first, "older edit failed".into());

    let target = KeycapsEditTarget::Key("sw1".into());
    let retained = drafts
        .get(&(target.clone(), KeycapEditField::UnitsWidth))
        .unwrap();
    assert_eq!(retained.request_id, 2);
    assert_eq!(retained.change, KeycapEditChange::UnitsWidth(3.0));
    assert!(!draft_owner_mismatch(
        retained,
        11,
        &scope,
        3,
        8,
        Some("sw1"),
        "Keycaps"
    ));
    assert!(draft_owner_mismatch(
        retained,
        11,
        &scope,
        3,
        9,
        Some("sw1"),
        "Keycaps"
    ));
    let identity = retry_identity(retained);
    assert!(retry_identity_matches(retained, &identity));
    let mut stale_identity = identity.clone();
    stale_identity.request_id = first.request_id;
    assert!(!retry_identity_matches(retained, &stale_identity));
    remove_retry_draft_through_map(
        &mut drafts,
        &target,
        &KeycapEditField::UnitsWidth,
        first.request_id,
    );
    assert_eq!(
        drafts
            .get(&(target.clone(), KeycapEditField::UnitsWidth))
            .map(|draft| draft.request_id),
        Some(latest.request_id)
    );
    remove_retry_draft_through_map(
        &mut drafts,
        &target,
        &KeycapEditField::UnitsWidth,
        latest.request_id,
    );
    assert!(!drafts.contains_key(&(target, KeycapEditField::UnitsWidth)));
}

#[test]
fn untouched_inherited_legend_does_not_materialize_an_explicit_blank() {
    assert!(!legend_change_required("", None));
    assert!(!legend_change_required("", Some("")));
    assert!(legend_change_required("A", None));
    assert!(legend_blur_requires_reconciliation("", None, true));
    assert!(!legend_blur_requires_reconciliation("", None, false));
}

#[test]
fn board_and_matrix_settings_use_existing_core_operations() {
    let scope = test_scope();
    let document = ProjectDoc::empty("keycaps-settings-test", "Keycaps settings test");
    assert_eq!(
        edit_operation(
            &document,
            &scope,
            &KeycapsEditTarget::Board,
            &KeycapEditChange::BoardColor("#abcdef".into())
        ),
        Some(EditOperation::SetKeycapBoard {
            board_id: "board".into(),
            change: KeycapBoardChange::Color {
                value: "#abcdef".into()
            }
        })
    );
    assert_eq!(
        edit_operation(
            &document,
            &scope,
            &KeycapsEditTarget::Matrix("matrix-a".into()),
            &KeycapEditChange::MatrixProfile(Some(KeycapProfile::Cherry))
        ),
        Some(EditOperation::SetMatrixKeycaps {
            matrix_id: "matrix-a".into(),
            change: KeycapMatrixChange::Profile {
                value: Some(KeycapProfile::Cherry)
            }
        })
    );
    assert!(change_is_noop(
        &document,
        &scope,
        &KeycapsEditTarget::Board,
        &KeycapEditChange::BoardClearance(KeycapBoardSettings::default().clearance)
    ));
    assert!(change_is_noop(
        &document,
        &scope,
        &KeycapsEditTarget::Matrix("matrix-a".into()),
        &KeycapEditChange::MatrixFirstRow(KeycapMatrixSettings::default().first_row)
    ));
}
