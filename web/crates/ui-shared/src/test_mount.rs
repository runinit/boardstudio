//! Owns the render loop and root of a mounted browser test fixture.
use dioxus::prelude::VirtualDom;
use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    pin::Pin,
    rc::Rc,
    task::{Poll, Waker},
};
use web_sys::{Element, Storage};

struct RenderTask {
    future: Option<Pin<Box<dyn Future<Output = ()>>>>,
    waker: Option<Waker>,
}

/// Dropping the fixture stops rendering, disposes the VDOM, and removes its root.
pub struct MountedFixture {
    root: Element,
    render: Rc<RefCell<RenderTask>>,
}

impl MountedFixture {
    pub fn new(root: Element, dom: VirtualDom) -> Self {
        let config = dioxus_web::Config::new().rootnode(root.clone().into());
        let render = Rc::new(RefCell::new(RenderTask {
            future: Some(Box::pin(async move {
                dioxus_web::run(dom, config).await;
            })),
            waker: None,
        }));
        let task = render.clone();
        wasm_bindgen_futures::spawn_local(poll_fn(move |cx| {
            let mut task = task.borrow_mut();
            let Some(future) = task.future.as_mut() else {
                return Poll::Ready(());
            };
            let result = future.as_mut().poll(cx);
            task.waker = Some(cx.waker().clone());
            result
        }));
        Self { root, render }
    }
}

impl Drop for MountedFixture {
    fn drop(&mut self) {
        let (future, waker) = {
            let mut task = self.render.borrow_mut();
            (task.future.take(), task.waker.take())
        };
        drop(future);
        self.root.remove();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Restores the saved values, including keys that were originally absent.
pub struct SavedLocalStorage {
    storage: Storage,
    entries: Vec<(&'static str, Option<String>)>,
}

impl SavedLocalStorage {
    pub fn new(storage: Storage, keys: &[&'static str]) -> Result<Self, wasm_bindgen::JsValue> {
        let entries = keys
            .iter()
            .map(|&key| storage.get_item(key).map(|value| (key, value)))
            .collect::<Result<_, _>>()?;
        Ok(Self { storage, entries })
    }
}

impl Drop for SavedLocalStorage {
    fn drop(&mut self) {
        for (key, value) in &self.entries {
            let result = match value {
                Some(value) => self.storage.set_item(key, value),
                None => self.storage.remove_item(key),
            };
            if let Err(error) = result {
                web_sys::console::error_1(&error);
            }
        }
    }
}
