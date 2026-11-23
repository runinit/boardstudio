//! Case edit handle geometry and projection.

use crate::case_gesture_math::{MountMoveConstraints, mount_radius};
use boardstudio_core_mechanical::mechanical::change::MechanicalMountCollection as Collection;
use boardstudio_model::model::{
    CaseKind, MechanicalConfiguration, MechanicalGasketSupport, MechanicalMount, Mount,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_runtime::case_generation_lifecycle::CadScene;

#[derive(Clone, Debug, PartialEq)]
pub struct ViewerHandle {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tangent_x: f32,
    pub tangent_y: f32,
    pub normal_x: f32,
    pub normal_y: f32,
    pub length: f32,
    pub invalid: bool,
    pub target: ViewerHandleTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewerHandleTarget {
    Gasket {
        support_id: String,
    },
    Mount {
        owner: CaseMountTarget,
        mount_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseMountTarget {
    Mechanical(Collection),
    Authored { body_id: String },
}

pub(super) fn project_handles(
    scene: &CadScene,
    gaskets_editable: bool,
    configuration: Option<&MechanicalConfiguration>,
    authored_mounts_editable: bool,
) -> Vec<ViewerHandle> {
    if !scene.exact || scene.prepared.revision != scene.snapshot.document.revision {
        return Vec::new();
    }

    let mut handles = Vec::new();
    if gaskets_editable && let Some(assembly) = scene.mechanical.as_ref() {
        let z = assembly
            .stack
            .iter()
            .find(|layer| layer.id == "retainer")
            .map(|layer| layer.z + layer.thickness + 0.7)
            .unwrap_or(9.0);
        handles.extend(
            assembly
                .gasket_supports
                .iter()
                .map(|support| gasket_handle(support, z)),
        );
    }

    if let Some(configuration) = configuration
        && let Some(assembly) = scene.mechanical.as_ref()
    {
        for collection in [Collection::Suspension, Collection::Closure] {
            let (body, mounts) = mechanical_mounts(configuration, collection);
            let z = assembly
                .stack
                .iter()
                .find(|layer| layer.id == body)
                .map_or(0.8, |layer| layer.z + 0.8);
            append_mounts(
                &mut handles,
                scene,
                body,
                mounts,
                z,
                CaseMountTarget::Mechanical(collection),
            );
        }
    }
    if authored_mounts_editable
        && let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope)
    {
        for body in document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scene.scope.board_id)
        {
            append_mounts(
                &mut handles,
                scene,
                &body.id,
                body.mounts.as_deref().unwrap_or_default(),
                body.z.unwrap_or(0.) + 0.8,
                CaseMountTarget::Authored {
                    body_id: body.id.clone(),
                },
            );
        }
    }
    handles
}

fn gasket_handle(support: &MechanicalGasketSupport, z: f64) -> ViewerHandle {
    ViewerHandle {
        id: format!("gasket-handle:{}", support.id),
        x: support.at.x as f32,
        y: support.at.y as f32,
        z: z as f32,
        tangent_x: support.tangent.x as f32,
        tangent_y: support.tangent.y as f32,
        normal_x: support.normal.x as f32,
        normal_y: support.normal.y as f32,
        length: support.length as f32,
        invalid: support.fit_error.is_some(),
        target: ViewerHandleTarget::Gasket {
            support_id: support.id.clone(),
        },
    }
}

pub(super) fn mechanical_mounts(
    configuration: &MechanicalConfiguration,
    collection: Collection,
) -> (&'static str, &[Mount]) {
    match collection {
        Collection::Suspension if configuration.mount == MechanicalMount::Gasket => ("bottom", &[]),
        Collection::Suspension => (
            if configuration.mount == MechanicalMount::Rigid {
                "plate"
            } else {
                "bottom"
            },
            &configuration.mounts,
        ),
        Collection::Closure => (
            "bottom",
            configuration.closure_mounts.as_deref().unwrap_or_default(),
        ),
    }
}

fn append_mounts(
    handles: &mut Vec<ViewerHandle>,
    scene: &CadScene,
    body: &str,
    mounts: &[Mount],
    z: f64,
    target: CaseMountTarget,
) {
    if !mounts.is_empty() && mount_constraints(scene, body).is_some() {
        handles.extend(
            mounts
                .iter()
                .map(|mount| mount_handle(mount, z, target.clone())),
        );
    }
}

fn mount_handle(mount: &Mount, z: f64, target: CaseMountTarget) -> ViewerHandle {
    let name = match &target {
        CaseMountTarget::Mechanical(Collection::Suspension) => "suspension",
        CaseMountTarget::Mechanical(Collection::Closure) => "closure",
        CaseMountTarget::Authored { body_id } => body_id,
    };
    ViewerHandle {
        id: format!("case-mount:{name}/{}", mount.id),
        x: mount.at.x as f32,
        y: mount.at.y as f32,
        z: z as f32,
        tangent_x: 1.0,
        tangent_y: 0.0,
        normal_x: 0.0,
        normal_y: 1.0,
        length: (mount_radius(mount) * 2.0 + 3.0) as f32,
        invalid: false,
        target: ViewerHandleTarget::Mount {
            owner: target,
            mount_id: mount.id.clone(),
        },
    }
}

pub(super) fn gasket_handles(
    original_handles: &[ViewerHandle],
    supports: &[MechanicalGasketSupport],
    invalid: Option<&str>,
) -> Vec<ViewerHandle> {
    let mut result = original_handles.to_vec();
    for support in supports {
        let id = format!("gasket-handle:{}", support.id);
        if let Some(handle) = result.iter_mut().find(|handle| handle.id == id) {
            handle.x = support.at.x as f32;
            handle.y = support.at.y as f32;
            handle.tangent_x = support.tangent.x as f32;
            handle.tangent_y = support.tangent.y as f32;
            handle.normal_x = support.normal.x as f32;
            handle.normal_y = support.normal.y as f32;
            handle.invalid = invalid == Some(support.id.as_str()) || support.fit_error.is_some();
        }
    }
    result
}

pub(super) fn mount_handles(
    original_handles: &[ViewerHandle],
    target: &CaseMountTarget,
    mounts: &[Mount],
    invalid: Option<&str>,
) -> Vec<ViewerHandle> {
    let mut result = original_handles.to_vec();
    for handle in &mut result {
        if let ViewerHandleTarget::Mount { owner, mount_id } = &handle.target
            && owner == target
            && let Some(mount) = mounts.iter().find(|mount| mount.id == *mount_id)
        {
            handle.x = mount.at.x as f32;
            handle.y = mount.at.y as f32;
            handle.invalid = invalid == Some(mount.id.as_str());
        }
    }
    result
}

pub(super) fn mount_constraints(scene: &CadScene, body_id: &str) -> Option<MountMoveConstraints> {
    let body = scene
        .prepared
        .bodies
        .iter()
        .find(|body| body.body.id == body_id)?;
    let base = body.body.z.unwrap_or(0.0);
    let top = base
        + body.body.thickness
        + if body.body.kind == CaseKind::Plate {
            0.0
        } else {
            body.body.wall_height.unwrap_or(0.0)
        };
    let mut holes = body
        .regions
        .iter()
        .flat_map(|region| region.holes.iter().cloned())
        .collect::<Vec<_>>();
    holes.extend(
        body.body
            .openings
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|opening| opening.z < top && opening.z + opening.height > base)
            .map(|opening| opening.points.clone()),
    );
    let outer = body
        .regions
        .iter()
        .map(|region| region.outer.clone())
        .filter(|outer| outer.len() >= 3)
        .collect::<Vec<_>>();
    (!outer.is_empty()).then_some(MountMoveConstraints { outer, holes })
}

pub(super) fn active_case_handles(
    handles: &[ViewerHandle],
    preview: Option<&[ViewerHandle]>,
    edit_mode: Option<usize>,
) -> Vec<ViewerHandle> {
    preview
        .unwrap_or(handles)
        .iter()
        .filter(|handle| match handle.target {
            ViewerHandleTarget::Gasket { .. } => edit_mode == Some(0),
            ViewerHandleTarget::Mount { .. } => edit_mode == Some(1),
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::{MountKind, Vec2};

    #[test]
    fn gasket_draft_keeps_pick_identity_height_and_length() {
        let mut support = MechanicalGasketSupport {
            id: "s1".into(),
            fit_error: None,
            placement: None,
            region_id: "plate".into(),
            outline_key: "outer".into(),
            anchor: 0.25,
            at: Vec2 { x: 2.0, y: 4.0 },
            tangent: Vec2 { x: 1.0, y: 0.0 },
            normal: Vec2 { x: 0.0, y: 1.0 },
            length: 6.0,
            width: 2.0,
            z: 0.0,
            thickness: 1.0,
            pair_id: None,
            mirror_axis: None,
            unlinked: false,
        };
        let handle = gasket_handle(&support, 9.0);
        assert_eq!(handle.id, "gasket-handle:s1");
        assert_eq!(
            (handle.x, handle.y, handle.z, handle.length),
            (2.0, 4.0, 9.0, 6.0)
        );
        support.at = Vec2 { x: 5.0, y: 8.0 };
        support.tangent = Vec2 { x: 0.0, y: 1.0 };
        support.normal = Vec2 { x: -1.0, y: 0.0 };
        let preview = gasket_handles(std::slice::from_ref(&handle), &[support], Some("s1"));
        assert_eq!(
            preview,
            [ViewerHandle {
                x: 5.0,
                y: 8.0,
                tangent_x: 0.0,
                tangent_y: 1.0,
                normal_x: -1.0,
                normal_y: 0.0,
                invalid: true,
                ..handle
            }]
        );
    }

    #[test]
    fn mount_draft_keeps_other_owners_with_the_same_mount_id() {
        let authored = CaseMountTarget::Authored {
            body_id: "lid".into(),
        };
        let mut mount = Mount {
            id: "m1".into(),
            kind: MountKind::Hole,
            at: Vec2 { x: 2.0, y: 4.0 },
            hole_diameter: 3.0,
            boss_diameter: None,
            height: None,
        };
        let lid = mount_handle(&mount, 2.0, authored.clone());
        let closure = mount_handle(
            &mount,
            3.0,
            CaseMountTarget::Mechanical(Collection::Closure),
        );
        assert_eq!(lid.id, "case-mount:lid/m1");
        assert_eq!(closure.id, "case-mount:closure/m1");
        mount.at = Vec2 { x: 8.0, y: 9.0 };
        let preview = mount_handles(
            &[lid.clone(), closure.clone()],
            &authored,
            &[mount],
            Some("m1"),
        );
        assert_eq!(
            preview,
            [
                ViewerHandle {
                    x: 8.0,
                    y: 9.0,
                    invalid: true,
                    ..lid
                },
                closure
            ]
        );
    }
}
