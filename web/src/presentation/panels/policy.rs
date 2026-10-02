#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum PanelSide {
    Objects,
    Inspector,
}

impl PanelSide {
    pub(super) const fn storage_side(self) -> &'static str {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum PanelMode {
    Pinned,
    Autohide,
    Collapsed,
}

impl PanelMode {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "pinned" => Some(Self::Pinned),
            "autohide" => Some(Self::Autohide),
            "collapsed" => Some(Self::Collapsed),
            _ => None,
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Autohide => "autohide",
            Self::Collapsed => "collapsed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::presentation) struct PanelSettings {
    pub(in crate::presentation) mode: PanelMode,
    pub(in crate::presentation) width: Option<f64>,
}

impl PanelSettings {
    pub(super) const fn default_pinned() -> Self {
        Self {
            mode: PanelMode::Pinned,
            width: None,
        }
    }
}

pub(super) fn decode_settings(
    side: PanelSide,
    mode: Option<&str>,
    width: Option<f64>,
) -> PanelSettings {
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
}
