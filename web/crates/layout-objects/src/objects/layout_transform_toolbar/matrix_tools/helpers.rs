//! Angular annotation and the Ctrl-modifier listener for the matrix rail overlay.
use super::*;

// Presentation-only angular annotation: the accepted Core scene supplies origin and angle.
pub(super) fn splay_arc(origin: Vec2, end: Vec2, degrees: f64) -> String {
    let dx = end.x - origin.x;
    let dy = end.y - origin.y;
    let radius = dx.hypot(dy);
    let angle = dy.atan2(dx);
    let sweep_degrees = degrees.clamp(-359.9, 359.9);
    let start = angle - sweep_degrees.to_radians();
    let x = origin.x + radius * start.cos();
    let y = origin.y + radius * start.sin();
    let large = u8::from(sweep_degrees.abs() > 180.0);
    let sweep = u8::from(sweep_degrees >= 0.0);
    format!(
        "M{x} {y} A{radius} {radius} 0 {large} {sweep} {} {}",
        end.x, end.y
    )
}

pub(super) fn angle_handle_point(
    projection: &MatrixScene,
    column: u32,
    pitch_y: f64,
    points: &[Vec2],
) -> Vec2 {
    if points.is_empty() {
        let origin = projection
            .columns
            .iter()
            .find(|basis| basis.column == column)
            .map_or(Vec2::default(), |basis| basis.splay_origin);
        return Vec2 {
            x: origin.x,
            y: origin.y + 20.0,
        };
    }
    let x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min)
        - pitch_y * 0.9;
    let y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        + pitch_y * 0.9;
    Vec2 { x, y }
}

// The modifier changes presentation immediately; pointer-down latches the edit intent.
pub(super) struct CtrlModifierListener {
    pub(super) document: web_sys::Document,
    pub(super) window: web_sys::Window,
    pub(super) down: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    pub(super) up: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    pub(super) blur: Closure<dyn FnMut(web_sys::Event)>,
}
impl CtrlModifierListener {
    pub(super) fn install(mut pressed: Signal<bool>) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                pressed.set(true);
            }
        }) as Box<dyn FnMut(_)>);
        let up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                pressed.set(false);
            }
        }) as Box<dyn FnMut(_)>);
        let blur = Closure::wrap(
            Box::new(move |_: web_sys::Event| pressed.set(false)) as Box<dyn FnMut(_)>
        );
        let listener = Self {
            document,
            window,
            down,
            up,
            blur,
        };
        listener
            .document
            .add_event_listener_with_callback("keydown", listener.down.as_ref().unchecked_ref())
            .ok()?;
        listener
            .document
            .add_event_listener_with_callback("keyup", listener.up.as_ref().unchecked_ref())
            .ok()?;
        listener
            .window
            .add_event_listener_with_callback("blur", listener.blur.as_ref().unchecked_ref())
            .ok()?;
        Some(listener)
    }
}
impl Drop for CtrlModifierListener {
    fn drop(&mut self) {
        let _ = self
            .document
            .remove_event_listener_with_callback("keydown", self.down.as_ref().unchecked_ref());
        let _ = self
            .document
            .remove_event_listener_with_callback("keyup", self.up.as_ref().unchecked_ref());
        let _ = self
            .window
            .remove_event_listener_with_callback("blur", self.blur.as_ref().unchecked_ref());
    }
}
