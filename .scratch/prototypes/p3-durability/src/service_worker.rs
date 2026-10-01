use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{
    Cache, ExtendableEvent, FetchEvent, Request, ServiceWorkerGlobalScope, WorkerGlobalScope,
};

const SHELL_FILES: &[&str] = &[
    "index.html",
    "leave.html",
    "app.js",
    "sw-bootstrap.mjs",
    "p3_sw.js",
    "pkg/p3_durability.js",
    "pkg/p3_durability_bg.wasm",
    "fixtures/reviung41-original.boardstudio",
    "version.txt",
];

fn scope() -> String {
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();
    global.registration().scope()
}

fn cache_name() -> String {
    format!(
        "{}{}",
        crate::scope_cache_prefix(&scope()),
        option_env!("P3_CACHE_VERSION").unwrap_or("v1")
    )
}

fn build_version() -> &'static str {
    option_env!("P3_CACHE_VERSION").unwrap_or("v1")
}

fn cache_storage() -> Result<web_sys::CacheStorage, JsValue> {
    let global = js_sys::global().unchecked_into::<WorkerGlobalScope>();
    global.caches()
}

async fn install_policy() -> Result<(), JsValue> {
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();
    let cache_storage = cache_storage()?;
    let cache = JsFuture::from(cache_storage.open(&cache_name()))
        .await?
        .dyn_into::<Cache>()?;
    let scope = scope();
    let files = Array::new();
    for file in SHELL_FILES {
        files.push(&JsValue::from_str(&format!("{scope}{file}")));
    }
    files.push(&JsValue::from_str(&format!(
        "{scope}p3_sw_{}_bg.wasm",
        build_version()
    )));
    JsFuture::from(cache.add_all_with_str_sequence(files.as_ref())).await?;
    JsFuture::from(global.skip_waiting()?).await?;
    Ok(())
}

async fn activate_policy() -> Result<(), JsValue> {
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();
    let cache_storage = cache_storage()?;
    let names = JsFuture::from(cache_storage.keys()).await?;
    let names = Array::from(&names);
    let current_prefix = crate::scope_cache_prefix(&scope());
    for name in names.iter() {
        if let Some(name) = name.as_string()
            && name.starts_with(&current_prefix)
            && name != cache_name()
        {
            JsFuture::from(cache_storage.delete(&name)).await?;
        }
    }
    JsFuture::from(global.clients().claim()).await?;
    Ok(())
}

async fn cached_fetch(request: Request) -> Result<JsValue, JsValue> {
    let cache_storage = cache_storage()?;
    let cache = JsFuture::from(cache_storage.open(&cache_name()))
        .await?
        .dyn_into::<Cache>()?;
    let cached = JsFuture::from(cache.match_with_request(&request)).await?;
    if !cached.is_undefined() {
        return Ok(cached);
    }
    let global = js_sys::global().unchecked_into::<WorkerGlobalScope>();
    JsFuture::from(global.fetch_with_request(&request)).await
}

#[wasm_bindgen(start)]
pub fn register_policy_handlers() -> Result<(), JsValue> {
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();

    let install = Closure::<dyn FnMut(ExtendableEvent)>::new(|event: ExtendableEvent| {
        let promise =
            future_to_promise(async { install_policy().await.map(|_| JsValue::UNDEFINED) });
        let _ = event.wait_until(&promise);
    });
    global.add_event_listener_with_callback("install", install.as_ref().unchecked_ref())?;
    install.forget();

    let activate = Closure::<dyn FnMut(ExtendableEvent)>::new(|event: ExtendableEvent| {
        let promise =
            future_to_promise(async { activate_policy().await.map(|_| JsValue::UNDEFINED) });
        let _ = event.wait_until(&promise);
    });
    global.add_event_listener_with_callback("activate", activate.as_ref().unchecked_ref())?;
    activate.forget();

    let scope = scope();
    let fetch = Closure::<dyn FnMut(FetchEvent)>::new(move |event: FetchEvent| {
        if event.request().url().starts_with(&scope) {
            let request = event.request();
            let promise = future_to_promise(async move { cached_fetch(request).await });
            let _ = event.respond_with(&promise);
        }
    });
    global.add_event_listener_with_callback("fetch", fetch.as_ref().unchecked_ref())?;
    fetch.forget();
    Ok(())
}
