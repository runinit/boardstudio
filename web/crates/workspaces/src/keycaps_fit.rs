//! Accepted-snapshot keycap-fit request lifecycle and contextual findings presentation.
#[cfg(target_arch = "wasm32")]
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{KeycapResolution, ProjectDoc, Severity};
use boardstudio_web_ui_model::finding_navigation::{
    FindingNavigationRequest, FindingNavigationTarget, KeycapsFitSource, KeycapsFitState,
    finding_navigation_target_with_layers, grouped_findings, target_board_id,
};
use boardstudio_web_ui_shared::inspector_fields::InspectorSection;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    item::{Item, ItemContent, ItemSize},
    spinner::Spinner,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct KeycapsFitActions {
    pub state: Option<KeycapsFitState>,
    pub on_retry: EventHandler<()>,
}

/// Port the fit hook resolves through: the browser Runtime answers from Core plus its CAD
/// scenes, while native tests count resolutions with a deterministic fake.
pub trait KeycapsPreviewResolver {
    fn resolve_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<KeycapResolution, String>;
}

#[cfg(target_arch = "wasm32")]
impl KeycapsPreviewResolver for Runtime {
    fn resolve_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<KeycapResolution, String> {
        Runtime::resolve_keycaps_preview(self, scope, token, revision)
    }
}

#[cfg(any(target_arch = "wasm32", test))]
pub fn use_keycaps_fit<R: KeycapsPreviewResolver + 'static>(
    runtime: Rc<R>,
    source: Option<KeycapsFitSource>,
) -> KeycapsFitActions {
    let state = use_signal(|| None);
    use_keycaps_fit_with_state(runtime, source, state)
}

#[cfg(any(target_arch = "wasm32", test))]
pub fn use_keycaps_fit_with_state<R: KeycapsPreviewResolver + 'static>(
    runtime: Rc<R>,
    source: Option<KeycapsFitSource>,
    mut state: Signal<Option<KeycapsFitState>>,
) -> KeycapsFitActions {
    let mut retry_gen = use_signal(|| 0_u64);
    use_effect(use_reactive((&source, &retry_gen()), move |(source, _)| {
        let Some(source) = source else {
            state.set(None);
            return;
        };
        let res =
            runtime.resolve_keycaps_preview(source.scope.clone(), source.token, source.revision);
        let mut next = KeycapsFitState::begin(source, state.peek().as_ref());
        next.finish(res);
        state.set(Some(next));
    }));
    let on_retry = use_callback(move |()| retry_gen.set(retry_gen().saturating_add(1)));
    KeycapsFitActions {
        state: state.read().clone(),
        on_retry,
    }
}

