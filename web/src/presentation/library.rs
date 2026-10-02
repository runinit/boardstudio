use crate::runtime::Runtime;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[component]
pub(super) fn Library() -> Element {
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
            button { onclick: move |_| { super::close_project_menu(); reviung.open_fixture("reviung41"); }, "REVIUNG41 copy" }
            button { onclick: move |_| { super::close_project_menu(); sofle.open_fixture("sofle"); }, "Sofle v2 copy" }
            label { "Import .boardstudio"
                input { r#type: "file", accept: ".boardstudio", onchange: move |event: FormEvent| {
                    let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                    let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                    super::close_project_menu();
                    import.import_file(file);
                    input.set_value("");
                }}
            }
            for (id, name) in saved() {
                button { key: "{id}", onclick: { let runtime = runtime.clone(); move |_| { super::close_project_menu(); runtime.open_saved(id.clone()); } }, if recovery_required { "Recover from {name} (discard pending changes)" } else { "{name}" } }
            }
        }
    }
}
