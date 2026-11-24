use super::*;
use crate::support::fixture;

#[test]
fn the_evaluator_reports_changed_and_unchanged_without_touching_the_document() {
    let document = document();
    let before = document.clone();

    let changed = evaluate_edit(&document, &[] as &[BoardContours], &plate_thickness(1.8)).unwrap();
    assert_eq!(changed, MechanicalEditDisposition::Changed);

    let unchanged = evaluate_edit(&document, &[], &plate_thickness(1.5)).unwrap();
    assert_eq!(unchanged, MechanicalEditDisposition::Unchanged);

    assert_eq!(
        document, before,
        "evaluation is read-only: it never mutates the accepted document"
    );
}

#[test]
fn a_linked_feature_is_read_from_the_single_authored_value_on_both_halves() {
    let mut document = linked_pair_document();

    // Move the pair's single authored value. Nothing else is written.
    document.hardware.as_mut().unwrap().pairs[0].features[0].value =
        MechanicalFeatureValue::Mount {
            mount: mount("mount-1", 4.0, 0.0),
        };

    let (left, right) = both_instance_inputs(&document);

    // The author reads the stored value; the other half reads the reflected one.
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 4.0, y: 0.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -4.0, y: 0.0 }
    );

    // The linked feature has exactly one writable value: neither member stores a
    // second copy that could drift from the pair's authored value.
    assert!(
        stored_mounts(&document, "left")
            .iter()
            .all(|mount| mount.id != "mount-1"),
        "the authoring half must not keep a second writable copy"
    );
    assert!(
        stored_mounts(&document, "right")
            .iter()
            .all(|mount| mount.id != "mount-1"),
        "the linked half must not keep a second writable copy"
    );
}

#[test]
fn an_ordinary_delete_of_a_linked_feature_removes_both_halves() {
    let mut document = linked_pair_document();

    // Ordinary Delete is the linked deletion: it removes both counterparts and the
    // relationship itself. Remove-on-this-half is the explicit Omit action.
    apply_with_template(&mut document, instance_target("right"), delete_mount()).unwrap();

    let pair = pair(&document);
    assert!(
        pair.features.is_empty(),
        "a linked delete removes the relationship, not just one half"
    );
    for instance in ["left", "right"] {
        let configuration = boardstudio_core::mechanical::effective_inputs(
            &document,
            &[],
            &instance_target(instance),
        )
        .unwrap()
        .configuration;
        assert!(
            configuration
                .mounts
                .iter()
                .all(|mount| mount.id != "mount-1"),
            "{instance} no longer shows the deleted linked mount"
        );
    }
}

#[test]
fn accepted_contours_drive_absent_closure_mount_initialization() {
    use boardstudio_core::mechanical::edit::apply;

    // The document's own outline is a large board...
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 80.0, y: 60.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    });
    document.boards.push(board("board"));
    document.boards[0].outline_ids = vec!["outline".into()];
    let mut configuration = fixture::mechanical("board");
    configuration.closure_mounts = None;
    document.mechanical = Some(configuration);

    // ...but the accepted scene projection for that board is a small outline.
    let accepted = vec![BoardContours {
        board_id: "board".into(),
        contours: vec![small_rect()],
    }];

    let initialization = with_template(
        board_target(),
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
    );

    let mut from_accepted = document.clone();
    apply(&mut from_accepted, &accepted, &initialization).unwrap();
    let mut from_recomputed = document.clone();
    apply(&mut from_recomputed, &[], &initialization).unwrap();

    let accepted_mounts = closure_mounts(&from_accepted);
    let recomputed_mounts = closure_mounts(&from_recomputed);
    assert!(
        !accepted_mounts.is_empty(),
        "the accepted outline proposes closure mounts"
    );
    assert!(
        accepted_mounts
            .iter()
            .all(|mount| mount.at.x.abs() < 15.0 && mount.at.y.abs() < 10.0),
        "initialization follows the accepted projection: {accepted_mounts:?}"
    );
    assert!(
        recomputed_mounts
            .iter()
            .any(|mount| mount.at.x.abs() > 15.0),
        "the document's own outline would have proposed wider mounts: {recomputed_mounts:?}"
    );
    assert_ne!(
        accepted_mounts, recomputed_mounts,
        "the accepted projection is what initialization consumes"
    );
}

