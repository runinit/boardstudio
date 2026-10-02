//! Presentation drafts and DOM input are separate from the durable session state.
use crate::{cad_presentation::CasePanel, runtime::Runtime};
use boardstudio_application::{Durability, Event, SelectionMode};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, Position, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlElement, HtmlInputElement, SvgElement};

#[derive(Clone)]
struct Drag {
    pointer: i64,
    origin: Vec2,
    client_x: f64,
    client_y: f64,
    positions: Vec<Position>,
    active: bool,
    pan: bool,
    camera: Vec2,
}

#[derive(Clone)]
struct NumericEdit {
    id: String,
    revision: u64,
    transaction_id: String,
    start: Vec2,
}
type KeyboardHandler = Rc<RefCell<Box<dyn FnMut(KeyboardEvent)>>>;

struct PointerLocation {
    world: Vec2,
    x_fraction: f64,
    y_fraction: f64,
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let runtime = use_hook(Runtime::new);
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => return rsx! { main { role: "alert", "Browser startup failed: {error}" } },
    };
    let version = use_signal(|| 0u64);
    let active = use_hook(|| Rc::new(Cell::new(true)));
    use_hook({
        let runtime = runtime.clone();
        let active = active.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                if active.get() {
                    let mut signal = version;
                    signal += 1;
                }
            }))
        }
    });
    use_drop({
        let runtime = runtime.clone();
        move || {
            active.set(false);
            runtime.unsubscribe();
            runtime.submit(Event::Close {
                operation_id: runtime.operation(),
            });
        }
    });
    let _ = version();
    use_context_provider(|| runtime.clone());
    use_context_provider(|| version);
    rsx! {
        link { rel: "stylesheet", href: "assets/m1.css" }
        main { class: "m1-workbench",
            header { class: "m1-toolbar", h1 { "BoardStudio" } span { "M1 migration candidate" } }
            Library {}
            if runtime.model().accepted.is_some() { Editor {} }
            p { role: "status", "aria-live": "polite", class: "m1-status", "{runtime.status()}" }
        }
    }
}

#[component]
fn Library() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let recovery_required =
        runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired;
    let mut saved = use_signal(Vec::<(String, String)>::new);
    let accepted_identity = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| (snapshot.document.id.clone(), snapshot.document.name.clone()));
    let list_runtime = runtime.clone();
    use_effect(use_reactive!(|accepted_identity| {
        let _ = accepted_identity;
        let runtime = list_runtime.clone();
        spawn_local(async move {
            match runtime.store.list_documents().await {
                Ok(documents) => saved.set(documents.into_iter().map(|d| (d.id, d.name)).collect()),
                Err(error) => runtime.report(error.to_string()),
            }
        });
    }));
    let reviung = runtime.clone();
    let sofle = runtime.clone();
    let import = runtime.clone();
    rsx! {
        section { class: "m1-library", "aria-label": "Keyboard library",
            h2 { "Open a keyboard" }
            button { onclick: move |_| reviung.open_fixture("reviung41"), "REVIUNG41 copy" }
            button { onclick: move |_| sofle.open_fixture("sofle"), "Sofle v2 copy" }
            label { "Import .boardstudio"
                input { r#type: "file", accept: ".boardstudio", onchange: move |event: FormEvent| {
                    let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                    let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                    import.import_file(file);
                    input.set_value("");
                }}
            }
            for (id, name) in saved() {
                button { key: "{id}", onclick: { let runtime = runtime.clone(); move |_| runtime.open_saved(id.clone()) }, if recovery_required { "Recover from {name} (discard pending changes)" } else { "{name}" } }
            }
        }
    }
}

