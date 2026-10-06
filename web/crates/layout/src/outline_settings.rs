//! Private mapping from accepted outline settings intents to existing Core edit operations.
use boardstudio_application::OperationId;
use boardstudio_core::model::{
    Board, CornerStyle, EditOperation, Operation, OutlineFeature, OutlineRepairSettings,
    OutlineSettings, ProjectDoc, ProtectedOutlineGap, SceneDelta,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OutlineEdit {
    RenameVersion { version_id: String, name: String },
    CreateAutomatic,
    SetMargin(f64),
    SetCorners(CornerStyle),
    SetSize(f64),
    SetBridgeWidth(f64),
    SetRepairEnabled(bool),
    SetMaximumGapSpan(f64),
    SetMinimumConnectionWidth(f64),
    SetEdgeClearance(f64),
    SetProtectedGap { gap_id: String, protected: bool },
    RemoveProtectedGap { gap_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OutlineExpectation {
    VersionName {
        version_id: String,
        name: String,
    },
    VersionSettings {
        version_id: String,
        settings: OutlineSettings,
    },
    GeneratedFeature(OutlineFeature),
    VersionFeature {
        version_id: String,
        feature: OutlineFeature,
    },
}

pub(crate) fn generated_feature<'a>(
    document: &'a ProjectDoc,
    board: &Board,
) -> Option<&'a OutlineFeature> {
    board
        .outline_ids
        .iter()
        .filter_map(|id| document.outline.iter().find(|feature| feature.id() == id))
        .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
}

pub(crate) fn generated_settings(feature: &OutlineFeature) -> Option<OutlineSettings> {
    match feature {
        OutlineFeature::PartEnvelope { settings, .. } => Some(settings.clone()),
        _ => None,
    }
}

pub(crate) fn generated_margin(feature: &OutlineFeature) -> Option<f64> {
    match feature {
        OutlineFeature::PartEnvelope { margin, .. } => Some(*margin),
        _ => None,
    }
}

pub(crate) fn reference_outline_settings() -> OutlineSettings {
    OutlineSettings {
        corners: CornerStyle::Fillet,
        size: 2.0,
        bridge_width: 10.0,
        repair: None,
    }
}

pub(crate) fn expectation_applied(
    document: &ProjectDoc,
    board_id: &str,
    expected: &OutlineExpectation,
) -> bool {
    let state = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    match expected {
        OutlineExpectation::VersionName { version_id, name } => state
            .into_iter()
            .flat_map(|state| &state.versions)
            .any(|version| version.id == *version_id && version.name == *name),
        OutlineExpectation::VersionSettings {
            version_id,
            settings,
        } => state
            .into_iter()
            .flat_map(|state| &state.versions)
            .any(|version| version.id == *version_id && version.geometry.settings == *settings),
        OutlineExpectation::GeneratedFeature(feature) => {
            document
                .boards
                .iter()
                .find(|board| board.id == board_id)
                .is_some_and(|board| board.outline_ids.iter().any(|id| id == feature.id()))
                && document
                    .outline
                    .iter()
                    .any(|candidate| candidate == feature)
        }
        OutlineExpectation::VersionFeature {
            version_id,
            feature,
        } => state
            .into_iter()
            .flat_map(|state| &state.versions)
            .find(|version| version.id == *version_id)
            .is_some_and(|version| version.geometry.features.iter().any(|item| item == feature)),
    }
}

pub(crate) fn apply_outline_edit(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    edit: &OutlineEdit,
    operation_id: OperationId,
) -> Option<(EditOperation, OutlineExpectation, Vec<String>)> {
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let state = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    let active_version = state.and_then(|state| {
        state
            .active_version_id
            .as_ref()
            .and_then(|id| state.versions.iter().find(|version| version.id == *id))
    });

    if let OutlineEdit::RenameVersion { version_id, name } = edit {
        let name = name.trim();
        if name.is_empty() || name.encode_utf16().count() > 120 {
            return None;
        }
        let version = state?
            .versions
            .iter()
            .find(|version| version.id == *version_id)?;
        if version.name == name {
            return None;
        }
        return Some((
            EditOperation::RenameOutline {
                board_id: board_id.to_owned(),
                version_id: version_id.clone(),
                name: name.to_owned(),
            },
            OutlineExpectation::VersionName {
                version_id: version_id.clone(),
                name: name.to_owned(),
            },
            vec![board_id.to_owned(), version_id.clone()],
        ));
    }

    if matches!(edit, OutlineEdit::CreateAutomatic) {
        if generated_feature(document, board).is_some() {
            return None;
        }
        let feature_id = format!("board:{board_id}:automatic:outline:{}", operation_id.0);
        if document
            .outline
            .iter()
            .any(|feature| feature.id() == feature_id)
        {
            return None;
        }
        let feature = OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: reference_outline_settings(),
            id: feature_id.clone(),
            part_ids: board.part_ids.clone(),
            margin: 4.0,
            operation: Operation::Add,
        };
        let mut replacement = document.clone();
        replacement.outline.push(feature.clone());
        if let Some(target) = replacement
            .boards
            .iter_mut()
            .find(|item| item.id == board_id)
        {
            target.outline_ids.push(feature_id.clone());
        }
        return Some((
            EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
            OutlineExpectation::GeneratedFeature(feature),
            vec![board_id.to_owned(), feature_id],
        ));
    }

    let mut settings = active_version
        .map(|version| version.geometry.settings.clone())
        .or_else(|| generated_feature(document, board).and_then(generated_settings))
        .unwrap_or_default();
    let mut generated_margin = None;
    match edit {
        OutlineEdit::SetMargin(value) => {
            if active_version.is_some() || !valid_nonnegative(*value) {
                return None;
            }
            generated_margin = Some(*value);
        }
        OutlineEdit::SetCorners(value) => settings.corners = *value,
        OutlineEdit::SetSize(value) => {
            if !valid_nonnegative(*value) {
                return None;
            }
            settings.size = *value;
        }
        OutlineEdit::SetBridgeWidth(value) => {
            if active_version.is_some() || !valid_positive(*value) {
                return None;
            }
            settings.bridge_width = *value;
        }
        OutlineEdit::SetRepairEnabled(value) => {
            if active_version.is_some() {
                return None;
            }
            settings
                .repair
                .get_or_insert_with(OutlineRepairSettings::default)
                .enabled = *value;
        }
        OutlineEdit::SetMaximumGapSpan(value) => {
            if active_version.is_some() || !valid_nonnegative(*value) {
                return None;
            }
            settings
                .repair
                .get_or_insert_with(OutlineRepairSettings::default)
                .maximum_gap_span = *value;
        }
        OutlineEdit::SetMinimumConnectionWidth(value) => {
            if !valid_nonnegative(*value) {
                return None;
            }
            settings
                .repair
                .get_or_insert_with(OutlineRepairSettings::default)
                .minimum_connection_width = *value;
        }
        OutlineEdit::SetEdgeClearance(value) => {
            if !valid_nonnegative(*value) {
                return None;
            }
            settings
                .repair
                .get_or_insert_with(OutlineRepairSettings::default)
                .edge_clearance = *value;
        }
        OutlineEdit::SetProtectedGap { gap_id, protected } => {
            if active_version.is_some() {
                return None;
            }
            let gap = scene
                .board_outline_scenes
                .iter()
                .find(|scene| scene.board_id == board_id)?
                .gaps
                .iter()
                .find(|gap| gap.id == *gap_id)?;
            let feature = generated_feature(document, board)?;
            if feature.id() != gap.feature_id {
                return None;
            }
            let repair = settings
                .repair
                .get_or_insert_with(OutlineRepairSettings::default);
            let matches = std::iter::once(gap.id.as_str())
                .chain(gap.protected_ids.iter().map(String::as_str))
                .collect::<std::collections::BTreeSet<_>>();
            let was_protected = repair
                .keep_gaps
                .iter()
                .any(|item| matches.contains(item.id.as_str()));
            if was_protected == *protected {
                return None;
            }
            repair
                .keep_gaps
                .retain(|item| !matches.contains(item.id.as_str()));
            if *protected {
                repair.keep_gaps.push(ProtectedOutlineGap {
                    id: gap.id.clone(),
                    points: gap.points.clone(),
                });
            }
        }
        OutlineEdit::RemoveProtectedGap { gap_id } => {
            if active_version.is_some() {
                return None;
            }
            let repair = settings
                .repair
                .get_or_insert_with(OutlineRepairSettings::default);
            let index = repair
                .keep_gaps
                .iter()
                .position(|item| item.id == *gap_id)?;
            repair.keep_gaps.remove(index);
        }
        OutlineEdit::RenameVersion { .. } | OutlineEdit::CreateAutomatic => return None,
    }

    if let Some(version) = active_version {
        let mut replacement = document.clone();
        let target = replacement
            .board_outlines
            .iter_mut()
            .find(|state| state.board_id == board_id)?
            .versions
            .iter_mut()
            .find(|item| item.id == version.id)?;
        if target.geometry.settings == settings {
            return None;
        }
        target.geometry.settings = settings.clone();
        Some((
            EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
            OutlineExpectation::VersionSettings {
                version_id: version.id.clone(),
                settings,
            },
            vec![board_id.to_owned(), version.id.clone()],
        ))
    } else {
        let feature = generated_feature(document, board)?;
        let OutlineFeature::PartEnvelope {
            connections,
            settings: _,
            id,
            part_ids,
            margin,
            operation,
        } = feature
        else {
            return None;
        };
        let next = OutlineFeature::PartEnvelope {
            connections: connections.clone(),
            settings,
            id: id.clone(),
            part_ids: part_ids.clone(),
            margin: generated_margin.unwrap_or(*margin),
            operation: *operation,
        };
        if next == *feature {
            return None;
        }
        Some((
            EditOperation::SetOutline {
                feature: next.clone(),
            },
            OutlineExpectation::GeneratedFeature(next),
            vec![board_id.to_owned(), id.clone()],
        ))
    }
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        BoardOutline, BoardOutlineScene, OutlineControlPoint, OutlineGap, Vec2,
    };

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Project");
        document.boards.push(Board {
            id: "board".into(),
            name: "Main".into(),
            outline_ids: vec!["generated".into()],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: reference_outline_settings(),
            id: "generated".into(),
            part_ids: vec![],
            margin: 4.0,
            operation: Operation::Add,
        });
        document
    }

    fn scene(gaps: Vec<OutlineGap>) -> SceneDelta {
        SceneDelta {
            module_scenes: vec![],
            revision: 1,
            transaction_id: "fixture".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![BoardOutlineScene {
                board_id: "board".into(),
                source_contours: vec![],
                bridges: vec![],
                gaps,
            }],
            finding_markers: vec![],
            findings: vec![],
            readiness: boardstudio_core::model::Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        }
    }

    #[test]
    fn generated_creation_uses_react_defaults_and_registers_board_membership() {
        let document = document();
        let mut without_generator = document.clone();
        without_generator.outline.clear();
        without_generator.boards[0].outline_ids.clear();
        let (operation, expectation, targets) = apply_outline_edit(
            &without_generator,
            &scene(vec![]),
            "board",
            &OutlineEdit::CreateAutomatic,
            OperationId(8),
        )
        .unwrap();
        let EditOperation::ReplaceDocument {
            document: replacement,
        } = operation
        else {
            panic!("creation must register the generator and board membership together")
        };
        let generated_board = replacement
            .boards
            .iter()
            .find(|board| board.id == "board")
            .unwrap();
        let generated = generated_feature(&replacement, generated_board).unwrap();
        let OutlineFeature::PartEnvelope {
            margin, settings, ..
        } = generated
        else {
            panic!("automatic outline feature expected")
        };
        assert_eq!(*margin, 4.0);
        assert_eq!(settings.corners, CornerStyle::Fillet);
        assert_eq!(settings.size, 2.0);
        assert_eq!(settings.bridge_width, 10.0);
        assert_eq!(targets.len(), 2);
        assert!(expectation_applied(&replacement, "board", &expectation));
    }

    #[test]
    fn generated_settings_use_set_outline_and_reject_invalid_values() {
        let document = document();
        let edit = apply_outline_edit(
            &document,
            &scene(vec![]),
            "board",
            &OutlineEdit::SetMargin(6.5),
            OperationId(9),
        )
        .unwrap();
        let EditOperation::SetOutline { feature } = edit.0 else {
            panic!("generated field updates use existing SetOutline")
        };
        let OutlineFeature::PartEnvelope { margin, .. } = &feature else {
            panic!("generated feature expected")
        };
        assert_eq!(*margin, 6.5);
        assert!(
            apply_outline_edit(
                &document,
                &scene(vec![]),
                "board",
                &OutlineEdit::SetBridgeWidth(f64::NAN),
                OperationId(10),
            )
            .is_none()
        );
        assert!(
            apply_outline_edit(
                &document,
                &scene(vec![]),
                "other-board",
                &OutlineEdit::SetMargin(6.5),
                OperationId(11),
            )
            .is_none()
        );
    }

    #[test]
    fn fixed_settings_replace_only_the_named_accepted_version() {
        let mut document = document();
        document.board_outlines.push(BoardOutline {
            board_id: "board".into(),
            active_version_id: Some("fixed".into()),
            generated_last_valid: None,
            versions: vec![boardstudio_core::model::OutlineVersion {
                id: "fixed".into(),
                name: "Fixed".into(),
                source: boardstudio_core::model::OutlineProvenance {
                    revision: 1,
                    version_id: None,
                },
                geometry: boardstudio_core::model::OutlineSnapshot {
                    features: vec![],
                    settings: reference_outline_settings(),
                    expected_regions: 1,
                    bridges: vec![],
                    protected_gaps: vec![],
                },
            }],
        });
        let (operation, expectation, _) = apply_outline_edit(
            &document,
            &scene(vec![]),
            "board",
            &OutlineEdit::SetEdgeClearance(1.25),
            OperationId(12),
        )
        .unwrap();
        let EditOperation::ReplaceDocument {
            document: replacement,
        } = operation
        else {
            panic!("fixed settings live in BoardOutline snapshot state")
        };
        assert_eq!(document.outline[0], replacement.outline[0]);
        assert_eq!(
            replacement.board_outlines[0].versions[0]
                .geometry
                .settings
                .repair
                .as_ref()
                .unwrap()
                .edge_clearance,
            1.25
        );
        assert!(expectation_applied(&replacement, "board", &expectation));
    }

    #[test]
    fn protected_gap_keeps_accepted_source_controls_and_rejects_stale_gap() {
        let document = document();
        let gap = OutlineGap {
            id: "gap-1".into(),
            feature_id: "generated".into(),
            span: 3.0,
            points: vec![OutlineControlPoint {
                at: Vec2 { x: 1.0, y: 2.0 },
                part_id: Some("part-1".into()),
            }],
            protected: false,
            protected_ids: vec![],
        };
        let (operation, expectation, _) = apply_outline_edit(
            &document,
            &scene(vec![gap.clone()]),
            "board",
            &OutlineEdit::SetProtectedGap {
                gap_id: "gap-1".into(),
                protected: true,
            },
            OperationId(13),
        )
        .unwrap();
        let EditOperation::SetOutline { feature } = operation else {
            panic!("gap settings update the generator feature")
        };
        let OutlineFeature::PartEnvelope { settings, .. } = &feature else {
            panic!("generated feature expected")
        };
        let protected = &settings.repair.as_ref().unwrap().keep_gaps[0];
        assert_eq!(protected.id, gap.id);
        assert_eq!(protected.points, gap.points);
        let accepted = ProjectDoc {
            outline: vec![feature],
            ..document.clone()
        };
        assert!(expectation_applied(&accepted, "board", &expectation,));
        assert!(
            apply_outline_edit(
                &document,
                &scene(vec![]),
                "board",
                &OutlineEdit::SetProtectedGap {
                    gap_id: "removed-gap".into(),
                    protected: true,
                },
                OperationId(14),
            )
            .is_none()
        );
    }
}