#[component]
pub fn KeycapsFitInspector(
    document: Rc<ProjectDoc>,
    state: Option<KeycapsFitState>,
    mechanical_layer_ids: Rc<[String]>,
    on_retry: EventHandler<()>,
    on_navigate: EventHandler<FindingNavigationRequest>,
) -> Element {
    let current_case = state
        .as_ref()
        .is_some_and(|s| s.source.case_preview_current);
    let accepted = state.as_ref().and_then(|s| s.accepted.as_ref());
    let current = state.as_ref().is_some_and(KeycapsFitState::is_current);
    let findings = accepted.map_or(&[][..], |a| a.result.findings.as_slice());
    let groups = grouped_findings(findings, &document);
    let count: usize = groups.iter().map(|g| g.findings.len()).sum();
    rsx! {
        InspectorSection { title: "Clearance findings", badge: count.to_string(), default_open: count > 0,
            section { class: "grid min-w-0 gap-2 p-3 text-foreground [&_p]:m-0", "aria-label": "Clearance findings details",
                if let Some(st) = state.as_ref() {
                    if st.refreshing {
                        p { class: "text-sm text-muted-foreground", role: "status", Spinner {} if accepted.is_some() { "Refreshing…" } else { "Checking keycap fit…" } }
                    }
                    if let Some(error) = st.error.as_ref() {
                        Alert { variant: AlertVariant::Destructive, p { "{error}" }
                            Button { variant: ButtonVariant::Secondary, onclick: move |_| on_retry.call(()), "Retry assessment" }
                        }
                    }
                } else {
                    p { class: "text-sm text-muted-foreground", role: "status", "Waiting for an accepted board snapshot." }
                }
                if !current && let Some(acc) = accepted {
                    Callout { variant: CalloutVariant::Info, "Earlier accepted findings · revision {acc.source.revision}" }
                }
                if !current_case {
                    Callout { variant: CalloutVariant::Warning, "Current Case walls and solids have not been checked. Generate or update the Case preview for this revision." }
                }
                if findings.is_empty() {
                    if let Some(acc) = accepted {
                        if acc.result.specs.is_empty() { p { "No generated keycaps are configured for this board." } }
                        else { p { class: "text-sm text-success-dark", "No active findings for the conservative full switch-travel envelope check." } }
                    }
                } else {
                    div { class: "wb-finding-groups",
                        for group in groups {
                            section { "aria-label": "{group.label}", h3 { "{group.label}" }
                                ul { class: "wb-findings",
                                    for finding in group.findings {
                                        {
                                            let (sev, var) = match &finding.severity {
                                                Severity::Error => ("Error", BadgeVariant::Destructive),
                                                Severity::Warning => ("Warning", BadgeVariant::Warning),
                                                Severity::Info => ("Information", BadgeVariant::Info),
                                            };
                                            let src = accepted.unwrap().source.clone();
                                            let nav = finding_navigation_target_with_layers(&finding, &document, &src.scope.board_id, &mechanical_layer_ids)
                                                .filter(|t| target_board_id(t) == src.scope.board_id)
                                                .map(|target| (
                                                    if matches!(target, FindingNavigationTarget::Outline { .. }) { "Show outline" } else { "Select affected geometry" },
                                                    FindingNavigationRequest { source: src, finding: finding.clone(), target },
                                                ));
                                            rsx! {
                                                li { Item { size: ItemSize::Sm, ItemContent {
                                                    Badge { variant: var, "{sev}" }
                                                    p { "{finding.message}" }
                                                    if let Some((label, req)) = nav {
                                                        Button { variant: ButtonVariant::Secondary, onclick: move |_| on_navigate.call(req.clone()), "{label}" }
                                                    }
                                                } } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod lifecycle_tests {
    //! Mounted hook invariants: the production hook with a deterministic request port. Browser
    //! worker transport and scene projection are outside this test; Dioxus signal tracking and
    //! effect scheduling are real.
    use super::*;
    use boardstudio_application::SessionEpoch;
    use std::{
        cell::Cell,
        future::Future,
        rc::Rc,
        task::{Context, Waker},
    };

    #[derive(Default)]
    pub struct Runtime {
        pub requests: Cell<usize>,
    }

    impl KeycapsPreviewResolver for Runtime {
        fn resolve_keycaps_preview(
            &self,
            _scope: Scope,
            _token: SnapshotToken,
            revision: u64,
        ) -> Result<KeycapResolution, String> {
            self.requests.set(self.requests.get() + 1);
            Ok(KeycapResolution {
                revision,
                specs: Vec::new(),
                findings: Vec::new(),
            })
        }
    }

    #[derive(Clone)]
    struct Probe {
        runtime: Rc<Runtime>,
        source: Option<KeycapsFitSource>,
        renders: Rc<Cell<usize>>,
    }

    fn mounted_fit() -> Element {
        let probe = use_context::<Probe>();
        probe.renders.set(probe.renders.get() + 1);
        let fit = use_keycaps_fit(probe.runtime, probe.source);
        let document = Rc::new(ProjectDoc::empty("document", "Test document"));
        rsx! {
            KeycapsFitInspector {
                document,
                state: fit.state,
                mechanical_layer_ids: Rc::from([]),
                on_retry: fit.on_retry,
                on_navigate: |_| {},
            }
        }
    }

    fn assert_settles(source: Option<KeycapsFitSource>, expected_requests: usize) {
        let probe = Probe {
            runtime: Rc::default(),
            source,
            renders: Rc::default(),
        };
        let mut dom = VirtualDom::new(mounted_fit);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        let mut settled = false;
        for _ in 0..16 {
            dom.render_immediate_to_vec();
            dom.render_immediate_to_vec();
            let mut pending = std::pin::pin!(dom.wait_for_work());
            if pending
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
            {
                settled = true;
                break;
            }
        }
        assert!(
            settled,
            "Keycap fit did not settle for an unchanged source: {} renders, {} requests",
            probe.renders.get(),
            probe.runtime.requests.get()
        );
        assert_eq!(probe.runtime.requests.get(), expected_requests);
    }

    #[test]
    fn absent_source_settles_without_requests() {
        assert_settles(None, 0);
    }

    #[test]
    fn accepted_source_settles_after_one_resolution() {
        assert_settles(
            Some(KeycapsFitSource {
                scope: Scope {
                    session_epoch: SessionEpoch(1),
                    document_id: "document".into(),
                    board_id: "board".into(),
                    instance_id: None,
                },
                token: SnapshotToken(1),
                revision: 1,
                case_preview_current: false,
            }),
            1,
        );
    }
}
