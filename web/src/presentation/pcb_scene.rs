//! Read-only host scene projection for the currently accepted PCB board.
use crate::presentation::footprint_graphics::FootprintGraphics;
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{PadShape, Side, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PcbPartHit {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) token: SnapshotToken,
    pub(in crate::presentation) generation: u64,
    pub(in crate::presentation) part_id: String,
    pub(in crate::presentation) additive: bool,
    pub(in crate::presentation) range: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PcbPartPointerDown {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) token: SnapshotToken,
    pub(in crate::presentation) generation: u64,
    pub(in crate::presentation) part_id: String,
    pub(in crate::presentation) pointer_id: i64,
    pub(in crate::presentation) client_x: i32,
    pub(in crate::presentation) client_y: i32,
    pub(in crate::presentation) additive: bool,
    pub(in crate::presentation) range: bool,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct PcbSceneProps {
    pub(in crate::presentation) snapshot: AcceptedSnapshot,
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) selected_ids: Vec<String>,
    pub(in crate::presentation) generation: u64,
    pub(in crate::presentation) on_part_hit: EventHandler<PcbPartHit>,
    pub(in crate::presentation) on_part_pointer_down: EventHandler<PcbPartPointerDown>,
    pub(in crate::presentation) on_module_select: EventHandler<String>,
}