#[test]
fn removing_a_paired_member_materializes_the_survivor_and_clears_the_pair() {
    let mut document = linked_pair_document();
    let before = instance_inputs(&document, "left");
    let survivor_geometry = mount_at(&before.configuration, "mount-1");

    let changed = remove_member(&mut document, "right").unwrap();

    let hardware = document.hardware.as_ref().unwrap();
    assert_eq!(hardware.instances.len(), 1, "the removed half is gone");
    assert_eq!(hardware.instances[0].id, "left");
    assert!(hardware.pairs.is_empty(), "the pair is cleared");
    let survivor = &hardware.instances[0];
    assert_eq!(
        survivor.construction_ownership,
        MechanicalConstructionOwnership::Local,
        "the survivor owns its materialized construction locally"
    );
    assert_eq!(
        mount_at(survivor.mechanical.as_ref().unwrap(), "mount-1"),
        survivor_geometry,
        "the survivor keeps the geometry it showed"
    );
    assert_eq!(
        survivor.mechanical.as_ref().unwrap().plate_thickness,
        before.configuration.plate_thickness
    );
    for id in ["pair-left-right", "left", "right"] {
        assert!(changed.contains(&id.to_string()), "{id} in {changed:?}");
    }

    // The survivor's effective read still shows the same geometry.
    let after = instance_inputs(&document, "left");
    assert_eq!(mount_at(&after.configuration, "mount-1"), survivor_geometry);
}

#[test]
fn a_topology_replacement_never_leaves_a_dangling_pair() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::EditOperation;

    let document = linked_pair_document();

    let mut engine = CoreEngine::new();
    let accepted = match engine.open(document) {
        Ok(CommittedScene { document, .. }) => document,
        other => panic!("expected scene: {other:?}"),
    };

    // The incoming document drops the right half but still lists the pair.
    let mut replacement = accepted.clone();
    replacement
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .retain(|instance| instance.id != "right");
    assert_eq!(
        replacement.hardware.as_ref().unwrap().pairs.len(),
        1,
        "the incoming document still lists the pair"
    );

    let replaced = match engine.commit_edit(fixture::commit(
        "replace",
        "replace",
        accepted.revision,
        EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
    )) {
        Ok(CommittedScene { document, .. }) => document,
        other => panic!("expected scene: {other:?}"),
    };

    let hardware = replaced.hardware.as_ref().unwrap();
    assert_eq!(hardware.instances.len(), 1);
    assert!(
        hardware.pairs.is_empty(),
        "a pair whose member is gone cannot survive the replacement"
    );
    let survivor = &hardware.instances[0];
    assert_eq!(
        survivor.construction_ownership,
        MechanicalConstructionOwnership::Local
    );
    assert_eq!(
        mount_at(survivor.mechanical.as_ref().unwrap(), "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
        "the survivor keeps the geometry it showed"
    );
}

