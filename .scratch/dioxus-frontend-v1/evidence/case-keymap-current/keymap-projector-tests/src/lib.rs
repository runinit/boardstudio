#![allow(dead_code)]

mod presentation {
    pub mod keymap {
        mod view {
            include!("/tmp/frontend-run/keymap-projector-tests/view.rs.source-2bdc7946");

            #[cfg(test)]
            mod private_contract_tests {
                use super::{KeyBinding, binding_title};

                #[test]
                fn missing_macro_reference_has_a_safe_display_label() {
                    assert_eq!(
                        binding_title(
                            &KeyBinding::Macro {
                                macro_id: "removed-macro".into(),
                            },
                            &[],
                            &[],
                        ),
                        "Macro"
                    );
                }
            }
        }

        #[cfg(test)]
        mod contract_tests {
            use super::view::{KeymapView, project};
            use boardstudio_application::{
                AcceptedSnapshot, Completion, Effect, Event, ExecutorEpoch, OperationId, RequestId,
                SaveResult, Session,
            };
            use boardstudio_core::{
                CoreEngine,
                model::{
                    CoreRequest, KeyBinding, KeymapConfiguration, KeymapLayer, PartKind, ProjectDoc,
                },
            };
            use std::{collections::BTreeMap, sync::Arc};

            const REVIUNG: &str = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fixtures/reviung41.json"
            ));

            fn fixture() -> ProjectDoc {
                serde_json::from_str(REVIUNG)
                    .expect("real REVIUNG41 project fixture deserializes as ProjectDoc")
            }

            fn core_effect(effects: &[Effect]) -> (RequestId, ExecutorEpoch, CoreRequest) {
                effects
                    .iter()
                    .find_map(|effect| match effect {
                        Effect::Core {
                            request_id,
                            executor_epoch,
                            request,
                            ..
                        } => Some((*request_id, *executor_epoch, (**request).clone())),
                        _ => None,
                    })
                    .expect("Session emits its actual Core request")
            }

