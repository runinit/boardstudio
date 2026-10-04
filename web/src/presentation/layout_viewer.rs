//! Layout's private consumer for the canonical board source and shared viewer.
use super::{
    case_display::CaseDisplay,
    objects::{self, TreeSelectRequest},
    selection::{self, SelectionAdapter},
    shared_viewer::{
        CaseSharedViewer, ScopedDisplayChange, ScopedViewerSignal, ViewerFocusRequest,
        ViewerSignalKind,
    },
};
use crate::runtime::Runtime;
use boardstudio_application::SelectionMode;
use boardstudio_core::model::{Finding, ProjectDoc};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
#[cfg(test)]
use std::{future::Future, pin::Pin};
use wasm_bindgen_futures::spawn_local;

#[cfg(test)]
type KeycapsPreviewFuture =
    Pin<Box<dyn Future<Output = Result<crate::runtime::KeycapsCadPreview, String>>>>;

#[cfg(test)]
#[derive(Clone)]
struct KeycapsPreviewProvider(
    Rc<dyn Fn(crate::runtime::KeycapsPreviewInput) -> KeycapsPreviewFuture>,
);

#[cfg(test)]
impl KeycapsPreviewProvider {
    fn request(&self, input: crate::runtime::KeycapsPreviewInput) -> KeycapsPreviewFuture {
        (self.0)(input)
    }
}

