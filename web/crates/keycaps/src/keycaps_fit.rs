//! Accepted-snapshot keycap-fit request lifecycle and contextual findings presentation.
#[cfg(target_arch = "wasm32")]
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{
    Contour, Finding, KeycapResolution, Matrix, Part, PartKind, Pose2, ProjectDoc, SceneDelta,
    Scope as FindingScope, Severity, Side, Vec2,
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
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsFitSource {
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
pub struct KeycapsFitState {
    source: KeycapsFitSource,
    accepted: Option<AcceptedFit>,
    refreshing: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindingNavigationTarget {
    MechanicalLayer { board_id: String, layer_id: String },
    Outline { board_id: String },
    Part { board_id: String, part_id: String },
    Matrix { board_id: String, matrix_id: String },
    Body { board_id: String, body_id: String },
    Board { board_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FindingNavigationRequest {
    pub source: KeycapsFitSource,
    pub finding: Finding,
    pub target: FindingNavigationTarget,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraBasis {
    pub bounds: (f64, f64, f64, f64),
    pub surface: (f64, f64),
}

#[derive(Clone)]
pub struct KeycapsFitActions {
    pub state: Option<KeycapsFitState>,
    pub on_retry: EventHandler<()>,
}

impl KeycapsFitState {
    fn begin(source: KeycapsFitSource, previous: Option<&Self>) -> Self {
        let accepted = previous
            .and_then(|p| p.accepted.as_ref())
            .filter(|a| a.source.same_board(&source))
            .cloned();
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
                .is_some_and(|a| a.source == self.source)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn current_preview_input(&self) -> Option<crate::runtime::KeycapsPreviewInput> {
        let accepted = self.accepted.as_ref()?;
        (self.is_current()
            && accepted.result.revision == self.source.revision
            && !accepted
                .result
                .findings
                .iter()
                .any(|f| f.severity == Severity::Error)
            && !accepted.result.specs.is_empty())
        .then(|| crate::runtime::KeycapsPreviewInput {
            scope: self.source.scope.clone(),
            token: self.source.token,
            revision: self.source.revision,
            specs: accepted.result.specs.clone(),
        })
    }

    pub fn accepts_navigation(
        &self,
        request: &FindingNavigationRequest,
        document: &ProjectDoc,
    ) -> bool {
        self.is_current()
            && self.accepted.as_ref().is_some_and(|a| {
                a.source == request.source
                    && presented_findings(&a.result.findings, document)
                        .iter()
                        .any(|f| f == &request.finding)
            })
    }

    pub fn preview_blocker_message(&self) -> Option<String> {
        if self.refreshing {
            return Some("Checking keycap fit…".into());
        }
        if let Some(error) = &self.error {
            return Some(format!("Keycap preview unavailable: {error}"));
        }
        let accepted = self.accepted.as_ref()?;
        if let Some(err) = accepted
            .result
            .findings
            .iter()
            .find(|f| f.severity == Severity::Error)
        {
            return Some(format!("Keycap preview unavailable: {}", err.message));
        }
        accepted
            .result
            .specs
            .is_empty()
            .then(|| "Choose a keycap profile in Keycaps to preview generated caps.".into())
    }
}

#[cfg(target_arch = "wasm32")]
pub fn use_keycaps_fit(
    runtime: Rc<Runtime>,
    source: Option<KeycapsFitSource>,
) -> KeycapsFitActions {
    let state = use_signal(|| None);
    use_keycaps_fit_with_state(runtime, source, state)
}

#[cfg(target_arch = "wasm32")]
pub fn use_keycaps_fit_with_state(
    runtime: Rc<Runtime>,
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

#[derive(Clone, Debug, PartialEq)]
pub struct FindingGroup {
    pub label: String,
    pub findings: Vec<Finding>,
}

pub fn grouped_findings(findings: &[Finding], document: &ProjectDoc) -> Vec<FindingGroup> {
    let mut groups: Vec<FindingGroup> = Vec::new();
    for f in presented_findings(findings, document) {
        let label = finding_target_label(&f, document)
            .unwrap_or_else(|| format!("{} review", scope_title(&f.scope)));
        if let Some(g) = groups.iter_mut().find(|g| g.label == label) {
            g.findings.push(f);
        } else {
            groups.push(FindingGroup {
                label,
                findings: vec![f],
            });
        }
    }
    groups
}

pub fn presented_findings(findings: &[Finding], document: &ProjectDoc) -> Vec<Finding> {
    let board_ids: HashSet<&str> = document.boards.iter().map(|b| b.id.as_str()).collect();
    let mut result: Vec<Finding> = Vec::new();
    let mut indices = HashMap::<String, usize>::new();
    for f in findings {
        let source =
            f.id.find(":feature:")
                .map_or(f.id.as_str(), |i| &f.id[i + 9..]);
        let mut geom: Vec<&str> = f
            .target_ids
            .iter()
            .map(String::as_str)
            .filter(|id| !board_ids.contains(id))
            .collect();
        if geom.is_empty() {
            geom = f.target_ids.iter().map(String::as_str).collect();
        }
        geom.sort_unstable();
        let key = serde_json::to_string(&(source, severity_rank(&f.severity), &f.message, geom))
            .expect("finding key serializable");
        if let Some(&idx) = indices.get(&key) {
            for tid in &f.target_ids {
                if !result[idx].target_ids.contains(tid) {
                    result[idx].target_ids.push(tid.clone());
                }
            }
        } else {
            indices.insert(key, result.len());
            result.push(f.clone());
        }
    }
    result.sort_by_key(|f| severity_rank(&f.severity));
    result
}

fn severity_rank(s: &Severity) -> u8 {
    match s {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

fn scope_title(s: &FindingScope) -> &'static str {
    match s {
        FindingScope::Layout => "Layout",
        FindingScope::Outline => "Outline",
        FindingScope::Pcb => "Pcb",
        FindingScope::Case => "Case",
    }
}

fn find_outline_hit<'a>(finding: &Finding, doc: &'a ProjectDoc) -> (Option<&'a str>, bool) {
    let active = doc.board_outlines.iter().find_map(|o| {
        o.versions
            .iter()
            .find(|v| {
                Some(&v.id) == o.active_version_id.as_ref()
                    && v.geometry
                        .features
                        .iter()
                        .any(|f| finding.target_ids.iter().any(|id| id == f.id()))
            })
            .map(|_| o.board_id.as_str())
    });
    if active.is_some() {
        return (active, true);
    }
    let feat = doc
        .outline
        .iter()
        .find(|f| finding.target_ids.iter().any(|id| id == f.id()));
    let board = feat.and_then(|f| {
        doc.boards
            .iter()
            .find(|b| b.outline_ids.iter().any(|id| id == f.id()))
            .map(|b| b.id.as_str())
    });
    (board, feat.is_some())
}

pub fn finding_target_label(finding: &Finding, doc: &ProjectDoc) -> Option<String> {
    let (outline_board, has_outline) = find_outline_hit(finding, doc);
    let board = doc.boards.iter().find(|b| {
        finding.target_ids.iter().any(|id| id == &b.id) || outline_board == Some(b.id.as_str())
    });
    if has_outline {
        return Some(format!(
            "{} · Outline",
            board.map_or("Project", |b| b.name.as_str())
        ));
    }
    if let Some(part) = doc
        .parts
        .iter()
        .find(|p| finding.target_ids.iter().any(|id| id == &p.id))
    {
        let name = doc
            .definitions
            .iter()
            .find(|d| d.id == part.definition_id)
            .map_or("Component", |d| d.name.as_str());
        return Some(format!("{} · {name}", part.reference));
    }
    doc.matrices
        .iter()
        .find(|m| finding.target_ids.iter().any(|id| id == &m.id))
        .and_then(|m| m.name.clone())
        .or_else(|| {
            doc.case_bodies
                .iter()
                .find(|b| finding.target_ids.iter().any(|id| id == &b.id))
                .map(|b| b.name.clone())
        })
        .or_else(|| board.map(|b| b.name.clone()))
}

pub fn finding_navigation_target(
    finding: &Finding,
    doc: &ProjectDoc,
) -> Option<FindingNavigationTarget> {
    if let (Some(board_id), true) = find_outline_hit(finding, doc) {
        return Some(FindingNavigationTarget::Outline {
            board_id: board_id.to_owned(),
        });
    }
    if let Some(part) = doc
        .parts
        .iter()
        .find(|p| finding.target_ids.iter().any(|id| id == &p.id))
    {
        let b = doc.boards.iter().find(|b| b.part_ids.contains(&part.id))?;
        return Some(FindingNavigationTarget::Part {
            board_id: b.id.clone(),
            part_id: part.id.clone(),
        });
    }
    if let Some(m) = doc
        .matrices
        .iter()
        .find(|m| finding.target_ids.iter().any(|id| id == &m.id))
    {
        let bid = m.board_id.as_ref().or_else(|| {
            doc.boards
                .iter()
                .find(|b| m.part_ids.iter().any(|id| b.part_ids.contains(id)))
                .map(|b| &b.id)
        })?;
        return Some(FindingNavigationTarget::Matrix {
            board_id: bid.clone(),
            matrix_id: m.id.clone(),
        });
    }
    if let Some(body) = doc
        .case_bodies
        .iter()
        .find(|b| finding.target_ids.iter().any(|id| id == &b.id))
    {
        return Some(FindingNavigationTarget::Body {
            board_id: body.board_id.clone(),
            body_id: body.id.clone(),
        });
    }
    doc.boards
        .iter()
        .find(|b| finding.target_ids.iter().any(|id| id == &b.id))
        .map(|b| FindingNavigationTarget::Board {
            board_id: b.id.clone(),
        })
}

pub fn finding_navigation_target_with_layers(
    finding: &Finding,
    doc: &ProjectDoc,
    board_id: &str,
    mechanical_layer_ids: &[String],
) -> Option<FindingNavigationTarget> {
    if let Some(layer_id) = mechanical_layer_ids
        .iter()
        .find(|l| finding.target_ids.contains(l))
    {
        return Some(FindingNavigationTarget::MechanicalLayer {
            board_id: board_id.to_owned(),
            layer_id: layer_id.clone(),
        });
    }
    finding_navigation_target(finding, doc)
}

pub fn accepted_navigation_target(
    state: &KeycapsFitState,
    request: &FindingNavigationRequest,
    doc: &ProjectDoc,
    mechanical_layer_ids: &[String],
) -> Option<FindingNavigationTarget> {
    if !state.accepts_navigation(request, doc) {
        return None;
    }
    let target = finding_navigation_target_with_layers(
        &request.finding,
        doc,
        &request.source.scope.board_id,
        mechanical_layer_ids,
    )?;
    (target_board_id(&target) == request.source.scope.board_id).then_some(target)
}

pub fn layout_navigation_bounds(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    target: &FindingNavigationTarget,
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds = None;
    match target {
        FindingNavigationTarget::Part { part_id, board_id } => {
            let b = doc.boards.iter().find(|b| b.id == *board_id)?;
            if !b.part_ids.contains(part_id) {
                return None;
            }
            let p = doc.parts.iter().find(|p| p.id == *part_id)?;
            include_layout_part_bounds(doc, scene, p, &mut bounds);
        }
        FindingNavigationTarget::Matrix {
            matrix_id,
            board_id,
        } => {
            let b = doc.boards.iter().find(|b| b.id == *board_id)?;
            let m = doc.matrices.iter().find(|m| {
                m.id == *matrix_id && m.part_ids.iter().any(|id| b.part_ids.contains(id))
            })?;
            for p in doc
                .parts
                .iter()
                .filter(|p| b.part_ids.contains(&p.id) && m.part_ids.contains(&p.id))
            {
                include_layout_part_bounds(doc, scene, p, &mut bounds);
            }
            include_matrix_cell_bounds(doc, scene, m, &mut bounds);
        }
        FindingNavigationTarget::Outline { board_id }
        | FindingNavigationTarget::Board { board_id } => {
            include_board_contours(doc, scene, board_id, &mut bounds);
        }
        FindingNavigationTarget::MechanicalLayer { .. } | FindingNavigationTarget::Body { .. } => {
            return None;
        }
    }
    let (x0, x1, y0, y1) = bounds?;
    Some((x0 - 12.0, x1 + 12.0, y0 - 12.0, y1 + 12.0))
}

pub fn selected_part_bounds(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    selected_ids: &[String],
) -> Option<(f64, f64, f64, f64)> {
    if selected_ids.is_empty() {
        return None;
    }
    let b = doc.boards.iter().find(|b| b.id == board_id)?;
    let mut bounds = None;
    for p in doc
        .parts
        .iter()
        .filter(|p| selected_ids.contains(&p.id) && b.part_ids.contains(&p.id))
    {
        include_layout_part_bounds(doc, scene, p, &mut bounds);
    }
    bounds
}

pub fn finding_marker_bounds(
    scene: &SceneDelta,
    finding_id: &str,
    board_id: &str,
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds = None;
    for pt in scene
        .finding_markers
        .iter()
        .filter(|m| m.finding_id == finding_id && m.board_id == board_id)
        .flat_map(|m| m.contours.iter())
        .flat_map(|c| c.points.iter())
        .filter(|p| p.x.is_finite() && p.y.is_finite())
    {
        include_point(&mut bounds, *pt);
    }
    let (x0, x1, y0, y1) = bounds?;
    Some((x0 - 12.0, x1 + 12.0, y0 - 12.0, y1 + 12.0))
}

pub fn finding_navigation_bounds(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    finding_id: &str,
    target: &FindingNavigationTarget,
) -> Option<(f64, f64, f64, f64)> {
    finding_marker_bounds(scene, finding_id, target_board_id(target))
        .or_else(|| layout_navigation_bounds(doc, scene, target))
}

pub fn layout_canvas_bounds(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
) -> Option<(f64, f64, f64, f64)> {
    let board = doc.boards.iter().find(|b| b.id == board_id)?;
    let mut bounds = None;
    include_board_contours(doc, scene, board_id, &mut bounds);
    let visible: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let parts: BTreeMap<_, _> = doc.parts.iter().map(|p| (p.id.as_str(), p)).collect();
    let defs: BTreeMap<_, _> = doc.definitions.iter().map(|d| (d.id.as_str(), d)).collect();
    let xforms: BTreeMap<_, _> = scene
        .transforms
        .iter()
        .map(|t| (t.id.as_str(), t.pose))
        .collect();
    let matrices: Vec<_> = doc
        .matrices
        .iter()
        .filter(|m| {
            m.board_id.as_deref().is_none_or(|o| o == board_id)
                || m.part_ids.iter().any(|id| visible.contains(id.as_str()))
        })
        .collect();
    let mut matrix_caps = BTreeMap::new();
    for m in &matrices {
        let Some(proj) = scene.matrix_scenes.iter().find(|p| p.matrix_id == m.id) else {
            continue;
        };
        for c in proj.cells.iter().filter(|c| c.enabled) {
            if let Some(mid) = c.member_id.as_deref().filter(|id| visible.contains(id)) {
                let part = parts.get(mid).copied();
                let def = part
                    .and_then(|p| defs.get(p.definition_id.as_str()).copied())
                    .or_else(|| defs.get(m.definition_id.as_str()).copied());
                let size = part
                    .and_then(|p| p.keycap)
                    .or_else(|| def.and_then(|d| d.keycap))
                    .unwrap_or(Vec2 {
                        x: (m.pitch.x - m.edge_gap.map_or(1.0, |g| g.x)).max(1.0),
                        y: (m.pitch.y - m.edge_gap.map_or(1.0, |g| g.y)).max(1.0),
                    });
                matrix_caps.insert(mid, size);
            } else {
                include_rotated_rect(&mut bounds, c.pose, m.pitch);
            }
        }
    }
    for p in doc.parts.iter().filter(|p| visible.contains(p.id.as_str())) {
        let pose = xforms.get(p.id.as_str()).copied().unwrap_or(p.pose);
        include_point(&mut bounds, pose.at);
        let def = defs.get(p.definition_id.as_str()).copied();
        let cap = def
            .is_some_and(|d| matches!(d.kind, PartKind::Switch))
            .then(|| {
                matrix_caps
                    .get(p.id.as_str())
                    .copied()
                    .or(p.keycap)
                    .or_else(|| def.and_then(|d| d.keycap))
            })
            .flatten();
        if let Some(size) = cap {
            include_rotated_rect(&mut bounds, pose, size);
        } else if let Some(d) = def {
            for pt in &d.courtyard {
                include_local_point(&mut bounds, pose, false, *pt);
            }
        }
    }
    let (x0, x1, y0, y1) = bounds.unwrap_or((-35.0, 35.0, -25.0, 25.0));
    Some((x0 - 12.0, x1 + 12.0, y0 - 12.0, y1 + 12.0))
}

pub fn aspect_bounds(bounds: (f64, f64, f64, f64), surface: (f64, f64)) -> (f64, f64, f64, f64) {
    let (x0, x1, y0, y1) = bounds;
    let ratio = surface.0.max(1.0) / surface.1.max(1.0);
    let w = (x1 - x0).max((y1 - y0) * ratio);
    let h = (y1 - y0).max((x1 - x0) / ratio);
    let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    (cx - w * 0.5, cx + w * 0.5, cy - h * 0.5, cy + h * 0.5)
}

pub fn layout_camera_basis(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    surface: (f64, f64),
) -> Option<CameraBasis> {
    Some(CameraBasis {
        bounds: aspect_bounds(layout_canvas_bounds(doc, scene, board_id)?, surface),
        surface,
    })
}

fn include_layout_part_bounds(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    part: &Part,
    bounds: &mut Option<(f64, f64, f64, f64)>,
) {
    let pose = scene
        .transforms
        .iter()
        .find(|t| t.id == part.id)
        .map_or(part.pose, |t| t.pose);
    let def = doc.definitions.iter().find(|d| d.id == part.definition_id);
    if let Some(d) = def.filter(|d| !d.courtyard.is_empty()) {
        for pt in &d.courtyard {
            include_local_point(bounds, pose, matches!(part.side, Side::Back), *pt);
        }
    } else {
        include_point(bounds, pose.at);
    }
    if let Some(size) = part.keycap.or_else(|| def.and_then(|d| d.keycap)) {
        include_rotated_rect(bounds, pose, size);
    }
}

fn include_matrix_cell_bounds(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    m: &Matrix,
    bounds: &mut Option<(f64, f64, f64, f64)>,
) {
    let Some(proj) = scene.matrix_scenes.iter().find(|p| p.matrix_id == m.id) else {
        return;
    };
    for c in proj.cells.iter().filter(|c| c.enabled) {
        let part = c
            .member_id
            .as_deref()
            .and_then(|id| doc.parts.iter().find(|p| p.id == id));
        let def = part
            .and_then(|p| doc.definitions.iter().find(|d| d.id == p.definition_id))
            .or_else(|| doc.definitions.iter().find(|d| d.id == m.definition_id));
        let size = part
            .and_then(|p| p.keycap)
            .or_else(|| def.and_then(|d| d.keycap))
            .unwrap_or(Vec2 {
                x: (m.pitch.x - m.edge_gap.map_or(1.0, |g| g.x)).max(1.0),
                y: (m.pitch.y - m.edge_gap.map_or(1.0, |g| g.y)).max(1.0),
            });
        include_rotated_rect(bounds, c.pose, size);
    }
}

fn include_board_contours(
    doc: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    bounds: &mut Option<(f64, f64, f64, f64)>,
) {
    if let Some(b) = scene.board_contours.iter().find(|b| b.board_id == board_id) {
        for pt in b.contours.iter().flat_map(|c: &Contour| &c.points) {
            include_point(bounds, *pt);
        }
    } else if doc.boards.len() == 1 {
        for pt in scene.contours.iter().flat_map(|c| &c.points) {
            include_point(bounds, *pt);
        }
    }
}

fn include_rotated_rect(bounds: &mut Option<(f64, f64, f64, f64)>, pose: Pose2, size: Vec2) {
    let (hx, hy) = (size.x / 2.0, size.y / 2.0);
    for pt in [
        Vec2 { x: -hx, y: -hy },
        Vec2 { x: -hx, y: hy },
        Vec2 { x: hx, y: -hy },
        Vec2 { x: hx, y: hy },
    ] {
        include_local_point(bounds, pose, false, pt);
    }
}

fn include_local_point(
    bounds: &mut Option<(f64, f64, f64, f64)>,
    pose: Pose2,
    mirrored: bool,
    pt: Vec2,
) {
    let lx = if mirrored { -pt.x } else { pt.x };
    let (sin, cos) = pose.rotation.to_radians().sin_cos();
    include_point(
        bounds,
        Vec2 {
            x: pose.at.x + lx * cos - pt.y * sin,
            y: pose.at.y + lx * sin + pt.y * cos,
        },
    );
}

fn include_point(bounds: &mut Option<(f64, f64, f64, f64)>, pt: Vec2) {
    *bounds = Some(bounds.map_or((pt.x, pt.x, pt.y, pt.y), |(x0, x1, y0, y1)| {
        (x0.min(pt.x), x1.max(pt.x), y0.min(pt.y), y1.max(pt.y))
    }));
}

pub fn target_board_id(target: &FindingNavigationTarget) -> &str {
    match target {
        FindingNavigationTarget::Outline { board_id }
        | FindingNavigationTarget::MechanicalLayer { board_id, .. }
        | FindingNavigationTarget::Part { board_id, .. }
        | FindingNavigationTarget::Matrix { board_id, .. }
        | FindingNavigationTarget::Body { board_id, .. }
        | FindingNavigationTarget::Board { board_id } => board_id,
    }
}

pub fn target_has_live_layout_destination(
    target: &FindingNavigationTarget,
    doc: &ProjectDoc,
) -> bool {
    let bid = target_board_id(target);
    let Some(b) = doc.boards.iter().find(|b| b.id == bid) else {
        return false;
    };
    match target {
        FindingNavigationTarget::Outline { .. } | FindingNavigationTarget::Board { .. } => true,
        FindingNavigationTarget::Part { part_id, .. } => {
            b.part_ids.contains(part_id) && doc.parts.iter().any(|p| p.id == *part_id)
        }
        FindingNavigationTarget::Matrix { matrix_id, .. } => doc.matrices.iter().any(|m| {
            m.id == *matrix_id
                && m.board_id.as_deref().is_none_or(|o| o == bid)
                && m.part_ids.iter().any(|id| b.part_ids.contains(id))
        }),
        FindingNavigationTarget::Body { .. } | FindingNavigationTarget::MechanicalLayer { .. } => {
            false
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_model::model::Board;

    #[test]
    fn presented_findings_deduplicates_and_sorts_by_severity() {
        let mut doc = ProjectDoc::empty("d", "P");
        doc.boards.push(Board {
            id: "b1".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let findings = vec![
            Finding {
                id: "a:feature:dup".into(),
                severity: Severity::Warning,
                scope: FindingScope::Layout,
                message: "W".into(),
                target_ids: vec!["b1".into()],
            },
            Finding {
                id: "b:feature:dup".into(),
                severity: Severity::Warning,
                scope: FindingScope::Layout,
                message: "W".into(),
                target_ids: vec!["b1".into()],
            },
            Finding {
                id: "e1".into(),
                severity: Severity::Error,
                scope: FindingScope::Layout,
                message: "E".into(),
                target_ids: vec!["b1".into()],
            },
        ];
        let out = presented_findings(&findings, &doc);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].severity, Severity::Error);
        assert_eq!(grouped_findings(&findings, &doc)[0].label, "Main");
    }

    #[test]
    fn aspect_bounds_expands_without_cropping() {
        let b = aspect_bounds((0.0, 100.0, 0.0, 50.0), (100.0, 100.0));
        assert_eq!(b, (0.0, 100.0, -25.0, 75.0));
    }
}
