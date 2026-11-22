//! Authored case-body edits and their validation, shared by the inspector and the viewer's
//! mount drags. Pure model code so it is unit-tested natively.
use boardstudio_model::model::{CaseBody, CaseKind, Gasket, Mount, MountKind, ProjectDoc, Vec2};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    Finite,
    Nonnegative,
    Positive,
}

impl Rule {
    /// The lowest value the inspector offers (Core still validates the commit).
    pub fn min(self) -> Option<f64> {
        match self {
            Self::Finite => None,
            Self::Nonnegative => Some(0.0),
            Self::Positive => Some(0.01),
        }
    }

    fn check(self, value: f64, label: &str) -> Result<(), String> {
        let (ok, need) = match self {
            Self::Finite => (value.is_finite(), "finite"),
            Self::Nonnegative => (value.is_finite() && value >= 0.0, "zero or greater"),
            Self::Positive => (value.is_finite() && value > 0.0, "greater than zero"),
        };
        ok.then_some(())
            .ok_or_else(|| format!("{label} must be {need}."))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyField {
    Thickness,
    Clearance,
    Z,
    WallHeight,
    WallThickness,
    GasketInset,
    GasketWidth,
    GasketDepth,
}

impl BodyField {
    pub fn meta(self) -> (&'static str, Rule) {
        match self {
            Self::Thickness => ("Thickness", Rule::Positive),
            Self::Clearance => ("Clearance", Rule::Nonnegative),
            Self::Z => ("Z offset", Rule::Finite),
            Self::WallHeight => ("Wall height", Rule::Positive),
            Self::WallThickness => ("Wall thickness", Rule::Positive),
            Self::GasketInset => ("Inset", Rule::Nonnegative),
            Self::GasketWidth => ("Width", Rule::Positive),
            Self::GasketDepth => ("Depth", Rule::Positive),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MountField {
    X,
    Y,
    HoleDiameter,
    BossDiameter,
    Height,
}

impl MountField {
    pub fn meta(self) -> (&'static str, Rule) {
        match self {
            Self::X => ("X position", Rule::Finite),
            Self::Y => ("Y position", Rule::Finite),
            Self::HoleDiameter => ("Hole diameter", Rule::Positive),
            Self::BossDiameter => ("Boss diameter", Rule::Positive),
            Self::Height => ("Height", Rule::Positive),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseBodyEdit {
    AddBody,
    SetKind {
        body_id: String,
        kind: CaseKind,
    },
    SetField {
        body_id: String,
        field: BodyField,
        value: f64,
    },
    AddMount {
        body_id: String,
    },
    SetMountKind {
        body_id: String,
        mount_id: String,
        kind: MountKind,
    },
    SetMountField {
        body_id: String,
        mount_id: String,
        field: MountField,
        value: f64,
    },
    SetMountPosition {
        body_id: String,
        mount_id: String,
        at: Vec2,
    },
    RemoveMount {
        body_id: String,
        mount_id: String,
    },
    SetGasket {
        body_id: String,
        gasket: Option<Gasket>,
    },
}

impl CaseBodyEdit {
    pub fn body_id(&self) -> Option<&str> {
        match self {
            Self::AddBody => None,
            Self::SetKind { body_id, .. }
            | Self::SetField { body_id, .. }
            | Self::AddMount { body_id }
            | Self::SetMountKind { body_id, .. }
            | Self::SetMountField { body_id, .. }
            | Self::SetMountPosition { body_id, .. }
            | Self::RemoveMount { body_id, .. }
            | Self::SetGasket { body_id, .. } => Some(body_id),
        }
    }

    /// Apply this edit to `body`, a copy of the accepted body it names.
    pub fn apply(
        &self,
        seed: u64,
        document: &ProjectDoc,
        body: &mut CaseBody,
    ) -> Result<(), String> {
        if self.body_id().is_some_and(|id| id != body.id) {
            return Err("The Case edit target changed before it could be applied.".into());
        }
        match self {
            Self::AddBody => {}
            Self::SetKind { kind, .. } => {
                body.kind = kind.clone();
                body.wall_height.get_or_insert(14.0);
                body.wall_thickness.get_or_insert(2.0);
            }
            Self::SetField { field, value, .. } => {
                let (label, rule) = field.meta();
                rule.check(*value, label)?;
                match field {
                    BodyField::Thickness => body.thickness = *value,
                    BodyField::Clearance => body.clearance = *value,
                    BodyField::Z => body.z = Some(*value),
                    BodyField::WallHeight => body.wall_height = Some(*value),
                    BodyField::WallThickness => body.wall_thickness = Some(*value),
                    BodyField::GasketInset | BodyField::GasketWidth | BodyField::GasketDepth => {
                        let gasket = body
                            .gasket
                            .as_mut()
                            .ok_or("Gasket is no longer available.")?;
                        match field {
                            BodyField::GasketInset => gasket.inset = *value,
                            BodyField::GasketWidth => gasket.width = *value,
                            _ => gasket.depth = *value,
                        }
                    }
                }
            }
            Self::AddMount { .. } => {
                let existing = document
                    .case_bodies
                    .iter()
                    .flat_map(|body| body.mounts.iter().flatten())
                    .map(|mount| mount.id.as_str())
                    .collect::<BTreeSet<_>>();
                let id = unique_id(seed, "case-mount", |candidate| existing.contains(candidate));
                body.mounts.get_or_insert_with(Vec::new).push(Mount {
                    id,
                    at: Vec2::default(),
                    kind: MountKind::Hole,
                    hole_diameter: 2.5,
                    boss_diameter: Some(5.0),
                    height: Some(5.0),
                });
            }
            Self::SetMountKind { mount_id, kind, .. } => {
                mount_mut(body, mount_id)?.kind = kind.clone()
            }
            Self::SetMountField {
                mount_id,
                field,
                value,
                ..
            } => {
                let (label, rule) = field.meta();
                rule.check(*value, label)?;
                let mount = mount_mut(body, mount_id)?;
                match field {
                    MountField::X => mount.at.x = *value,
                    MountField::Y => mount.at.y = *value,
                    MountField::HoleDiameter => mount.hole_diameter = *value,
                    MountField::BossDiameter => mount.boss_diameter = Some(*value),
                    MountField::Height => mount.height = Some(*value),
                }
            }
            Self::SetMountPosition { mount_id, at, .. } => {
                Rule::Finite.check(at.x, "Mount X position")?;
                Rule::Finite.check(at.y, "Mount Y position")?;
                mount_mut(body, mount_id)?.at = *at;
            }
            Self::RemoveMount { mount_id, .. } => {
                let mounts = body
                    .mounts
                    .as_mut()
                    .ok_or("Mount is no longer available.")?;
                let before = mounts.len();
                mounts.retain(|mount| mount.id != *mount_id);
                if mounts.len() == before {
                    return Err("Mount is no longer available.".into());
                }
            }
            Self::SetGasket { gasket, .. } => {
                if let Some(gasket) = gasket {
                    Rule::Nonnegative.check(gasket.inset, "Gasket inset")?;
                    Rule::Positive.check(gasket.width, "Gasket width")?;
                    Rule::Positive.check(gasket.depth, "Gasket depth")?;
                }
                body.gasket = gasket.clone();
            }
        }
        Ok(())
    }
}

pub fn new_case_body(seed: u64, document: &ProjectDoc, board_id: &str) -> Result<CaseBody, String> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("The selected board is unavailable.")?;
    let existing = document
        .case_bodies
        .iter()
        .map(|body| body.id.as_str())
        .collect::<BTreeSet<_>>();
    Ok(CaseBody {
        features: None,
        openings: None,
        id: unique_id(seed, "case-body", |candidate| existing.contains(candidate)),
        name: format!("{} plate", board.name),
        board_id: board.id.clone(),
        kind: CaseKind::Plate,
        thickness: 3.0,
        clearance: 0.5,
        material_id: crate::case_generation_admission::default_case_material(document),
        z: Some(0.0),
        wall_height: Some(14.0),
        wall_thickness: Some(2.0),
        mounts: Some(vec![]),
        pcb_source: None,
        gasket: None,
    })
}

fn mount_mut<'a>(body: &'a mut CaseBody, mount_id: &str) -> Result<&'a mut Mount, String> {
    body.mounts
        .as_mut()
        .and_then(|mounts| mounts.iter_mut().find(|mount| mount.id == mount_id))
        .ok_or_else(|| "Mount is no longer available.".into())
}

fn unique_id(seed: u64, prefix: &str, exists: impl Fn(&str) -> bool) -> String {
    (0_u64..)
        .map(|suffix| format!("{prefix}-{seed}-{suffix}"))
        .find(|candidate| !exists(candidate))
        .expect("identifier space exhausted")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> CaseBody {
        CaseBody {
            features: None,
            openings: None,
            id: "b".into(),
            name: "b".into(),
            board_id: "x".into(),
            kind: CaseKind::Plate,
            thickness: 3.0,
            clearance: 0.5,
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        }
    }

    #[test]
    fn edits_validate_and_apply_to_the_named_body() {
        let document = ProjectDoc::empty("d", "d");
        let mut body = body();
        let edit = |e: CaseBodyEdit, body: &mut CaseBody| e.apply(7, &document, body);
        edit(
            CaseBodyEdit::AddMount {
                body_id: "b".into(),
            },
            &mut body,
        )
        .unwrap();
        let mount_id = body.mounts.as_ref().unwrap()[0].id.clone();
        let set = |field, value| CaseBodyEdit::SetMountField {
            body_id: "b".into(),
            mount_id: mount_id.clone(),
            field,
            value,
        };
        edit(set(MountField::X, 4.0), &mut body).unwrap();
        assert_eq!(body.mounts.as_ref().unwrap()[0].at.x, 4.0);
        assert!(edit(set(MountField::HoleDiameter, 0.0), &mut body).is_err());
        assert!(
            edit(
                CaseBodyEdit::SetField {
                    body_id: "b".into(),
                    field: BodyField::GasketInset,
                    value: 1.0
                },
                &mut body
            )
            .is_err()
        );
        assert!(
            edit(
                CaseBodyEdit::SetField {
                    body_id: "other".into(),
                    field: BodyField::Z,
                    value: 1.0
                },
                &mut body
            )
            .is_err()
        );
        edit(
            CaseBodyEdit::RemoveMount {
                body_id: "b".into(),
                mount_id,
            },
            &mut body,
        )
        .unwrap();
        assert!(body.mounts.unwrap().is_empty());
    }
}
