use crate::runtime::Runtime;
use boardstudio_core::model::{PartKind, ProjectDoc};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, JsString, Object};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[derive(Clone, Copy, PartialEq)]
enum ListStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Clone)]
struct PreviewKey {
    id: String,
    x: f64,
    y: f64,
    angle: f64,
    width: f64,
    height: f64,
}

struct Preview {
    keys: Vec<PreviewKey>,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

fn project_name(document: &ProjectDoc) -> String {
    if document.name.trim().is_empty() {
        "Untitled keyboard".into()
    } else {
        document.name.clone()
    }
}

fn preview(document: &ProjectDoc) -> Result<Preview, ()> {
    let mut keys = Vec::new();
    for part in &document.parts {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id);
        if !definition.is_some_and(|definition| definition.kind == PartKind::Switch) {
            continue;
        }

        let (width, height) = part
            .keycap
            .or_else(|| definition.and_then(|definition| definition.keycap))
            .map(|size| (size.x, size.y))
            .unwrap_or((18.0, 18.0));
        let key = PreviewKey {
            id: part.id.clone(),
            x: part.pose.at.x,
            y: -part.pose.at.y,
            angle: -part.pose.rotation,
            width,
            height,
        };
        if ![key.x, key.y, key.angle, key.width, key.height]
            .into_iter()
            .all(f64::is_finite)
            || key.width <= 0.0
            || key.height <= 0.0
        {
            return Err(());
        }
        keys.push(key);
    }

    if keys.is_empty() {
        return Ok(Preview {
            keys,
            left: 0.0,
            top: 0.0,
            width: 0.0,
            height: 0.0,
        });
    }

    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for key in &keys {
        let angle = key.angle.to_radians();
        let dx = (angle.cos().abs() * key.width + angle.sin().abs() * key.height) / 2.0;
        let dy = (angle.sin().abs() * key.width + angle.cos().abs() * key.height) / 2.0;
        left = left.min(key.x - dx);
        top = top.min(key.y - dy);
        right = right.max(key.x + dx);
        bottom = bottom.max(key.y + dy);
    }
    left -= 8.0;
    top -= 8.0;
    let width = right - left + 8.0;
    let height = bottom - top + 8.0;
    if ![left, top, width, height].into_iter().all(f64::is_finite) || width <= 0.0 || height <= 0.0
    {
        return Err(());
    }
    Ok(Preview {
        keys,
        left,
        top,
        width,
        height,
    })
}

fn sort_saved(documents: &mut [ProjectDoc]) {
    documents.sort_by(|left, right| {
        JsString::from(project_name(left))
            .locale_compare(&project_name(right), &Array::new(), &Object::new())
            .cmp(&0)
    });
}

