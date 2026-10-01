use crate::offline::{OfflineManifest, scope_cache_prefix};
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{
    Cache, ExtendableEvent, FetchEvent, Request, RequestDestination, ServiceWorkerGlobalScope,
    WorkerGlobalScope,
};

include!(concat!(env!("OUT_DIR"), "/offline_manifest.rs"));

fn manifest() -> Result<OfflineManifest, JsValue> {
    let manifest = OfflineManifest {
        version: BUILD_OFFLINE_VERSION.into(),
        assets: BUILD_OFFLINE_ASSETS
            .iter()
            .map(|asset| (*asset).to_owned())
            .collect(),
    };
    manifest
        .validate()
        .map_err(|error| JsValue::from_str(&error))?;
    Ok(manifest)
}

fn scope() -> String {
    js_sys::global()
        .unchecked_into::<ServiceWorkerGlobalScope>()
        .registration()
        .scope()
}

fn cache_prefix(scope_url: &str) -> Result<String, JsValue> {
    scope_cache_prefix(scope_url).map_err(|error| JsValue::from_str(&error))
}

fn cache_name(scope_url: &str, manifest: &OfflineManifest) -> Result<String, JsValue> {
    Ok(format!(
        "{}{version}",
        cache_prefix(scope_url)?,
        version = manifest.version
    ))
}

fn cache_storage() -> Result<web_sys::CacheStorage, JsValue> {
    js_sys::global()
        .unchecked_into::<WorkerGlobalScope>()
        .caches()
}

async fn open_current_cache(scope_url: &str, manifest: &OfflineManifest) -> Result<Cache, JsValue> {
    JsFuture::from(cache_storage()?.open(&cache_name(scope_url, manifest)?))
        .await?
        .dyn_into::<Cache>()
}

async fn install_policy() -> Result<(), JsValue> {
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();
    let manifest = manifest()?;
    let scope_url = scope();
    let cache = open_current_cache(&scope_url, &manifest).await?;
    let files = Array::new();
    for asset in &manifest.assets {
        files.push(&JsValue::from_str(&format!("{scope_url}{asset}")));
    }
    JsFuture::from(cache.add_all_with_str_sequence(files.as_ref())).await?;
    JsFuture::from(global.skip_waiting()?).await?;
    Ok(())
}

async fn activate_policy() -> Result<(), JsValue> {
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();
    let manifest = manifest()?;
    let scope_url = scope();
    let cache_storage = cache_storage()?;
    let names = Array::from(&JsFuture::from(cache_storage.keys()).await?);
    let prefix = cache_prefix(&scope_url)?;
    let active_name = cache_name(&scope_url, &manifest)?;
    for name in names.iter() {
        if let Some(name) = name.as_string()
            && name.starts_with(&prefix)
            && name != active_name
        {
            JsFuture::from(cache_storage.delete(&name)).await?;
        }
    }
    JsFuture::from(global.clients().claim()).await?;
    Ok(())
}

async fn cached_fetch(request: Request, scope_url: &str) -> Result<JsValue, JsValue> {
    let manifest = manifest()?;
    let cache = open_current_cache(scope_url, &manifest).await?;
    let cached = JsFuture::from(cache.match_with_request(&request)).await?;
    if !cached.is_undefined() {
        return Ok(cached);
    }

    if request.destination() == RequestDestination::Document {
        let shell = JsFuture::from(cache.match_with_str(&format!("{scope_url}index.html"))).await?;
        if !shell.is_undefined() {
            return Ok(shell);
        }
    }

    let global = js_sys::global().unchecked_into::<WorkerGlobalScope>();
    JsFuture::from(global.fetch_with_request(&request)).await
}

#[wasm_bindgen(start)]
pub fn register_policy_handlers() -> Result<(), JsValue> {
    manifest()?;
    let global = js_sys::global().unchecked_into::<ServiceWorkerGlobalScope>();

    let install = Closure::<dyn FnMut(ExtendableEvent)>::new(|event: ExtendableEvent| {
        let promise =
            future_to_promise(async { install_policy().await.map(|()| JsValue::UNDEFINED) });
        let _ = event.wait_until(&promise);
    });
    global.add_event_listener_with_callback("install", install.as_ref().unchecked_ref())?;
    install.forget();

    let activate = Closure::<dyn FnMut(ExtendableEvent)>::new(|event: ExtendableEvent| {
        let promise =
            future_to_promise(async { activate_policy().await.map(|()| JsValue::UNDEFINED) });
        let _ = event.wait_until(&promise);
    });
    global.add_event_listener_with_callback("activate", activate.as_ref().unchecked_ref())?;
    activate.forget();

    let scope_url = scope();
    let fetch = Closure::<dyn FnMut(FetchEvent)>::new(move |event: FetchEvent| {
        if event.request().url().starts_with(&scope_url) {
            let request = event.request();
            let scope_url = scope_url.clone();
            let promise = future_to_promise(async move { cached_fetch(request, &scope_url).await });
            let _ = event.respond_with(&promise);
        }
    });
    global.add_event_listener_with_callback("fetch", fetch.as_ref().unchecked_ref())?;
    fetch.forget();
    Ok(())
}
