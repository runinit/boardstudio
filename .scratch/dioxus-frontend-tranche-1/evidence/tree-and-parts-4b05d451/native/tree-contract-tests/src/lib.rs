#![allow(dead_code)]

mod presentation {
    pub mod objects {
        #[path = "/tmp/frontend-run/tree-contract-tests/tree.rs.source-8ea71a84"]
        mod tree;

        #[cfg(test)]
        mod contract_tests {
            use super::tree::{
                Grouping, TreeContext, TreeKind, build_tree, context_for_cell, context_for_part,
                resolve_selection,
            };
            use boardstudio_application::{
                Completion, Effect, Event, ExecutorEpoch, OperationId, ReadModel, RequestId,
                SaveAttemptId, SaveResult, Session,
            };
            use boardstudio_core::{CoreEngine, model::{CoreRequest, LayoutMirrorLink, ProjectDoc}};
            use std::collections::{BTreeSet, HashMap, HashSet};

            const REVIUNG: &str = include_str!("/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/m1-release-20261001-050e9282/fixtures/reviung41.json");
            const SOFLE: &str = include_str!("/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/m1-release-20261001-050e9282/fixtures/sofle.json");

            fn fixture(json: &str) -> ProjectDoc {
                serde_json::from_str(json).expect("actual project fixture parses as ProjectDoc")
            }

            fn core_effect(effects: &[Effect]) -> (RequestId, ExecutorEpoch, CoreRequest) {
                effects.iter().find_map(|effect| match effect {
                    Effect::Core { request_id, executor_epoch, request, .. } =>
                        Some((*request_id, *executor_epoch, (**request).clone())),
                    _ => None,
                }).expect("Session emits its actual Core request")
            }

            fn persist_effect(effects: &[Effect]) -> SaveAttemptId {
                effects.iter().find_map(|effect| match effect {
                    Effect::Persist { save_attempt_id, .. } => Some(*save_attempt_id),
                    _ => None,
                }).expect("Core acceptance emits persistence")
            }

            fn open_session(document: ProjectDoc) -> Session {
                let mut session = Session::new();
                let mut engine = CoreEngine::new();
                let effects = session.submit(Event::Open { operation_id: OperationId(1), document });
                let (request_id, executor_epoch, request) = core_effect(&effects);
                let reply = engine.handle(request);
                let reply_debug = format!("{reply:?}");
                let effects = session.complete(Completion::Core {
                    request_id, executor_epoch, reply: Box::new(reply),
                });
                let save_attempt_id = effects.iter().find_map(|effect| match effect {
                    Effect::Persist { save_attempt_id, .. } => Some(*save_attempt_id),
                    _ => None,
                }).unwrap_or_else(|| panic!("Session did not accept Core open; reply was {reply_debug}; effects were {effects:?}"));
                session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                });
                session
            }

            fn open(document: ProjectDoc) -> ReadModel {
                open_session(document).read_model().clone()
            }

            fn expected_matrix_members(model: &ReadModel, matrix_id: &str) -> (Vec<String>, HashMap<String, (u32, u32)>) {
                let snapshot = model.accepted.as_ref().unwrap();
                let doc = &snapshot.document;
                let board = doc.boards.iter().find(|b| b.id == model.active_board_id).unwrap();
                let board_ids: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
                let live_ids: HashSet<_> = doc.parts.iter().filter(|p| board_ids.contains(p.id.as_str())).map(|p| p.id.as_str()).collect();
                let matrix = doc.matrices.iter().find(|m| m.id == matrix_id).unwrap();
                let scene = snapshot.scene.matrix_scenes.iter().find(|s| s.matrix_id == matrix_id).unwrap();
                let mut coordinates = HashMap::new();
                for cell in scene.cells.iter().filter(|cell| cell.enabled) {
                    let Some(primary) = cell.member_id.as_deref().filter(|id| live_ids.contains(id)) else { continue };
                    if matrix.part_ids.iter().any(|id| id == primary) {
                        coordinates.insert(primary.to_owned(), (cell.row, cell.column));
                    }
                    if let Some(config) = matrix.cells.iter().find(|c| c.row == cell.row && c.column == cell.column) {
                        for assembly in &config.assemblies {
                            let candidate = format!("{primary}/{}", assembly.id);
                            if matrix.part_ids.contains(&candidate) && live_ids.contains(candidate.as_str()) {
                                coordinates.insert(candidate, (cell.row, cell.column));
                            }
                        }
                    }
                }
                let ordered = matrix.part_ids.iter().filter(|id| coordinates.contains_key(id.as_str())).cloned().collect();
                (ordered, coordinates)
            }

