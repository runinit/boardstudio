//! Page-private source capture for isolated Parts library samples.
//!
//! Parts previews borrow the active project's source assets and accepted
//! selection identity, but compile a disposable sample document. They never
//! reuse the live document as the renderer scene or map sample IDs back into
//! the project.

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{
    Board, Contour, ExportTarget, Part, PartDefinition, Pose2, PrepareExportRequest, ProjectDoc,
    Side, Vec2,
};
use std::{cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartsPreviewOwnerIdentity {
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) accepted_revision: u64,
    pub(crate) accepted_document_identity: usize,
    pub(crate) definition_id: String,
    pub(crate) source_generation: u64,
    pub(crate) request_token: String,
}

#[derive(Debug)]
pub(crate) struct PartsPreviewOwnerLease {
    active: Cell<bool>,
    identity: PartsPreviewOwnerIdentity,
}

impl PartsPreviewOwnerLease {
    fn new(identity: PartsPreviewOwnerIdentity) -> Rc<Self> {
        Rc::new(Self {
            active: Cell::new(true),
            identity,
        })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    pub(crate) fn matches(&self, identity: &PartsPreviewOwnerIdentity) -> bool {
        self.is_active() && self.identity == *identity
    }

    pub(crate) fn invalidate(&self) {
        self.active.set(false);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PartsPreviewCapture {
    pub(crate) owner: PartsPreviewOwnerIdentity,
    pub(crate) lease: Rc<PartsPreviewOwnerLease>,
    pub(crate) sample_document: ProjectDoc,
    pub(crate) contours: Vec<Contour>,
    pub(crate) path_assets: BTreeMap<String, String>,
    pub(crate) request: PrepareExportRequest,
    pub(crate) sample_scope: Scope,
}

#[derive(Clone, Debug)]
pub(crate) struct PartsPreviewSnapshot {
    pub(crate) owner: PartsPreviewOwnerIdentity,
    pub(crate) lease: Rc<PartsPreviewOwnerLease>,
    pub(crate) sample_document: ProjectDoc,
    pub(crate) contours: Vec<Contour>,
    pub(crate) preview: boardstudio_core::model::PcbPreview,
    pub(crate) model_rows: Option<crate::presentation::model_delivery::ModelDeliveryRows>,
}

#[derive(Default)]
pub(crate) struct PartsPreviewLeaseSlot {
    current: std::cell::RefCell<Option<Rc<PartsPreviewOwnerLease>>>,
}

impl PartsPreviewLeaseSlot {
    pub(crate) fn replace(&self, lease: Rc<PartsPreviewOwnerLease>) {
        if let Some(previous) = self.current.replace(Some(lease)) {
            previous.invalidate();
        }
    }

    pub(crate) fn invalidate(&self) {
        if let Some(current) = self.current.borrow_mut().take() {
            current.invalidate();
        }
    }
}

impl Drop for PartsPreviewLeaseSlot {
    fn drop(&mut self) {
        self.invalidate();
    }
}

impl PartialEq for PartsPreviewSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && Rc::ptr_eq(&self.lease, &other.lease)
    }
}

impl Eq for PartsPreviewSnapshot {}

impl PartsPreviewCapture {
    pub(crate) fn capture(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
        request_token: String,
        definition: &PartDefinition,
    ) -> Result<Self, String> {
        if request_token.is_empty() || source_generation == 0 {
            return Err("Parts preview needs a current request identity".into());
        }
        if snapshot.session_epoch != scope.session_epoch
            || snapshot.document.id != scope.document_id
            || snapshot.scene.revision != snapshot.document.revision
        {
            return Err("Parts preview scope does not match its accepted source".into());
        }

        let owner = PartsPreviewOwnerIdentity {
            scope: scope.clone(),
            snapshot_token: snapshot.token,
            accepted_revision: snapshot.document.revision,
            accepted_document_identity: Arc::as_ptr(&snapshot.document) as usize,
            definition_id: definition.id.clone(),
            source_generation,
            request_token: request_token.clone(),
        };
        let mut sample_document = ProjectDoc::empty("parts-library-sample", "Parts sample");
        sample_document.revision = snapshot.document.revision;
        sample_document.assets.clone_from(&snapshot.document.assets);
        sample_document.definitions.push(definition.clone());
        let sample_part = Part {
            keycap: definition.keycap.clone(),
            outline: None,
            id: "parts-sample-0".into(),
            definition_id: definition.id.clone(),
            reference: "P1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: Some(
                definition
                    .terminals
                    .keys()
                    .map(|terminal| (terminal.clone(), serde_json::Value::String(String::new())))
                    .collect(),
            ),
        };
        sample_document.parts.push(sample_part);
        let contours = sample_contour(definition);
        sample_document.boards.push(Board {
            id: "sample-board".into(),
            name: "Sample PCB".into(),
            outline_ids: Vec::new(),
            part_ids: vec!["parts-sample-0".into()],
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let sample_scope = Scope {
            session_epoch: scope.session_epoch,
            document_id: sample_document.id.clone(),
            board_id: "sample-board".into(),
            instance_id: None,
        };
        let path_assets = crate::case_preview::preview_model_paths(&sample_document);
        let request = PrepareExportRequest {
            snapshot_token: request_token,
            expected_revision: sample_document.revision,
            document: sample_document.clone(),
            target: ExportTarget::Board {
                board_id: sample_scope.board_id.clone(),
            },
            contours: contours.clone(),
            model_paths: path_assets.clone(),
        };
        let lease = PartsPreviewOwnerLease::new(owner.clone());
        Ok(Self {
            owner,
            lease,
            sample_document,
            contours,
            path_assets,
            request,
            sample_scope,
        })
    }

    pub(crate) fn accept_preview(
        self,
        preview: boardstudio_core::model::PcbPreview,
        model_rows: Option<crate::presentation::model_delivery::ModelDeliveryRows>,
    ) -> Result<PartsPreviewSnapshot, String> {
        if preview.revision != self.owner.accepted_revision
            || self.request.expected_revision != self.sample_document.revision
            || self.request.document.id != self.sample_document.id
            || self.request.target
                != (ExportTarget::Board {
                    board_id: self.sample_scope.board_id.clone(),
                })
        {
            return Err("Core returned a Parts preview for a different sample source".into());
        }
        Ok(PartsPreviewSnapshot {
            owner: self.owner,
            lease: self.lease,
            sample_document: self.sample_document,
            contours: self.contours,
            preview,
            model_rows,
        })
    }
}

fn sample_contour(definition: &PartDefinition) -> Vec<Contour> {
    let keycap_points = definition
        .keycap
        .filter(|size| size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0)
        .map(|size| {
            vec![
                Vec2 {
                    x: -size.x / 2.0,
                    y: -size.y / 2.0,
                },
                Vec2 {
                    x: size.x / 2.0,
                    y: size.y / 2.0,
                },
            ]
        })
        .unwrap_or_default();
    let points = definition
        .courtyard
        .iter()
        .chain(&keycap_points)
        .collect::<Vec<_>>();
    let (min_x, max_x, min_y, max_y) = if points.is_empty() {
        (-10.0, 10.0, -10.0, 10.0)
    } else {
        points.iter().fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, max_x, min_y, max_y), point| {
                (
                    min_x.min(point.x),
                    max_x.max(point.x),
                    min_y.min(point.y),
                    max_y.max(point.y),
                )
            },
        )
    };
    let (min_x, max_x, min_y, max_y) = (min_x - 3.0, max_x + 3.0, min_y - 3.0, max_y + 3.0);
    vec![Contour {
        hole: false,
        points: vec![
            Vec2 { x: min_x, y: min_y },
            Vec2 { x: max_x, y: min_y },
            Vec2 { x: max_x, y: max_y },
            Vec2 { x: min_x, y: max_y },
        ],
    }]
}
