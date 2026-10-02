//! Accepted-snapshot keycap-fit request lifecycle and contextual findings presentation.
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Finding, KeycapResolution, ProjectDoc, Severity};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapsFitSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub case_preview_current: bool,
}

impl KeycapsFitSource {
    fn same_board(&self, other: &Self) -> bool {
        self.scope.session_epoch == other.scope.session_epoch
            && self.scope.document_id == other.scope.document_id
            && self.scope.board_id == other.scope.board_id
    }
}

#[derive(Clone, Debug, PartialEq)]
struct AcceptedFit {
    source: KeycapsFitSource,
    result: KeycapResolution,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapsFitState {
    source: KeycapsFitSource,
    accepted: Option<AcceptedFit>,
    refreshing: bool,
    error: Option<String>,
}

#[derive(Clone)]
pub(super) struct KeycapsFitActions {
    pub state: Option<KeycapsFitState>,
    pub on_retry: EventHandler<()>,
}

impl KeycapsFitState {
    fn begin(source: KeycapsFitSource, previous: Option<&Self>) -> Self {
        let accepted = previous
            .filter(|previous| {
                previous
                    .accepted
                    .as_ref()
                    .is_some_and(|accepted| accepted.source.same_board(&source))
            })
            .and_then(|previous| previous.accepted.clone());
        Self {
            source,
            accepted,
            refreshing: true,
            error: None,
        }
    }