async fn request_keycaps_preview(
    runtime: &Rc<Runtime>,
    input: crate::runtime::KeycapsPreviewInput,
    #[cfg(test)] provider: Option<KeycapsPreviewProvider>,
) -> Result<crate::runtime::KeycapsCadPreview, String> {
    #[cfg(test)]
    if let Some(provider) = provider {
        return provider.request(input).await;
    }

    runtime.request_keycaps_cad_preview(input).await
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct LayoutCanonicalViewerProps {
    #[props(default)]
    pub(crate) keycaps_fit: Option<super::keycaps_fit::KeycapsFitState>,
    #[props(default)]
    pub(crate) focused_finding: Option<super::keycaps_finding_marker::FocusedFinding>,
}

#[component]
pub(crate) fn LayoutCanonicalViewer(props: LayoutCanonicalViewerProps) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let selection = use_context::<SelectionAdapter>();
    let theme = use_context::<super::ResolvedTheme>().0;
    let _ = use_context::<Signal<u64>>()();
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            runtime.cancel_keycaps_cad_preview();
            if let Some(generation) = runtime.layout_source_generation() {
                runtime.retire_layout_source(generation);
            }
        }
    });

    let keycaps_input = props
        .keycaps_fit
        .as_ref()
        .and_then(super::keycaps_fit::KeycapsFitState::current_preview_input);
    let mut keycaps_preview = use_signal(|| None::<crate::runtime::KeycapsCadPreview>);
    let mut keycaps_preview_pending = use_signal(|| false);
    let mut keycaps_preview_error = use_signal(|| None::<String>);
    let mut keycaps_preview_sequence = use_signal(|| 0_u64);
    let mut keycaps_retry_generation = use_signal(|| 0_u64);
    #[cfg(test)]
    let test_preview_provider = try_consume_context::<KeycapsPreviewProvider>();
    use_effect(use_reactive(
        (&keycaps_input, &keycaps_retry_generation()),
        {
            let runtime = runtime.clone();
            let alive = alive.clone();
            #[cfg(test)]
            let test_preview_provider = test_preview_provider.clone();
            move |(input, _retry)| {
                // This effect owns the sequence; reading it reactively here would make its
                // own write restart the effect and continually cancel the CAD request.
                let sequence = keycaps_preview_sequence.peek().saturating_add(1);
                keycaps_preview_sequence.set(sequence);
                runtime.cancel_keycaps_cad_preview();
                keycaps_preview.set(None);
                keycaps_preview_error.set(None);
                let Some(input) = input else {
                    keycaps_preview_pending.set(false);
                    return;
                };
                keycaps_preview_pending.set(true);
                let runtime = runtime.clone();
                let alive = alive.clone();
                #[cfg(test)]
                let test_preview_provider = test_preview_provider.clone();
                spawn_local(async move {
                    let result = request_keycaps_preview(
                        &runtime,
                        input,
                        #[cfg(test)]
                        test_preview_provider,
                    )
                    .await;
                    if !alive.get() || keycaps_preview_sequence.peek().ne(&sequence) {
                        return;
                    }
                    keycaps_preview_pending.set(false);
                    match result {
                        Ok(preview) => {
                            keycaps_preview.set(Some(preview));
                            keycaps_preview_error.set(None);
                        }
                        Err(error) => keycaps_preview_error.set(Some(error)),
                    }
                });
            }
        },
    ));

    let model = runtime.model();
    let request = runtime.scope().and_then(|scope| {
        model
            .accepted
            .as_ref()
            .map(|accepted| (scope, accepted.token, accepted.document.revision))
    });
    use_effect(use_reactive((&request,), {
        let runtime = runtime.clone();
        let alive = alive.clone();
        move |(request,)| {
            runtime.reconcile_layout_source_request(
                request
                    .as_ref()
                    .map(|(scope, token, revision)| (scope, *token, *revision)),
            );
            let Some((scope, token, revision)) = request else {
                return;
            };
            if runtime.layout_preview().is_some() || runtime.layout_preview_pending() {
                return;
            }
            let runtime = runtime.clone();
            let alive = alive.clone();
            spawn_local(async move {
                if !alive.get() {
                    return;
                }
                let _ = runtime.prepare_layout_preview(scope, token, revision).await;
            });
        }
    }));

    let preview = runtime.layout_preview();
    let model_rows = preview
        .as_ref()
        .and_then(|preview| runtime.layout_model_delivery(preview));
    let mut display = use_signal(CaseDisplay::default);
    let on_signal = {
        let runtime = runtime.clone();
        let selection = selection.clone();
        let preview = preview.clone();
        move |event: ScopedViewerSignal| {
            if !event.is_current() {
                return;
            }
            let Some(preview) = preview.as_ref() else {
                return;
            };
            let Some(current) = runtime.layout_preview() else {
                return;
            };
            if current.owner != preview.owner
                || !Rc::ptr_eq(&current.lease, &preview.lease)
                || event.identity.scope != preview.owner.scope
                || event.identity.snapshot_token != preview.owner.snapshot_token
            {
                return;
            }
            match event.kind {
                ViewerSignalKind::Picked(reference) => {
                    let model = runtime.model();
                    if model.active_board_id != preview.owner.scope.board_id
                        || model.active_instance_id != preview.owner.scope.instance_id
                    {
                        return;
                    }
                    let Some(accepted) = model.accepted.as_ref() else {
                        return;
                    };
                    let Some(part_id) = preview.part_for_current_pick(
                        accepted,
                        &preview.owner.scope,
                        preview.owner.source_generation,
                        &reference,
                    ) else {
                        return;
                    };
                    let Some(context) = objects::context_for_part(&model, &part_id) else {
                        return;
                    };
                    selection::submit_context(
                        &runtime,
                        &selection,
                        TreeSelectRequest {
                            scope: preview.owner.scope.clone(),
                            context,
                            mode: SelectionMode::Replace,
                            outline_action: None,
                        },
                    );
                }
                ViewerSignalKind::Failed(message) => runtime.report(message),
                _ => {}
            }
        }
    };
    let on_display_change = {
        let preview = preview.clone();
        move |event: ScopedDisplayChange| {
            if event.is_current()
                && preview.as_ref().is_some_and(|preview| {
                    event.identity.scope == preview.owner.scope
                        && event.identity.snapshot_token == preview.owner.snapshot_token
                        && preview.lease.matches(&preview.owner)
                })
            {
                display.set(event.display);
            }
        }
    };

    let Some(preview) = preview else {
        let message = if let Some(error) = runtime.layout_preview_error() {
            format!("3D preview unavailable: {error}")
        } else if runtime.layout_preview_pending() {
            "Preparing canonical board preview…".to_owned()
        } else if request.is_none() {
            "Open a project and select a board to view its assembly.".to_owned()
        } else {
            "Preparing canonical board preview…".to_owned()
        };
        return rsx! {
            div {
                p { role: "status", class: "m1-layout-viewer-status", "{message}" }
                if let Some(blocker) = props.keycaps_fit.as_ref().and_then(|fit| fit.preview_blocker_message()) {
                    p { role: "status", class: "m1-layout-viewer-status", "{blocker}" }
                } else if keycaps_preview_pending() {
                    p { role: "status", class: "m1-layout-viewer-status", "Generating keycap CAD…" }
                }
                if let Some(error) = keycaps_preview_error() {
                    p { role: "alert", class: "m1-layout-viewer-status", "Keycap preview failed: {error}" }
                    button {
                        r#type: "button",
                        onclick: move |_| keycaps_retry_generation.set(keycaps_retry_generation().saturating_add(1)),
                        "Retry keycaps"
                    }
                }
            }
        };
    };

    let focus_request = props.focused_finding.as_ref().and_then(|focused| {
        let accepted = runtime.model().accepted?;
        let current_preview = runtime.layout_preview()?;
        if focused.scope != preview.owner.scope
            || focused.scope != current_preview.owner.scope
            || focused.token != preview.owner.snapshot_token
            || focused.token != accepted.token
            || focused.revision != preview.owner.accepted_revision
            || focused.revision != accepted.document.revision
            || !preview.lease.matches(&preview.owner)
            || !current_preview.lease.matches(&current_preview.owner)
            || !Rc::ptr_eq(&current_preview.lease, &preview.lease)
        {
            return None;
        }
        let finding = accepted
            .scene
            .findings
            .iter()
            .find(|finding| finding.id == focused.finding_id)?;
        finding_focus_target_ids(finding, &accepted.document, &focused.scope.board_id).map(
            |target_ids| ViewerFocusRequest {
                scope: focused.scope.clone(),
                snapshot_token: focused.token,
                revision: focused.revision,
                target_ids,
            },
        )
    });

    rsx! {
        div { class: "m1-layout-canonical-viewer",
            if let Some(blocker) = props.keycaps_fit.as_ref().and_then(|fit| fit.preview_blocker_message()) {
                p { role: "status", class: "m1-layout-viewer-status", "{blocker}" }
            } else if keycaps_preview_pending() {
                p { role: "status", class: "m1-layout-viewer-status", "Generating keycap CAD…" }
            }
            if let Some(error) = keycaps_preview_error() {
                div { role: "alert", class: "m1-layout-viewer-status",
                    p { "Keycap preview failed: {error}" }
                    button {
                        r#type: "button",
                        onclick: move |_| keycaps_retry_generation.set(keycaps_retry_generation().saturating_add(1)),
                        "Retry keycaps"
                    }
                }
            }
            CaseSharedViewer {
                scene: None,
                preview: None,
                layout_preview: Some(preview),
                keycaps_preview: keycaps_preview(),
                parts_preview: None,
                model_rows,
                selected_layer: "pcb".to_owned(),
                selected_reference: None,
                display: display(),
                resolved_theme: theme,
                on_signal,
                on_display_change,
                mechanical_settings: None,
                inline_case_controls: false,
                focus_request,
            }
        }
    }
}