#[test]
fn a_valid_asymmetric_clash_is_retained_with_a_located_finding() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::Scope;

    let mut document = conflicting_pair_document();
    link_pair(&mut document);

    // The reflected mount is exactly where the right half's own hardware sits.
    let right = instance_inputs(&document, "right");
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
        "the intended geometry stays exactly where the pair puts it"
    );
    let finding = right
        .findings
        .iter()
        .find(|finding| finding.target_ids.contains(&"mount-1".to_string()))
        .unwrap_or_else(|| panic!("no located finding: {:?}", right.findings));
    assert_eq!(finding.scope, Scope::Case);
    assert_eq!(
        finding.target_ids,
        vec![
            "board".to_string(),
            "right".to_string(),
            "mount-1".to_string()
        ],
        "the finding locates the board, the half and the feature"
    );

    // The accepted scene carries the same located finding, so readiness and the
    // export gates see accepted state rather than a transient assembly warning.
    let mut engine = CoreEngine::new();
    let scene = match engine.open(document.clone()) {
        Ok(CommittedScene { scene, .. }) => scene,
        other => panic!("expected scene: {other:?}"),
    };
    let located = scene
        .findings
        .iter()
        .find(|finding| finding.id == "mechanical:pair:pair-left-right:right:mount-1:fit")
        .unwrap_or_else(|| panic!("no accepted pair finding: {:?}", scene.findings));
    assert_eq!(located.scope, Scope::Case);
    assert_eq!(
        located.target_ids,
        vec![
            "board".to_string(),
            "right".to_string(),
            "mount-1".to_string()
        ],
        "the accepted finding locates board, half and feature"
    );
    let readiness = scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == "board")
        .expect("board readiness");
    assert!(
        readiness.pcb,
        "a Case-only clash must not block the board's manufacturing export"
    );

    // Without the local hardware there is no clash finding at all: the conflict, not
    // the pairing, is what is reported.
    let mut control = document.clone();
    control
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "right")
        .unwrap()
        .mechanical
        .as_mut()
        .unwrap()
        .mounts
        .retain(|mount| mount.id != "mount-encoder");
    let control_findings =
        boardstudio_core::mechanical::effective_inputs(&control, &[], &instance_target("right"))
            .unwrap()
            .findings;
    assert!(
        control_findings.is_empty(),
        "the same pair without a local conflict reports nothing: {control_findings:?}"
    );
}

#[test]
fn a_removed_opening_vertex_identity_is_never_reused() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = document();
    let opening = add_opening(&mut document);
    let add_vertex = |document: &mut ProjectDoc| {
        apply(
            document,
            &[],
            &with_template(
                board_target(),
                MechanicalChange::AddOpeningVertex {
                    opening_id: opening.clone(),
                },
            ),
        )
        .unwrap();
        document
            .mechanical
            .as_ref()
            .unwrap()
            .openings
            .as_ref()
            .unwrap()
            .iter()
            .find(|candidate| candidate.id == opening)
            .unwrap()
            .vertices
            .last()
            .unwrap()
            .id
            .clone()
    };
    let first = add_vertex(&mut document);
    apply_with_template(
        &mut document,
        board_target(),
        MechanicalChange::RemoveOpeningVertex {
            opening_id: opening.clone(),
            vertex_id: first.clone(),
        },
    )
    .unwrap();
    let second = add_vertex(&mut document);
    assert_ne!(
        second, first,
        "a removed vertex identity must not be handed out again"
    );

    let queued = with_template(
        board_target(),
        MechanicalChange::SetOpeningDimension {
            opening_id: opening.clone(),
            vertex_id: Some(first.clone()),
            field: MechanicalDimension::OpeningPointX,
            value: 4.0,
        },
    );
    assert_eq!(
        evaluate_edit(&document, &[], &queued).unwrap(),
        MechanicalEditDisposition::Retire
    );
}

