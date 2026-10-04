//! Page-private source capture for isolated Parts library samples.
//!
//! Parts previews borrow the active project's source assets and accepted
//! selection identity, but compile a disposable sample document. They never
//! reuse the live document as the renderer scene or map sample IDs back into
//! the project.

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{
    Asset, Board, Contour, ExportTarget, Part, PartDefinition, Pose2, PrepareExportRequest,
    ProjectDoc, Side, Vec2,
};
use std::{cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartsPreviewOwnerIdentity {
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) accepted_revision: u64,
    pub(crate) accepted_document_identity: usize,
    pub(crate) definition_id: String,
    pub(crate) recipe_identity: String,
    pub(crate) source_generation: u64,
    pub(crate) request_token: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct PartsPreviewRecipeMember {
    pub(crate) id: String,
    pub(crate) definition: PartDefinition,
    #[serde(default)]
    pub(crate) assets: Vec<Asset>,
    pub(crate) at: Vec2,
    pub(crate) rotation: f64,
    pub(crate) side: Side,
    pub(crate) generator_parameters: BTreeMap<String, serde_json::Value>,
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
    active_generation: Cell<u64>,
}

impl PartsPreviewLeaseSlot {
    pub(crate) fn select_generation(&self, generation: u64) {
        self.active_generation.set(generation);
        let obsolete = self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|lease| lease.identity.source_generation != generation);
        if obsolete {
            self.invalidate();
        }
    }

    pub(crate) fn replace(&self, lease: Rc<PartsPreviewOwnerLease>) {
        if lease.identity.source_generation != self.active_generation.get() {
            lease.invalidate();
            return;
        }
        if let Some(previous) = self.current.replace(Some(lease)) {
            previous.invalidate();
        }
    }

    pub(crate) fn invalidate_generation(&self, generation: u64) {
        let matches = self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|lease| lease.identity.source_generation == generation);
        if matches {
            self.invalidate();
        }
    }

    pub(crate) fn invalidate(&self) {
        if let Some(current) = self.current.borrow_mut().take() {
            current.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;

    fn lease(generation: u64) -> Rc<PartsPreviewOwnerLease> {
        PartsPreviewOwnerLease::new(PartsPreviewOwnerIdentity {
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: "doc".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            snapshot_token: SnapshotToken(1),
            accepted_revision: 1,
            accepted_document_identity: 1,
            definition_id: "definition".into(),
            recipe_identity: "recipe".into(),
            source_generation: generation,
            request_token: format!("request-{generation}"),
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn late_old_generation_invalidation_cannot_revoke_new_parts_preview_lease() {
        let slot = PartsPreviewLeaseSlot::default();
        slot.select_generation(1);
        let old = lease(1);
        slot.replace(old.clone());

        slot.select_generation(2);
        assert!(!old.is_active(), "selection change retires the old lease");
        let late_old = lease(1);
        slot.replace(late_old.clone());
        assert!(
            !late_old.is_active(),
            "a late old task cannot reclaim the slot"
        );

        let current = lease(2);
        slot.replace(current.clone());
        slot.invalidate_generation(1);
        assert!(
            current.is_active(),
            "the old deferred effect must not revoke the new lease"
        );
        assert!(
            slot.current
                .borrow()
                .as_ref()
                .is_some_and(|lease| Rc::ptr_eq(lease, &current))
        );
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
        members: &[PartsPreviewRecipeMember],
    ) -> Result<Self, String> {
        if request_token.is_empty() || source_generation == 0 || members.is_empty() {
            return Err("Parts preview needs a current request identity".into());
        }
        if snapshot.session_epoch != scope.session_epoch
            || snapshot.document.id != scope.document_id
            || snapshot.scene.revision != snapshot.document.revision
        {
            return Err("Parts preview scope does not match its accepted source".into());
        }

        let recipe_identity = serde_json::to_string(members)
            .map_err(|error| format!("Could not identify the Parts preview recipe: {error}"))?;
        let definition = &members[0].definition;
        let owner = PartsPreviewOwnerIdentity {
            scope: scope.clone(),
            snapshot_token: snapshot.token,
            accepted_revision: snapshot.document.revision,
            accepted_document_identity: Arc::as_ptr(&snapshot.document) as usize,
            definition_id: definition.id.clone(),
            recipe_identity,
            source_generation,
            request_token: request_token.clone(),
        };
        let mut sample_document = ProjectDoc::empty("parts-library-sample", "Parts sample");
        sample_document.revision = snapshot.document.revision;
        sample_document.assets.clone_from(&snapshot.document.assets);
        let mut contours_points = Vec::new();
        for (index, member) in members.iter().enumerate() {
            for asset in &member.assets {
                if let Some(existing) = sample_document
                    .assets
                    .iter()
                    .find(|existing| existing.id == asset.id)
                {
                    if existing != asset {
                        return Err(
                            "Parts preview asset identity conflicts with its accepted source"
                                .into(),
                        );
                    }
                } else {
                    sample_document.assets.push(asset.clone());
                }
            }
            if !sample_document
                .definitions
                .iter()
                .any(|entry| entry.id == member.definition.id)
            {
                sample_document.definitions.push(member.definition.clone());
            }
            let part_id = format!("parts-sample-{index}");
            sample_document.parts.push(Part {
                keycap: member.definition.keycap,
                outline: None,
                id: part_id,
                definition_id: member.definition.id.clone(),
                reference: format!("P{}", index + 1),
                pose: Pose2 {
                    at: member.at,
                    rotation: member.rotation,
                },
                side: member.side.clone(),
                locked: None,
                properties: None,
                generator_parameters: Some(
                    member
                        .definition
                        .terminals
                        .keys()
                        .map(|terminal| {
                            (terminal.clone(), serde_json::Value::String(String::new()))
                        })
                        .chain(member.generator_parameters.clone())
                        .collect(),
                ),
            });
            contours_points.extend(sample_envelope_points(member));
        }
        let contours = sample_contour(contours_points);
        sample_document.boards.push(Board {
            id: "sample-board".into(),
            name: "Sample PCB".into(),
            outline_ids: Vec::new(),
            part_ids: sample_document
                .parts
                .iter()
                .map(|part| part.id.clone())
                .collect(),
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

fn sample_envelope_points(member: &PartsPreviewRecipeMember) -> Vec<Vec2> {
    let keycap_points = member
        .definition
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
                    y: -size.y / 2.0,
                },
                Vec2 {
                    x: size.x / 2.0,
                    y: size.y / 2.0,
                },
                Vec2 {
                    x: -size.x / 2.0,
                    y: size.y / 2.0,
                },
            ]
        })
        .unwrap_or_default();
    let points = member
        .definition
        .courtyard
        .iter()
        .chain(&keycap_points)
        .collect::<Vec<_>>();
    let rotation = member.rotation.to_radians();
    points
        .iter()
        .map(|point| Vec2 {
            x: member.at.x + point.x * rotation.cos() - point.y * rotation.sin(),
            y: member.at.y + point.x * rotation.sin() + point.y * rotation.cos(),
        })
        .collect()
}

fn sample_contour(points: Vec<Vec2>) -> Vec<Contour> {
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
