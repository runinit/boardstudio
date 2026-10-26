//! Ticket 06: view-only value popup and commit toast state.
//!
//! While a Layout transform tool is chosen or dragged, the canvas shows a
//! value popup (field name, value, live delta); each committed drag raises a
//! commit toast ("… · 1 Undo step") for 1.8 s. Both are view state only: they
//! never reach the document, history, saves or the selection, and the canvas
//! only renders what this reducer returns. Native-tested like
//! `spacing_ghost.rs`; rendering and timers live in the overlay.

/// A drag delta in display units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FeedbackDelta {
    /// Millimetres, e.g. a Spread or Stagger step.
    Mm(f64),
    /// Millimetre pair, e.g. a Shift or Offset move.
    Mm2(f64, f64),
    /// Degrees, e.g. a Bend or Splay turn.
    Deg(f64),
}

impl FeedbackDelta {
    /// Signed display text with the prototype's plus and U+2212 minus:
    /// "+1.00 mm", "−1.50 mm", "+8.00, +4.00 mm", "+1.00°".
    pub fn text(&self) -> String {
        match *self {
            Self::Mm(value) => format!("{} mm", signed_two(value)),
            Self::Mm2(x, y) => format!("{}, {} mm", signed_two(x), signed_two(y)),
            Self::Deg(value) => format!("{}°", signed_two(value)),
        }
    }

    /// True when the drag moved nothing (within float dust): a zero delta
    /// raises no toast.
    pub fn is_zero(&self) -> bool {
        match *self {
            Self::Mm(value) | Self::Deg(value) => value.abs() < 1e-9,
            Self::Mm2(x, y) => x.abs() < 1e-9 && y.abs() < 1e-9,
        }
    }
}

/// "+1.00" / "−1.50": an explicit plus for zero and upward, U+2212 for
/// downward, matching the prototype delta format and `spacing_note`.
fn signed_two(value: f64) -> String {
    if value < 0.0 {
        format!("−{:.2}", value.abs())
    } else {
        format!("+{:.2}", value)
    }
}

/// The value popup content: field name, current value and, while dragging,
/// the live delta. Position (docked above the rail, or following the pointer)
/// is canvas geometry; only the content lives here.
#[derive(Clone, Debug, PartialEq)]
pub struct ValuePopup {
    pub name: String,
    pub value: String,
    pub delta: Option<String>,
}

/// The commit toast content: the committed value in bold and the delta after
/// it. The canvas renders it at the top centre for 1.8 s with `role="status"`.
#[derive(Clone, Debug, PartialEq)]
pub struct CommitToast {
    pub value: String,
    pub delta: String,
}

impl CommitToast {
    /// The announced and asserted toast line:
    /// "Spread 19.00 mm (+1.00 mm) · 1 Undo step".
    pub fn text(&self) -> String {
        format!("{} ({}) · 1 Undo step", self.value, self.delta)
    }
}

/// The popup and toast content behind the canvas chrome.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeedbackState {
    pub popup: Option<ValuePopup>,
    pub toast: Option<CommitToast>,
}

/// What the canvas reports: a tool chosen (docked popup), a live drag
/// (following popup with delta), a committed drag (toast unless the delta is
/// zero), Escape (the chosen tool is gone) and toast expiry.
#[derive(Clone, Debug, PartialEq)]
pub enum FeedbackEvent {
    Choose {
        name: String,
        value: String,
    },
    Drag {
        name: String,
        value: String,
        delta: FeedbackDelta,
    },
    Commit {
        value: String,
        delta: FeedbackDelta,
    },
    Dismiss,
    Expire,
}