#[component]
fn KeyboardCard(document: ProjectDoc, current: bool, recovery_required: bool) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let name = project_name(&document);
    let preview = preview(&document);
    let action_name = if recovery_required {
        format!("Recover from {name} (discard pending changes)")
    } else {
        format!("Open {name}")
    };
    let document_id = document.id.clone();
    let detail = match &preview {
        Err(()) => "Open to check this keyboard".into(),
        Ok(preview) => format!(
            "{} keys · {}",
            preview.keys.len(),
            if document.boards.len() > 1 {
                format!("{} boards", document.boards.len())
            } else {
                "Single board".into()
            }
        ),
    };
    rsx! {
        article { class: if current { "m1-keyboard-card is-current" } else { "m1-keyboard-card" },
            button {
                class: "m1-keyboard-tile",
                r#type: "button",
                aria_label: "{action_name}",
                aria_current: current.then_some("true"),
                onclick: move |_| {
                    super::close_project_menu();
                    runtime.open_saved(document_id.clone());
                },
                match &preview {
                    Err(()) => rsx! {
                            div { class: "m1-keyboard-preview is-empty",
                                svg { class: "m1-library-keyboard-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                                }
                                span { "Preview unavailable" }
                            }
                    },
                    Ok(preview) if preview.keys.is_empty() => rsx! {
                            div { class: "m1-keyboard-preview is-empty",
                                svg { class: "m1-library-keyboard-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                                }
                                span { "No keys placed" }
                            }
                    },
                    Ok(preview) => rsx! {
                        div { class: "m1-keyboard-preview",
                            svg { view_box: "{preview.left} {preview.top} {preview.width} {preview.height}", "aria-hidden": "true",
                                for key in &preview.keys {
                                    rect {
                                        key: "{key.id}",
                                        transform: "translate({key.x} {key.y}) rotate({key.angle})",
                                        x: "{-key.width / 2.0}",
                                        y: "{-key.height / 2.0}",
                                        width: "{key.width}",
                                        height: "{key.height}",
                                        rx: "2",
                                    }
                                }
                            }
                        },
                    }
                }
                span { class: "m1-keyboard-title", "{name}" }
                span { class: "m1-keyboard-detail",
                    "{detail}"
                    if current {
                        span { class: "m1-keyboard-current",
                            svg { class: "m1-library-check-icon", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m4 10 4 4 8-8" } }
                            "Current"
                            }
                        }
                    },
                }
        }
    }
}

#[component]
pub(super) fn Library() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let recovery_required =
        runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired;
    let current = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| snapshot.document.as_ref().clone());
    let accepted_identity = current.as_ref().map(|document| document.id.clone());
    let mut saved = use_signal(Vec::<ProjectDoc>::new);
    let mut status = use_signal(|| ListStatus::Loading);
    let mut retry = use_signal(|| 0_u64);
    let retry_value = retry();
    let request_generation = use_hook(|| Rc::new(Cell::new(0_u64)));
    let mounted = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let mounted = mounted.clone();
        move || mounted.set(false)
    });

    let list_runtime = runtime.clone();
    let generations = request_generation.clone();
    let mounted_requests = mounted.clone();
    use_effect(use_reactive!(|accepted_identity, retry_value| {
        let _ = (&accepted_identity, retry_value);
        let Some(generation) = generations.get().checked_add(1) else {
            status.set(ListStatus::Failed);
            return;
        };
        generations.set(generation);
        status.set(ListStatus::Loading);
        let runtime = list_runtime.clone();
        let mounted = mounted_requests.clone();
        let generations = generations.clone();
        let request_identity = accepted_identity.clone();
        spawn_local(async move {
            let result = runtime.store.list_documents().await;
            let latest_identity = runtime
                .model()
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.clone());
            if !mounted.get()
                || generations.get() != generation
                || latest_identity != request_identity
            {
                return;
            }
            match result {
                Ok(mut documents) => {
                    sort_saved(&mut documents);
                    saved.set(documents);
                    status.set(ListStatus::Ready);
                }
                Err(_) => status.set(ListStatus::Failed),
            }
        });
    }));

    let mut cards = Vec::new();
    if let Some(current) = current {
        cards.push((current.clone(), true));
        cards.extend(
            saved()
                .into_iter()
                .filter(|document| document.id != current.id)
                .map(|document| (document, false)),
        );
    } else {
        cards.extend(saved().into_iter().map(|document| (document, false)));
    }
    let reviung = runtime.clone();
    let sofle = runtime.clone();
    let import = runtime.clone();
    let retry_generations = request_generation.clone();
    rsx! {
        section { class: "m1-library", "aria-label": "Your keyboards",
            header { class: "m1-library-heading",
                h2 { "Your keyboards" if status() == ListStatus::Ready { span { "{cards.len()}" } } }
                span { "Saved in this browser" }
            }
            if status() == ListStatus::Loading {
                p { role: "status", "Loading saved keyboards…" }
            }
            if status() == ListStatus::Failed {
                p { role: "alert", "Saved keyboards could not be loaded. "
                    button { class: "m1-library-text-action", r#type: "button", onclick: move |_| {
                        if let Some(generation) = retry_generations.get().checked_add(1) {
                            retry_generations.set(generation);
                            retry += 1;
                        }
                    }, "Try again" }
                }
            }
            div { class: "m1-keyboard-grid",
                for (document, is_current) in cards {
                    KeyboardCard { key: "{document.id}", document, current: is_current, recovery_required }
                }
            }
            div { class: "m1-library-actions",
                button { r#type: "button", onclick: move |_| { super::close_project_menu(); reviung.open_fixture("reviung41"); }, "REVIUNG41 copy" }
                button { r#type: "button", onclick: move |_| { super::close_project_menu(); sofle.open_fixture("sofle"); }, "Sofle v2 copy" }
                label { "Import .boardstudio"
                    input { r#type: "file", accept: ".boardstudio", onchange: move |event: FormEvent| {
                        let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                        let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                        super::close_project_menu();
                        import.import_file(file);
                        input.set_value("");
                    }}
                }
            }
        }
    }
}
