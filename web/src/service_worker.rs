use boardstudio_web_worker_host::offline::{OfflineManifest, scope_cache_prefix};
use js_sys::{Array, Function, Promise};
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{
    Cache, ExtendableEvent, FetchEvent, Request, RequestDestination, Response,
    ServiceWorkerGlobalScope,
};

thread_local! {
    static STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct FinishWait(Option<Function>);

impl Drop for FinishWait {
    fn drop(&mut self) {
        if let Some(resolve) = self.0.take() {
            let _ = resolve.call0(&JsValue::UNDEFINED);
        }
    }
}

fn global() -> ServiceWorkerGlobalScope {
    js_sys::global().unchecked_into()
}

async fn open_cache(name: &str) -> Result<Cache, JsValue> {
    JsFuture::from(global().caches()?.open(name))
        .await?
        .dyn_into()
}

async fn install_policy(cache_name: String, files: Array) -> Result<JsValue, JsValue> {
    // The active worker retains its cache until its clients close.
    let cache = open_cache(&cache_name).await?;
    JsFuture::from(cache.add_all_with_str_sequence(files.as_ref())).await
}

async fn activate_policy(prefix: String, active_name: String) -> Result<JsValue, JsValue> {
    let global = global();
    let cache_storage = global.caches()?;
    let names = Array::from(&JsFuture::from(cache_storage.keys()).await?);
    for name in names.iter() {
        if let Some(name) = name.as_string()
            && name.starts_with(&prefix)
            && name != active_name
        {
            JsFuture::from(cache_storage.delete(&name)).await?;
        }
    }
    JsFuture::from(global.clients().claim()).await
}

async fn cached_fetch(
    request: Request,
    scope_url: &str,
    cache_name: &str,
    runtime_cacheable: bool,
    finish: FinishWait,
) -> Result<JsValue, JsValue> {
    let cache = open_cache(cache_name).await?;
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

    let response = JsFuture::from(global().fetch_with_request(&request)).await?;
    if runtime_cacheable
        && request.method() == "GET"
        && let Some(response) = response.dyn_ref::<Response>()
        && response.ok()
        && let Ok(cloned) = response.clone()
    {
        let _ = future_to_promise(async move {
            let _finish = finish;
            let _ = JsFuture::from(cache.put_with_request(&request, &cloned)).await;
            Ok(JsValue::UNDEFINED)
        });
    }
    Ok(response)
}

/// Validate the route's asset manifest and register the offline cache policy.
///
/// The release bootstrap (`service-worker.js`) calls this synchronously after
/// `initSync`, so the handlers exist before the worker's first event. One worker
/// build serves every route because the manifest is not compiled in.
#[wasm_bindgen]
pub fn start_offline_worker(
    version: String,
    shell_assets: Vec<String>,
    runtime_cache_prefixes: Vec<String>,
) -> Result<(), JsValue> {
    let manifest = OfflineManifest {
        version,
        shell_assets,
        runtime_cache_prefixes,
    };
    manifest
        .validate()
        .map_err(|error| JsValue::from_str(&error))?;
    if STARTED.with(|started| started.replace(true)) {
        return Err(JsValue::from_str("offline worker was already started"));
    }
    let global = global();
    let scope_url = global.registration().scope();
    let prefix = scope_cache_prefix(&scope_url).map_err(|error| JsValue::from_str(&error))?;
    let cache_name = format!("{prefix}{}", manifest.version);
    let files: Array = manifest
        .shell_assets
        .iter()
        .map(|asset| JsValue::from_str(&format!("{scope_url}{asset}")))
        .collect();

    let install_cache = cache_name.clone();
    let install = Closure::<dyn FnMut(ExtendableEvent)>::new(move |event: ExtendableEvent| {
        let _ = event.wait_until(&future_to_promise(install_policy(
            install_cache.clone(),
            files.clone(),
        )));
    });
    global.set_oninstall(Some(install.as_ref().unchecked_ref()));
    install.forget();

    let active_cache = cache_name.clone();
    let activate = Closure::<dyn FnMut(ExtendableEvent)>::new(move |event: ExtendableEvent| {
        let _ = event.wait_until(&future_to_promise(activate_policy(
            prefix.clone(),
            active_cache.clone(),
        )));
    });
    global.set_onactivate(Some(activate.as_ref().unchecked_ref()));
    activate.forget();

    let fetch = Closure::<dyn FnMut(FetchEvent)>::new(move |event: FetchEvent| {
        let request = event.request();
        if let Some(relative_path) = request.url().strip_prefix(&scope_url) {
            let asset_path = relative_path
                .split(['?', '#'])
                .next()
                .unwrap_or(relative_path);
            let runtime_cacheable = manifest.is_runtime_cacheable(asset_path);
            let mut resolve_cache = None;
            let cache_done = Promise::new(&mut |resolve, _| resolve_cache = Some(resolve));
            let _ = event.wait_until(&cache_done);
            let (scope_url, name) = (scope_url.clone(), cache_name.clone());
            let promise = future_to_promise(async move {
                cached_fetch(
                    request,
                    &scope_url,
                    &name,
                    runtime_cacheable,
                    FinishWait(resolve_cache),
                )
                .await
            });
            let _ = event.respond_with(&promise);
        }
    });
    global.set_onfetch(Some(fetch.as_ref().unchecked_ref()));
    fetch.forget();
    Ok(())
}
