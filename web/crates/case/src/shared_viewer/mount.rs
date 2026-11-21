use super::*;

pub(super) struct RendererMount {
    pub(super) projection: Rc<RendererSceneProjection>,
    pub(super) host: Rc<RefCell<Option<RendererPageHost>>>,
    pub(super) canvas_state: Rc<RefCell<Option<HtmlCanvasElement>>>,
    pub(super) owner: Rc<ViewerOwner>,
    pub(super) runtime: Rc<crate::runtime::Runtime>,
    pub(super) live_projection: Rc<RefCell<Rc<RendererSceneProjection>>>,
    pub(super) live_source: Rc<RefCell<SourceGuard>>,
    pub(super) applied_identity: Rc<RefCell<Option<ViewerIdentity>>>,
    pub(super) on_signal: EventHandler<ScopedViewerSignal>,
    pub(super) status: Signal<String>,
    pub(super) applied_sequence: Rc<Cell<u64>>,
    pub(super) mounted: Signal<bool>,
}

impl RendererMount {
    pub(super) fn handler(self) -> impl FnMut(MountedEvent) {
        let Self {
            projection,
            host,
            canvas_state,
            owner,
            runtime,
            live_projection,
            live_source,
            applied_identity,
            on_signal,
            mut status,
            applied_sequence,
            mounted,
        } = self;
        move |event: MountedEvent| {
            let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|value| value.dyn_into::<HtmlCanvasElement>().ok())
            else {
                let identity = projection.identity.clone();
                if owner_is_current(&owner, &identity) && (live_source.borrow().0)() {
                    status.set("3D canvas could not be attached.".to_owned());
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::Failed("3D canvas could not be attached.".to_owned()),
                    );
                }
                return;
            };
            *canvas_state.borrow_mut() = Some(element.clone());
            let host = host.clone();
            let owner = owner.clone();
            let runtime = runtime.clone();
            let live_projection = live_projection.clone();
            let live_source = live_source.clone();
            let applied_identity = applied_identity.clone();
            let on_signal = on_signal;
            let mut status = status;
            let mut mounted = mounted;
            let initial_projection = projection.clone();
            let applied_sequence = applied_sequence.clone();
            if owner_is_current(&owner, &initial_projection.identity) && (live_source.borrow().0)()
            {
                emit_signal(
                    &owner,
                    on_signal,
                    &initial_projection.identity,
                    ViewerSignalKind::Lifecycle(ViewerLifecycle::Starting),
                );
                status.set("Starting the 3D renderer…".to_owned());
            }
            spawn_local(async move {
                let mut request = initial_projection;
                loop {
                    if !owner.active.get()
                        || runtime.scope().as_ref() != Some(&request.identity.scope)
                    {
                        break;
                    }
                    if !owner_is_current(&owner, &request.identity) {
                        request = live_projection.borrow().clone();
                        continue;
                    }
                    let request_identity = request.identity.clone();
                    let request_owner = owner.clone();
                    let request_runtime = runtime.clone();
                    let guard_identity = request_identity.clone();
                    let is_current = Rc::new(move || {
                        owner_is_current(&request_owner, &guard_identity)
                            && request_runtime.scope().as_ref() == Some(&guard_identity.scope)
                    });
                    let mount_guard: Rc<dyn Fn() -> bool> = is_current.clone();
                    let report_owner = owner.clone();
                    let report_runtime = runtime.clone();
                    let report_source = live_source.clone();
                    let report_applied = applied_identity.clone();
                    let report_request_identity = request.identity.clone();
                    let report_status = status;
                    let status_callback =
                        Rc::new(move |message: String| {
                            let identity = report_applied
                                .borrow()
                                .clone()
                                .unwrap_or_else(|| report_request_identity.clone());
                            if !owner_is_current(&report_owner, &identity)
                                || report_runtime.scope().as_ref() != Some(&identity.scope)
                                || !(report_source.borrow().0)()
                                || !report_runtime.model().accepted.as_ref().is_some_and(
                                    |snapshot| snapshot.token == identity.snapshot_token,
                                )
                            {
                                return;
                            }
                            let kind = if message.to_ascii_lowercase().contains("context lost") {
                                ViewerSignalKind::Lifecycle(ViewerLifecycle::ContextLost)
                            } else if message.contains("initialized") {
                                ViewerSignalKind::Lifecycle(ViewerLifecycle::Ready)
                            } else if message.to_ascii_lowercase().contains("failed") {
                                ViewerSignalKind::Failed(message.clone())
                            } else {
                                return;
                            };
                            let mut report_status = report_status;
                            report_status.set(message);
                            on_signal.call(ScopedViewerSignal {
                                identity,
                                kind,
                                owner: Rc::downgrade(&report_owner),
                            });
                        });
                    #[cfg(any(test, feature = "test-support"))]
                    let injected_mount_error = take_renderer_mount_error_for_test();
                    #[cfg(not(any(test, feature = "test-support")))]
                    let injected_mount_error: Option<String> = None;
                    let mount_result = if let Some(error) = injected_mount_error {
                        Err(error)
                    } else {
                        RendererPageHost::mount(
                            element.clone(),
                            request.input.clone(),
                            request.identity.renderer_sequence,
                            status_callback,
                            mount_guard,
                        )
                        .await
                    };
                    match mount_result {
                        Ok((renderer, accepted)) if is_current() => {
                            *host.borrow_mut() = Some(renderer);
                            applied_sequence.set(request.identity.renderer_sequence);
                            if accepted {
                                *applied_identity.borrow_mut() = Some(request.identity.clone());
                            }
                            mounted.set(true);
                            if owner_is_current(&owner, &request.identity)
                                && (live_source.borrow().0)()
                            {
                                status.set(if accepted {
                                    "3D preview ready.".to_owned()
                                } else {
                                    "A stale 3D scene update was ignored.".to_owned()
                                });
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &request.identity,
                                    ViewerSignalKind::SceneAccepted(accepted),
                                );
                            }
                            break;
                        }
                        Ok((renderer, _)) => {
                            let _ = renderer.dispose();
                            if !owner.active.get()
                                || runtime.scope().as_ref() != Some(&request.identity.scope)
                            {
                                break;
                            }
                            request = live_projection.borrow().clone();
                        }
                        Err(_error) if !is_current() => {
                            if !owner.active.get()
                                || runtime.scope().as_ref() != Some(&request.identity.scope)
                            {
                                break;
                            }
                            request = live_projection.borrow().clone();
                            if request.identity == request_identity {
                                break;
                            }
                        }
                        Err(error) => {
                            if owner_is_current(&owner, &request.identity)
                                && (live_source.borrow().0)()
                                && runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                                    snapshot.token == request.identity.snapshot_token
                                })
                            {
                                status.set(format!(
                                    "3D preview unavailable: {error}. Case forms and the 2D route remain available."
                                ));
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &request.identity,
                                    ViewerSignalKind::Failed(error),
                                );
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &request.identity,
                                    ViewerSignalKind::Lifecycle(ViewerLifecycle::Failed),
                                );
                            }
                            break;
                        }
                    }
                }
            });
        }
    }
}
