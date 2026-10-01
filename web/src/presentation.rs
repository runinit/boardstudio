//! Presentation drafts and DOM input are separate from the durable session state.
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Event, SelectionMode};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, Position, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::{Cell, RefCell}, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, SvgElement};

#[derive(Clone)]
struct Drag {
    pointer: i64,
    origin: Vec2,
    positions: Vec<Position>,
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let runtime = use_hook(Runtime::new);
    let runtime = match runtime { Ok(runtime) => runtime, Err(error) => return rsx! { main { role: "alert", "Browser startup failed: {error}" } } };
    let mut version = use_signal(|| 0u64);
    let active = use_hook(|| Rc::new(Cell::new(true)));
    use_hook({
        let runtime = runtime.clone();
        let active = active.clone();
        move || runtime.subscribe(Rc::new(move || { if active.get() { version += 1; } }))
    });
    use_drop({
        let runtime = runtime.clone();
        move || { active.set(false); runtime.unsubscribe(); runtime.submit(Event::Close { operation_id: runtime.operation() }); }
    });
    let _ = version();
    use_context_provider(|| runtime.clone());
    rsx! {
        document::Stylesheet { href: "assets/m1.css" }
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
    let mut saved = use_signal(Vec::<(String, String)>::new);
    use_effect({ let runtime = runtime.clone(); move || {
        let runtime = runtime.clone();
        spawn_local(async move { match runtime.store.list_documents().await {
            Ok(documents) => saved.set(documents.into_iter().map(|d| (d.id, d.name)).collect()),
            Err(error) => runtime.report(error.to_string()),
        }});
    }});
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
                    let runtime = import.clone();
                    spawn_local(async move {
                        match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                            Ok(buffer) => if let Err(error) = runtime.import_archive(js_sys::Uint8Array::new(&buffer).to_vec()).await { runtime.report(error); },
                            Err(error) => runtime.report(format!("Import read failed: {error:?}")),
                        }
                    });
                    input.set_value("");
                }}
            }
            for (id, name) in saved() {
                button { key: "{id}", onclick: { let runtime = runtime.clone(); move |_| runtime.open_saved(id.clone()) }, "{name}" }
            }
        }
    }
}