            fn expanded_tree(model: &ReadModel, grouping: Grouping) -> Vec<super::tree::TreeItem> {
                let snapshot = model.accepted.as_ref().unwrap();
                let doc = &snapshot.document;
                let board_id = &model.active_board_id;
                let mut expanded = BTreeSet::from([format!("board:{board_id}"), format!("layout:{board_id}")]);
                let board_parts: HashSet<_> = doc.boards.iter().find(|b| b.id == *board_id).unwrap().part_ids.iter().map(String::as_str).collect();
                let visible = super::tree::visible_matrices(doc, board_id, &board_parts);
                let layouts: Vec<_> = doc.layouts.iter().filter(|l| l.board_id == *board_id && visible.iter().any(|m| m.id == l.matrix_id)).collect();
                let split = layouts.iter().any(|l| l.mirror_link.is_some()) || layouts.iter().any(|l| layouts.iter().any(|o| o.mirror_link.as_ref().is_some_and(|link| link.source_id == l.id)));
                let _ = split;
                for layout in &layouts {
                    expanded.insert(format!("half:{}", layout.id));
                }
                for matrix in visible {
                    expanded.insert(format!("matrix:{}", matrix.id));
                    match grouping {
                        Grouping::Column => for group in 0..matrix.columns { expanded.insert(format!("column:{}:{group}", matrix.id)); },
                        Grouping::Row => for group in 0..matrix.rows { expanded.insert(format!("row:{}:{group}", matrix.id)); },
                    }
                    for row in 0..matrix.rows { for column in 0..matrix.columns { expanded.insert(format!("key:{}:{row}:{column}", matrix.id)); } }
                }
                build_tree(doc, board_id, grouping, &expanded, &snapshot.scene.matrix_scenes)
            }