/// Advance the popup/toast content for one canvas event. Choosing or dragging
/// never disturbs a visible toast; committing replaces it; Escape clears only
/// the popup (a committed toast is a fact, not a choice); expiry clears only
/// the toast.
pub fn advance_feedback(state: &FeedbackState, event: FeedbackEvent) -> FeedbackState {
    match event {
        FeedbackEvent::Choose { name, value } => FeedbackState {
            popup: Some(ValuePopup {
                name,
                value,
                delta: None,
            }),
            toast: state.toast.clone(),
        },
        FeedbackEvent::Drag { name, value, delta } => FeedbackState {
            popup: Some(ValuePopup {
                name,
                value,
                delta: Some(delta.text()),
            }),
            toast: state.toast.clone(),
        },
        FeedbackEvent::Commit { value, delta } => FeedbackState {
            popup: None,
            toast: if delta.is_zero() {
                state.toast.clone()
            } else {
                Some(CommitToast {
                    value,
                    delta: delta.text(),
                })
            },
        },
        FeedbackEvent::Dismiss => FeedbackState {
            popup: None,
            toast: state.toast.clone(),
        },
        FeedbackEvent::Expire => FeedbackState {
            popup: state.popup.clone(),
            toast: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> FeedbackState {
        FeedbackState::default()
    }

    /// Ticket 06, RED: choosing Spread and committing a +1.00 mm drag gives
    /// the toast "Spread 19.00 mm (+1.00 mm) · 1 Undo step".
    #[test]
    fn choose_then_commit_reports_the_toast() {
        let choosing = advance_feedback(
            &state(),
            FeedbackEvent::Choose {
                name: "Spread".into(),
                value: "19.00 mm".into(),
            },
        );
        assert_eq!(
            choosing.popup,
            Some(ValuePopup {
                name: "Spread".into(),
                value: "19.00 mm".into(),
                delta: None,
            })
        );
        assert_eq!(choosing.toast, None);
        let dragging = advance_feedback(
            &choosing,
            FeedbackEvent::Drag {
                name: "Spread".into(),
                value: "19.00 mm".into(),
                delta: FeedbackDelta::Mm(1.0),
            },
        );
        assert_eq!(
            dragging
                .popup
                .as_ref()
                .and_then(|popup| popup.delta.clone()),
            Some("+1.00 mm".into())
        );
        let committed = advance_feedback(
            &dragging,
            FeedbackEvent::Commit {
                value: "Spread 19.00 mm".into(),
                delta: FeedbackDelta::Mm(1.0),
            },
        );
        assert_eq!(committed.popup, None);
        assert_eq!(
            committed.toast.as_ref().map(|toast| toast.text()),
            Some("Spread 19.00 mm (+1.00 mm) · 1 Undo step".into())
        );
    }

    /// Ticket 06, RED: a zero-delta commit raises no toast and clears the popup.
    #[test]
    fn zero_delta_commit_raises_no_toast() {
        for delta in [
            FeedbackDelta::Mm(0.0),
            FeedbackDelta::Mm2(0.0, 0.0),
            FeedbackDelta::Deg(0.0),
        ] {
            let committed = advance_feedback(
                &state(),
                FeedbackEvent::Commit {
                    value: "Spread 19.00 mm".into(),
                    delta,
                },
            );
            assert_eq!(committed.popup, None, "{delta:?} clears the popup");
            assert_eq!(committed.toast, None, "{delta:?} raises no toast");
        }
    }

    #[test]
    fn dismiss_clears_the_popup_and_keeps_the_toast() {
        let toasted = advance_feedback(
            &state(),
            FeedbackEvent::Commit {
                value: "Spread 19.00 mm".into(),
                delta: FeedbackDelta::Mm(1.0),
            },
        );
        let showing = advance_feedback(
            &toasted,
            FeedbackEvent::Choose {
                name: "Bend".into(),
                value: "-5.0°".into(),
            },
        );
        assert!(showing.popup.is_some() && showing.toast.is_some());
        let dismissed = advance_feedback(&showing, FeedbackEvent::Dismiss);
        assert_eq!(dismissed.popup, None);
        assert_eq!(dismissed.toast, toasted.toast);
    }

    #[test]
    fn expire_clears_the_toast_and_keeps_the_popup() {
        let toasted = advance_feedback(
            &state(),
            FeedbackEvent::Commit {
                value: "Spread 19.00 mm".into(),
                delta: FeedbackDelta::Mm(1.0),
            },
        );
        let showing = advance_feedback(
            &toasted,
            FeedbackEvent::Choose {
                name: "Bend".into(),
                value: "-5.0°".into(),
            },
        );
        let expired = advance_feedback(&showing, FeedbackEvent::Expire);
        assert_eq!(expired.toast, None);
        assert_eq!(expired.popup, showing.popup);
    }

    #[test]
    fn delta_text_signs_every_component_like_the_prototype() {
        assert_eq!(FeedbackDelta::Mm(1.0).text(), "+1.00 mm");
        assert_eq!(FeedbackDelta::Mm(-1.5).text(), "−1.50 mm");
        assert_eq!(FeedbackDelta::Mm2(8.0, -4.0).text(), "+8.00, −4.00 mm");
        assert_eq!(FeedbackDelta::Deg(5.0).text(), "+5.00°");
        assert_eq!(FeedbackDelta::Deg(-5.0).text(), "−5.00°");
        assert!(!FeedbackDelta::Mm(0.1).is_zero());
        assert!(!FeedbackDelta::Mm2(0.0, 0.1).is_zero());
    }
}