#[component]
fn Editor() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else { return rsx! {}; };
    let document = snapshot.document.clone();
    let scene = model.display_preview.clone().unwrap_or_else(|| snapshot.scene.clone());
    let board = document.boards.iter().find(|b| b.id == model.active_board_id);
    let visible: Vec<_> = document.parts.iter().filter(|p| board.is_some_and(|b| b.part_ids.contains(&p.id))).collect();
    let points: Vec<_> = visible.iter().map(|p| p.pose.at).collect();
    let min_x = points.iter().map(|p| p.x).reduce(f64::min).unwrap_or(-50.0) - 20.0;
    let max_x = points.iter().map(|p| p.x).reduce(f64::max).unwrap_or(50.0) + 20.0;
    let min_y = points.iter().map(|p| p.y).reduce(f64::min).unwrap_or(-50.0) - 20.0;
    let max_y = points.iter().map(|p| p.y).reduce(f64::max).unwrap_or(50.0) + 20.0;
    let width = (max_x-min_x).max(50.0) / model.camera.zoom;
    let height = (max_y-min_y).max(50.0) / model.camera.zoom;
    let view_x = (min_x+max_x-width)*0.5 + model.camera.center.x;
    let view_y = -(min_y+max_y+height)*0.5 - model.camera.center.y;
    let view_box = format!("{view_x} {view_y} {width} {height}");
    let svg = use_hook(|| Rc::new(RefCell::new(None::<SvgElement>)));
    let drag = use_hook(|| Rc::new(RefCell::new(None::<Drag>)));
    let mount = { let runtime = runtime.clone(); let svg = svg.clone(); move |event: MountedEvent| {
        if let Some(element) = event.data().try_as_web_event().and_then(|e| e.dyn_into::<SvgElement>().ok()) {
            runtime.surface(element.clone()); *svg.borrow_mut() = Some(element);
        }
    }};
    let move_pointer = { let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone(); move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else { return; };
        let Some(current) = drag.borrow().clone().filter(|d| d.pointer == i64::from(pointer.pointer_id())) else { return; };
        let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
        runtime.submit(Event::GestureSample { pointer_id: current.pointer, positions: moved(&current, point), alt: pointer.alt_key() });
    }};
    let end_pointer = { let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone(); move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else { return; };
        let Some(current) = drag.borrow().clone().filter(|d| d.pointer == i64::from(pointer.pointer_id())) else { return; };
        if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
            runtime.submit(Event::GestureEnd { pointer_id: current.pointer, final_positions: moved(&current, point), alt: pointer.alt_key() });
        } else { runtime.submit(Event::GestureCancel { pointer_id: current.pointer }); }
        drag.borrow_mut().take();
    }};
    let cancel_pointer = { let runtime = runtime.clone(); let drag = drag.clone(); move |_| {
        if let Some(current) = drag.borrow_mut().take() { runtime.submit(Event::GestureCancel { pointer_id: current.pointer }); }
    }};
    let undo = runtime.clone(); let redo = runtime.clone(); let retry = runtime.clone(); let export = runtime.clone(); let navigate = runtime.clone();
    let keyboard = { let runtime = runtime.clone(); let drag = drag.clone(); move |event: KeyboardEvent| {
        let Some(key) = event.data().try_as_web_event() else { return; };
        if key.key() == "Escape" {
            if let Some(current) = drag.borrow_mut().take() { runtime.submit(Event::GestureCancel { pointer_id: current.pointer }); }
        } else if (key.ctrl_key() || key.meta_key()) && key.key().eq_ignore_ascii_case("z") {
            key.prevent_default();
            runtime.submit(if key.shift_key() { Event::Redo { operation_id: runtime.operation() } } else { Event::Undo { operation_id: runtime.operation() } });
        }
    }};
    rsx! {
        section { class: "m1-editor", "aria-label": "Keyboard editor",
            nav { class: "m1-toolbar",
                strong { "{document.name}" }
                label { "Board"
                    select { value: "{model.active_board_id}", onchange: move |event| navigate.submit(Event::Navigate { operation_id: navigate.operation(), board_id: event.value(), instance_id: None }),
                        for board in &document.boards { option { value: "{board.id}", "{board.name}" } }
                    }
                }
                button { onclick: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), "Undo" }
                button { onclick: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), "Redo" }
                button { onclick: move |_| retry.submit(Event::RetrySave { operation_id: retry.operation() }), disabled: !matches!(model.durability, Durability::Failed {..}), "Retry save" }
                button { onclick: move |_| { if let Some(scope) = export.scope() { export.submit(Event::StartExport { operation_id: export.operation(), scope }); } }, "Export archive" }
                span { "Revision {document.revision} · {model.durability:?}" }
            }
            div { class: "m1-editor-body",
                svg { class: "m1-canvas", view_box: "{view_box}", preserve_aspect_ratio: "none", tabindex: "0", role: "group", "aria-label": "Keyboard layout; drag components or use position controls", onmounted: mount,
                    onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer, onkeydown: keyboard,
                    g { transform: "scale(1,-1)",
                        for contour in scene.board_contours.iter().filter(|b| b.board_id == model.active_board_id).flat_map(|b| &b.contours) {
                            polygon { points: polygon_points(&contour.points), class: "m1-outline" }
                        }
                        for part in visible {
                            {
                                let pose = scene.transforms.iter().find(|t| t.id == part.id).map(|t| t.pose).unwrap_or(part.pose);
                                let courtyard = document.definitions.iter().find(|d| d.id == part.definition_id).map(|d| polygon_points(&d.courtyard)).unwrap_or_default();
                                let selected = model.selected_part_ids.contains(&part.id);
                                let id = part.id.clone();
                                let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone();
                                rsx! { g { key: "{part.id}", transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation})", "data-part-id": "{part.id}",
                                    onpointerdown: move |event: PointerEvent| {
                                        let Some(pointer) = event.data().try_as_web_event() else { return; };
                                        if pointer.button() != 0 { return; }
                                        pointer.prevent_default(); pointer.stop_propagation();
                                        if let Some(svg) = svg.borrow().as_ref() { let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true); let _ = svg.focus_with_options(&options); }
                                        let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
                                        let mode = if pointer.shift_key() { SelectionMode::Add } else if pointer.ctrl_key() || pointer.meta_key() { SelectionMode::Toggle } else { SelectionMode::Replace };
                                        let current = runtime.model();
                                        if !current.selected_part_ids.contains(&id) || mode != SelectionMode::Replace {
                                            runtime.submit(Event::SelectParts { operation_id: runtime.operation(), part_ids: vec![id.clone()], range_part_ids: vec![], mode });
                                        }
                                        let current = runtime.model();
                                        let Some(snapshot) = current.accepted else { return; };
                                        let positions: Vec<_> = snapshot.document.parts.iter().filter(|p| current.selected_part_ids.contains(&p.id)).map(|p| Position { id: p.id.clone(), at: p.pose.at }).collect();
                                        let operation = runtime.operation();
                                        runtime.submit(Event::GestureBegin { operation_id: operation, pointer_id: i64::from(pointer.pointer_id()), target_ids: current.selected_part_ids, transaction_id: format!("drag-{}", operation.0), start: positions.clone(), pitch: Vec2 { x:19.05, y:19.05 }, snap_fraction:0.25, geometry_snap:true, gap:None, alt:pointer.alt_key() });
                                        if runtime.model().gesture.is_some() { *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), origin, positions }); }
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
        }
    }
}

