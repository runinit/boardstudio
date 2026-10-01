//! Canvas lifetime and case controls consume immutable session snapshots.
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Event, GenerationStatus};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use boardstudio_web::{case_settings, renderer_host::RendererHost};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, Float32Array, Object, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlCanvasElement;

#[component]
pub fn CasePanel() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let settings = case_settings::initial_settings(&snapshot.document, &model.active_board_id);
    let initialize = runtime.clone();
    let generate = runtime.clone();
    let cancel = runtime.clone();
    let export = runtime.clone();
    let update = runtime.clone();
    let scene = runtime.cad_scene();
    let stale = scene
        .as_ref()
        .is_some_and(|scene| scene.token != snapshot.token);
    let title = match &model.generation {
        GenerationStatus::Preparing { .. } | GenerationStatus::Running { .. } => {
            "Generating case…".to_owned()
        }
        GenerationStatus::Blocked { reason, .. } => format!("Case generation blocked: {reason}"),
        GenerationStatus::Failed { reason, .. } => format!("Case generation failed: {reason}"),
        GenerationStatus::Cancelled { .. } => "Case generation cancelled.".to_owned(),
        _ if stale => "Previous case geometry — regenerate for current changes.".to_owned(),
        _ if scene.as_ref().is_some_and(|s| s.exact) => "Exact case geometry ready.".to_owned(),
        _ if scene.is_some() => {
            "Case preview ready; exact assembly is still being built.".to_owned()
        }
        _ => "Generate a case from the saved keyboard.".to_owned(),
    };
    rsx! {
        section { class: "m1-case-panel", "aria-label": "Case assembly",
            h2 { "Case assembly" }
            if snapshot.document.mechanical.as_ref().is_none_or(|c| c.board_id != model.active_board_id) {
                button { onclick: move |_| set_settings(&initialize, None), "Add case settings" }
            }
            if let Ok(config) = settings {
                label { "Bottom thickness (mm)"
                    input { r#type: "number", min: "0.1", step: "0.1", value: "{config.bottom_thickness}", onchange: move |event: FormEvent| {
                        if let Ok(value) = event.value().parse::<f64>() { set_settings(&update, Some(value)); }
                    } }
                }
            }
            button { onclick: move |_| if let Some(scope) = generate.scope() { generate.submit(Event::StartGeneration { operation_id: generate.operation(), scope }); }, "Generate case" }
            button { disabled: !matches!(model.generation, GenerationStatus::Preparing {..} | GenerationStatus::Running {..}), onclick: move |_| cancel.submit(Event::CancelGeneration { operation_id: cancel.operation() }), "Cancel generation" }
            button { onclick: move |_| export.export_step(), "Export STEP" }
            p { role: "status", "aria-live": "polite", "{title}" }
            if let Some(scene) = scene {
                CaseCanvas { key: "{scene.scope.session_epoch.0}:{scene.scope.board_id}:{scene.scope.instance_id:?}", scene }
            }
        }
    }
}
fn set_settings(runtime: &Rc<Runtime>, bottom: Option<f64>) {
    let model = runtime.model();
    let Some(snapshot) = model.accepted else {
        return;
    };
    // Instance policy must be updated through its own supported document command.
    if model.active_instance_id.is_some() {
        runtime.report("Select the canonical board to edit shared case settings.");
        return;
    }
    let result = case_settings::initial_settings(&snapshot.document, &model.active_board_id)
        .and_then(|mut config| {
            if let Some(bottom) = bottom {
                case_settings::update_bottom_thickness(&mut config, bottom)?;
            }
            Ok(config)
        });
    match result {
        Ok(config) => {
            let operation_id = runtime.operation();
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("case-settings-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![],
                    operation: EditOperation::SetMechanical {
                        configuration: Some(Box::new(config)),
                    },
                },
            });
        }
        Err(error) => runtime.report(error),
    }
}

