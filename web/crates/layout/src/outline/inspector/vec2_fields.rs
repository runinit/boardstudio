use super::{OutlineCoordinate, Vec2};
use dioxus::prelude::*;

pub(super) fn vec2_fields(
    label: String,
    value: Vec2,
    editable: bool,
    on_commit: impl Fn(Vec2) + 'static,
) -> Element {
    let on_commit = EventHandler::new(on_commit);
    rsx! {
        div { class: "grid gap-2.5",
            OutlineCoordinate {
                label: "{label} X mm",
                value: value.x,
                editable,
                on_commit: move |x| on_commit.call(Vec2 { x, y: value.y }),
            }
            OutlineCoordinate {
                label: "{label} Y mm",
                value: value.y,
                editable,
                on_commit: move |y| on_commit.call(Vec2 { x: value.x, y }),
            }
        }
    }
}