#[component]
fn Inspector() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let model = runtime.model();
    let selected = model.accepted.as_ref().and_then(|s| s.document.parts.iter().find(|p| model.selected_part_ids.contains(&p.id))).cloned();
    let mut x = use_signal(String::new); let mut y = use_signal(String::new);
    let key = selected.as_ref().map(|p| (p.id.clone(), p.pose.at.x, p.pose.at.y));
    use_effect(use_reactive((&key,), move |(key,)| { if let Some((_, px, py)) = key { x.set(px.to_string()); y.set(py.to_string()); } }));
    let submit = { let runtime = runtime.clone(); move |_| {
        let (Ok(px), Ok(py)) = (x().parse::<f64>(), y().parse::<f64>()) else { runtime.report("Enter finite X and Y coordinates."); return; };
        if !px.is_finite() || !py.is_finite() { runtime.report("Enter finite X and Y coordinates."); return; }
        let model = runtime.model(); let Some(snapshot) = model.accepted else { return; };
        let Some(part) = snapshot.document.parts.iter().find(|p| model.selected_part_ids.contains(&p.id)) else { return; };
        let operation = runtime.operation();
        runtime.submit(Event::Edit { operation_id: operation, command: EditCommand { base_revision: snapshot.document.revision, transaction_id: format!("position-{}", operation.0), phase: EditPhase::Commit, target_ids: vec![part.id.clone()], operation: EditOperation::MoveParts { positions: vec![Position { id: part.id.clone(), at: Vec2 { x:px, y:py } }] } } });
    }};
    rsx! { aside { class: "m1-inspector", "aria-label": "Component inspector",
        h2 { "Position" }
        if let Some(part) = selected {
            p { "{part.reference}" }
            label { "X (mm)" input { r#type: "number", step: "any", value: "{x}", oninput: move |event| x.set(event.value()) } }
            label { "Y (mm)" input { r#type: "number", step: "any", value: "{y}", oninput: move |event| y.set(event.value()) } }
            button { onclick: submit, "Apply position" }
        } else { p { "Select a component to edit its position." } }
    }}
}

fn polygon_points(points: &[Vec2]) -> String { points.iter().map(|p| format!("{},{}", p.x,p.y)).collect::<Vec<_>>().join(" ") }
fn coordinates(svg: &Rc<RefCell<Option<SvgElement>>>, pointer: &web_sys::PointerEvent, x:f64, y:f64, width:f64, height:f64) -> Option<Vec2> {
    let rect = svg.borrow().as_ref()?.get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 { return None; }
    Some(Vec2 { x: x+(f64::from(pointer.client_x())-rect.left())*width/rect.width(), y: -(y+(f64::from(pointer.client_y())-rect.top())*height/rect.height()) })
}
fn moved(drag: &Drag, point: Vec2) -> Vec<Position> { drag.positions.iter().map(|p| Position { id:p.id.clone(), at:Vec2 { x:p.at.x+point.x-drag.origin.x, y:p.at.y+point.y-drag.origin.y } }).collect() }
