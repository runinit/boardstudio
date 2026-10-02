//! Accepted-snapshot keycap-fit request lifecycle and contextual findings presentation.
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    Finding, KeycapResolution, ProjectDoc, Scope as FindingScope, Severity,
};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
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
        !self.refreshing
            && self.error.is_none()
            && self
                .accepted
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
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    use_effect(use_reactive((&source, &retry_generation()), {
        let runtime = runtime.clone();
        let alive = alive.clone();
        move |(source, _retry_generation)| {
            // Bookkeeping must not subscribe this effect to the signals it updates.
            let current = (*sequence.peek()).saturating_add(1);
            sequence.set(current);
            let Some(source) = source else {
                state.set(None);
                return;
            };
            let previous = state.peek().clone();
            state.set(Some(KeycapsFitState::begin(
                source.clone(),
                previous.as_ref(),
            )));
            let mut state = state;
            let runtime = runtime.clone();
            let alive = alive.clone();
            spawn_local(async move {
                let result = runtime
                    .resolve_keycaps_preview(source.scope.clone(), source.token, source.revision)
                    .await;
                // Browser-local tasks outlive this component's scope. Check the mount guard
                // before touching any signal after await; stale scopes must never be read/written.
                if !alive.get() || *sequence.peek() != current {
                    return;
                }
                let mut current = state.write();
                if let Some(current_state) = current.as_mut()
                    && current_state.source == source
                {
                    current_state.finish(result);
                }
            });
        }
    }));

    let on_retry = use_callback(move |()| {
        retry_generation.set(retry_generation().saturating_add(1));
    });
    KeycapsFitActions {
        state: state.read().clone(),
        on_retry,
    }
}

