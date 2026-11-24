use super::*;
use web_sys::{DomException, IdbDatabase, IdbObjectStoreParameters};

pub(super) struct CachedDatabase {
    handle: IdbDatabase,
    _invalidate: RequestHandler,
}

impl fmt::Debug for CachedDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.handle.fmt(f)
    }
}

impl Drop for CachedDatabase {
    fn drop(&mut self) {
        self.handle.set_onversionchange(None);
        self.handle.set_onclose(None);
        self.handle.close();
    }
}

impl BrowserStore {
    async fn database(&self) -> Result<IdbDatabase, PersistError> {
        if let Some(database) = self.database.borrow().as_ref() {
            return Ok(database.handle.clone());
        }
        let handle = open_database(&self.database_name).await?;
        let mut cache = self.database.borrow_mut();
        if let Some(database) = cache.as_ref() {
            // Another operation filled the cache during this open.
            handle.close();
            return Ok(database.handle.clone());
        }
        let cache_ref = Rc::downgrade(&self.database);
        let closing = handle.clone();
        let invalidate = Closure::<dyn FnMut(Event)>::new(move |_| {
            closing.close();
            if let Some(cache) = cache_ref.upgrade() {
                cache
                    .borrow_mut()
                    .take_if(|database| database.handle == closing);
            }
        });
        handle.set_onversionchange(Some(invalidate.as_ref().unchecked_ref()));
        handle.set_onclose(Some(invalidate.as_ref().unchecked_ref()));
        *cache = Some(CachedDatabase {
            handle: handle.clone(),
            _invalidate: invalidate,
        });
        Ok(handle)
    }

    pub(super) async fn transaction(
        &self,
        stores: &[&str],
        mode: IdbTransactionMode,
    ) -> Result<IdbTransaction, PersistError> {
        let names: Array = stores.iter().map(|name| JsValue::from_str(name)).collect();
        let open = |db: &IdbDatabase| db.transaction_with_str_sequence_and_mode(&names, mode);
        let database = self.database().await?;
        match open(&database) {
            Err(error)
                if error
                    .dyn_ref::<DomException>()
                    .is_some_and(|error| error.name() == "InvalidStateError") =>
            {
                self.database
                    .borrow_mut()
                    .take_if(|cached| cached.handle == database);
                open(&self.database().await?).map_err(js_error)
            }
            result => result.map_err(js_error),
        }
    }
}

pub async fn open_database(name: &str) -> Result<IdbDatabase, PersistError> {
    let factory = window()?
        .indexed_db()
        .map_err(js_error)?
        .ok_or_else(|| PersistError("IndexedDB is unavailable".into()))?;
    let request = factory.open_with_u32(name, 2).map_err(js_error)?;
    let upgrade = request.clone();
    let on_upgrade = Closure::<dyn FnMut(Event)>::new(move |_| {
        let result = (|| -> Result<(), JsValue> {
            let database = upgrade.result()?.dyn_into::<IdbDatabase>()?;
            // Read the DOMStringList through JS so the workspace web-sys features stay unified.
            let names = Array::from(&Reflect::get(&database, &"objectStoreNames".into())?);
            if !names.includes(&PROJECT_STORE.into(), 0) {
                let params = IdbObjectStoreParameters::new();
                params.set_key_path_opt_str(Some("id"));
                database.create_object_store_with_optional_parameters(PROJECT_STORE, &params)?;
            }
            for name in [ASSET_STORE, PROJECT_TOKEN_STORE] {
                if !names.includes(&name.into(), 0) {
                    database.create_object_store(name)?;
                }
            }
            Ok(())
        })();
        if result.is_err()
            && let Some(transaction) = upgrade.transaction()
        {
            let _ = transaction.abort();
        }
    });
    request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
    let database = request_result(request.unchecked_into())
        .await?
        .dyn_into()
        .map_err(js_error);
    drop(on_upgrade);
    database
}