#[test]
fn a_new_local_opening_never_collides_with_a_visible_pair_owned_identity() {
    let mut document = pair_document_with_openings();
    // The opening is authored by the second half, so it leaves both raw
    // configurations and becomes visible on both through the pair.
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Opening,
            "opening-1",
            "opening-1",
            Some("right"),
        )],
    )
    .unwrap();
    let left = instance_inputs(&document, "left");
    assert!(
        left.configuration
            .openings
            .as_ref()
            .is_some_and(|openings| openings.iter().any(|opening| opening.id == "opening-1")),
        "the non-author half shows the pair-owned opening"
    );

    // A new local addition must not take the visible pair-owned identity, or a queued
    // intent naming the pair feature could reach the new opening instead.
    let new_id = add_instance_opening(&mut document, "left");
    assert_ne!(
        new_id, "opening-1",
        "the visible pair-owned identity is reserved on the member"
    );
    let after = instance_inputs(&document, "left");
    let ids: Vec<&str> = after
        .configuration
        .openings
        .as_ref()
        .unwrap()
        .iter()
        .map(|opening| opening.id.as_str())
        .collect();
    assert!(
        ids.contains(&"opening-1"),
        "the linked opening survives: {ids:?}"
    );
    assert!(ids.contains(&new_id.as_str()));
    assert_eq!(ids.len(), 2);

    // A commit against the pair-owned identity still lands on the linked opening, not
    // the local addition.
    apply_with_template(
        &mut document,
        instance_target("left"),
        MechanicalChange::SetOpeningDimension {
            opening_id: "opening-1".into(),
            vertex_id: None,
            field: MechanicalDimension::OpeningHeight,
            value: 7.0,
        },
    )
    .unwrap();
    let pair = pair(&document);
    let MechanicalFeatureValue::Opening { opening } = &pair.features[0].value else {
        panic!("a linked opening stores an opening");
    };
    assert_eq!(
        opening.height, 7.0,
        "the edit reached the pair's authored value, not the local addition"
    );
}

#[test]
fn disabling_a_paired_half_unlinks_it_and_keeps_the_other_half() {
    let mut document = linked_pair_document();
    let before_left = instance_inputs(&document, "left");

    let changed = apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::Disable,
    )
    .unwrap();

    assert!(
        changed.contains(&"pair-left-right".to_string()),
        "the dissolved pair is reported: {changed:?}"
    );
    let hardware = document.hardware.as_ref().unwrap();
    assert!(hardware.pairs.is_empty(), "the pair is cleared");
    assert_eq!(hardware.instances.len(), 2, "both instances remain");
    let right = hardware
        .instances
        .iter()
        .find(|instance| instance.id == "right")
        .unwrap();
    assert!(
        right.mechanical.is_none(),
        "only the selected half's mechanical stack is disabled"
    );
    let left = hardware
        .instances
        .iter()
        .find(|instance| instance.id == "left")
        .unwrap();
    assert_eq!(
        left.construction_ownership,
        MechanicalConstructionOwnership::Local,
        "the surviving half owns its materialized construction"
    );
    assert_eq!(
        mount_at(left.mechanical.as_ref().unwrap(), "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
        "the surviving half keeps the geometry it showed"
    );
    let after_left = instance_inputs(&document, "left");
    assert_eq!(
        mount_at(&after_left.configuration, "mount-1"),
        Vec2 { x: 2.0, y: 3.0 }
    );
    assert_eq!(
        after_left.configuration.plate_thickness,
        before_left.configuration.plate_thickness
    );
}

#[test]
fn a_case_conflict_on_one_half_does_not_scope_to_the_other_half() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = conflicting_pair_document();
    link_pair(&mut document);

    let right = instance_inputs(&document, "right");
    assert!(
        right
            .findings
            .iter()
            .any(|finding| finding.target_ids.contains(&"right".to_string())),
        "the conflicting half carries the finding: {:?}",
        right.findings
    );
    let left = instance_inputs(&document, "left");
    assert!(
        left.findings.is_empty(),
        "the other half on the same PCB is unaffected: {:?}",
        left.findings
    );
    // A board-scoped read still sees it, because the violation is on that board.
    let board = effective_inputs(
        &document,
        &[],
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    assert!(!board.findings.is_empty());
}

#[test]
fn a_case_only_conflict_never_blocks_manufacturing_export() {
    use boardstudio_core::model::{ExportTarget, PrepareExportRequest};
    use std::collections::BTreeMap;

    let mut document = conflicting_pair_document();
    link_pair(&mut document);

    let export = boardstudio_core::artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "token".into(),
        expected_revision: document.revision,
        document: document.clone(),
        target: ExportTarget::Board {
            board_id: "board".into(),
        },
        contours: vec![rect_contour(40.0, 30.0)],
        model_paths: BTreeMap::new(),
    });
    assert!(
        export.is_ok(),
        "a Case-only clash must not block the board's manufacturing export: {export:?}"
    );
}