            #[test]
            fn actual_reviung_tree_resolves_enabled_members_and_companions_in_document_order() {
                let model = open(fixture(REVIUNG));
                let doc = &model.accepted.as_ref().unwrap().document;
                let tree = expanded_tree(&model, Grouping::Column);
                assert!(tree.iter().any(|item| item.kind == TreeKind::Matrix));
                assert!(tree.iter().any(|item| item.kind == TreeKind::Key));
                let matrix = doc.matrices.iter().find(|m| m.id == "main-right-keys").unwrap();
                let (expected, coords) = expected_matrix_members(&model, &matrix.id);
                let actual = resolve_selection(&model, &TreeContext::Matrix { matrix_id: matrix.id.clone() }).unwrap();
                assert_eq!(actual, expected, "matrix selection uses actual projected enabled/live membership and source matrix order");
                assert!(actual.iter().any(|id| id.ends_with("/diode")), "fixture has generated companions selected by matrix context");
                let matrix_item = tree.iter().find(|item| item.id == format!("matrix:{}", matrix.id)).unwrap();
                let projected_live_primary_count = doc.matrices.iter().find(|m| m.id == matrix.id).unwrap().cells.iter().filter(|config| {
                    model.accepted.as_ref().unwrap().scene.matrix_scenes.iter().find(|scene| scene.matrix_id == matrix.id).unwrap().cells.iter().any(|cell| {
                        cell.row == config.row && cell.column == config.column && cell.enabled && cell.member_id.as_ref().is_some_and(|id| matrix.part_ids.contains(id) && doc.parts.iter().any(|part| part.id == *id))
                    })
                }).count();
                assert_eq!(matrix_item.detail.as_deref(), Some(format!("{projected_live_primary_count} keys").as_str()));
                for column_index in 0..matrix.columns {
                    let expected_count = model.accepted.as_ref().unwrap().scene.matrix_scenes.iter().find(|scene| scene.matrix_id == matrix.id).unwrap().cells.iter().filter(|cell| cell.enabled && cell.column == column_index && cell.member_id.as_ref().is_some_and(|id| matrix.part_ids.contains(id) && doc.parts.iter().any(|part| part.id == *id))).count();
                    let group = tree.iter().find(|item| item.id == format!("column:{}:{column_index}", matrix.id)).unwrap();
                    assert_eq!(group.detail.as_deref(), Some(format!("{expected_count} keys").as_str()));
                }
                let row = resolve_selection(&model, &TreeContext::Row { matrix_id: matrix.id.clone(), row: 0 }).unwrap();
                let expected_row: Vec<_> = expected.iter().filter(|id| coords.get(*id).is_some_and(|(r, _)| *r == 0)).cloned().collect();
                assert_eq!(row, expected_row);
                let column = resolve_selection(&model, &TreeContext::Column { matrix_id: matrix.id.clone(), column: 0 }).unwrap();
                let expected_column: Vec<_> = expected.iter().filter(|id| coords.get(*id).is_some_and(|(_, c)| *c == 0)).cloned().collect();
                assert_eq!(column, expected_column);
                let (row0, col0) = coords.iter().find(|(id, _)| !id.ends_with("/diode")).map(|(_, rc)| *rc).unwrap();
                let key = resolve_selection(&model, &TreeContext::Key { matrix_id: matrix.id.clone(), row: row0, column: col0 }).unwrap();
                assert_eq!(key.len(), 1);
                assert!(!key[0].ends_with("/diode"));
                let companion = expected.iter().find(|id| id.ends_with("/diode")).unwrap();
                let context = context_for_part(&model, companion).expect("actual generated companion maps to its cell component context");
                assert_eq!(resolve_selection(&model, &context).unwrap(), vec![companion.clone()]);
                let standalone = doc.boards.iter().find(|board| board.id == model.active_board_id).unwrap().part_ids.iter().find(|id| !doc.matrices.iter().filter(|m| m.board_id.as_deref().is_none_or(|owner| owner == model.active_board_id)).any(|matrix| matrix.part_ids.contains(id))).expect("real fixture has a standalone board part");
                let standalone_context = context_for_part(&model, standalone).expect("live standalone part maps to component context");
                assert_eq!(resolve_selection(&model, &standalone_context).unwrap(), vec![standalone.clone()]);
                assert!(tree.iter().any(|item| item.id == format!("component:{standalone}")), "active-board standalone parts appear in the tree");
                let deleted_matrix = TreeContext::Matrix { matrix_id: "removed-matrix-from-real-document".into() };
                assert_eq!(resolve_selection(&model, &deleted_matrix), None);
            }

            #[test]
            fn actual_sofle_board_filter_keeps_other_board_matrix_unresolvable() {
                let mut session = open_session(fixture(SOFLE));
                let model = session.read_model().clone();
                assert_eq!(model.active_board_id, "left");
                let right = TreeContext::Matrix { matrix_id: "right-keys".into() };
                assert_eq!(resolve_selection(&model, &right), None);
                let tree = expanded_tree(&model, Grouping::Column);
                assert!(!tree.iter().any(|item| item.id == "matrix:right-keys"));
                session.submit(Event::Navigate { operation_id: OperationId(2), board_id: "right".into(), instance_id: None });
                let right_model = session.read_model();
                assert_eq!(right_model.active_board_id, "right");
                assert!(resolve_selection(right_model, &right).is_some(), "Session navigation selects the actual right-board matrix");
            }

