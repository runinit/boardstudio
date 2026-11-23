//! The one 3D-model transform (offset/rotation/scale) vocabulary, validation and
//! 3-axis `NumericField` row shared by the component, assembly-member and
//! module-profile model editors.
use boardstudio_model::model::{Asset, PartModel, Vec3};
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelVector {
    Offset,
    Rotation,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelAxis {
    X,
    Y,
    Z,
}

pub const MODEL_AXES: [(ModelAxis, &str); 3] = [
    (ModelAxis::X, "X"),
    (ModelAxis::Y, "Y"),
    (ModelAxis::Z, "Z"),
];

/// A freshly attached model: identity offset/rotation, unit scale.
pub fn new_model(asset_id: String) -> PartModel {
    PartModel {
        asset_id,
        offset: Vec3::default(),
        rotation: Vec3::default(),
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }
}

pub fn model_vector(model: &PartModel, field: ModelVector) -> Vec3 {
    match field {
        ModelVector::Offset => model.offset,
        ModelVector::Rotation => model.rotation,
        ModelVector::Scale => model.scale,
    }
}

pub fn axis_value(vector: Vec3, axis: ModelAxis) -> f64 {
    match axis {
        ModelAxis::X => vector.x,
        ModelAxis::Y => vector.y,
        ModelAxis::Z => vector.z,
    }
}

pub fn set_model_axis(model: &mut PartModel, field: ModelVector, axis: ModelAxis, value: f64) {
    let vector = match field {
        ModelVector::Offset => &mut model.offset,
        ModelVector::Rotation => &mut model.rotation,
        ModelVector::Scale => &mut model.scale,
    };
    match axis {
        ModelAxis::X => vector.x = value,
        ModelAxis::Y => vector.y = value,
        ModelAxis::Z => vector.z = value,
    }
}

/// Transform coordinates must be finite; scale must also be positive.
pub fn valid_transform_value(field: ModelVector, value: f64) -> bool {
    value.is_finite() && (field != ModelVector::Scale || value > 0.0)
}

pub fn model_transform_is_valid(model: &PartModel) -> bool {
    [
        ModelVector::Offset,
        ModelVector::Rotation,
        ModelVector::Scale,
    ]
    .iter()
    .all(|field| {
        MODEL_AXES.iter().all(|(axis, _)| {
            valid_transform_value(*field, axis_value(model_vector(model, *field), *axis))
        })
    })
}

/// One labelled `fieldset` for a single transform vector's axis row.
#[component]
pub fn ModelTransformFieldset(
    title: String,
    value: Vec3,
    #[props(default)] labels: Option<[String; 3]>,
    #[props(default)] aria_labels: Option<[String; 3]>,
    #[props(default)] step: Option<f64>,
    #[props(default)] min: Option<f64>,
    #[props(into, optional)] disabled_reason: Option<String>,
    on_commit: EventHandler<(ModelAxis, f64)>,
) -> Element {
    rsx! {
        fieldset { class: "grid gap-2",
            legend { "{title}" }
            ModelVec3Fields {
                value,
                labels: labels.unwrap_or(["X".into(), "Y".into(), "Z".into()]),
                aria_labels,
                step,
                min,
                disabled_reason,
                on_commit,
            }
        }
    }
}

/// Read, store and wrap one dropped model file while the owner stays current:
/// returns the deduplicated `Asset` plus the accepted snapshot it was built
/// against. Both model editors share this flow; only the document edit differs.
pub async fn upload_model_asset(
    file: web_sys::File,
    runtime: &crate::runtime::Runtime,
    extra_assets: &[Asset],
    is_current: impl Fn() -> bool,
    stale_message: &str,
) -> Result<(Asset, boardstudio_application::AcceptedSnapshot), String> {
    let imported = crate::presentation::model_asset_import::read_model_file(file).await?;
    if !is_current() {
        return Err(stale_message.to_owned());
    }
    imported.store(&runtime.store).await?;
    if !is_current() {
        return Err(stale_message.to_owned());
    }
    let current = runtime
        .model()
        .accepted
        .ok_or("The accepted project is unavailable.")?;
    let asset = current
        .document
        .assets
        .iter()
        .chain(extra_assets)
        .find(|asset| asset.sha256 == imported.sha256)
        .cloned()
        .unwrap_or_else(|| {
            let root = format!("model-asset-{}", runtime.operation().0);
            let id = crate::parts_custom_definition::unique_suffixed_id_capped(&root, |id| {
                current
                    .document
                    .assets
                    .iter()
                    .chain(extra_assets)
                    .any(|asset| asset.id == id)
            })
            .unwrap_or_else(|| format!("{root}-new"));
            Asset {
                id,
                name: imported.filename.clone(),
                media_type: imported.media_type.clone(),
                sha256: imported.sha256.clone(),
                license: None,
                source: Some("local file".into()),
            }
        });
    Ok((asset, current))
}

/// One 3-column row of axis `NumericField`s for a single transform vector.
#[component]
pub fn ModelVec3Fields(
    value: Vec3,
    labels: [String; 3],
    #[props(default)] aria_labels: Option<[String; 3]>,
    #[props(default)] step: Option<f64>,
    #[props(default)] min: Option<f64>,
    #[props(into, optional)] disabled_reason: Option<String>,
    on_commit: EventHandler<(ModelAxis, f64)>,
) -> Element {
    rsx! {
        div { class: "grid grid-cols-3 gap-2",
            for (index, (axis, _)) in MODEL_AXES.iter().enumerate() {
                NumericField { key: "{index}", label: labels[index].clone(), value: axis_value(value, *axis),
                    aria_label: aria_labels.as_ref().map(|labels| labels[index].clone()),
                    step, min, disabled_reason: disabled_reason.clone(),
                    on_commit: move |number: f64| on_commit.call((*axis, number)),
                }
            }
        }
    }
}
