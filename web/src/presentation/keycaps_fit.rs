//! Accepted-snapshot keycap-fit request lifecycle and contextual findings presentation.
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    Contour, Finding, KeycapResolution, Matrix, Part, Pose2, ProjectDoc, SceneDelta,
    Scope as FindingScope, Severity, Side, Vec2,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FindingNavigationTarget {
    MechanicalLayer { board_id: String, layer_id: String },
    Outline { board_id: String },
    Part { board_id: String, part_id: String },
    Matrix { board_id: String, matrix_id: String },
    Body { board_id: String, body_id: String },
    Board { board_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FindingNavigationRequest {
    pub source: KeycapsFitSource,
    pub finding: Finding,
    pub target: FindingNavigationTarget,
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

    pub(super) fn accepts_navigation(
        &self,
        request: &FindingNavigationRequest,
        document: &ProjectDoc,
    ) -> bool {
        self.is_current()
            && self.accepted.as_ref().is_some_and(|accepted| {
                accepted.source == request.source
                    && presented_findings(&accepted.result.findings, document)
                        .iter()
                        .any(|finding| finding == &request.finding)
            })
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
    mechanical_layer_ids: Rc<[String]>,
    on_retry: EventHandler<()>,
    on_navigate: EventHandler<FindingNavigationRequest>,
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
                                    KeycapsFitFinding {
                                        finding,
                                        document: document.clone(),
                                        source: accepted.unwrap().source.clone(),
                                        mechanical_layer_ids: mechanical_layer_ids.clone(),
                                        on_navigate,
                                    }
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
fn KeycapsFitFinding(
    finding: Finding,
    document: Rc<ProjectDoc>,
    source: KeycapsFitSource,
    mechanical_layer_ids: Rc<[String]>,
    on_navigate: EventHandler<FindingNavigationRequest>,
) -> Element {
    let (severity, severity_class) = match &finding.severity {
        Severity::Error => ("Error", "error"),
        Severity::Warning => ("Warning", "warning"),
        Severity::Info => ("Information", "info"),
    };
    let fitted = finding.id.ends_with("outline:corners:fitted");
    let navigation_target = finding_navigation_target_with_layers(
        &finding,
        &document,
        &source.scope.board_id,
        &mechanical_layer_ids,
    );
    let action_label = finding_action_label(
        &finding,
        &document,
        &source.scope.board_id,
        &mechanical_layer_ids,
    );
    let request = navigation_target.map(|target| FindingNavigationRequest {
        source,
        finding: finding.clone(),
        target,
    });
    rsx! {
        li { class: "m1-keycaps-fit-finding is-{severity_class}",
            span { class: "wb-finding-mark", "aria-hidden": "true" }
            div {
                strong { class: "wb-finding-severity", "{severity}" }
                p { "{finding.message}" }
                if fitted {
                    p { "The resulting outline has smaller corners than requested. Review the corner size and nearby spacing." }
                }
                if let (Some(action_label), Some(request)) = (action_label, request) {
                    button {
                        class: "wb-finding-action",
                        onclick: move |_| on_navigate.call(request.clone()),
                        "{action_label}"
                    }
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

pub(super) fn finding_navigation_target(
    finding: &Finding,
    document: &ProjectDoc,
) -> Option<FindingNavigationTarget> {
    // Keep the same precedence as React's findingTarget: outline, part, matrix, body, board.
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
    let outline_board = active_outline
        .map(|(owner, _)| owner.board_id.as_str())
        .or_else(|| {
            outline.and_then(|feature| {
                document.boards.iter().find_map(|board| {
                    board
                        .outline_ids
                        .iter()
                        .any(|outline_id| outline_id == feature.id())
                        .then_some(board.id.as_str())
                })
            })
        });
    if let Some(board_id) = outline_board {
        return Some(FindingNavigationTarget::Outline {
            board_id: board_id.to_owned(),
        });
    }
    if let Some(part) = document
        .parts
        .iter()
        .find(|part| finding.target_ids.iter().any(|id| id == &part.id))
    {
        let board_id = document
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&part.id))?;
        return Some(FindingNavigationTarget::Part {
            board_id: board_id.id.clone(),
            part_id: part.id.clone(),
        });
    }
    if let Some(matrix) = document
        .matrices
        .iter()
        .find(|matrix| finding.target_ids.iter().any(|id| id == &matrix.id))
    {
        let board_id = matrix.board_id.as_ref().or_else(|| {
            document
                .boards
                .iter()
                .find(|board| matrix.part_ids.iter().any(|id| board.part_ids.contains(id)))
                .map(|board| &board.id)
        })?;
        return Some(FindingNavigationTarget::Matrix {
            board_id: board_id.clone(),
            matrix_id: matrix.id.clone(),
        });
    }
    if let Some(body) = document
        .case_bodies
        .iter()
        .find(|body| finding.target_ids.iter().any(|id| id == &body.id))
    {
        return Some(FindingNavigationTarget::Body {
            board_id: body.board_id.clone(),
            body_id: body.id.clone(),
        });
    }
    document
        .boards
        .iter()
        .find(|board| finding.target_ids.iter().any(|id| id == &board.id))
        .map(|board| FindingNavigationTarget::Board {
            board_id: board.id.clone(),
        })
}

/// Match the pinned workbench's current mechanical-layer precedence before the
/// document target resolver. The assembly is supplied only by the root after
/// its accepted scope/token/revision check.
pub(super) fn finding_navigation_target_with_layers(
    finding: &Finding,
    document: &ProjectDoc,
    board_id: &str,
    mechanical_layer_ids: &[String],
) -> Option<FindingNavigationTarget> {
    if let Some(layer_id) = mechanical_layer_ids
        .iter()
        .find(|layer_id| finding.target_ids.contains(layer_id))
    {
        return Some(FindingNavigationTarget::MechanicalLayer {
            board_id: board_id.to_owned(),
            layer_id: layer_id.clone(),
        });
    }
    finding_navigation_target(finding, document)
}

pub(super) fn accepted_navigation_target(
    state: &KeycapsFitState,
    request: &FindingNavigationRequest,
    document: &ProjectDoc,
    mechanical_layer_ids: &[String],
) -> Option<FindingNavigationTarget> {
    if !state.accepts_navigation(request, document) {
        return None;
    }
    let target = finding_navigation_target_with_layers(
        &request.finding,
        document,
        &request.source.scope.board_id,
        mechanical_layer_ids,
    )?;
    (target_board_id(&target) == request.source.scope.board_id).then_some(target)
}

pub(super) fn layout_navigation_bounds(
    document: &ProjectDoc,
    scene: &SceneDelta,
    target: &FindingNavigationTarget,
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    match target {
        FindingNavigationTarget::Part { part_id, board_id } => {
            let board = document.boards.iter().find(|board| board.id == *board_id)?;
            if !board.part_ids.contains(part_id) {
                return None;
            }
            let part = document.parts.iter().find(|part| part.id == *part_id)?;
            include_layout_part_bounds(document, scene, part, &mut bounds);
        }
        FindingNavigationTarget::Matrix {
            matrix_id,
            board_id,
        } => {
            let board = document.boards.iter().find(|board| board.id == *board_id)?;
            let matrix = document.matrices.iter().find(|matrix| {
                matrix.id == *matrix_id
                    && matrix.part_ids.iter().any(|id| board.part_ids.contains(id))
            })?;
            for part in document.parts.iter().filter(|part| {
                board.part_ids.contains(&part.id) && matrix.part_ids.contains(&part.id)
            }) {
                include_layout_part_bounds(document, scene, part, &mut bounds);
            }
            include_matrix_cell_bounds(document, scene, matrix, &mut bounds);
        }
        FindingNavigationTarget::Outline { board_id }
        | FindingNavigationTarget::Board { board_id } => {
            include_board_contours(document, scene, board_id, &mut bounds);
        }
        FindingNavigationTarget::MechanicalLayer { .. } | FindingNavigationTarget::Body { .. } => {
            return None;
        }
    }
    let (min_x, max_x, min_y, max_y) = bounds?;
    Some((min_x - 12.0, max_x + 12.0, min_y - 12.0, max_y + 12.0))
}

/// React fits a finding's complete Core marker contours after fitting the primary
/// target. Reuse the accepted scene projection rather than reconstructing finding
/// geometry from its target IDs or message.
pub(super) fn finding_marker_bounds(
    scene: &SceneDelta,
    finding_id: &str,
    board_id: &str,
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for point in scene
        .finding_markers
        .iter()
        .filter(|marker| marker.finding_id == finding_id && marker.board_id == board_id)
        .flat_map(|marker| marker.contours.iter())
        .flat_map(|contour| contour.points.iter())
        .filter(|point| point.x.is_finite() && point.y.is_finite())
    {
        bounds = Some(match bounds {
            Some((min_x, max_x, min_y, max_y)) => (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            ),
            None => (point.x, point.x, point.y, point.y),
        });
    }
    let (min_x, max_x, min_y, max_y) = bounds?;
    Some((min_x - 12.0, max_x + 12.0, min_y - 12.0, max_y + 12.0))
}

pub(super) fn finding_navigation_bounds(
    document: &ProjectDoc,
    scene: &SceneDelta,
    finding_id: &str,
    target: &FindingNavigationTarget,
) -> Option<(f64, f64, f64, f64)> {
    finding_marker_bounds(scene, finding_id, target_board_id(target))
        .or_else(|| layout_navigation_bounds(document, scene, target))
}

fn include_layout_part_bounds(
    document: &ProjectDoc,
    scene: &SceneDelta,
    part: &Part,
    bounds: &mut Option<(f64, f64, f64, f64)>,
) {
    let pose = scene
        .transforms
        .iter()
        .find(|transform| transform.id == part.id)
        .map_or(part.pose, |transform| transform.pose);
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id);
    if let Some(definition) = definition.filter(|definition| !definition.courtyard.is_empty()) {
        for point in &definition.courtyard {
            include_local_point(bounds, pose, matches!(part.side, Side::Back), *point);
        }
    } else {
        include_point(bounds, pose.at);
    }
    if let Some(size) = part
        .keycap
        .or_else(|| definition.and_then(|definition| definition.keycap))
    {
        include_rotated_rect(bounds, pose, size);
    }
}

fn include_matrix_cell_bounds(
    document: &ProjectDoc,
    scene: &SceneDelta,
    matrix: &Matrix,
    bounds: &mut Option<(f64, f64, f64, f64)>,
) {
    let Some(projected) = scene
        .matrix_scenes
        .iter()
        .find(|projected| projected.matrix_id == matrix.id)
    else {
        return;
    };
    for cell in projected.cells.iter().filter(|cell| cell.enabled) {
        let part = cell
            .member_id
            .as_deref()
            .and_then(|id| document.parts.iter().find(|part| part.id == id));
        let definition = part
            .and_then(|part| {
                document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
            })
            .or_else(|| {
                document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == matrix.definition_id)
            });
        let size = part
            .and_then(|part| part.keycap)
            .or_else(|| definition.and_then(|definition| definition.keycap))
            .unwrap_or(Vec2 {
                x: (matrix.pitch.x - matrix.edge_gap.map_or(1.0, |gap| gap.x)).max(1.0),
                y: (matrix.pitch.y - matrix.edge_gap.map_or(1.0, |gap| gap.y)).max(1.0),
            });
        include_rotated_rect(bounds, cell.pose, size);
    }
}

fn include_board_contours(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    bounds: &mut Option<(f64, f64, f64, f64)>,
) {
    if let Some(board) = scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
    {
        for point in board
            .contours
            .iter()
            .flat_map(|contour: &Contour| &contour.points)
        {
            include_point(bounds, *point);
        }
    } else if document.boards.len() == 1 {
        for point in scene.contours.iter().flat_map(|contour| &contour.points) {
            include_point(bounds, *point);
        }
    }
}

fn include_rotated_rect(bounds: &mut Option<(f64, f64, f64, f64)>, pose: Pose2, size: Vec2) {
    for point in [
        Vec2 {
            x: -size.x / 2.0,
            y: -size.y / 2.0,
        },
        Vec2 {
            x: -size.x / 2.0,
            y: size.y / 2.0,
        },
        Vec2 {
            x: size.x / 2.0,
            y: -size.y / 2.0,
        },
        Vec2 {
            x: size.x / 2.0,
            y: size.y / 2.0,
        },
    ] {
        include_local_point(bounds, pose, false, point);
    }
}

fn include_local_point(
    bounds: &mut Option<(f64, f64, f64, f64)>,
    pose: Pose2,
    mirrored: bool,
    point: Vec2,
) {
    let local_x = if mirrored { -point.x } else { point.x };
    let angle = pose.rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    include_point(
        bounds,
        Vec2 {
            x: pose.at.x + local_x * cos - point.y * sin,
            y: pose.at.y + local_x * sin + point.y * cos,
        },
    );
}

fn include_point(bounds: &mut Option<(f64, f64, f64, f64)>, point: Vec2) {
    *bounds = Some(bounds.map_or(
        (point.x, point.x, point.y, point.y),
        |(min_x, max_x, min_y, max_y)| {
            (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            )
        },
    ));
}

fn finding_action_label(
    finding: &Finding,
    document: &ProjectDoc,
    active_board_id: &str,
    mechanical_layer_ids: &[String],
) -> Option<&'static str> {
    finding_navigation_target_with_layers(finding, document, active_board_id, mechanical_layer_ids)
        .filter(|target| target_board_id(target) == active_board_id)
        .map(|target| match target {
            FindingNavigationTarget::Outline { .. } => "Show outline",
            FindingNavigationTarget::Part { .. }
            | FindingNavigationTarget::Matrix { .. }
            | FindingNavigationTarget::Body { .. }
            | FindingNavigationTarget::MechanicalLayer { .. }
            | FindingNavigationTarget::Board { .. } => "Select affected geometry",
        })
}

fn target_board_id(target: &FindingNavigationTarget) -> &str {
    match target {
        FindingNavigationTarget::Outline { board_id }
        | FindingNavigationTarget::MechanicalLayer { board_id, .. }
        | FindingNavigationTarget::Part { board_id, .. }
        | FindingNavigationTarget::Matrix { board_id, .. }
        | FindingNavigationTarget::Body { board_id, .. }
        | FindingNavigationTarget::Board { board_id } => board_id,
    }
}

#[cfg(test)]
pub(super) fn browser_navigation_fixture() -> (FindingNavigationRequest, KeycapsFitState, ProjectDoc)
{
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{Board, KeycapResolution};

    let scope = Scope {
        session_epoch: SessionEpoch(2),
        document_id: "doc".into(),
        board_id: "left".into(),
        instance_id: None,
    };
    let source = KeycapsFitSource {
        scope,
        token: SnapshotToken(7),
        revision: 11,
        case_preview_current: false,
    };
    let mut document = ProjectDoc::empty("doc", "Project");
    document.revision = source.revision;
    document.boards.push(Board {
        id: "left".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let finding = Finding {
        id: "board:left:invalid-settings".into(),
        severity: Severity::Warning,
        scope: FindingScope::Layout,
        message: "Board needs review".into(),
        target_ids: vec!["left".into()],
    };
    let mut state = KeycapsFitState::begin(source.clone(), None);
    state.finish(Ok(KeycapResolution {
        revision: source.revision,
        specs: vec![],
        findings: vec![finding.clone()],
    }));
    let request = FindingNavigationRequest {
        source,
        finding,
        target: FindingNavigationTarget::Board {
            board_id: "left".into(),
        },
    };
    (request, state, document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{
        Board, Contour, FindingMarker, KeycapResolution, Part, Pose2, Readiness, SceneDelta, Side,
        Transform, Vec2,
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

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

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
    fn findings_are_deduplicated_severity_sorted_and_grouped_like_react() {
        let mut document = ProjectDoc::empty("doc", "Project");
        document.boards.push(Board {
            id: "board".into(),
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
            target_ids: vec!["board".into()],
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

    #[wasm_bindgen_test]
    fn actionable_board_target_has_react_label_and_targetless_finding_has_no_action() {
        let mut document = ProjectDoc::empty("doc", "Project");
        document.boards.push(Board {
            id: "board".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let finding = Finding {
            id: "board:board:invalid-settings".into(),
            severity: Severity::Error,
            scope: FindingScope::Layout,
            message: "Invalid board setting".into(),
            target_ids: vec!["board".into()],
        };
        assert_eq!(
            finding_action_label(&finding, &document, "board", &[]),
            Some("Select affected geometry")
        );
        assert_eq!(
            finding_action_label(&finding, &document, "other-board", &[]),
            None
        );
        let targetless = Finding {
            target_ids: vec![],
            ..finding
        };
        assert_eq!(
            finding_action_label(&targetless, &document, "board", &[]),
            None
        );
    }

    #[wasm_bindgen_test]
    fn current_case_stack_target_precedes_companion_part_target() {
        let mut document = ProjectDoc::empty("doc", "Project");
        document.boards.push(Board {
            id: "board".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec!["matrix/keys/diode-1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.parts.push(Part {
            id: "matrix/keys/diode-1".into(),
            definition_id: "diode".into(),
            reference: "D1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
            keycap: None,
            outline: None,
        });
        let finding = Finding {
            id: "case:collision".into(),
            severity: Severity::Warning,
            scope: FindingScope::Case,
            message: "Mechanical collision".into(),
            target_ids: vec!["matrix/keys/diode-1".into(), "case:plate".into()],
        };
        let layer_ids = vec!["case:plate".to_owned()];
        assert_eq!(
            finding_navigation_target_with_layers(&finding, &document, "board", &layer_ids),
            Some(FindingNavigationTarget::MechanicalLayer {
                board_id: "board".into(),
                layer_id: "case:plate".into(),
            })
        );
        assert!(matches!(
            finding_navigation_target(&finding, &document),
            Some(FindingNavigationTarget::Part { .. })
        ));
    }

    #[wasm_bindgen_test]
    fn production_navigation_admission_rejects_stale_results_and_removed_targets() {
        let mut document = ProjectDoc::empty("document", "Project");
        document.boards.push(Board {
            id: "board".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let finding = Finding {
            id: "board:board:invalid-settings".into(),
            severity: Severity::Warning,
            scope: FindingScope::Layout,
            message: "Board needs review".into(),
            target_ids: vec!["board".into()],
        };
        let current_source = source(1);
        let mut accepted = KeycapsFitState::begin(current_source.clone(), None);
        accepted.finish(Ok(KeycapResolution {
            revision: 1,
            specs: vec![],
            findings: vec![finding.clone()],
        }));
        let request = FindingNavigationRequest {
            source: current_source.clone(),
            finding: finding.clone(),
            target: FindingNavigationTarget::Board {
                board_id: "board".into(),
            },
        };
        let owner = super::super::keycaps_navigation::OwnerIdentity {
            scope: &current_source.scope,
            generation: 4,
        };
        let accepted_source = super::super::keycaps_navigation::AcceptedNavigationSource {
            scope: current_source.scope.clone(),
            session_epoch: current_source.scope.session_epoch,
            token: current_source.token,
            revision: current_source.revision,
            active_board_id: "board".into(),
        };
        let admitted = super::super::keycaps_navigation::admit_accepted_request(
            &request,
            super::super::keycaps_navigation::NavigationAdmission {
                current_workspace: "Keycaps",
                owner,
                live_scope: Some(&current_source.scope),
                live_generation: 4,
                accepted: &accepted_source,
                fit_state: &accepted,
                document: &document,
                live_mechanical_layers: Some(&[]),
            },
            |_| None,
        )
        .expect("the production admission seam accepts current source and target");
        assert_eq!(
            admitted.target, request.target,
            "the production admission seam retains the accepted target"
        );
        assert_eq!(
            admitted.owner.destination,
            super::super::keycaps_navigation::Destination::Layout(
                super::super::objects::TreeContext::Outline {
                    board_id: "board".into()
                }
            )
        );

        let pending = KeycapsFitState::begin(source(2), Some(&accepted));
        assert!(
            super::super::keycaps_navigation::admit_accepted_request(
                &request,
                super::super::keycaps_navigation::NavigationAdmission {
                    current_workspace: "Keycaps",
                    owner,
                    live_scope: Some(&current_source.scope),
                    live_generation: 4,
                    accepted: &accepted_source,
                    fit_state: &pending,
                    document: &document,
                    live_mechanical_layers: Some(&[]),
                },
                |_| None,
            )
            .is_none()
        );
        assert_eq!(
            accepted_navigation_target(&pending, &request, &document, &[]),
            None,
            "an older displayed result cannot navigate after a same-board revision change"
        );

        let deleted = Finding {
            target_ids: vec!["deleted-part".into()],
            ..finding
        };
        let mut old_result = KeycapsFitState::begin(current_source.clone(), None);
        old_result.finish(Ok(KeycapResolution {
            revision: 1,
            specs: vec![],
            findings: vec![deleted.clone()],
        }));
        let removed_request = FindingNavigationRequest {
            source: current_source,
            finding: deleted,
            target: FindingNavigationTarget::Part {
                board_id: "board".into(),
                part_id: "deleted-part".into(),
            },
        };
        assert_eq!(
            accepted_navigation_target(&old_result, &removed_request, &document, &[]),
            None,
            "a retained finding whose target was removed safely declines navigation"
        );
    }

    #[wasm_bindgen_test]
    fn layout_target_bounds_include_non_keycap_matrix_companions() {
        let mut document = ProjectDoc::empty("doc", "Project");
        document.boards.push(Board {
            id: "board".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec!["matrix/keys/diode-1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.parts.push(Part {
            id: "matrix/keys/diode-1".into(),
            definition_id: "diode".into(),
            reference: "D1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
            keycap: None,
            outline: None,
        });
        let scene = SceneDelta {
            module_scenes: vec![],
            revision: document.revision,
            transaction_id: "accepted".into(),
            changed_ids: vec![],
            transforms: vec![Transform {
                id: "matrix/keys/diode-1".into(),
                pose: Pose2 {
                    at: Vec2 { x: 40.0, y: 70.0 },
                    rotation: 0.0,
                },
            }],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![FindingMarker {
                finding_id: "overlap:SW1-SW2".into(),
                board_id: "board".into(),
                contours: vec![Contour {
                    points: vec![Vec2 { x: -20.0, y: -10.0 }, Vec2 { x: 80.0, y: 110.0 }],
                    hole: false,
                }],
            }],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        };
        assert_eq!(
            layout_navigation_bounds(
                &document,
                &scene,
                &FindingNavigationTarget::Part {
                    board_id: "board".into(),
                    part_id: "matrix/keys/diode-1".into(),
                },
            ),
            Some((28.0, 52.0, 58.0, 82.0))
        );
        let part_target = FindingNavigationTarget::Part {
            board_id: "board".into(),
            part_id: "matrix/keys/diode-1".into(),
        };
        assert_eq!(
            finding_navigation_bounds(&document, &scene, "overlap:SW1-SW2", &part_target),
            Some((-32.0, 92.0, -22.0, 122.0)),
            "the marker contour is the final camera target even when the primary Part has narrower bounds"
        );
        assert_eq!(
            finding_navigation_bounds(&document, &scene, "different-finding", &part_target),
            Some((28.0, 52.0, 58.0, 82.0)),
            "an absent matching marker falls back to target bounds"
        );
    }

    #[wasm_bindgen_test]
    fn finding_marker_bounds_include_every_contour_of_the_current_board_finding() {
        let mut scene = SceneDelta {
            module_scenes: vec![],
            revision: 11,
            transaction_id: "accepted".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![
                FindingMarker {
                    finding_id: "overlap:sw1-sw2".into(),
                    board_id: "left".into(),
                    contours: vec![
                        Contour {
                            points: vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 18.0, y: 0.0 }],
                            hole: false,
                        },
                        Contour {
                            points: vec![Vec2 { x: 42.0, y: 10.0 }, Vec2 { x: 60.0, y: 30.0 }],
                            hole: false,
                        },
                    ],
                },
                FindingMarker {
                    finding_id: "overlap:sw1-sw2".into(),
                    board_id: "right".into(),
                    contours: vec![Contour {
                        points: vec![Vec2 {
                            x: -500.0,
                            y: -500.0,
                        }],
                        hole: false,
                    }],
                },
                FindingMarker {
                    finding_id: "other-finding".into(),
                    board_id: "left".into(),
                    contours: vec![Contour {
                        points: vec![Vec2 {
                            x: -100.0,
                            y: -100.0,
                        }],
                        hole: false,
                    }],
                },
            ],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        };

        assert_eq!(
            finding_marker_bounds(&scene, "overlap:sw1-sw2", "left"),
            Some((-12.0, 72.0, -12.0, 42.0)),
            "the fit includes all same-finding contours and excludes other boards/findings"
        );
        scene.finding_markers[0].contours.clear();
        assert_eq!(
            finding_marker_bounds(&scene, "overlap:sw1-sw2", "left"),
            None,
            "an empty marker leaves target-bounds fallback available"
        );
    }

    fn mounted_action() -> Element {
        let mut navigated = use_signal(|| false);
        let accepted_source = source(1);
        let finding = Finding {
            id: "board:board:invalid-settings".into(),
            severity: Severity::Error,
            scope: FindingScope::Layout,
            message: "Invalid board setting".into(),
            target_ids: vec!["board".into()],
        };
        let mut accepted = KeycapsFitState::begin(accepted_source.clone(), None);
        accepted.finish(Ok(KeycapResolution {
            revision: 1,
            specs: vec![],
            findings: vec![finding],
        }));
        let mut document = ProjectDoc::empty("doc", "Project");
        document.boards.push(Board {
            id: "board".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let on_navigate = EventHandler::new(move |request: FindingNavigationRequest| {
            navigated.set(
                request.target
                    == FindingNavigationTarget::Board {
                        board_id: "board".into(),
                    },
            );
        });
        rsx! {
            div {
                KeycapsFitInspector {
                    document: Rc::new(document),
                    state: Some(accepted),
                    mechanical_layer_ids: Rc::from([]),
                    on_retry: EventHandler::new(|()| {}),
                    on_navigate,
                }
                p { id: "fit-navigation-received", "{navigated()}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_finding_action_uses_react_label_and_emits_current_target() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(mounted_action),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        let action = root
            .query_selector(".wb-finding-action")
            .unwrap()
            .expect("current board target has an action");
        assert_eq!(
            action.text_content().as_deref(),
            Some("Select affected geometry")
        );
        action.dyn_ref::<web_sys::HtmlElement>().unwrap().click();
        gloo_timers::future::TimeoutFuture::new(50).await;
        let received = root
            .query_selector("#fit-navigation-received")
            .unwrap()
            .unwrap()
            .text_content();
        assert_eq!(received.as_deref(), Some("true"));
        root.remove();
    }
}