#[component]
pub(in crate::presentation) fn PcbScene(props: PcbSceneProps) -> Element {
    let snapshot = props.snapshot;
    let scope = props.scope;
    let selected_ids = props.selected_ids;
    let on_part_hit = props.on_part_hit;
    let on_part_pointer_down = props.on_part_pointer_down;
    let on_module_select = props.on_module_select;
    let generation = props.generation;
    let hidden_layers = (use_context::<super::LayerVisibility>().hidden)();

    if snapshot.session_epoch != scope.session_epoch || snapshot.document.id != scope.document_id {
        return rsx! {
            g { class: "m1-pcb-scene-state", "data-scene-status": "stale", role: "status" }
        };
    }

    let document = &snapshot.document;
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
    else {
        return rsx! {
            g { class: "m1-pcb-scene-state", "data-scene-status": "unavailable", role: "status" }
        };
    };
    let board_id = board.id.as_str();
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == board_id)
        .map(|entry| entry.contours.as_slice())
        .or_else(|| (document.boards.len() == 1).then_some(snapshot.scene.contours.as_slice()))
        .unwrap_or_default();
    let member_ids: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let scene_transforms = snapshot.scene.transforms.as_slice();

    rsx! {
        g { class: "m1-pcb-scene", "data-board-id": board_id, "data-scene-status": "ready",
            if !hidden_layers.contains("Edge.Cuts") {
                for (index, contour) in contours.iter().enumerate() {
                    polygon {
                        key: "outline-{index}",
                        points: points(&contour.points),
                        class: if contour.hole { "m1-outline is-hole" } else { "m1-outline" },
                    }
                }
            }
            for part in document.parts.iter().filter(|part| member_ids.contains(part.id.as_str())) {
                {
                    let definition = document.definitions.iter().find(|definition| definition.id == part.definition_id);
                    let pose = scene_transforms
                        .iter()
                        .find(|transform| transform.id == part.id)
                        .map(|transform| transform.pose)
                        .unwrap_or(part.pose);
                    let side_transform = if matches!(&part.side, Side::Back) { "scale(-1 1)" } else { "" };
                    let selected = selected_ids.iter().any(|id| id == &part.id);
                    let part_id = part.id.clone();
                    let keyboard_id = part_id.clone();
                    let click_scope = scope.clone();
                    let pointer_scope = scope.clone();
                    let keyboard_scope = scope.clone();
                    let token = snapshot.token;
                    let click_hit = on_part_hit;
                    let pointer_down = on_part_pointer_down;
                    let keyboard_hit = on_part_hit;
                    let click_generation = generation;
                    let keyboard_generation = generation;
                    let reference = part.reference.clone();
                    let click_id = part_id.clone();
                    let definition_name = definition.map(|definition| definition.name.as_str()).unwrap_or("Part definition unavailable");
                    let label = format!("{reference}, {definition_name}, X {:.2} Y {:.2}", pose.at.x, pose.at.y);
                    let courtyard_points = definition.map(|definition| definition.courtyard.as_slice()).unwrap_or_default();
                    let courtyard = points(courtyard_points);
                    let hit_bounds = courtyard_bounds(courtyard_points);
                    let generator_parameters = part.generator_parameters.clone();
                    rsx! {
                        g {
                            key: "part-{part_id}",
                            class: if selected { "m1-scene-part m1-pcb-part is-selected" } else { "m1-scene-part m1-pcb-part" },
                            style: "cursor: pointer",
                            transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation}) {side_transform}",
                            role: "button",
                            tabindex: "0",
                            "aria-pressed": "{selected}",
                            "aria-label": "{label}",
                            "data-part-id": "{part.id}",
                            onpointerdown: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                if pointer.button() != 0 { return; }
                                // Stop Dioxus bubbling as well as the native event below.
                                // A deselected part starts no drag to guard the empty-space path.
                                event.stop_propagation();
                                let modifiers = event.data().modifiers();
                                pointer_down.call(PcbPartPointerDown {
                                    scope: pointer_scope.clone(),
                                    token,
                                    generation,
                                    part_id: part_id.clone(),
                                    pointer_id: i64::from(pointer.pointer_id()),
                                    client_x: pointer.client_x(),
                                    client_y: pointer.client_y(),
                                    additive: modifiers.ctrl() || modifiers.meta(),
                                    range: modifiers.shift(),
                                });
                                pointer.prevent_default();
                                pointer.stop_propagation();
                            },
                            onclick: move |event: MouseEvent| {
                                event.stop_propagation();
                                // Pointer selection is already applied on pointerdown.
                                // Capture can retarget this compatibility click to the SVG.
                                // Keep click-only accessibility activation, without toggling twice.
                                if event.data().try_as_web_event().is_some_and(|event| event.detail() != 0) {
                                    return;
                                }
                                let modifiers = event.data().modifiers();
                                click_hit.call(PcbPartHit {
                                    scope: click_scope.clone(),
                                    token,
                                    generation: click_generation,
                                    part_id: click_id.clone(),
                                    additive: modifiers.ctrl() || modifiers.meta(),
                                    range: modifiers.shift(),
                                });
                            },
                            onkeydown: move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                if key != "Enter" && key != " " { return; }
                                event.prevent_default();
                                event.stop_propagation();
                                let modifiers = event.data().modifiers();
                                keyboard_hit.call(PcbPartHit {
                                    scope: keyboard_scope.clone(),
                                    token,
                                    generation: keyboard_generation,
                                    part_id: keyboard_id.clone(),
                                    additive: modifiers.ctrl() || modifiers.meta(),
                                    range: modifiers.shift(),
                                });
                            },
                            if !courtyard.is_empty() && !hidden_layers.contains("Courtyards") {
                                polygon { points: "{courtyard}", class: if selected { "m1-part selected" } else { "m1-part" } }
                            }
                            if let Some(definition) = definition {
                                if definition.generator.is_some() {
                                    FootprintGraphics {
                                        definition: definition.clone(),
                                        parameters: generator_parameters.clone(),
                                        hidden_layers: Some(hidden_layers.clone()),
                                        part_side: Some(part.side.clone()),
                                    }
                                }
                                for pad in &definition.pads {
                                    {
                                        let radius = match &pad.shape {
                                            PadShape::Circle | PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
                                            PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
                                            PadShape::Rect => 0.0,
                                        };
                                        let drill = pad.drill;
                                        let pad_rotation = pad.rotation.unwrap_or(0.0);
                                        rsx! {
                                            g {
                                                key: "pad-{part_id}-{pad.id}",
                                                transform: "translate({pad.at.x} {pad.at.y}) rotate({pad_rotation})",
                                                if super::pcb_layers::pad_is_visible(pad, &part.side, &hidden_layers) {
                                                    rect {
                                                        class: "m1-part-pad",
                                                        x: "{-pad.size.x / 2.0}",
                                                        y: "{-pad.size.y / 2.0}",
                                                        width: "{pad.size.x}",
                                                        height: "{pad.size.y}",
                                                        rx: "{radius}",
                                                    }
                                                }
                                                if let Some(drill) = drill.filter(|_| !hidden_layers.contains("Holes")) {
                                                    circle { class: "m1-part-drill", r: "{drill / 2.0}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !hidden_layers.contains("References") {
                                text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", x: "0", y: "-5.2", "{reference}" }
                            }
                            rect {
                                class: "m1-part-hit-area",
                                style: "cursor: pointer",
                                x: "{hit_bounds.0}",
                                y: "{hit_bounds.1}",
                                width: "{hit_bounds.2}",
                                height: "{hit_bounds.3}",
                            }
                        }
                    }
                }
            }
            for module in snapshot.scene.module_scenes.iter().filter(|module| {
                document.modules.iter().any(|instance| instance.id == module.id && instance.host_board_id == scope.board_id)
            }) {
                super::pcb_module_footprints::ModuleSourceFootprints {
                    key: "{module.id}",
                    module_id: module.id.clone(),
                    snapshot: snapshot.clone(),
                    on_select: on_module_select,
                }
            }
            super::pcb_module_footprints::ModuleFindingMarkers {
                snapshot: snapshot.clone(), board_id: scope.board_id.clone(),
            }
        }
    }
}

fn points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

fn courtyard_bounds(points: &[Vec2]) -> (f64, f64, f64, f64) {
    let Some(first) = points.first() else {
        // Match React’s missing-definition hit affordance without drawing a
        // fabricated courtyard into the accepted scene.
        return (-4.0, -4.0, 8.0, 8.0);
    };
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_layer_tests {
    use super::*;
    use crate::presentation::canvas_layers::CanvasLayer;
    use crate::presentation::pcb_layers::PcbLayerControls;
    use boardstudio_application::{Scope, SessionEpoch};
    use boardstudio_core::model::{
        Board, BoardContours, Contour, Pad, Part, PartDefinition, PartKind, Pose2, ProjectDoc,
        Readiness, SceneDelta,
    };
    use std::{collections::BTreeMap, sync::Arc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn fixture() -> (AcceptedSnapshot, Scope) {
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "layer-doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let mut document = ProjectDoc::empty("layer-doc", "Layer controls");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["part".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let pad = |id: &str, side: Option<Side>, drill| Pad {
            id: id.into(),
            number: id.into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Rect,
            drill,
            plated: Some(true),
            side,
            rotation: None,
            net_id: None,
        };
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "definition".into(),
            name: "Test footprint".into(),
            kind: PartKind::Custom,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![
                Vec2 { x: -2.0, y: -2.0 },
                Vec2 { x: 2.0, y: -2.0 },
                Vec2 { x: 2.0, y: 2.0 },
                Vec2 { x: -2.0, y: 2.0 },
            ],
            pads: vec![
                pad("front", Some(Side::Front), None),
                pad("back", Some(Side::Back), None),
                pad("through", Some(Side::Front), Some(0.8)),
            ],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "part".into(),
            definition_id: "definition".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        let contour = Contour {
            points: vec![
                Vec2 { x: -10.0, y: -10.0 },
                Vec2 { x: 10.0, y: -10.0 },
                Vec2 { x: 10.0, y: 10.0 },
                Vec2 { x: -10.0, y: 10.0 },
            ],
            hole: false,
        };
        let scene = SceneDelta {
            module_scenes: vec![],
            revision: 1,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board".into(),
                contours: vec![contour],
            }],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        };
        (
            AcceptedSnapshot {
                token: SnapshotToken(11),
                session_epoch: scope.session_epoch,
                document: Arc::new(document),
                scene: Arc::new(scene),
            },
            scope,
        )
    }

    fn mounted_scene() -> Element {
        let (snapshot, scope) = fixture();
        let hidden = use_signal(BTreeSet::new);
        let footprints = use_signal(|| true);
        let modules_hidden = use_signal(super::super::pcb_module_footprints::default_hidden_layers);
        use_context_provider(|| super::super::LayerVisibility {
            hidden,
            modules_hidden,
            footprints,
        });
        rsx! {
            div { id: "pcb-layer-mount",
                PcbLayerControls {
                    snapshot: snapshot.clone(),
                    scope: scope.clone(),
                }
                svg { PcbScene {
                    snapshot,
                    scope,
                    selected_ids: Vec::new(),
                    generation: 0,
                    on_part_hit: |_| {},
                    on_part_pointer_down: |_| {},
                    on_module_select: |_| {},
                } }
            }
        }
    }

    fn mounted_board_switch() -> Element {
        let (mut snapshot, scope) = fixture();
        let mut document = (*snapshot.document).clone();
        document.boards.push(Board {
            id: "empty-board".into(),
            name: "Empty board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        snapshot.document = Arc::new(document);
        let empty_scope = Scope {
            board_id: "empty-board".into(),
            ..scope.clone()
        };
        let mut empty_selected = use_signal(|| false);
        let current_scope = if empty_selected() { empty_scope } else { scope };
        let hidden = use_signal(BTreeSet::new);
        let footprints = use_signal(|| true);
        let modules_hidden = use_signal(super::super::pcb_module_footprints::default_hidden_layers);
        use_context_provider(|| super::super::LayerVisibility {
            hidden,
            modules_hidden,
            footprints,
        });
        rsx! {
            div { id: "pcb-layer-mount",
                button { id: "switch-pcb-board", onclick: move |_| empty_selected.set(!empty_selected()), "Switch board" }
                PcbLayerControls {
                    snapshot,
                    scope: current_scope,
                }
            }
        }
    }

    fn count(selector: &str) -> u32 {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector_all(&format!("#pcb-layer-mount {selector}"))
            .unwrap()
            .length()
    }

    fn click(selector: &str) {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#pcb-layer-mount {selector}"))
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }

    #[wasm_bindgen_test]
    async fn mounted_controls_toggle_only_their_accepted_pcb_scene_layers() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("pcb-layer-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(mounted_scene),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        assert_eq!(count(".m1-outline"), 1);
        assert_eq!(count(".m1-part"), 1);
        assert_eq!(count(".m1-part-pad"), 3);
        assert_eq!(count(".m1-part-drill"), 1);
        assert_eq!(count(".m1-part-label"), 1);

        click("#m1-pcb-layers-trigger");
        settle().await;
        assert_eq!(count("button[aria-label='Hide F.Cu']"), 1);
        assert_eq!(count("button[aria-label='Hide B.Cu']"), 1);
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("#pcb-layer-mount button[aria-label='Hide F.Cu'] .m1-layer-label")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Front copper")
        );
        click("button[aria-label='Hide F.Cu']");
        settle().await;
        assert_eq!(
            count(".m1-part-pad"),
            2,
            "back and through-hole pads remain"
        );
        click("button[aria-label='Hide B.Cu']");
        settle().await;
        assert_eq!(
            count(".m1-part-pad"),
            0,
            "through-hole needs either copper face"
        );

        click("button[aria-label='Show F.Cu']");
        settle().await;
        assert_eq!(count(".m1-part-pad"), 2);
        click("button[aria-label='Hide Pads']");
        settle().await;
        assert_eq!(count(".m1-part-pad"), 0);
        assert_eq!(count(".m1-part-drill"), 1, "Pads and Holes are independent");
        click("button[aria-label='Hide Holes']");
        settle().await;
        assert_eq!(count(".m1-part-drill"), 0);
        assert_eq!(count(".m1-part-label"), 1);

        click("button[aria-label='Hide Edge.Cuts']");
        click("button[aria-label='Hide Courtyards']");
        click("button[aria-label='Hide References']");
        settle().await;
        assert_eq!(count(".m1-outline"), 0);
        assert_eq!(count(".m1-part"), 0);
        assert_eq!(count(".m1-part-label"), 0);

        let row = document
            .query_selector("#pcb-layer-mount button[aria-label='Show References']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        row.focus().unwrap();
        let event_init = web_sys::KeyboardEventInit::new();
        event_init.set_key("Escape");
        event_init.set_bubbles(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event_init)
                .unwrap();
        row.dispatch_event(&event).unwrap();
        settle().await;
        assert!(count("#m1-pcb-layers-list[hidden]") > 0);
        assert_eq!(
            document.active_element().unwrap().id(),
            "m1-pcb-layers-trigger"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn production_layer_component_recomputes_rows_when_board_scope_changes() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("pcb-layer-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(mounted_board_switch),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        assert_eq!(count("button[aria-label='Hide F.Cu']"), 1);
        click("#switch-pcb-board");
        settle().await;
        assert_eq!(count("button[aria-label='Hide F.Cu']"), 0);
        click("#switch-pcb-board");
        settle().await;
        assert_eq!(count("button[aria-label='Hide F.Cu']"), 1);
        root.remove();
    }

    #[wasm_bindgen_test]
    fn pcb_rows_do_not_offer_the_layout_only_footprints_toggle() {
        let (snapshot, scope) = fixture();
        let groups = crate::presentation::pcb_layers::layer_groups_for_scene(
            &snapshot.document,
            &snapshot.scene.board_contours,
            &snapshot.scene.contours,
            &scope,
            BTreeSet::new(),
        );
        assert_eq!(
            groups
                .iter()
                .map(|group| group.title.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
            ["Copper", "Objects", "Mounted modules"]
        );
        assert!(groups.iter().flat_map(|group| &group.layers).all(|layer| {
            !matches!(
                layer.target,
                crate::presentation::canvas_layers::LayerTarget::LayoutFootprints
            ) && layer.id != "Footprints"
        }));
        assert!(!groups.iter().flat_map(|group| &group.layers).any(|layer| {
            layer.id == "Board" || layer.id == "Components" || layer.id == "Keys"
        }));
        assert!(
            groups
                .iter()
                .flat_map(|group| &group.layers)
                .any(|layer| { layer == &CanvasLayer::hidden("F.Cu", "Front copper") })
        );

        let generated_layers = ["F.SilkS".to_owned(), "B.Mask".to_owned()]
            .into_iter()
            .collect();
        let groups = crate::presentation::pcb_layers::layer_groups_for_scene(
            &snapshot.document,
            &snapshot.scene.board_contours,
            &snapshot.scene.contours,
            &scope,
            generated_layers,
        );
        assert_eq!(
            groups
                .iter()
                .map(|group| group.title.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
            ["Copper", "Technical", "Objects", "Mounted modules"]
        );
        let rows = groups
            .iter()
            .flat_map(|group| &group.layers)
            .collect::<Vec<_>>();
        assert_eq!(
            groups
                .iter()
                .find(|group| group.title.as_deref() == Some("Objects"))
                .unwrap()
                .layers
                .iter()
                .map(|layer| layer.label.as_str())
                .collect::<Vec<_>>(),
            ["Board outline", "Courtyards", "Pads", "Holes", "References"]
        );
        assert!(rows.iter().any(|layer| layer.id == "F.SilkS"));
        assert!(rows.iter().any(|layer| layer.id == "B.Mask"));
        assert_eq!(
            crate::presentation::footprint_graphics::resolve_board_layer("F.SilkS", &Side::Back),
            "B.SilkS"
        );

        let generated_edge = crate::presentation::pcb_layers::layer_groups_for_scene(
            &snapshot.document,
            &[],
            &[],
            &scope,
            BTreeSet::from(["Edge.Cuts".into()]),
        );
        let edge_rows = generated_edge
            .iter()
            .flat_map(|group| &group.layers)
            .filter(|layer| layer.id == "Edge.Cuts")
            .collect::<Vec<_>>();
        assert_eq!(edge_rows.len(), 1);
        assert_eq!(edge_rows[0].label, "Board outline");
    }
}
