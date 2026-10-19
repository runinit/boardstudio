//! Complete collision checks over captured Case inputs. Runtime owns execution and
//! currency; the Case view owns controls, presentation and view lifetime.
use super::{CadScene, Runtime};
use crate::case_preview::NativePreviewSnapshot;
use crate::model_delivery::{
    CadCollisionPairFinding, CadCollisionSubject, ModelDeliveryRows,
    physical_pcb_model_row_transform_f64,
};
use std::rc::Rc;

/// The accepted scene, native preview and delivered model rows captured for one check.
#[derive(Clone, PartialEq)]
pub struct CollisionCheckInput {
    scene: Rc<CadScene>,
    preview: Option<Rc<NativePreviewSnapshot>>,
    rows: Option<ModelDeliveryRows>,
}

impl CollisionCheckInput {
    /// Capture the source inputs displayed by the Case view.
    pub fn new(
        scene: Rc<CadScene>,
        preview: Option<Rc<NativePreviewSnapshot>>,
        rows: Option<ModelDeliveryRows>,
    ) -> Self {
        Self {
            scene,
            preview,
            rows,
        }
    }

    fn is_current(&self, runtime: &Runtime) -> bool {
        runtime.scope().as_ref() == Some(&self.scene.scope)
            && runtime.model().accepted.is_some_and(|accepted| {
                accepted.token == self.scene.token
                    && accepted.document.revision == self.scene.snapshot.document.revision
            })
            && runtime
                .cad_scene()
                .is_some_and(|scene| Rc::ptr_eq(&scene, &self.scene))
            && runtime.case_gesture_preview_scene(&self.scene).is_none()
            && self.preview.as_ref().is_none_or(|expected| {
                runtime
                    .native_case_preview()
                    .is_some_and(|current| current == *expected)
                    && runtime.native_model_delivery(expected) == self.rows
            })
    }

    fn is_ready(&self, runtime: &Runtime) -> bool {
        self.scene.exact
            && self.is_current(runtime)
            && (self
                .preview
                .as_ref()
                .is_none_or(|preview| preview.preview.models.is_empty())
                || self
                    .rows
                    .as_ref()
                    .is_some_and(|rows| rows.pending.is_empty()))
    }
}

/// One settled collision check: the findings for every unordered pair attempted, the
/// errors that did not stop the remaining pairs, and the fewer-than-two-operands note.
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionCheckReport {
    pub findings: Vec<CadCollisionPairFinding>,
    pub errors: Vec<String>,
    pub message: Option<String>,
}

impl Runtime {
    /// Whether this captured input is the current exact assembly with no pending model
    /// work, so a check may start now.
    pub fn collision_check_ready(&self, input: &CollisionCheckInput) -> bool {
        input.is_ready(self)
    }

    /// Run one complete collision check over `input`. The runtime retires the previous
    /// check, binds the new batch to the captured input and the caller's view lifetime,
    /// and settles only a report produced while both stayed current. `None` is a retired
    /// or unavailable request, never an apparently successful empty report.
    pub async fn check_collisions(
        self: &Rc<Self>,
        input: CollisionCheckInput,
        clearance_threshold_mm: f64,
        requested_accuracy_mm: Option<f64>,
        is_view_current: Rc<dyn Fn() -> bool>,
    ) -> Option<CollisionCheckReport> {
        if !clearance_threshold_mm.is_finite() || clearance_threshold_mm < 0.0 {
            return None;
        }
        if !is_view_current() || !self.collision_check_ready(&input) {
            return None;
        }
        self.cancel_collision_checks();
        let expected = input.clone();
        let weak = Rc::downgrade(self);
        let input_is_current: Rc<dyn Fn() -> bool> = Rc::new(move || {
            weak.upgrade()
                .is_some_and(|runtime| expected.is_current(&runtime))
        });
        let current =
            self.collision_current(Rc::new(move || is_view_current() && input_is_current()));
        let report = run_checks(
            self,
            &input,
            clearance_threshold_mm,
            requested_accuracy_mm,
            current.clone(),
        )
        .await?;
        current().then_some(report)
    }
}

async fn run_checks(
    runtime: &Rc<Runtime>,
    input: &CollisionCheckInput,
    clearance_threshold_mm: f64,
    requested_accuracy_mm: Option<f64>,
    current: Rc<dyn Fn() -> bool>,
) -> Option<CollisionCheckReport> {
    let scene = &input.scene;
    let mut subjects = Vec::new();
    let mut errors = Vec::new();
    if let (Some(preview), Some(rows)) = (&input.preview, &input.rows) {
        for model in &rows.delivered {
            let placement = preview.preview.models.iter().find(|row| row.id == model.id);
            let transform = model.matrix.map(Ok).unwrap_or_else(|| {
                placement
                    .ok_or_else(|| "model placement is missing".to_owned())
                    .and_then(|row| {
                        physical_pcb_model_row_transform_f64(
                            row,
                            preview.preview.thickness,
                            preview.board_reference.as_ref(),
                            scene
                                .mechanical
                                .as_ref()
                                .is_some_and(|assembly| !assembly.stack.is_empty()),
                        )
                    })
            });
            match transform {
                Ok(row_transform_f64) => {
                    let mut subject = CadCollisionSubject::VendorModel {
                        id: model.id.clone(),
                        reference: placement
                            .map(|row| row.reference.clone())
                            .unwrap_or_else(|| model.id.clone()),
                        row_transform_f64,
                        vendor_step: model.mesh.vendor_step.clone(),
                    };
                    if let Some(target) = requested_accuracy_mm {
                        match runtime
                            .refine_collision_vendor(scene, &subject, target, current.clone())
                            .await
                        {
                            Ok(refined) => subject = refined,
                            Err(error) => {
                                errors.push(error);
                                if let CadCollisionSubject::VendorModel { vendor_step, .. } =
                                    &mut subject
                                {
                                    *vendor_step = None;
                                }
                            }
                        }
                    }
                    subjects.push(subject);
                }
                Err(error) => errors.push(format!("{}: {error}", model.id)),
            }
            if !current() {
                return None;
            }
        }
        errors.extend(
            rows.failures
                .iter()
                .map(|failure| format!("{}: {}", failure.reference, failure.reason)),
        );
    }
    let (generated, generated_errors) = runtime
        .generated_collision_subjects(scene, requested_accuracy_mm, current.clone())
        .await;
    subjects.extend(generated);
    errors.extend(generated_errors);
    if !current() {
        return None;
    }
    let mut findings = Vec::new();
    for (index, left) in subjects.iter().enumerate() {
        for right in &subjects[index + 1..] {
            match runtime
                .query_collision_subjects(
                    left,
                    right,
                    clearance_threshold_mm,
                    requested_accuracy_mm,
                    scene.scope.clone(),
                    scene.token,
                    scene.snapshot.document.revision,
                    current.clone(),
                )
                .await
            {
                Ok(result) => findings.push(CadCollisionPairFinding {
                    left_id: left.id().into(),
                    left_label: left.label().into(),
                    right_id: right.id().into(),
                    right_label: right.label().into(),
                    result,
                }),
                Err(error) => errors.push(format!("{} ↔ {}: {error}", left.label(), right.label())),
            }
            if !current() {
                return None;
            }
        }
    }
    let message = (subjects.len() < 2).then(|| {
        "Fewer than two collision operands are available; no pair was checked.".to_owned()
    });
    Some(CollisionCheckReport {
        findings,
        errors,
        message,
    })
}