#[component]
fn CaseCanvas(scene: Rc<CadScene>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let host = use_hook(|| Rc::new(RefCell::new(None::<RendererHost>)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let mounted = use_signal(|| false);
    let source = use_hook(|| Rc::new(RefCell::new(scene.clone())));
    *source.borrow_mut() = scene.clone();
    use_drop({
        let alive = alive.clone();
        let host = host.clone();
        move || {
            alive.set(false);
            if let Some(host) = host.borrow_mut().take() {
                let _ = host.dispose();
            }
        }
    });
    use_effect(use_reactive((&scene,), {
        let host = host.clone();
        let runtime = runtime.clone();
        move |(scene,)| {
            if mounted() {
                if let Some(host) = host.borrow().as_ref()
                    && let Err(error) =
                        scene_input(&scene, true).and_then(|input| host.update_scene(input))
                {
                    runtime.report(error);
                }
            }
        }
    }));
    let mount = {
        let host = host.clone();
        let alive = alive.clone();
        let runtime = runtime.clone();
        let source = source.clone();
        move |event: MountedEvent| {
            let Some(canvas) = event
                .data()
                .try_as_web_event()
                .and_then(|value| value.dyn_into::<HtmlCanvasElement>().ok())
            else {
                runtime.report("Case canvas is unavailable.");
                return;
            };
            let host = host.clone();
            let alive = alive.clone();
            let runtime = runtime.clone();
            let mut mounted = mounted;
            let source = source.clone();
            spawn_local(async move {
                let input = match scene_input(&source.borrow(), false) {
                    Ok(value) => value,
                    Err(error) => {
                        runtime.report(error);
                        return;
                    }
                };
                let report = runtime.clone();
                let current = alive.clone();
                match RendererHost::mount(
                    canvas,
                    input,
                    Rc::new(move |status| report.report(status)),
                    Rc::new(move || current.get()),
                )
                .await
                {
                    Ok(renderer) if alive.get() => {
                        *host.borrow_mut() = Some(renderer);
                        mounted.set(true);
                    }
                    Ok(renderer) => {
                        let _ = renderer.dispose();
                    }
                    Err(error) if alive.get() => runtime.report(error),
                    Err(_) => {}
                }
            });
        }
    };
    let fit = host.clone();
    let top = host.clone();
    let iso = host.clone();
    let left = host.clone();
    let right = host.clone();
    let zoom = host.clone();
    rsx! {
        div { class: "m1-case-view",
            div { role: "group", "aria-label": "Case camera",
                button { onclick: move |_| { if let Some(host) = fit.borrow().as_ref() { let _ = host.fit(); } }, "Fit case" }
                button { onclick: move |_| { if let Some(host) = top.borrow().as_ref() { let _ = host.view("top"); } }, "Top view" }
                button { onclick: move |_| { if let Some(host) = iso.borrow().as_ref() { let _ = host.view("iso"); } }, "Isometric view" }
                button { onclick: move |_| { if let Some(host) = left.borrow().as_ref() { let _ = host.orbit(-0.3, 0.0); } }, "Rotate left" }
                button { onclick: move |_| { if let Some(host) = right.borrow().as_ref() { let _ = host.orbit(0.3, 0.0); } }, "Rotate right" }
                button { onclick: move |_| { if let Some(host) = zoom.borrow().as_ref() { let _ = host.zoom(1.15); } }, "Zoom case" }
            }
            canvas { style: "width:100%;height:320px;display:block", tabindex: "0", role: "img", "aria-label": "Generated case assembly; use camera controls to inspect", onmounted: mount }
        }
    }
}
fn scene_input(scene: &CadScene, keep_camera: bool) -> Result<JsValue, String> {
    let document = &scene.snapshot.document;
    let board = document
        .boards
        .iter()
        .find(|b| b.id == scene.scope.board_id)
        .ok_or("Case board unavailable")?;
    let contours = scene
        .snapshot
        .scene
        .board_contours
        .iter()
        .find(|b| b.board_id == scene.scope.board_id)
        .map(|b| &b.contours)
        .ok_or("Case contours unavailable")?;
    let stack = scene
        .mechanical
        .as_ref()
        .map(|assembly| assembly.stack.as_slice())
        .unwrap_or_default();
    let packet = serde_json::json!({ "revision": scene.result.revision, "kind":"assembly", "theme":"light", "view":"iso", "keepCamera":keep_camera, "hidden":[], "board": {"revision":scene.result.revision,"thickness":board.thickness,"contours":contours,"surfaces":[],"holes":[],"models":[]}, "models":[], "mechanicalStack":stack });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(|e| format!("{e:?}"))?;
    let bodies = Array::new();
    for body in &scene.result.bodies {
        let value = Object::new();
        let mesh = Object::new();
        for (name, buffer) in [("positions", &body.positions), ("normals", &body.normals)] {
            Reflect::set(&mesh, &name.into(), &Float32Array::from(buffer.as_slice()))
                .map_err(|e| format!("{e:?}"))?;
        }
        Reflect::set(&value, &"id".into(), &body.id.clone().into())
            .map_err(|e| format!("{e:?}"))?;
        Reflect::set(&value, &"name".into(), &body.name.clone().into())
            .map_err(|e| format!("{e:?}"))?;
        Reflect::set(&value, &"mesh".into(), &mesh).map_err(|e| format!("{e:?}"))?;
        bodies.push(&value);
    }
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(|e| format!("{e:?}"))?;
    Ok(input)
}