            #[test]
            fn modified_actual_fixture_keeps_disabled_slot_context_empty_and_live_id_out_of_selection() {
                let mut doc = fixture(REVIUNG);
                let deleted_matrix_id = doc.matrices.iter().find(|m| m.id == "main-thumbs").unwrap().id.clone();
                let matrix = doc.matrices.iter_mut().find(|m| m.id == "main-right-keys").unwrap();
                let cell = matrix.cells.iter_mut().find(|c| c.enabled).unwrap();
                let (row, column) = (cell.row, cell.column);
                let member_before = matrix.part_ids.iter().find(|id| id.contains(&format!("/r{row}c{column}"))).cloned();
                cell.enabled = false;
                let _ = matrix;
                doc.matrices.retain(|candidate| candidate.id != deleted_matrix_id);
                doc.layouts.retain(|layout| layout.matrix_id != deleted_matrix_id);
                let model = open(doc);
                let context = context_for_cell(&model, "main-right-keys", row, column).expect("Core still projects the configured disabled cell");
                let selection = resolve_selection(&model, &context).expect("disabled key is a valid semantic context");
                assert!(selection.is_empty());
                let tree = expanded_tree(&model, Grouping::Column);
                let key = tree.iter().find(|item| item.id == format!("key:main-right-keys:{row}:{column}")).unwrap();
                assert!(key.primary_id.is_none());
                assert_eq!(key.detail.as_deref(), Some("Empty slot"));
                if let Some(id) = member_before {
                    assert!(!selection.contains(&id), "disabled stale-looking membership cannot be selected");
                }
                // This context names a matrix that existed in the real source fixture but was
                // removed from the modified ProjectDoc before Core produced the accepted scene.
                let absent = TreeContext::Matrix { matrix_id: deleted_matrix_id };
                assert_eq!(resolve_selection(&model, &absent), None);
            }

            #[test]
            fn removed_real_part_id_is_rejected_after_core_accepts_modified_fixture() {
                let mut doc = fixture(REVIUNG);
                let board_id = doc.boards[0].id.clone();
                let matrix_members: HashSet<_> = doc.matrices.iter().flat_map(|m| m.part_ids.iter().cloned()).collect();
                let removed_id = doc.boards.iter().find(|b| b.id == board_id).unwrap().part_ids.iter().find(|id| !matrix_members.contains(*id)).unwrap().clone();
                doc.parts.retain(|part| part.id != removed_id);
                for layout in &mut doc.layouts {
                    layout.part_ids.retain(|id| id != &removed_id);
                }
                let model = open(doc);
                assert!(!model.accepted.as_ref().unwrap().document.parts.iter().any(|part| part.id == removed_id), "Core preserves the removed component as absent in the accepted modified fixture");
                let stale_component = TreeContext::Component {
                    part_id: Some(removed_id.clone()), matrix_id: None, row: None, column: None, assembly_id: None,
                };
                assert_eq!(resolve_selection(&model, &stale_component), None);
                assert_eq!(context_for_part(&model, &removed_id), None);
                assert!(!build_tree(
                    &model.accepted.as_ref().unwrap().document,
                    &model.active_board_id,
                    Grouping::Column,
                    &BTreeSet::from([format!("board:{}", model.active_board_id), format!("layout:{}", model.active_board_id)]),
                    &model.accepted.as_ref().unwrap().scene.matrix_scenes,
                ).iter().any(|item| item.id == format!("component:{removed_id}")));
            }

            #[test]
            fn modified_real_layouts_project_linked_half_and_source_status() {
                let mut doc = fixture(REVIUNG);
                let source_id = doc.layouts.iter().find(|l| l.id == "main-left-keys-layout").unwrap().id.clone();
                let target = doc.layouts.iter_mut().find(|l| l.id == "main-right-keys-layout").unwrap();
                target.mirror_link = Some(LayoutMirrorLink { source_id: source_id.clone(), axis_x: 80.0 });
                let model = open(doc);
                let tree = expanded_tree(&model, Grouping::Row);
                assert!(tree.iter().any(|item| item.label == "Left half" && item.kind == TreeKind::HalfGroup));
                assert!(tree.iter().any(|item| item.label == "Right half" && item.kind == TreeKind::HalfGroup));
                let source = tree.iter().find(|item| item.id == format!("half:{source_id}")).unwrap_or_else(|| panic!("source layout missing: active={} layouts={:?} tree={:?}", model.active_board_id, model.accepted.as_ref().unwrap().document.layouts.iter().map(|l| (&l.id, &l.board_id, &l.matrix_id, &l.mirror_link)).collect::<Vec<_>>(), tree.iter().map(|item| &item.id).collect::<Vec<_>>()));
                let target = tree.iter().find(|item| item.id == "half:main-right-keys-layout").unwrap();
                assert_eq!(source.detail.as_deref(), Some("Linked"));
                assert_eq!(target.detail.as_deref(), Some("Linked"));
                assert!(tree.iter().any(|item| item.kind == TreeKind::Row));
            }
        }
    }
}