#[test]
fn a_re_enabled_half_never_reuses_an_identity_its_removed_stack_reserved() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document_with_openings();
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Opening,
            "opening-1",
            "opening-1",
            Some("left"),
        )],
    )
    .unwrap();
    // The right half is unlinked and disabled (Q14), which removes its stack.
    apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::Disable,
    )
    .unwrap();
    assert!(
        stored_instance(&document, "right")
            .mechanical_identity_reservation
            .is_some(),
        "the removed stack's reservations are kept on the half"
    );

    // Re-enabling builds fresh defaults, and the next authored addition must still avoid
    // the identity the removed stack reserved.
    apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::Enable,
    )
    .unwrap();
    let new_id = add_instance_opening(&mut document, "right");
    assert_ne!(
        new_id, "opening-1",
        "a re-enabled half must not hand out a removed opening identity"
    );

    // The same holds for a half that was disabled without ever being paired.
    let mut unpaired = document_with_an_opening();
    apply(
        &mut unpaired,
        &[],
        &with_template(instance_target("left"), MechanicalChange::Disable),
    )
    .unwrap();
    apply(
        &mut unpaired,
        &[],
        &with_template(instance_target("left"), MechanicalChange::Enable),
    )
    .unwrap();
    let new_id = add_instance_opening(&mut unpaired, "left");
    assert_ne!(
        new_id, "opening-1",
        "an unpaired disabled half keeps its reservations too"
    );
}

#[test]
fn generated_gasket_geometry_uses_the_reflected_author_placement() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::inputs::effective_document;

    // Exercise both the forward and the inverse pair map.
    for (source, mirror) in [("left", "right"), ("right", "left")] {
        let (mut document, accepted) = same_board_gasket_pair_document();
        let author = link_gasket_support(&mut document, &accepted, source, "board");
        let target = instance_target(mirror);
        let inputs = effective_inputs(&document, &accepted, &target).unwrap();
        let reflected = inputs
            .gasket_supports
            .first()
            .expect("the linked support is reflected for the read path");
        assert!(
            (reflected.at.x - author.at.x).abs() > 1.0,
            "{source}: reflection differs from the authored placement"
        );

        // Generation must use that same placement, not re-resolve its perimeter fraction.
        let (effective, _) = effective_document(&document, &target).unwrap();
        let assembly = boardstudio_core::mechanical::resolve(&effective, &accepted[0].contours);
        assert!(
            !assembly.generation_blocked,
            "{source}: {:?}",
            assembly.diagnostics
        );
        let generated = assembly
            .gasket_supports
            .iter()
            .find(|support| support.id == reflected.id)
            .unwrap_or_else(|| {
                panic!(
                    "{source}: no generated support {}: {:?}",
                    reflected.id, assembly.gasket_supports
                )
            });
        assert_close(generated.at, reflected.at, "generated placement");
        assert_close(generated.tangent, reflected.tangent, "generated tangent");
        assert_close(generated.normal, reflected.normal, "generated normal");
        assert_eq!(
            generated.fit_error, reflected.fit_error,
            "{source}: generation reports the located association"
        );
        assert_eq!(generated.pair_id, reflected.pair_id, "{source}");
    }
}

#[test]
fn pair_dissolution_refuses_an_exhausted_linked_identity_and_keeps_the_pair() {
    let max_id = format!("opening-{}", u64::MAX);
    let mut document = pair_document();
    for instance in document.hardware.as_mut().unwrap().instances.iter_mut() {
        instance.mechanical.as_mut().unwrap().openings = Some(vec![opening_json(&max_id)]);
    }
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Opening,
            &max_id,
            &max_id,
            Some("right"),
        )],
    )
    .expect("the max-identity opening links");
    let before = document.clone();

    let error = apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::UnlinkMirrorPair {
            instance_id: "right".into(),
        },
    )
    .unwrap_err();

    assert!(error.contains("exhausted"), "{error}");
    assert_eq!(document, before, "a refused dissolution changed the pair");
}