            fn accept(document: ProjectDoc) -> (Session, CoreEngine) {
                let mut session = Session::new();
                let mut engine = CoreEngine::new();
                let effects = session.submit(Event::Open {
                    operation_id: OperationId(1),
                    document,
                });
                let (request_id, executor_epoch, request) = core_effect(&effects);
                let reply = engine.handle(request);
                let reply_debug = format!("{reply:?}");
                let effects = session.complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(reply),
                });
                let save_attempt_id = effects
                    .iter()
                    .find_map(|effect| match effect {
                        Effect::Persist { save_attempt_id, .. } => Some(*save_attempt_id),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        panic!("Session did not accept Core open; reply={reply_debug}; effects={effects:?}")
                    });
                session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                });
                assert!(session.read_model().accepted.is_some());
                (session, engine)
            }

            fn snapshot(session: &Session) -> AcceptedSnapshot {
                session
                    .read_model()
                    .accepted
                    .as_ref()
                    .expect("Session accepted the Core scene")
                    .clone()
            }

            fn first_switch(snapshot: &AcceptedSnapshot) -> String {
                let board = snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == "main")
                    .expect("real REVIUNG41 main board");
                snapshot
                    .document
                    .parts
                    .iter()
                    .find(|part| {
                        board.part_ids.contains(&part.id)
                            && snapshot.document.definitions.iter().any(|definition| {
                                definition.id == part.definition_id
                                    && definition.kind == PartKind::Switch
                            })
                    })
                    .expect("real board has a switch")
                    .id
                    .clone()
            }

            fn switch_ids(snapshot: &AcceptedSnapshot) -> Vec<String> {
                let board = snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == "main")
                    .expect("real REVIUNG41 main board");
                snapshot
                    .document
                    .parts
                    .iter()
                    .filter(|part| {
                        board.part_ids.contains(&part.id)
                            && snapshot.document.definitions.iter().any(|definition| {
                                definition.id == part.definition_id
                                    && definition.kind == PartKind::Switch
                            })
                    })
                    .map(|part| part.id.clone())
                    .collect()
            }

            fn keymap_with_layers(layers: Vec<KeymapLayer>) -> KeymapConfiguration {
                KeymapConfiguration {
                    layers,
                    macros: Vec::new(),
                }
            }

            fn layer(id: &str, name: &str, bindings: BTreeMap<String, KeyBinding>) -> KeymapLayer {
                KeymapLayer {
                    id: id.into(),
                    name: name.into(),
                    bindings,
                    sensors: BTreeMap::new(),
                }
            }

            fn key<'a>(view: &'a KeymapView, id: &str) -> &'a super::view::KeymapKey {
                view.keys
                    .iter()
                    .find(|key| key.id.as_ref() == id)
                    .expect("projected key exists")
            }

            #[test]
            fn real_reviung_virtual_base_uses_legacy_binding_and_exact_switch_membership() {
                let mut document = fixture();
                assert!(
                    document.keymap.is_none(),
                    "fixture starts in virtual-base mode"
                );
                let original = {
                    let accepted_probe = {
                        let (session, _) = accept(document.clone());
                        snapshot(&session)
                    };
                    first_switch(&accepted_probe)
                };
                document
                    .hardware
                    .as_mut()
                    .expect("fixture has real hardware data")
                    .boards
                    .iter_mut()
                    .find(|board| board.board_id == "main")
                    .expect("fixture has hardware mapping for main")
                    .key_bindings
                    .insert(original.clone(), "&kp Q".into());

                let (session, _) = accept(document);
                let accepted = snapshot(&session);
                let scope = session.scope().expect("accepted board scope");
                let view = project(&accepted, Some(&scope), "main", "base")
                    .expect("matching accepted scope projects");

                assert_eq!(view.layers.len(), 1);
                assert_eq!(view.layers[0].id.as_ref(), "base");
                assert_eq!(view.layers[0].name.as_ref(), "Base");
                assert_eq!(key(&view, &original).binding_title.as_ref(), "Q");

                let expected = switch_ids(&accepted);
                let actual: Vec<_> = view.keys.iter().map(|key| key.id.to_string()).collect();
                assert_eq!(expected.len(), 41, "fixture's real switch membership");
                assert_eq!(
                    actual, expected,
                    "only board-member switches are keymap keys, in fixture order"
                );
                assert!(!actual.iter().any(|id| id.ends_with("/diode")));
                assert!(!actual.iter().any(|id| id.ends_with("/stabilizer")));
            }

            #[test]
            fn persisted_base_overrides_legacy_and_transparent_differs_from_none() {
                let mut document = fixture();
                let mut probe = document.clone();
                probe.keymap = None;
                let (probe_session, _) = accept(probe);
                let probe_snapshot = snapshot(&probe_session);
                let mut ids = switch_ids(&probe_snapshot).into_iter();
                let first = ids.next().expect("first real switch");
                let second = ids.next().expect("second real switch");
                let absent = ids.next().expect("third real switch");

                document
                    .hardware
                    .as_mut()
                    .expect("fixture hardware")
                    .boards
                    .iter_mut()
                    .find(|board| board.board_id == "main")
                    .expect("main hardware")
                    .key_bindings
                    .insert(first.clone(), "&kp Q".into());
                document.keymap = Some(keymap_with_layers(vec![
                    layer(
                        "base",
                        "Primary",
                        BTreeMap::from([
                            (first.clone(), KeyBinding::None),
                            (second.clone(), KeyBinding::Transparent),
                        ]),
                    ),
                    layer(
                        "nav",
                        "Navigation",
                        BTreeMap::from([
                            (first.clone(), KeyBinding::Transparent),
                            (second.clone(), KeyBinding::None),
                        ]),
                    ),
                ]));

                let (session, _) = accept(document);
                let accepted = snapshot(&session);
                let scope = session.scope().expect("accepted board scope");
                let base = project(&accepted, Some(&scope), "main", "base").unwrap();
                let nav = project(&accepted, Some(&scope), "main", "nav").unwrap();

                assert_eq!(base.layers[0].name.as_ref(), "Primary");
                assert_eq!(base.layers[1].name.as_ref(), "Navigation");
                assert_eq!(key(&base, &first).binding_title.as_ref(), "Unassigned");
                assert_eq!(key(&base, &second).binding_title.as_ref(), "Transparent");
                assert_eq!(key(&nav, &first).binding_title.as_ref(), "Transparent");
                assert_eq!(key(&nav, &second).binding_title.as_ref(), "Unassigned");
                assert_eq!(key(&nav, &absent).binding_title.as_ref(), "Transparent");
            }

            #[test]
            fn accepted_core_scene_transforms_feed_the_keymap_pose() {
                let (session, _) = accept(fixture());
                let accepted = snapshot(&session);
                let scope = session.scope().expect("accepted board scope");
                let view = project(&accepted, Some(&scope), "main", "base").unwrap();
                let accepted_switches = switch_ids(&accepted);
                let transformed = accepted_switches
                    .iter()
                    .find_map(|id| {
                        accepted
                            .scene
                            .transforms
                            .iter()
                            .find(|transform| transform.id == *id)
                            .map(|transform| (id, transform))
                    })
                    .expect("Core accepted scene contains a switch transform");
                let projected = key(&view, transformed.0);
                assert_eq!(projected.pose, transformed.1.pose);
                assert_eq!(
                    projected.pose,
                    accepted
                        .document
                        .parts
                        .iter()
                        .find(|part| part.id == *transformed.0)
                        .unwrap()
                        .pose,
                    "accepted Core scene and accepted ProjectDoc pose agree for this fixture"
                );
            }

            #[test]
            fn stale_active_layer_falls_back_to_first_persisted_layer_without_writes() {
                let mut document = fixture();
                let (probe_session, _) = accept(document.clone());
                let ids = switch_ids(&snapshot(&probe_session));
                let target = ids.first().expect("fixture switch").clone();
                document.keymap = Some(keymap_with_layers(vec![
                    layer(
                        "base",
                        "Primary",
                        BTreeMap::from([(
                            target.clone(),
                            KeyBinding::KeyPress {
                                keycode: "A".into(),
                            },
                        )]),
                    ),
                    layer(
                        "nav",
                        "Navigation",
                        BTreeMap::from([(
                            target.clone(),
                            KeyBinding::KeyPress {
                                keycode: "B".into(),
                            },
                        )]),
                    ),
                ]));

                let (session, _) = accept(document);
                let accepted = snapshot(&session);
                let original_document = Arc::clone(&accepted.document);
                let before = serde_json::to_vec(accepted.document.as_ref()).unwrap();
                let revision = accepted.document.revision;
                let scope = session.scope().expect("accepted board scope");
                let projected = project(&accepted, Some(&scope), "main", "deleted-layer")
                    .expect("stale layer selection falls back to an existing accepted layer");

                assert_eq!(projected.layers[0].name.as_ref(), "Primary");
                assert_eq!(key(&projected, &target).binding_title.as_ref(), "A");
                assert_eq!(accepted.document.revision, revision);
                assert!(Arc::ptr_eq(&original_document, &accepted.document));
                assert_eq!(
                    serde_json::to_vec(accepted.document.as_ref()).unwrap(),
                    before,
                    "projection is read-only for the accepted document"
                );
            }
        }
    }
}