    fn finish(&mut self, result: Result<KeycapResolution, String>) {
        self.refreshing = false;
        match result {
            Ok(result) if result.revision == self.source.revision => {
                self.accepted = Some(AcceptedFit {
                    source: self.source.clone(),
                    result,
                });
                self.error = None;
            }
            Ok(_) => {
                self.error = Some(
                    "Core returned findings for an older revision. Refresh the assessment.".into(),
                )
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn is_current(&self) -> bool {
        self.accepted
            .as_ref()
            .is_some_and(|accepted| accepted.source == self.source)
    }
}

/// Keep one accepted result per active board while a newer immutable snapshot is being checked.
/// The root calls this unconditionally so Keymap-originated legend edits also refresh the result.
pub(super) fn use_keycaps_fit(
    runtime: Rc<Runtime>,
    source: Option<KeycapsFitSource>,
) -> KeycapsFitActions {
    let mut state = use_signal(|| None::<KeycapsFitState>);
    let mut sequence = use_signal(|| 0_u64);
    let mut retry_generation = use_signal(|| 0_u64);

    use_effect(use_reactive((&source, &retry_generation()), {
        let runtime = runtime.clone();
        move |(source, _retry_generation)| {
            let Some(source) = source else {
                sequence.set(sequence().saturating_add(1));
                state.set(None);
                return;
            };
            let current = sequence().saturating_add(1);
            sequence.set(current);
            let previous = state.read().clone();
            state.set(Some(KeycapsFitState::begin(
                source.clone(),
                previous.as_ref(),
            )));
            let mut state = state;
            let runtime = runtime.clone();
            spawn_local(async move {
                let result = runtime
                    .resolve_keycaps_preview(source.scope.clone(), source.token, source.revision)
                    .await;
                if sequence() != current {
                    return;
                }
                let updated = {
                    let mut current = state.write();
                    current.as_mut().and_then(|current_state| {
                        if current_state.source != source {
                            return None;
                        }
                        current_state.finish(result);
                        Some(current_state.clone())
                    })
                };
                if let Some(updated) = updated {
                    state.set(Some(updated));
                }
            });
        }
    }));

    let on_retry = EventHandler::new(move |()| {
        retry_generation.set(retry_generation().saturating_add(1));
    });
    KeycapsFitActions {
        state: state.read().clone(),
        on_retry,
    }
}

#[component]
pub(super) fn KeycapsFitInspector(
    view: Rc<super::keycaps_scene::KeycapsView>,
    document: Rc<ProjectDoc>,
    state: Option<KeycapsFitState>,
    on_retry: EventHandler<()>,
) -> Element {
    let current_case = state
        .as_ref()
        .is_some_and(|state| state.source.case_preview_current);
    let accepted = state.as_ref().and_then(|state| state.accepted.as_ref());
    let current = state.as_ref().is_some_and(KeycapsFitState::is_current);
    let findings = accepted.map_or(&[][..], |accepted| accepted.result.findings.as_slice());
    let title_detail = findings.len().to_string();

    rsx! {
        section { class: "m1-keycaps-fit", "aria-label": "Clearance findings",
            div { class: "m1-keycaps-fit-heading",
                h2 { "Clearance findings" }
                span { "{title_detail}" }
            }
            if let Some(state) = state.as_ref() {
                if state.refreshing {
                    p { class: "m1-keycaps-fit-status", role: "status",
                        if accepted.is_some() { "Refreshing; the shown assessment is from an earlier revision." }
                        else { "Checking keycap fit…" }
                    }
                }
                if let Some(error) = state.error.as_ref() {
                    div { class: "m1-keycaps-fit-error", role: "alert",
                        p { "{error}" }
                        if accepted.is_some() {
                            p { "The last accepted assessment is retained and marked stale until a current result arrives." }
                        } else {
                            p { "No current assessment was accepted. Retry after checking the active board and worker." }
                        }
                        button { class: "m1-keycaps-fit-retry", onclick: move |_| on_retry.call(()), "Retry assessment" }
                    }
                }
            } else {
                p { class: "m1-keycaps-fit-status", role: "status", "Waiting for an accepted board snapshot." }
            }
            if !current && accepted.is_some() {
                p { class: "m1-keycaps-fit-stale", "Earlier accepted findings · revision {accepted.unwrap().source.revision}" }
            }
            if !current_case {
                p { class: "m1-keycaps-fit-case-note",
                    "Current Case walls and solids have not been checked. Generate or update the Case preview for this revision."
                }
            }
            if findings.is_empty() {
                if accepted.is_some() && accepted.unwrap().result.specs.is_empty() {
                    p { class: "m1-keycaps-fit-empty", "No generated keycaps are configured for this board." }
                } else if accepted.is_some() {
                    p { class: "m1-keycaps-fit-clean", "No active findings for the conservative full switch-travel envelope check." }
                } else if !state.as_ref().is_some_and(|state| state.refreshing) {
                    p { class: "m1-keycaps-fit-empty", "No fit assessment is available yet." }
                }
            } else {
                for finding in findings {
                    KeycapsFitFinding { finding: finding.clone(), view: view.clone(), document: document.clone() }
                }
            }
            p { class: "m1-keycaps-fit-note", "Checks use conservative keycap envelopes through full switch travel. Case walls and solids are included only for a current Case preview." }
        }
    }
}

#[component]
fn KeycapsFitFinding(
    finding: Finding,
    view: Rc<super::keycaps_scene::KeycapsView>,
    document: Rc<ProjectDoc>,
) -> Element {
    let (severity, severity_class) = match &finding.severity {
        Severity::Error => ("Error", "error"),
        Severity::Warning => ("Warning", "warning"),
        Severity::Info => ("Information", "info"),
    };
    let label = finding
        .target_ids
        .iter()
        .find_map(|id| view.keys.iter().find(|key| key.id.as_ref() == id))
        .map(|key| format!("{} · key", key.reference))
        .or_else(|| {
            finding.target_ids.iter().find_map(|id| {
                document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == *id)
                    .map(|matrix| matrix.name.clone().unwrap_or_else(|| matrix.id.clone()))
            })
        })
        .or_else(|| {
            finding.target_ids.iter().find_map(|id| {
                document
                    .boards
                    .iter()
                    .find(|board| board.id == *id)
                    .map(|board| board.name.clone())
            })
        })
        .unwrap_or_else(|| format!("{:?} review", finding.scope));
    rsx! {
        article { class: "m1-keycaps-fit-finding is-{severity_class}", "aria-label": "{severity}: {label}",
            h3 { "{label}" }
            strong { "{severity}" }
            p { "{finding.message}" }
        }
    }
}