#[component]
fn Editor() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = snapshot.document.clone();
    let scene = model
        .display_preview
        .clone()
        .unwrap_or_else(|| snapshot.scene.clone());
    let board = document
        .boards
        .iter()
        .find(|b| b.id == model.active_board_id);
    let physical_instances: Vec<_> = document
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .filter(|instance| instance.board_id == model.active_board_id)
                .collect()
        })
        .unwrap_or_default();
    let active_instance_value = model.active_instance_id.clone().unwrap_or_default();
    let visible: Vec<_> = document
        .parts
        .iter()
        .filter(|p| board.is_some_and(|b| b.part_ids.contains(&p.id)))
        .cloned()
        .collect();
    let visible_ids: Rc<Vec<String>> =
        Rc::new(visible.iter().map(|part| part.id.clone()).collect());
    let points: Vec<_> = visible.iter().map(|p| p.pose.at).collect();
    let min_x = points.iter().map(|p| p.x).reduce(f64::min).unwrap_or(-50.0) - 20.0;
    let max_x = points.iter().map(|p| p.x).reduce(f64::max).unwrap_or(50.0) + 20.0;
    let min_y = points.iter().map(|p| p.y).reduce(f64::min).unwrap_or(-50.0) - 20.0;
    let max_y = points.iter().map(|p| p.y).reduce(f64::max).unwrap_or(50.0) + 20.0;
    let width = (max_x - min_x).max(50.0) / model.camera.zoom;
    let height = (max_y - min_y).max(50.0) / model.camera.zoom;
    let view_x = (min_x + max_x - width) * 0.5 + model.camera.center.x;
    let view_y = -(min_y + max_y + height) * 0.5 - model.camera.center.y;
    let view_box = format!("{view_x} {view_y} {width} {height}");
    let svg = use_hook(|| Rc::new(RefCell::new(None::<SvgElement>)));
    let drag = use_hook(|| Rc::new(RefCell::new(None::<Drag>)));
    let space_down = use_hook(|| Rc::new(Cell::new(false)));
    let mount = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        move |event: MountedEvent| {
            if let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|e| e.dyn_into::<SvgElement>().ok())
            {
                runtime.surface(element.clone());
                *svg.borrow_mut() = Some(element);
            }
        }
    };
    let move_pointer = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            let Some(mut current) = drag
                .borrow()
                .clone()
                .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
            else {
                return;
            };
            let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) else {
                return;
            };
            if !current.active {
                let dx = f64::from(pointer.client_x()) - current.client_x;
                let dy = f64::from(pointer.client_y()) - current.client_y;
                if dx * dx + dy * dy < 16.0 {
                    return;
                }
                let operation = runtime.operation();
                runtime.submit(Event::GestureBegin {
                    operation_id: operation,
                    pointer_id: current.pointer,
                    target_ids: current.positions.iter().map(|p| p.id.clone()).collect(),
                    transaction_id: format!("drag-{}", operation.0),
                    start: current.positions.clone(),
                    pitch: Vec2 { x: 19.05, y: 19.05 },
                    snap_fraction: 0.25,
                    geometry_snap: true,
                    gap: None,
                    alt: pointer.alt_key(),
                });
                if runtime.model().gesture.is_none() {
                    drag.borrow_mut().take();
                    return;
                }
                current.active = true;
                *drag.borrow_mut() = Some(current.clone());
            }
            runtime.submit(Event::GestureSample {
                pointer_id: current.pointer,
                positions: moved(&current, point),
                alt: pointer.alt_key(),
            });
        }
    };
    let end_pointer = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            let Some(current) = drag
                .borrow()
                .clone()
                .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
            else {
                return;
            };
            if current.pan {
                let surface = svg.borrow();
                let Some(element) = surface.as_ref() else {
                    return;
                };
                let rect = element.get_bounding_client_rect();
                if rect.width() > 0.0 && rect.height() > 0.0 {
                    let scale = (rect.width() / width).min(rect.height() / height);
                    if scale <= 0.0 {
                        return;
                    }
                    let camera = runtime.model().camera;
                    runtime.submit(Event::SetCamera {
                        operation_id: runtime.operation(),
                        center: Vec2 {
                            x: current.camera.x
                                - (f64::from(pointer.client_x()) - current.client_x) / scale,
                            y: current.camera.y
                                + (f64::from(pointer.client_y()) - current.client_y) / scale,
                        },
                        zoom: camera.zoom,
                    });
                }
                return;
            }
            if current.pan {
                drag.borrow_mut().take();
            } else if current.active
                && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
            {
                runtime.submit(Event::GestureEnd {
                    pointer_id: current.pointer,
                    final_positions: moved(&current, point),
                    alt: pointer.alt_key(),
                });
            } else if current.active {
                runtime.submit(Event::GestureCancel {
                    pointer_id: current.pointer,
                });
            }
            drag.borrow_mut().take();
        }
    };
    let cancel_pointer = {
        let runtime = runtime.clone();
        let drag = drag.clone();
        move |_| {
            if let Some(current) = drag.borrow_mut().take()
                && current.active
                && !current.pan
            {
                runtime.submit(Event::GestureCancel {
                    pointer_id: current.pointer,
                });
            }
        }
    };
    let undo = runtime.clone();
    let redo = runtime.clone();
    let retry = runtime.clone();
    let recover = runtime.clone();
    let export = runtime.clone();
    let navigate = runtime.clone();
    let navigate_instance = runtime.clone();
    let instance_board_id = model.active_board_id.clone();
    let keyboard = {
        let runtime = runtime.clone();
        let drag = drag.clone();
        let space_down = space_down.clone();
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            let modifiers = event.data().modifiers();
            if key == " " || code == "Space" {
                space_down.set(true);
                event.prevent_default();
            } else if key == "Escape" {
                if let Some(current) = drag.borrow_mut().take()
                    && current.active
                    && !current.pan
                {
                    runtime.submit(Event::GestureCancel {
                        pointer_id: current.pointer,
                    });
                }
            } else if (modifiers.ctrl() || modifiers.meta()) && key.eq_ignore_ascii_case("z") {
                event.prevent_default();
                runtime.submit(if modifiers.shift() {
                    Event::Redo {
                        operation_id: runtime.operation(),
                    }
                } else {
                    Event::Undo {
                        operation_id: runtime.operation(),
                    }
                });
            }
        }
    };
    let key_up = {
        let space_down = space_down.clone();
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            if key == " " || code == "Space" {
                space_down.set(false);
            }
        }
    };
    let start_pan = {
        let drag = drag.clone();
        let runtime = runtime.clone();
        let svg = svg.clone();
        let space_down = space_down.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if !space_down.get() || pointer.button() != 0 {
                return;
            }
            pointer.prevent_default();
            pointer.stop_propagation();
            if let Some(surface) = svg.borrow().as_ref() {
                let _ = surface.set_pointer_capture(pointer.pointer_id());
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = surface.focus_with_options(&options);
            }
            *drag.borrow_mut() = Some(Drag {
                pointer: i64::from(pointer.pointer_id()),
                origin: Vec2::default(),
                client_x: f64::from(pointer.client_x()),
                client_y: f64::from(pointer.client_y()),
                positions: vec![],
                active: true,
                pan: true,
                camera: runtime.model().camera.center,
            });
        }
    };
    let wheel = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        move |event: WheelEvent| {
            let Some(wheel) = event.data().try_as_web_event() else {
                return;
            };
            wheel.prevent_default();
            let surface = svg.borrow();
            let Some(element) = surface.as_ref() else {
                return;
            };
            let rect = element.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return;
            }
            let old = runtime.model().camera;
            let base_width = (max_x - min_x).max(50.0);
            let base_height = (max_y - min_y).max(50.0);
            let Some(location) = pointer_location(
                &rect,
                wheel.client_x(),
                wheel.client_y(),
                view_x,
                view_y,
                width,
                height,
            ) else {
                return;
            };
            let world_x = location.world.x;
            let world_y = location.world.y;
            let zoom = (old.zoom * (-wheel.delta_y() * 0.001).exp()).clamp(0.15, 8.0);
            let center = Vec2 {
                x: world_x
                    - (min_x + max_x - base_width / zoom) * 0.5
                    - location.x_fraction * base_width / zoom,
                y: world_y - (min_y + max_y + base_height / zoom) * 0.5
                    + location.y_fraction * base_height / zoom,
            };
            runtime.submit(Event::SetCamera {
                operation_id: runtime.operation(),
                center,
                zoom,
            });
        }
    };
    rsx! {
        section { class: "m1-editor", "aria-label": "Keyboard editor",
            nav { class: "m1-toolbar",
                strong { "{document.name}" }
                label { "Board"
                    select { value: "{model.active_board_id}", onchange: move |event| navigate.submit(Event::Navigate { operation_id: navigate.operation(), board_id: event.value(), instance_id: None }),
                        for board in &document.boards { option { value: "{board.id}", "{board.name}" } }
                    }
                }
                if !physical_instances.is_empty() {
                    label { "Physical instance"
                        select { "aria-label": "Physical instance", value: "{active_instance_value}", onchange: move |event: FormEvent| {
                            let value = event.value();
                            navigate_instance.submit(Event::Navigate {
                                operation_id: navigate_instance.operation(),
                                board_id: instance_board_id.clone(),
                                instance_id: (!value.is_empty()).then_some(value),
                            });
                        },
                            option { value: "", "Canonical board" }
                            for instance in &physical_instances {
                                option { key: "{instance.id}", value: "{instance.id}", "{instance.name}" }
                            }
                        }
                    }
                }
                button { onclick: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), "Undo" }
                button { onclick: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), "Redo" }
                button { onclick: move |_| retry.submit(Event::RetrySave { operation_id: retry.operation() }), disabled: !matches!(model.durability, Durability::Failed {..}), "Retry save" }
                if model.lifecycle == boardstudio_application::Lifecycle::RecoveryRequired {
                    button { onclick: move |_| recover.recover_saved(), "Reopen last saved version (discard pending changes)" }
                }
                button { onclick: move |_| { if let Some(scope) = export.scope() { export.submit(Event::StartExport { operation_id: export.operation(), scope }); } }, "Export archive" }
                span { "Revision {document.revision} · {model.durability:?}" }
            }
            div { class: "m1-editor-body",
                svg { class: "m1-canvas", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls", onmounted: mount,
                    onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer.clone(), onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
                    g { transform: "scale(1,-1)",
                        for contour in scene.board_contours.iter().filter(|b| b.board_id == model.active_board_id).flat_map(|b| &b.contours) {
                            polygon { points: polygon_points(&contour.points), class: "m1-outline" }
                        }
                        for part in visible.iter().cloned() {
                            {
                                let pose = scene.transforms.iter().find(|t| t.id == part.id).map(|t| t.pose).unwrap_or(part.pose);
                                let courtyard = document.definitions.iter().find(|d| d.id == part.definition_id).map(|d| polygon_points(&d.courtyard)).unwrap_or_default();
                                let selected = model.selected_part_ids.contains(&part.id);
                                let id = part.id.clone();
                                let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone(); let space_down = space_down.clone();
                                let range_ids = visible_ids.clone();
                                rsx! { g { key: "{part.id}", transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation})", "data-part-id": "{part.id}",
                                    onpointerdown: move |event: PointerEvent| {
                                        let Some(pointer) = event.data().try_as_web_event() else { return; };
                                        if pointer.button() != 0 { return; }
                                        pointer.prevent_default(); pointer.stop_propagation();
                                        if let Some(svg) = svg.borrow().as_ref() { let _ = svg.set_pointer_capture(pointer.pointer_id()); let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true); let _ = svg.focus_with_options(&options); }
                                        if space_down.get() {
                                            *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), origin: Vec2::default(), client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions: vec![], active: true, pan: true, camera: runtime.model().camera.center });
                                            return;
                                        }
                                        let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
                                        let mode = if pointer.shift_key() { SelectionMode::Range } else if pointer.ctrl_key() || pointer.meta_key() { SelectionMode::Toggle } else { SelectionMode::Replace };
                                        let current = runtime.model();
                                        if !current.selected_part_ids.contains(&id) || mode != SelectionMode::Replace {
                                            let range_part_ids = if mode == SelectionMode::Range { range_ids.as_ref().clone() } else { vec![] };
                                            runtime.submit(Event::SelectParts { operation_id: runtime.operation(), part_ids: vec![id.clone()], range_part_ids, mode });
                                        }
                                        let current = runtime.model();
                                        if !current.selected_part_ids.contains(&id) { return; }
                                        let Some(snapshot) = current.accepted else { return; };
                                        let positions: Vec<_> = snapshot.document.parts.iter().filter(|p| current.selected_part_ids.contains(&p.id)).map(|p| Position { id: p.id.clone(), at: p.pose.at }).collect();
                                        *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), origin, client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions, active: false, pan: false, camera: Vec2::default() });
                                    },
                                    polygon { points: "{courtyard}", class: if selected { "m1-part selected" } else { "m1-part" } }
                                    text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", "{part.reference}" }
                                }}
                            }
                        }
                    }
                }
                Inspector {}
            }
            CasePanel {}
        }
    }
}

