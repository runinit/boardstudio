use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{RegistrationOptions, ServiceWorkerRegistration};

/// Registers the Rust-owned service worker for a root or slash-terminated deployment prefix.
/// The release build stages its generated worker at `{scope}service-worker.js`.
pub async fn register_offline(scope: &str) -> Result<ServiceWorkerRegistration, JsValue> {
    validate_scope(scope)?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let container = window.navigator().service_worker();
    let script_url = format!("{scope}service-worker.js");
    let options = RegistrationOptions::new();
    options.set_scope(scope);
    options.set_type("module");
    let registration =
        JsFuture::from(container.register_with_options(&script_url, &options)).await?;
    registration.dyn_into::<ServiceWorkerRegistration>()
}

fn validate_scope(scope: &str) -> Result<(), JsValue> {
    if !scope.starts_with('/')
        || !scope.ends_with('/')
        || scope
            .chars()
            .any(|character| matches!(character, '?' | '#' | '\\'))
        || scope.split('/').any(|segment| segment == "..")
    {
        return Err(JsValue::from_str(
            "service worker scope must be a normalized same-origin slash-terminated path",
        ));
    }
    Ok(())
}
