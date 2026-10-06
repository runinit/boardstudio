//! Maps pointer positions on a letterboxed SVG canvas to model coordinates.
use boardstudio_core::model::Vec2;
use std::{cell::RefCell, rc::Rc};
use web_sys::SvgElement;

pub struct PointerLocation {
    pub world: Vec2,
    pub x_fraction: f64,
    pub y_fraction: f64,
}

pub fn coordinates(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    pointer: &web_sys::PointerEvent,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Vec2> {
    coordinates_at(
        svg,
        pointer.client_x(),
        pointer.client_y(),
        x,
        y,
        width,
        height,
    )
}

pub fn coordinates_at(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    client_x: i32,
    client_y: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Vec2> {
    let surface = svg.borrow();
    let rect = surface.as_ref()?.get_bounding_client_rect();
    pointer_location(&rect, client_x, client_y, x, y, width, height).map(|location| location.world)
}

pub fn pointer_location(
    rect: &web_sys::DomRect,
    client_x: i32,
    client_y: i32,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Option<PointerLocation> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let scale = (rect.width() / width).min(rect.height() / height);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let content_width = width * scale;
    let content_height = height * scale;
    let left = rect.left() + (rect.width() - content_width) * 0.5;
    let top = rect.top() + (rect.height() - content_height) * 0.5;
    let x_fraction = (f64::from(client_x) - left) / content_width;
    let y_fraction = (f64::from(client_y) - top) / content_height;
    Some(PointerLocation {
        world: Vec2 {
            x: view_x + x_fraction * width,
            y: -(view_y + y_fraction * height),
        },
        x_fraction,
        y_fraction,
    })
}