#[component]
fn Inspector() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let selected = model
        .accepted
        .as_ref()
        .and_then(|s| {
            s.document
                .parts
                .iter()
                .find(|p| model.selected_part_ids.contains(&p.id))
        })
        .cloned();
    let component_items: Rc<Vec<_>> = Rc::new(
        model
            .accepted
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .document
                    .parts
                    .iter()
                    .filter(|part| {
                        snapshot
                            .document
                            .boards
                            .iter()
                            .find(|board| board.id == model.active_board_id)
                            .is_some_and(|board| board.part_ids.contains(&part.id))
                    })
                    .enumerate()
                    .map(|(index, part)| {
                        let kind = snapshot
                            .document
                            .definitions
                            .iter()
                            .find(|definition| definition.id == part.definition_id)
                            .map(|definition| format!("{:?}", definition.kind))
                            .unwrap_or_else(|| "component".into());
                        (
                            index,
                            part.id.clone(),
                            part.reference.clone(),
                            kind,
                            model.selected_part_ids.contains(&part.id),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
    );
    let has_selected_component = component_items.iter().any(|item| item.4);
    let mut x = use_signal(String::new);
    let mut y = use_signal(String::new);
    let numeric_edit = use_hook(|| Rc::new(RefCell::new(None::<NumericEdit>)));
    let key = selected
        .as_ref()
        .map(|p| (p.id.clone(), p.pose.at.x, p.pose.at.y));
    use_effect(use_reactive((&key,), {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |(key,)| {
            if let Some((_, px, py)) = key {
                x.set(px.to_string());
                y.set(py.to_string());
            }
            let changed_target = numeric_edit
                .borrow()
                .as_ref()
                .is_some_and(|edit| key.as_ref().is_none_or(|(id, _, _)| id != &edit.id));
            if changed_target && let Some(edit) = numeric_edit.borrow_mut().take() {
                let start = edit.start;
                submit_position(
                    &runtime,
                    &Rc::new(RefCell::new(None)),
                    edit,
                    start,
                    EditPhase::Preview,
                );
            }
        }
    }));
    let submit = {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |_| {
            commit_numeric(&runtime, &numeric_edit, &x(), &y());
        }
    };
    let cancel_numeric: KeyboardHandler = {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        let mut x = x;
        let mut y = y;
        Rc::new(RefCell::new(Box::new(move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            if key == "Escape" {
                event.prevent_default();
                let edit = numeric_edit.borrow_mut().take();
                if let Some(edit) = edit {
                    let start = edit.start;
                    submit_position(&runtime, &numeric_edit, edit, start, EditPhase::Preview);
                }
                if let Some(part) = runtime.model().accepted.and_then(|s| {
                    s.document
                        .parts
                        .iter()
                        .find(|p| runtime.model().selected_part_ids.contains(&p.id))
                        .cloned()
                }) {
                    let restored_x = part.pose.at.x.to_string();
                    let restored_y = part.pose.at.y.to_string();
                    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                        if let Some(input) = document
                            .get_element_by_id("m1-position-x")
                            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                        {
                            input.set_value(&restored_x);
                        }
                        if let Some(input) = document
                            .get_element_by_id("m1-position-y")
                            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                        {
                            input.set_value(&restored_y);
                        }
                    }
                    x.set(restored_x);
                    y.set(restored_y);
                }
                runtime.report("Position preview canceled.");
            } else if key == "Enter" {
                event.prevent_default();
                commit_numeric(&runtime, &numeric_edit, &x(), &y());
            }
        })))
    };
    let select_component: Rc<dyn Fn(String)> = Rc::new({
        let runtime = runtime.clone();
        move |id: String| {
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: vec![id],
                range_part_ids: vec![],
                mode: SelectionMode::Replace,
            })
        }
    });
    rsx! { aside { class: "m1-inspector", "aria-label": "Component inspector",
        details { class: "m1-component-picker", open: true,
            summary { "Components ({component_items.len()})" }
            div { role: "listbox", "aria-label": "Components on current board", class: "m1-component-list",
                for (index, id, reference, kind, is_selected) in component_items.iter().cloned() {
                    {
                        let id_for_click = id.clone();
                        let select_component_click = select_component.clone();
                        let select_component_key = select_component.clone();
                        let items_for_key = component_items.clone();
                        rsx! { button { key: "{id}", id: "m1-component-{index}", class: if is_selected { "m1-component selected" } else { "m1-component" }, role: "option", "aria-selected": "{is_selected}", tabindex: if is_selected || (!has_selected_component && index == 0) { "0" } else { "-1" }, onclick: move |_| select_component_click(id_for_click.clone()), onkeydown: {
                            let select_component = select_component_key.clone();
                            let items_for_key = items_for_key.clone();
                            move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                let next = match key.as_str() { "ArrowDown" => Some(index + 1), "ArrowUp" => Some(index.saturating_sub(1)), "Home" => Some(0), "End" => Some(items_for_key.len().saturating_sub(1)), _ => None };
                                let Some(next) = next.filter(|next| *next < items_for_key.len()) else { return; };
                                event.prevent_default();
                                if let Some((_, next_id, _, _, _)) = items_for_key.get(next) { select_component(next_id.clone()); }
                                if let Some(element) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id(&format!("m1-component-{next}"))).and_then(|element| element.dyn_into::<HtmlElement>().ok()) { let _ = element.focus(); }
                            }
                        }, "{reference} · {kind}" } }
                    }
                }
            }
        }
        h2 { "Position" }
        if let Some(part) = selected {
            p { "{part.reference}" }
            label { "X (mm)" input { id: "m1-position-x", r#type: "number", step: "any", value: "{x}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                x.set(event.value());
                let (Ok(px), Ok(py)) = (event.value().parse::<f64>(), y().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.contains(&p.id)) else { return; };
                let edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), revision: snapshot.document.revision, transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at });
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py }, EditPhase::Preview);
            }} } }
            label { "Y (mm)" input { id: "m1-position-y", r#type: "number", step: "any", value: "{y}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                y.set(event.value());
                let (Ok(px), Ok(py)) = (x().parse::<f64>(), event.value().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.contains(&p.id)) else { return; };
                let edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), revision: snapshot.document.revision, transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at });
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py }, EditPhase::Preview);
            }} } }
            button { onclick: submit, "Apply position" }
            if numeric_edit.borrow().is_some() { p { role: "status", "Preview only. Press Enter or Apply position to save, or Escape to cancel." } }
        } else { p { "Select a component to edit its position." } }
    }}
}

fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}
fn coordinates(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    pointer: &web_sys::PointerEvent,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Vec2> {
    let surface = svg.borrow();
    let rect = surface.as_ref()?.get_bounding_client_rect();
    pointer_location(
        &rect,
        pointer.client_x(),
        pointer.client_y(),
        x,
        y,
        width,
        height,
    )
    .map(|location| location.world)
}

fn pointer_location(
    rect: &web_sys::DomRect,
    client_x: i32,
    client_y: i32,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Option<PointerLocation> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let scale = (rect.width() / width).min(rect.height() / height);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let content_width = width * scale;
    let content_height = height * scale;
    let left = rect.left() + (rect.width() - content_width) * 0.5;
    let top = rect.top() + (rect.height() - content_height) * 0.5;
    let x_fraction = (f64::from(client_x) - left) / content_width;
    let y_fraction = (f64::from(client_y) - top) / content_height;
    Some(PointerLocation {
        world: Vec2 {
            x: view_x + x_fraction * width,
            y: -(view_y + y_fraction * height),
        },
        x_fraction,
        y_fraction,
    })
}
fn moved(drag: &Drag, point: Vec2) -> Vec<Position> {
    drag.positions
        .iter()
        .map(|p| Position {
            id: p.id.clone(),
            at: Vec2 {
                x: p.at.x + point.x - drag.origin.x,
                y: p.at.y + point.y - drag.origin.y,
            },
        })
        .collect()
}

fn submit_position(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    edit: NumericEdit,
    at: Vec2,
    phase: EditPhase,
) {
    runtime.submit(Event::Edit {
        operation_id: runtime.operation(),
        command: EditCommand {
            base_revision: edit.revision,
            transaction_id: edit.transaction_id.clone(),
            phase,
            target_ids: vec![edit.id.clone()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: edit.id.clone(),
                    at,
                }],
            },
        },
    });
    if phase == EditPhase::Preview && at != edit.start {
        *current.borrow_mut() = Some(edit);
    } else {
        current.borrow_mut().take();
    }
}

fn commit_numeric(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    x: &str,
    y: &str,
) {
    let (Ok(x), Ok(y)) = (x.parse::<f64>(), y.parse::<f64>()) else {
        runtime.report("Enter finite X and Y coordinates.");
        return;
    };
    if !x.is_finite() || !y.is_finite() {
        runtime.report("Enter finite X and Y coordinates.");
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted else {
        return;
    };
    let Some(part) = snapshot
        .document
        .parts
        .iter()
        .find(|part| model.selected_part_ids.contains(&part.id))
    else {
        return;
    };
    let start = part.pose.at;
    let edit = current.borrow_mut().take().unwrap_or_else(|| NumericEdit {
        id: part.id.clone(),
        revision: snapshot.document.revision,
        transaction_id: format!("position-{}", runtime.operation().0),
        start,
    });
    submit_position(runtime, current, edit, Vec2 { x, y }, EditPhase::Commit);
}