fn finding_focus_target_ids(
    finding: &Finding,
    document: &ProjectDoc,
    active_board_id: &str,
) -> Option<Vec<String>> {
    let target = super::keycaps_fit::finding_navigation_target(finding, document)?;
    finding_focus_ids_for_target(&target, &finding.target_ids, active_board_id)
}

fn finding_focus_ids_for_target(
    target: &super::keycaps_fit::FindingNavigationTarget,
    target_ids: &[String],
    active_board_id: &str,
) -> Option<Vec<String>> {
    let target_board_id = super::keycaps_fit::target_board_id(target);
    if target_board_id != active_board_id {
        return None;
    }

    Some(match target {
        super::keycaps_fit::FindingNavigationTarget::Outline { .. } => {
            vec!["pcb-selection".to_owned()]
        }
        super::keycaps_fit::FindingNavigationTarget::Board { .. } => vec!["pcb".to_owned()],
        _ if !target_ids.is_empty() => target_ids.to_vec(),
        _ => return None,
    })
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_keycaps_preview_tests {
    use super::*;
    use crate::presentation::selection::SelectionAdapter;
    use futures_channel::oneshot;
    use gloo_timers::future::TimeoutFuture;
    use std::{cell::RefCell, collections::VecDeque};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{Element as DomElement, HtmlElement};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn finding_focus_maps_outline_and_preserves_part_ids_for_current_board() {
        use super::super::keycaps_fit::FindingNavigationTarget;

        let outline = FindingNavigationTarget::Outline {
            board_id: "board-1".into(),
        };
        let part = FindingNavigationTarget::Part {
            board_id: "board-1".into(),
            part_id: "part-1".into(),
        };
        let board = FindingNavigationTarget::Board {
            board_id: "board-1".into(),
        };

        assert_eq!(
            finding_focus_ids_for_target(&outline, &["outline-corner".into()], "board-1"),
            Some(vec!["pcb-selection".into()])
        );
        assert_eq!(
            finding_focus_ids_for_target(&part, &["part-1".into()], "board-1"),
            Some(vec!["part-1".into()])
        );
        assert_eq!(
            finding_focus_ids_for_target(&board, &["board-1".into()], "board-1"),
            Some(vec!["pcb".into()])
        );
        assert_eq!(
            finding_focus_ids_for_target(&outline, &["outline-corner".into()], "board-2"),
            None
        );
    }

    struct PendingPreview {
        input: crate::runtime::KeycapsPreviewInput,
        reply: oneshot::Sender<Result<crate::runtime::KeycapsCadPreview, String>>,
    }

    #[derive(Clone)]
    struct PreviewProbe {
        runtime: Rc<Runtime>,
        first_fit: super::super::keycaps_fit::KeycapsFitState,
        newer_fit: super::super::keycaps_fit::KeycapsFitState,
    }

    fn mounted_preview_host() -> dioxus::prelude::Element {
        let probe = use_context::<PreviewProbe>();
        use_context_provider(|| probe.runtime.clone());
        let mut fit_signal = use_signal(|| Some(probe.first_fit.clone()));

        let selected_context = use_signal(|| None::<super::super::objects::ScopedTreeContext>);
        let anchor_scope = use_signal(|| None::<boardstudio_application::Scope>);
        let generation = use_signal(|| 1_u64);
        let adapter =
            use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, generation));
        use_context_provider(|| adapter.clone());
        let theme = use_memo(|| "light");
        use_context_provider(|| super::super::ResolvedTheme(theme));
        let _: Signal<u64> = use_context_provider(|| Signal::new(0_u64));

        let fit = fit_signal();
        let newer_fit = probe.newer_fit.clone();
        rsx! {
            button {
                id: "keycaps-preview-advance-owner",
                onclick: move |_| fit_signal.set(Some(newer_fit.clone())),
                "Advance board and revision"
            }
            LayoutCanonicalViewer { keycaps_fit: fit }
        }
    }

    fn preview_provider(requests: Rc<RefCell<VecDeque<PendingPreview>>>) -> KeycapsPreviewProvider {
        KeycapsPreviewProvider(Rc::new(move |input| {
            let (reply, response) = oneshot::channel();
            requests
                .borrow_mut()
                .push_back(PendingPreview { input, reply });
            Box::pin(async move {
                response
                    .await
                    .unwrap_or_else(|_| Err("controlled Keycaps provider was dropped".into()))
            })
        }))
    }

    async fn next_preview(requests: &Rc<RefCell<VecDeque<PendingPreview>>>) -> PendingPreview {
        for _ in 0..100 {
            if let Some(request) = requests.borrow_mut().pop_front() {
                return request;
            }
            TimeoutFuture::new(10).await;
        }
        panic!("mounted Keycaps hook did not request a preview");
    }

    fn completed_preview(
        input: &crate::runtime::KeycapsPreviewInput,
    ) -> crate::runtime::KeycapsCadPreview {
        crate::runtime::KeycapsCadPreview {
            generation: input.revision,
            scope: input.scope.clone(),
            token: input.token,
            revision: input.revision,
            specs: input.specs.clone(),
            bodies: Vec::new(),
        }
    }

    async fn settle() {
        TimeoutFuture::new(30).await;
    }

    fn alert(root: &DomElement) -> Option<String> {
        root.query_selector("[role=alert]")
            .unwrap()
            .and_then(|node| node.text_content())
    }

    #[wasm_bindgen_test]
    async fn mounted_preview_failure_retries_and_ignores_a_late_old_owner_reply() {
        let first_fit = super::super::keycaps_fit::cad_preview_fixture("left", 7, 11);
        let newer_fit = super::super::keycaps_fit::cad_preview_fixture("right", 8, 12);
        let runtime = Runtime::new().expect("browser runtime fixture initializes");
        let requests = Rc::new(RefCell::new(VecDeque::new()));
        let probe = PreviewProbe {
            runtime,
            first_fit,
            newer_fit,
        };
        let provider = preview_provider(requests.clone());

        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("mounted-keycaps-preview-provider-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = dioxus::prelude::VirtualDom::new(mounted_preview_host);
        dom.provide_root_context(probe.clone());
        dom.provide_root_context(provider);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );

        let initial = next_preview(&requests).await;
        assert_eq!(initial.input.scope.board_id, "left");
        assert_eq!(initial.input.revision, 11);
        initial
            .reply
            .send(Err("injected Keycaps CAD provider failure".into()))
            .unwrap();
        settle().await;
        assert_eq!(
            alert(&root).as_deref(),
            Some("Keycap preview failed: injected Keycaps CAD provider failure")
        );

        let buttons = root.query_selector_all("button").unwrap();
        let retry_button = buttons
            .item(1)
            .expect("retry follows the host owner-change control");
        assert_eq!(
            retry_button.text_content().as_deref(),
            Some("Retry keycaps")
        );
        retry_button.dyn_into::<HtmlElement>().unwrap().click();
        let retried = next_preview(&requests).await;
        assert_eq!(
            retried.input, initial.input,
            "Retry uses the same accepted input"
        );

        root.query_selector("#keycaps-preview-advance-owner")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .click();
        let newer = next_preview(&requests).await;
        assert_eq!(newer.input.scope.board_id, "right");
        assert_eq!(newer.input.revision, 12);
        newer
            .reply
            .send(Ok(completed_preview(&newer.input)))
            .unwrap();
        settle().await;
        assert_eq!(alert(&root), None, "the current owner recovers");

        retried
            .reply
            .send(Err("late failure from old board revision 11".into()))
            .unwrap();
        settle().await;
        assert_eq!(
            alert(&root),
            None,
            "an older board/revision reply cannot publish over the successful current request"
        );
        assert!(
            !root
                .text_content()
                .unwrap_or_default()
                .contains("late failure from old board"),
            "the old provider response stays invisible"
        );
        root.remove();
    }
}