#[component]
pub(super) fn KeycapsFitInspector(
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
    let groups = grouped_findings(findings, &document);
    let visible_finding_count: usize = groups.iter().map(|group| group.findings.len()).sum();
    let title_detail = visible_finding_count.to_string();

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
                    p { class: "m1-keycaps-fit-clean",
                        if current { "No active findings for the conservative full switch-travel envelope check." }
                        else { "The earlier assessment had no active findings for the conservative full switch-travel envelope check." }
                    }
                } else if !state.as_ref().is_some_and(|state| state.refreshing) {
                    p { class: "m1-keycaps-fit-empty", "No fit assessment is available yet." }
                }
            } else {
                div { class: "wb-finding-groups",
                    for group in groups {
                        section { "aria-label": "{group.label}",
                            h3 { "{group.label}" }
                            ul { class: "wb-findings",
                                for finding in group.findings {
                                    KeycapsFitFinding { finding, document: document.clone() }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "m1-keycaps-fit-note", "Checks use conservative keycap envelopes through full switch travel. Case walls and solids are included only for a current Case preview." }
        }
    }
}

#[component]
fn KeycapsFitFinding(finding: Finding, document: Rc<ProjectDoc>) -> Element {
    let (severity, severity_class) = match &finding.severity {
        Severity::Error => ("Error", "error"),
        Severity::Warning => ("Warning", "warning"),
        Severity::Info => ("Information", "info"),
    };
    let fitted = finding.id.ends_with("outline:corners:fitted");
    rsx! {
        li { class: "m1-keycaps-fit-finding is-{severity_class}",
            span { class: "wb-finding-mark", "aria-hidden": "true" }
            div {
                strong { class: "wb-finding-severity", "{severity}" }
                p { "{finding.message}" }
                if fitted {
                    p { "The resulting outline has smaller corners than requested. Review the corner size and nearby spacing." }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FindingGroup {
    label: String,
    findings: Vec<Finding>,
}

/// Mirror the established React FindingList oracle: collapse duplicate feature wrappers,
/// severity-sort the stable result, and group by the first resolved target label.
fn grouped_findings(findings: &[Finding], document: &ProjectDoc) -> Vec<FindingGroup> {
    let findings = presented_findings(findings, document);
    let mut groups: Vec<FindingGroup> = Vec::new();
    for finding in findings {
        let label = finding_target_label(&finding, document)
            .unwrap_or_else(|| format!("{} review", scope_title(&finding.scope)));
        if let Some(group) = groups.iter_mut().find(|group| group.label == label) {
            group.findings.push(finding);
        } else {
            groups.push(FindingGroup {
                label,
                findings: vec![finding],
            });
        }
    }
    groups
}

fn presented_findings(findings: &[Finding], document: &ProjectDoc) -> Vec<Finding> {
    let board_ids: std::collections::HashSet<&str> = document
        .boards
        .iter()
        .map(|board| board.id.as_str())
        .collect();
    let mut result: Vec<Finding> = Vec::new();
    let mut indices = std::collections::HashMap::<String, usize>::new();
    for finding in findings {
        let source = finding
            .id
            .find(":feature:")
            .map_or(finding.id.as_str(), |index| {
                &finding.id[index + ":feature:".len()..]
            });
        let mut geometry: Vec<&str> = finding
            .target_ids
            .iter()
            .map(String::as_str)
            .filter(|id| !board_ids.contains(id))
            .collect();
        if geometry.is_empty() {
            geometry = finding.target_ids.iter().map(String::as_str).collect();
        }
        geometry.sort_unstable();
        let key = serde_json::to_string(&(
            source,
            severity_rank(&finding.severity),
            &finding.message,
            geometry,
        ))
        .expect("finding deduplication key is serializable");
        if let Some(index) = indices.get(&key).copied() {
            let existing = &mut result[index];
            for target_id in &finding.target_ids {
                if !existing.target_ids.contains(target_id) {
                    existing.target_ids.push(target_id.clone());
                }
            }
        } else {
            indices.insert(key, result.len());
            result.push(finding.clone());
        }
    }
    // Vec::sort_by_key is stable, matching JS Array.sort's stable ordering for equal severity.
    result.sort_by_key(|finding| severity_rank(&finding.severity));
    result
}

fn severity_rank(severity: &Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

fn scope_title(scope: &FindingScope) -> &'static str {
    match scope {
        FindingScope::Layout => "Layout",
        FindingScope::Outline => "Outline",
        FindingScope::Pcb => "Pcb",
        FindingScope::Case => "Case",
    }
}

fn finding_target_label(finding: &Finding, document: &ProjectDoc) -> Option<String> {
    let active_outline = document.board_outlines.iter().find_map(|owner| {
        owner
            .versions
            .iter()
            .find(|version| {
                Some(&version.id) == owner.active_version_id.as_ref()
                    && version
                        .geometry
                        .features
                        .iter()
                        .any(|feature| finding.target_ids.iter().any(|id| id == feature.id()))
            })
            .map(|version| (owner, version))
    });
    let outline = document
        .outline
        .iter()
        .find(|feature| finding.target_ids.iter().any(|id| id == feature.id()))
        .or_else(|| {
            active_outline.and_then(|(_, version)| {
                version
                    .geometry
                    .features
                    .iter()
                    .find(|feature| finding.target_ids.iter().any(|id| id == feature.id()))
            })
        });
    let owner = active_outline.map(|(owner, _)| owner);
    let board = document.boards.iter().find(|board| {
        finding.target_ids.iter().any(|id| id == &board.id)
            || owner.is_some_and(|owner| owner.board_id == board.id)
            || outline.is_some_and(|feature| board.outline_ids.iter().any(|id| id == feature.id()))
    });
    let part = document
        .parts
        .iter()
        .find(|part| finding.target_ids.iter().any(|id| id == &part.id));
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| finding.target_ids.iter().any(|id| id == &matrix.id));
    let body = document
        .case_bodies
        .iter()
        .find(|body| finding.target_ids.iter().any(|id| id == &body.id));
    if outline.is_some() {
        return Some(format!(
            "{} · Outline",
            board.map_or("Project", |board| board.name.as_str())
        ));
    }
    if let Some(part) = part {
        let name = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .map_or("Component", |definition| definition.name.as_str());
        return Some(format!("{} · {}", part.reference, name));
    }
    matrix
        .and_then(|matrix| matrix.name.clone())
        .or_else(|| body.map(|body| body.name.clone()))
        .or_else(|| board.map(|board| board.name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Board, KeycapResolution};

    fn source(revision: u64) -> KeycapsFitSource {
        KeycapsFitSource {
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: "document".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            token: SnapshotToken(revision),
            revision,
            case_preview_current: false,
        }
    }

    #[test]
    fn same_source_retry_and_failure_keep_retained_result_stale() {
        let mut accepted = KeycapsFitState::begin(source(1), None);
        accepted.finish(Ok(KeycapResolution {
            revision: 1,
            specs: vec![],
            findings: vec![],
        }));
        assert!(accepted.is_current());

        let mut retrying = KeycapsFitState::begin(source(1), Some(&accepted));
        assert!(
            !retrying.is_current(),
            "same-source retry is pending, so prior result is stale"
        );
        retrying.finish(Err("worker unavailable".into()));
        assert!(
            !retrying.is_current(),
            "failed retry cannot make retained result current"
        );
    }

    #[test]
    fn findings_are_deduplicated_severity_sorted_and_grouped_like_react() {
        let mut document = ProjectDoc::empty("doc", "Project");
        document.boards.push(Board {
            id: "board-1".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let warning = Finding {
            id: "board:board-1:feature:clearance:near-key".into(),
            severity: Severity::Warning,
            scope: FindingScope::Layout,
            message: "Keycaps are close".into(),
            target_ids: vec!["board-1".into()],
        };
        let duplicate = Finding {
            id: "other-board:board-1:feature:clearance:near-key".into(),
            ..warning.clone()
        };
        let error = Finding {
            id: "worker:error".into(),
            severity: Severity::Error,
            scope: FindingScope::Layout,
            message: "Worker failed".into(),
            target_ids: vec![],
        };
        let groups = grouped_findings(&[warning, duplicate, error], &document);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].label, "Layout review");
        assert_eq!(groups[0].findings.len(), 1);
        assert_eq!(groups[0].findings[0].severity, Severity::Error);
        assert_eq!(groups[1].label, "Left PCB");
        assert_eq!(
            groups[1].findings.len(),
            1,
            "same feature/message/geometry collapses"
        );
    }
}
