//! Pure Case mechanical rules shared by the wasm inspector and native tests: the range each
//! dimension accepts and the stable field id a patch is reported under.
use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalDimension,
};
use boardstudio_model::model::MAX_COORDINATE_MM;

/// `(min, max, step)` the inspector offers for one dimension. Core still validates the commit.
pub fn dimension_range(field: MechanicalDimension) -> (f64, Option<f64>, f64) {
    use MechanicalDimension::*;
    match field {
        OpeningAllowance => (-1.0, Some(1.0), 0.1),
        OpeningHeight | BatteryWidth | BatteryDepth | BatteryHeight | BatteryCableWidth
        | MountHoleDiameter | MountBossDiameter | MountBossHeight | HardwareLength => {
            (0.1, None, 0.1)
        }
        HardwareQuantity => (1.0, None, 1.0),
        GasketSupportLength => (5.0, None, 5.0),
        GasketSupportWidth => (0.5, None, 0.5),
        OpeningBottomZ | OpeningPointX | OpeningPointY | BatteryPositionX | BatteryPositionY
        | BatteryCableExitX | BatteryCableExitY | MountPositionX | MountPositionY
        | CriticalFitFromX | CriticalFitFromY | CriticalFitToX | CriticalFitToY => {
            (-MAX_COORDINATE_MM, Some(MAX_COORDINATE_MM), 0.1)
        }
        _ => (0.0, None, 0.1),
    }
}

pub trait MechanicalSettingsPatchExt {
    /// Groups feedback for the same kind of edit; it is the patch's variant name.
    fn field_id(&self) -> String;
}

impl MechanicalSettingsPatchExt for MechanicalSettingsPatch {
    fn field_id(&self) -> String {
        let debug = format!("{self:?}");
        debug
            .split(['(', ' ', '{'])
            .next()
            .unwrap_or_default()
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_field_ids_follow_the_patch() {
        assert_eq!(
            dimension_range(MechanicalDimension::OpeningAllowance).1,
            Some(1.0)
        );
        assert_eq!(
            dimension_range(MechanicalDimension::HardwareQuantity).0,
            1.0
        );
        assert_eq!(
            dimension_range(MechanicalDimension::WallThickness),
            (0.0, None, 0.1)
        );
        let patch = MechanicalSettingsPatch::SetDimension {
            field: MechanicalDimension::Clearance,
            value: 0.2,
        };
        assert_eq!(patch.field_id(), "SetDimension");
        assert_eq!(MechanicalSettingsPatch::Enable.field_id(), "Enable");
    }
}
