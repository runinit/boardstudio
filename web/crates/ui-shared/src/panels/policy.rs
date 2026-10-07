#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelSide {
    Objects,
    Inspector,
}

impl PanelSide {
    pub const fn storage_side(self) -> &'static str {
        match self {
            Self::Objects => "left",
            Self::Inspector => "right",
        }
    }

    const fn width_bounds(self) -> (f64, f64) {
        match self {
            Self::Objects => (200.0, 420.0),
            Self::Inspector => (280.0, 480.0),
        }
    }

    pub fn resize_width(self, requested: f64, available: f64) -> f64 {
        let (minimum, maximum) = self.width_bounds();
        let maximum = maximum.min(available.max(minimum));
        if requested.is_finite() {
            requested.clamp(minimum, maximum)
        } else {
            minimum
        }
    }

    pub fn minimum_width(self) -> f64 {
        self.width_bounds().0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelMode {
    Pinned,
    Autohide,
    Collapsed,
}

impl PanelMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pinned" => Some(Self::Pinned),
            "autohide" => Some(Self::Autohide),
            "collapsed" => Some(Self::Collapsed),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Autohide => "autohide",
            Self::Collapsed => "collapsed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelSettings {
    pub mode: PanelMode,
    pub width: Option<f64>,
}

impl PanelSettings {
    pub const fn default_pinned() -> Self {
        Self {
            mode: PanelMode::Pinned,
            width: None,
        }
    }
}

pub fn decode_settings(side: PanelSide, mode: Option<&str>, width: Option<f64>) -> PanelSettings {
    let (minimum, maximum) = side.width_bounds();
    PanelSettings {
        mode: mode.and_then(PanelMode::parse).unwrap_or(PanelMode::Pinned),
        width: width
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(minimum, maximum)),
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelMode, PanelSettings, PanelSide, decode_settings};

    #[test]
    fn invalid_mode_keeps_and_clamps_a_valid_left_width() {
        assert_eq!(
            decode_settings(PanelSide::Objects, Some("future-mode"), Some(500.0)),
            PanelSettings {
                mode: PanelMode::Pinned,
                width: Some(420.0),
            }
        );
    }

    #[test]
    fn valid_mode_survives_an_invalid_or_missing_right_width() {
        assert_eq!(
            decode_settings(PanelSide::Inspector, Some("collapsed"), None),
            PanelSettings {
                mode: PanelMode::Collapsed,
                width: None,
            }
        );
        assert_eq!(
            decode_settings(PanelSide::Inspector, Some("autohide"), Some(f64::INFINITY)),
            PanelSettings {
                mode: PanelMode::Autohide,
                width: None,
            }
        );
    }

    #[test]
    fn finite_widths_clamp_at_each_sides_exact_limits() {
        assert_eq!(
            decode_settings(PanelSide::Objects, Some("pinned"), Some(0.0)).width,
            Some(200.0)
        );
        assert_eq!(
            decode_settings(PanelSide::Inspector, Some("pinned"), Some(600.0)).width,
            Some(480.0)
        );
    }

    #[test]
    fn malformed_or_absent_fields_default_independently() {
        assert_eq!(
            decode_settings(PanelSide::Objects, None, Some(350.0)),
            PanelSettings {
                mode: PanelMode::Pinned,
                width: Some(350.0),
            }
        );
        assert_eq!(
            decode_settings(PanelSide::Objects, Some("invalid"), None),
            PanelSettings::default_pinned()
        );
    }

    #[test]
    fn resize_width_respects_panel_and_available_workspace_bounds() {
        assert_eq!(PanelSide::Objects.resize_width(350.0, 520.0), 350.0);
        assert_eq!(PanelSide::Objects.resize_width(420.0, 360.0), 360.0);
        assert_eq!(PanelSide::Inspector.resize_width(100.0, 500.0), 280.0);
        assert_eq!(PanelSide::Inspector.resize_width(600.0, 700.0), 480.0);
    }
}
