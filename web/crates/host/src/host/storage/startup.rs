use super::*;
use js_sys::{Function, Object, Promise};
use wasm_bindgen_futures::{JsFuture, future_to_promise};

/// A shared storage lease held until the editor and its Undo history are dropped.
pub struct StorageSession {
    release: Option<oneshot::Sender<()>>,
}

impl Drop for StorageSession {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

impl BrowserStore {
    pub async fn start_session(
        &self,
    ) -> Result<(Option<StorageSession>, Option<PersistError>), PersistError> {
        let navigator = window()?.navigator();
        let manager = Reflect::get(&navigator, &"locks".into()).map_err(js_error)?;
        if manager.is_null() || manager.is_undefined() {
            return Ok((None, None));
        }
        let name = format!("boardstudio-storage:{}", self.database_name);
        let mut cleanup_error = None;
        if let Some(exclusive) = acquire(&manager, &name, "exclusive", true).await? {
            cleanup_error = collect_unused_assets(self).await.err();
            drop(exclusive);
        }
        let shared = acquire(&manager, &name, "shared", false).await?;
        Ok((shared, cleanup_error))
    }
}

async fn acquire(
    manager: &JsValue,
    name: &str,
    mode: &str,
    if_available: bool,
) -> Result<Option<StorageSession>, PersistError> {
    let options = Object::new();
    Reflect::set(&options, &"mode".into(), &mode.into()).map_err(js_error)?;
    Reflect::set(&options, &"ifAvailable".into(), &if_available.into()).map_err(js_error)?;
    let (ready_tx, ready_rx) = oneshot::channel();
    let ready = Rc::new(RefCell::new(Some(ready_tx)));
    let callback_ready = ready.clone();
    let (release_tx, release_rx) = oneshot::channel();
    let mut release = Some(release_rx);
    let callback = Closure::<dyn FnMut(JsValue) -> Promise>::new(move |lock: JsValue| {
        let available = !lock.is_null();
        if let Some(ready) = callback_ready.borrow_mut().take() {
            let _ = ready.send(Ok(available));
        }
        if !available {
            return Promise::resolve(&JsValue::UNDEFINED);
        }
        let release = release.take();
        future_to_promise(async move {
            if let Some(release) = release {
                let _ = release.await;
            }
            Ok(JsValue::UNDEFINED)
        })
    });
    let request = Reflect::get(manager, &"request".into())
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(js_error)?;
    let promise = request
        .call3(manager, &name.into(), &options, callback.as_ref())
        .map_err(js_error)?
        .dyn_into::<Promise>()
        .map_err(js_error)?;
    spawn_local(async move {
        if let Err(error) = JsFuture::from(promise).await
            && let Some(ready) = ready.borrow_mut().take()
        {
            let _ = ready.send(Err(js_error(error)));
        }
        drop(callback);
    });
    let acquired = ready_rx
        .await
        .map_err(|_| PersistError("storage lock request was abandoned".into()))??;
    Ok(acquired.then_some(StorageSession {
        release: Some(release_tx),
    }))
}

async fn collect_unused_assets(store: &BrowserStore) -> Result<(), PersistError> {
    let transaction = store.transaction(&[PROJECT_STORE, ASSET_STORE], IdbTransactionMode::Readwrite).await?;
    let failure = Rc::new(RefCell::new(None));
    let completion = transaction_completion(&transaction, failure.clone());
    let projects = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
    let assets = transaction.object_store(ASSET_STORE).map_err(js_error)?;
    let projects_request = projects.get_all().map_err(js_error)?;
    let keys_request = assets.get_all_keys().map_err(js_error)?;
    let references = Rc::new(RefCell::new(None::<BTreeSet<String>>));
    let refs = references.clone();
    let project_failure = failure.clone();
    let tx = transaction.clone();
    let project_check = projects_request.clone();
    let project_callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        let result: Result<(), PersistError> = (|| {
            let values = project_check.result().map_err(js_error)?;
            let mut hashes = BTreeSet::new();
            for value in Array::from(&values).iter() {
                let document = decode_document(value)?;
                hashes.extend(document.assets.into_iter().map(|asset| asset.sha256));
            }
            *refs.borrow_mut() = Some(hashes);
            Ok(())
        })();
        if let Err(error) = result {
            *project_failure.borrow_mut() = Some(error.0);
            let _ = tx.abort();
        }
    });
    let tx = transaction.clone();
    let key_check = keys_request.clone();
    let key_callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        let result: Result<(), PersistError> = (|| {
            let refs = references.borrow();
            let hashes = refs
                .as_ref()
                .ok_or_else(|| PersistError("asset reference scan did not complete".into()))?;
            for key in Array::from(&key_check.result().map_err(js_error)?).iter() {
                if !key.as_string().is_some_and(|hash| hashes.contains(&hash)) {
                    assets.delete(&key).map_err(js_error)?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            *failure.borrow_mut() = Some(error.0);
            let _ = tx.abort();
        }
    });
    projects_request.set_onsuccess(Some(project_callback.as_ref().unchecked_ref()));
    keys_request.set_onsuccess(Some(key_callback.as_ref().unchecked_ref()));
    let result = completion.await;
    drop((project_callback, key_callback));
    result
}
