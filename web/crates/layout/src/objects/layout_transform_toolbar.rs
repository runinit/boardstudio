//! Transform-property navigation for the Layout command pill.
use super::layout_toolbar::LayoutSelectionKind;
use super::layout_toolbar::{GestureSnapInputs, LayoutCommandMenu, close_layout_command_menu};
use super::{ScopedTreeContext, TreeContext};
use crate::anchor_pick::{
    AnchorMajorKind, AnchorPickCandidates, CANDIDATE_HIT_PX, CANDIDATE_RING_HOVER_PX,
    CANDIDATE_RING_PX, MAJOR_RING_PX, PickTarget, anchor_key_label, anchor_pick_candidates,
    matrix_display_name, pick_commit, preview_anchor_poses,
};
use crate::matrix_relations;
use crate::matrix_transform_operation::{
    ArrowKey, MatrixDragAnchor, MatrixTransformField, PlacementParameter, PlacementValue,
    SceneBasis, column_angle, gesture_value, handle_rail, matrix_drag_anchor_pose,
    matrix_drag_transport, nudge_value, placement_operation, solve_matrix_origin,
    solve_matrix_rotation,
};
use crate::presentation::{
    LayoutOwnerIdentity,
    canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, TerminalOutcome,
};
use boardstudio_core::evaluated_part_pose;
use boardstudio_core::layout_half_offset;
use boardstudio_core::model::{
    EditOperation, Matrix, MatrixPlacementChange, MatrixScene, MatrixSplayAffect,
    MatrixSplayChange, Mirror, PartAnchor, PartAnchorValueChange, Pose2, Position, ProjectDoc,
    Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEdits;
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_model::transform_feedback::{
    FeedbackDelta, FeedbackEvent, FeedbackState, advance_feedback,
};
use boardstudio_web_ui_shared::controls::{
    BoardButton, BoardButtonDensity, BoardPopover, BoardToolbar, BoardToolbarButton,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::SvgElement;

type TransformEscapeListener = Rc<
    RefCell<
        Option<(
            web_sys::Document,
            Closure<dyn FnMut(web_sys::KeyboardEvent)>,
        )>,
    >,
>;
type Cancellation =
    Rc<RefCell<Option<(LayoutOwnerIdentity, Rc<RefCell<Option<TerminalOutcome>>>)>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutTransformTool {
    Stagger,
    Spread,
    Offset,
    /// Carries one row or key sideways through its running frame.
    Shift,
    /// Turns one row's or one key's running frame, in degrees.
    Bend,
    Splay,
    Origin,
    /// Moves the selected matrix through its Origin.
    Move,
    /// Turns the selected matrix through its stored rotation.
    Rotate,
    /// Ticket 08: arms canvas Anchor pick mode on the selected matrix or
    /// part. The token toggles; it never starts a drag.
    Anchor,
}

/// Ticket 07: the native part-rail list maps onto the shared rail tools.
impl From<crate::part_rail::PartRailTool> for LayoutTransformTool {
    fn from(tool: crate::part_rail::PartRailTool) -> Self {
        match tool {
            crate::part_rail::PartRailTool::Move => Self::Move,
            crate::part_rail::PartRailTool::Rotate => Self::Rotate,
            crate::part_rail::PartRailTool::Spread => Self::Spread,
            crate::part_rail::PartRailTool::Stagger => Self::Stagger,
            crate::part_rail::PartRailTool::Anchor => Self::Anchor,
        }
    }
}

impl LayoutTransformTool {
    const ALL: [(Self, &'static str); 9] = [
        (Self::Stagger, "Stagger"),
        (Self::Spread, "Spread"),
        (Self::Offset, "Offset"),
        (Self::Shift, "Shift"),
        (Self::Bend, "Bend"),
        (Self::Splay, "Splay"),
        (Self::Origin, "Origin"),
        (Self::Move, "Move"),
        (Self::Rotate, "Rotate"),
    ];
}

/// What choosing a menu tool does: the tool that is active afterwards and the selection
/// kind to switch to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolActivation {
    pub active: Option<LayoutTransformTool>,
    pub selection_kind: Option<LayoutSelectionKind>,
}

/// The activation rules for a menu tool. Choosing the active tool again turns it off and
/// leaves the selection alone. Offset keeps a Row selection and turns every other matrix
/// selection into its Column. Shift and Bend keep a Row or Key selection and turn every
/// other matrix selection into its Column. Stagger is a column parameter: a Row selection becomes the
/// column of the retained key when there is one (the conversion reads the retained cell)
/// and otherwise column 1, and every other selection becomes its Column. Splay and Origin
/// turn the selection into its Column. Move and Rotate keep the Matrix selection: they
/// edit the matrix itself, not one of its parameters.
///
/// Ticket 03: the rail peers mounted in one SVG canvas. Each overlay registers
/// its tool on mount and unregisters on unmount, keyed by the shared svg
/// handle, so every overlay of one selection computes the same peer list
/// without new props on the shared mount site. The list holds the mounted
/// tools; the rail below filters it into canonical order minus Origin.
#[derive(Default)]
struct RailMount {
    tools: Vec<LayoutTransformTool>,
    active: Option<LayoutTransformTool>,
}

thread_local! {
    static RAIL_MOUNTS: RefCell<HashMap<usize, RailMount>> = RefCell::new(HashMap::new());
}

/// Canonical rail order: peers list in this order however the mounts
/// interleave, matching the `transform_tools` listing order.
const RAIL_ORDER: [LayoutTransformTool; 9] = [
    LayoutTransformTool::Spread,
    LayoutTransformTool::Stagger,
    LayoutTransformTool::Offset,
    LayoutTransformTool::Shift,
    LayoutTransformTool::Bend,
    LayoutTransformTool::Splay,
    LayoutTransformTool::Move,
    LayoutTransformTool::Rotate,
    LayoutTransformTool::Anchor,
];

fn rail_mount_tool(svg_key: usize, tool: LayoutTransformTool) {
    RAIL_MOUNTS.with(|mounts| {
        let mut mounts = mounts.borrow_mut();
        let mount = mounts.entry(svg_key).or_default();
        if !mount.tools.contains(&tool) {
            mount.tools.push(tool);
        }
    });
}

fn rail_unmount_tool(svg_key: usize, tool: LayoutTransformTool) {
    RAIL_MOUNTS.with(|mounts| {
        if let Some(mount) = mounts.borrow_mut().get_mut(&svg_key) {
            mount.tools.retain(|peer| *peer != tool);
            if mount.active == Some(tool) {
                mount.active = None;
            }
        }
    });
}

fn rail_set_active(svg_key: usize, tool: Option<LayoutTransformTool>) {
    RAIL_MOUNTS.with(|mounts| {
        if let Some(mount) = mounts.borrow_mut().get_mut(&svg_key) {
            mount.active = tool;
        }
    });
}

/// The rail peers for one overlay: the mounted tools in canonical order.
/// Origin stays a point on the canvas, so it never joins the rail.
fn rail_peers(svg_key: usize) -> Vec<LayoutTransformTool> {
    RAIL_MOUNTS.with(|mounts| {
        mounts
            .borrow()
            .get(&svg_key)
            .map_or_else(Vec::new, |mount| {
                RAIL_ORDER
                    .iter()
                    .filter(|peer| mount.tools.contains(peer))
                    .copied()
                    .collect()
            })
    })
}

fn rail_active_tool(svg_key: usize) -> Option<LayoutTransformTool> {
    RAIL_MOUNTS.with(|mounts| mounts.borrow().get(&svg_key).and_then(|mount| mount.active))
}

/// Prototype variant D token glyphs, in screen px. The rail draws them in a
/// y-down px group, so these match the spec sheet verbatim.
const GLYPH_SPREAD: &str = "M-5 0h10M-3 -2l-2 2 2 2M3 -2l2 2-2 2";
const GLYPH_STAGGER: &str = "M-5 0h10M-3 -2l-2 2 2 2M3 -2l2 2-2 2";
const GLYPH_FOUR: &str = "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4";
const GLYPH_SPLAY: &str = "M-5 4A9 9 0 0 1 4 -5M1.5 -6.5 4 -5l-1.3 2.4M-5 4h2";
/// Prototype variant D bend token: a curved arrow, shared by row and key.
const GLYPH_BEND: &str = "M-4 5C-4 0-1-4 4-4M1.5-6 4-4 1.5-2";

/// Ticket 06: the commit toast lingers for 1.8 s, matching the prototype.
const TOAST_TIMEOUT_MS: u32 = 1800;

/// Arm one commit-toast expiry timer. The overlay remounts on every scope and
/// selection change, so the timer can outlive its component: it only touches
/// its signals while they are still alive and stays silent otherwise.
fn arm_commit_toast_expiry(
    mut feedback: Signal<FeedbackState>,
    generation: Signal<u64>,
    seen: u64,
) {
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(TOAST_TIMEOUT_MS).await;
        let live = generation.try_peek().is_ok_and(|current| *current == seen);
        if live && let Ok(mut state) = feedback.try_write() {
            *state = advance_feedback(&state, FeedbackEvent::Expire);
        }
    });
}

/// Ticket 06: the canvas label for one transform tool in popup and toast text.
fn feedback_tool_name(tool: LayoutTransformTool) -> &'static str {
    match tool {
        LayoutTransformTool::Stagger => "Stagger",
        LayoutTransformTool::Spread => "Spread",
        LayoutTransformTool::Offset => "Offset",
        LayoutTransformTool::Shift => "Shift",
        LayoutTransformTool::Bend => "Bend",
        LayoutTransformTool::Splay => "Splay",
        LayoutTransformTool::Origin => "Origin",
        LayoutTransformTool::Move => "Move",
        LayoutTransformTool::Rotate => "Rotate",
        LayoutTransformTool::Anchor => "Anchor",
    }
}

/// Ticket 06: the committed value text for a placement result, matching the
/// token value style ("Spread 28.57 mm" comes from the name plus this).
fn format_placement_value(value: PlacementValue) -> String {
    match value {
        PlacementValue::Stagger(value) | PlacementValue::Spread(value) => {
            format!("{value:.2} mm")
        }
        PlacementValue::Offset(value) | PlacementValue::Shift(value) => {
            format!("X {:.2}, Y {:.2} mm", value.x, value.y)
        }
        PlacementValue::Bend(value) => format!("{value:.1}°"),
        PlacementValue::Padding(value) => format!("{value:.2} mm"),
    }
}

/// Ticket 06: a placement drag delta in the toast's display units.
fn placement_feedback_delta(delta: PlacementDelta) -> FeedbackDelta {
    match delta {
        PlacementDelta::Stagger(value)
        | PlacementDelta::Spread(value)
        | PlacementDelta::Padding(value) => FeedbackDelta::Mm(value),
        PlacementDelta::Offset(value) | PlacementDelta::Shift(value) => {
            FeedbackDelta::Mm2(value.x, value.y)
        }
        PlacementDelta::Bend(value) => FeedbackDelta::Deg(value),
    }
}

/// Ticket 06: estimated screen-px width of one chrome text line.
fn feedback_line_width(text: &str, size_px: f64) -> f64 {
    text.chars().count() as f64 * size_px * 0.58
}

/// Ticket 06: the value popup panel for resolved content, or `None` without
/// a position. `top_left` is world; `px` is world units per screen px.
fn feedback_popup_panel(
    popup: &boardstudio_web_ui_model::transform_feedback::ValuePopup,
    follow: Option<(f64, f64)>,
    rail: Option<(f64, f64, f64, f64)>,
    px: f64,
) -> Option<Element> {
    let mut rows: Vec<(String, f64, &'static str, bool)> = vec![
        (popup.name.clone(), 11.0, "var(--wb-muted)", false),
        (popup.value.clone(), 15.0, "inherit", true),
    ];
    if let Some(delta) = popup.delta.as_ref() {
        rows.push((delta.clone(), 12.0, "var(--wb-accent)", false));
    }
    rows.push((
        "Drag the handle, or type in the Inspector · Esc: close".to_owned(),
        10.5,
        "var(--wb-muted)",
        false,
    ));
    let panel_w = rows
        .iter()
        .map(|(text, size, _, _)| feedback_line_width(text, *size))
        .fold(0.0_f64, f64::max)
        + 16.0;
    // A following drag anchors its own top-left; a hover or focus centres
    // the panel above the rail.
    let top_left: Option<(f64, f64)> = follow
        .or_else(|| rail.map(|(x, y, w, h)| (x + w / 2.0 - panel_w * px / 2.0, y + h + 8.0 * px)));
    let (panel_x, panel_y) = top_left?;
    Some(rsx! {
        g {
            class: "m1-feedback-popup",
            transform: "translate({panel_x} {panel_y}) scale({px} {-px})",
            style: "pointer-events:none",
            "aria-hidden": "true",
            {feedback_panel_contents(panel_w, rows)}
        }
    })
}

/// Ticket 06: the commit toast panel at the top centre of the canvas view.
/// The value renders bold, the rest plain, inside one `role="status"` group
/// that never takes focus.
fn feedback_toast_panel(
    toast: &boardstudio_web_ui_model::transform_feedback::CommitToast,
    view_x: f64,
    view_y: f64,
    view_width: f64,
    px: f64,
) -> Element {
    let width_px = feedback_line_width(&toast.text(), 12.5) + 24.0;
    let panel_x = view_x + view_width / 2.0 - width_px * px / 2.0;
    let panel_y = -view_y - 8.0 * px;
    let value = toast.value.clone();
    let rest = format!(" ({}) · 1 Undo step", toast.delta);
    rsx! {
        g {
            class: "m1-feedback-toast",
            role: "status",
            transform: "translate({panel_x} {panel_y}) scale({px} {-px})",
            style: "pointer-events:none",
            rect {
                x: "0",
                y: "0",
                width: "{width_px}",
                height: "33.5",
                rx: "8",
                style: "fill:var(--wb-panel);stroke:var(--wb-rule-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 6px 18px rgb(24 35 49 / 18%))",
            }
            text {
                x: "12",
                y: "20.5",
                style: "font-size:12.5px",
                tspan { style: "font-weight:700", "{value}" }
                tspan { "{rest}" }
            }
        }
    }
}

/// Ticket 06: the rect plus left-aligned text rows of one view-only chrome
/// panel, drawn in a y-down px group the caller positions. Rows carry
/// (text, size px, fill, mono-600). Static panels only: reduced motion drops
/// travel, and there is no travel to drop.
fn feedback_panel_contents(width_px: f64, rows: Vec<(String, f64, &'static str, bool)>) -> Element {
    let pad = 8.0;
    let height_px: f64 = rows.iter().map(|(_, size, _, _)| size * 1.4).sum::<f64>() + pad * 2.0;
    let mut baseline = pad;
    let placed: Vec<(String, f64, &'static str, String, f64)> = rows
        .into_iter()
        .map(|(text, size, fill, mono)| {
            baseline += size;
            let y = baseline;
            baseline += size * 0.4;
            let style = if mono {
                "font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:600".to_owned()
            } else {
                String::new()
            };
            (text, size, fill, style, y)
        })
        .collect();
    rsx! {
        rect {
            x: "0",
            y: "0",
            width: "{width_px}",
            height: "{height_px}",
            rx: "8",
            style: "fill:var(--wb-panel);stroke:var(--wb-rule-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))",
        }
        for (text, size, fill, style, y) in placed {
            text {
                x: "{pad}",
                y: "{y}",
                style: "font-size:{size}px;fill:{fill};{style}",
                "{text}"
            }
        }
    }
}
const GLYPH_ROTATE: &str = "M4.5 -2A5 5 0 1 1 -2 -4.6M4.6 -5v3h-3";
/// Prototype variant D anchor token: the anchor shank with its ring.
const GLYPH_ANCHOR: &str = "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0";

/// The token glyph for one rail tool: the axis arrow along `glyph_degrees`
/// for the one-dimensional tools, the spec glyphs for Splay and Rotate, and
/// the four-way arrow for Move and the Ctrl quick offset.
fn rail_token_glyph(
    tool: LayoutTransformTool,
    quick_offset: bool,
    glyph_degrees: f64,
    stroke: &str,
) -> Element {
    let style = format!(
        "fill:none;stroke:{stroke};stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke"
    );
    match (tool, quick_offset) {
        (LayoutTransformTool::Spread, _) => rsx! {
            path { d: "{GLYPH_SPREAD}", style: "{style}" }
        },
        (LayoutTransformTool::Stagger | LayoutTransformTool::Offset, true) => rsx! {
            g { transform: "scale(1.5)",
                path { d: "{GLYPH_FOUR}", style: "{style}" }
            }
        },
        (LayoutTransformTool::Stagger | LayoutTransformTool::Offset, false) => rsx! {
            g { transform: "rotate({glyph_degrees})",
                path { d: "{GLYPH_STAGGER}", style: "{style}" }
            }
        },
        // A Shift token is always the four-way arrow: the drag is already a
        // free 2-D move, so no Ctrl quick-offset state gates it.
        (LayoutTransformTool::Shift, _) => rsx! {
            g { transform: "scale(1.5)",
                path { d: "{GLYPH_FOUR}", style: "{style}" }
            }
        },
        (LayoutTransformTool::Bend, _) => rsx! {
            path { d: "{GLYPH_BEND}", style: "{style}" }
        },
        (LayoutTransformTool::Splay, _) => rsx! {
            path { d: "{GLYPH_SPLAY}", style: "{style}" }
        },
        (LayoutTransformTool::Move, _) => rsx! {
            g { transform: "scale(1.5)",
                path { d: "{GLYPH_FOUR}", style: "{style}" }
            }
        },
        (LayoutTransformTool::Rotate, _) => rsx! {
            path { d: "{GLYPH_ROTATE}", style: "{style}" }
        },
        (LayoutTransformTool::Anchor, _) => rsx! {
            path { d: "{GLYPH_ANCHOR}", style: "{style}" }
        },
        (LayoutTransformTool::Origin, _) => rsx! {},
    }
}

/// The Inspector field whose amber preview matches one rail token's movers,
/// or `None` when the tool holds its keys still (Origin). Row Shift and Bend
/// carry downward like their Inspector fields; key Shift and Bend carry
/// upward through their column.
fn preview_field_for_tool(
    tool: LayoutTransformTool,
    context: &TreeContext,
) -> Option<MatrixTransformField> {
    match (tool, context) {
        (LayoutTransformTool::Stagger, TreeContext::Column { .. }) => {
            Some(MatrixTransformField::ColumnStagger)
        }
        (LayoutTransformTool::Spread, _) => Some(MatrixTransformField::ColumnSpread),
        (LayoutTransformTool::Splay, _) => Some(MatrixTransformField::ColumnSplay),
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => {
            Some(MatrixTransformField::RowOffsetX)
        }
        (LayoutTransformTool::Offset, _) => Some(MatrixTransformField::ColumnOffsetX),
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => {
            Some(MatrixTransformField::RowShiftX)
        }
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => {
            Some(MatrixTransformField::KeyShiftX)
        }
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => Some(MatrixTransformField::RowBend),
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => Some(MatrixTransformField::KeyBend),
        (LayoutTransformTool::Move, _) => Some(MatrixTransformField::OriginX),
        (LayoutTransformTool::Rotate, _) => Some(MatrixTransformField::MatrixRotation),
        (LayoutTransformTool::Origin, _)
        | (LayoutTransformTool::Stagger, _)
        | (LayoutTransformTool::Shift, _)
        | (LayoutTransformTool::Anchor, _)
        | (LayoutTransformTool::Bend, _) => None,
    }
}
pub fn activate_transform_tool(
    active: Option<LayoutTransformTool>,
    tool: LayoutTransformTool,
    context: &TreeContext,
) -> ToolActivation {
    if active == Some(tool) {
        return ToolActivation {
            active: None,
            selection_kind: None,
        };
    }
    match (tool, context) {
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => ToolActivation {
            active: Some(tool),
            selection_kind: Some(LayoutSelectionKind::Row),
        },
        (LayoutTransformTool::Shift | LayoutTransformTool::Bend, TreeContext::Row { .. }) => {
            ToolActivation {
                active: Some(tool),
                selection_kind: Some(LayoutSelectionKind::Row),
            }
        }
        (LayoutTransformTool::Shift | LayoutTransformTool::Bend, TreeContext::Key { .. }) => {
            ToolActivation {
                active: Some(tool),
                selection_kind: Some(LayoutSelectionKind::Key),
            }
        }
        (
            LayoutTransformTool::Move | LayoutTransformTool::Rotate | LayoutTransformTool::Anchor,
            _,
        ) => ToolActivation {
            active: Some(tool),
            selection_kind: None,
        },
        _ => ToolActivation {
            active: Some(tool),
            selection_kind: Some(LayoutSelectionKind::Column),
        },
    }
}

#[derive(Clone, PartialEq)]
pub struct LayoutTransformMenuMount {
    pub properties_available: bool,
    pub column_available: bool,
    pub row_available: bool,
    pub pointer_tools_visible: bool,
    pub pointer_tools_available: bool,
    pub active_tool: Option<LayoutTransformTool>,
    /// `None` when the selection is not a Matrix; otherwise whether the
    /// matrix follows rotation, which decides whether the Rotate entry is
    /// offered enabled or disabled with an explanation.
    pub matrix_rotate_followed: Option<bool>,
    pub on_selection_kind: EventHandler<LayoutSelectionKind>,
    pub on_show_properties: EventHandler<()>,
    pub on_transform_tool: EventHandler<LayoutTransformTool>,
}

#[derive(Clone)]
pub struct LayoutTransformRuntime(pub Rc<Runtime>);

impl PartialEq for LayoutTransformRuntime {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct LayoutTransformSvg(pub Rc<RefCell<Option<SvgElement>>>);

impl PartialEq for LayoutTransformSvg {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TransformGesture {
    /// Edits the column's stagger. `quick_offset` is latched from `ctrlKey` at pointer-down
    /// and turns the drag into a free 2-D column-offset gesture for its whole length.
    Stagger { column: u32, quick_offset: bool },
    /// Edits the column's inherited spread step.
    Spread { column: u32 },
    /// Edits one row's or one column's independent offset (`RowOffset` or `ColumnOffset`).
    Offset { parameter: PlacementParameter },
    /// Turns one row's or one key's running frame, in degrees.
    Bend { parameter: PlacementParameter },
    Splay {
        column: u32,
        start_angle: f64,
        origin: Vec2,
        /// Latched from `ctrlKey` at pointer-down: the drag swings the origin
        /// through `OriginSwing` instead of turning the angle.
        swing: bool,
    },
    Origin {
        column: u32,
        /// Latched from `ctrlKey` at pointer-down: the drag writes the origin
        /// without hold-still compensation.
        swing: bool,
    },
    /// Moves the selected matrix through its Origin. The anchor context is
    /// captured at pointer-down from the accepted document; `anchor` is
    /// `None` exactly when the matrix is unanchored, and an anchored matrix
    /// whose anchor target is not laid out refuses the drag at start.
    MatrixMove { anchor: Option<MatrixDragAnchor> },
    /// Turns the selected matrix through its stored rotation. The anchor
    /// context is captured at pointer-down like the move; the center, start
    /// angle and start rotation follow from the accepted matrix then.
    MatrixRotate {
        anchor: Option<MatrixDragAnchor>,
        center: Vec2,
        start_angle: f64,
        start_rotation: f64,
    },
}

impl TransformGesture {
    /// The placement parameter a Stagger, Spread, Offset or Bend gesture writes.
    fn parameter(&self) -> Option<PlacementParameter> {
        match *self {
            Self::Stagger {
                column,
                quick_offset,
            } => Some(stagger_parameter(column, quick_offset)),
            Self::Spread { column } => Some(PlacementParameter::ColumnSpread { column }),
            Self::Offset { parameter } => Some(parameter),
            Self::Bend { parameter } => Some(parameter),
            Self::Splay { .. }
            | Self::Origin { .. }
            | Self::MatrixMove { .. }
            | Self::MatrixRotate { .. } => None,
        }
    }
}

#[derive(Clone)]
struct TransformDrag {
    pointer_id: i32,
    capture: SvgElement,
    matrix: Rc<Matrix>,
    gesture: TransformGesture,
    transaction_id: String,
    owner: LayoutOwnerIdentity,
    context: TreeContext,
    tool: LayoutTransformTool,
    /// The scene frame at pointer-down; the world delta is projected onto it.
    basis: Option<SceneBasis>,
    /// The dragged column's world base and world origin at pointer-down: the
    /// snap target and the no-op point for an origin swing.
    origin_base: Option<Vec2>,
    start_origin: Option<Vec2>,
    start_client_x: f64,
    start_client_y: f64,
    pending: Option<EditOperation>,
    preview_submitted: bool,
    cancellation_dependency: Option<Rc<RefCell<Option<TerminalOutcome>>>>,
}

#[component]
pub fn LayoutTransformToolOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(into)] matrix: Rc<Matrix>,
    #[props(into)] projection: Rc<MatrixScene>,
    context: TreeContext,
    tool: LayoutTransformTool,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    snap_settings: super::layout_toolbar::LayoutSnapSettings,
    #[props(into)] snap_origins: Rc<Vec<super::super::outline_snapping::Origin>>,
    splay_affect: Signal<MatrixSplayAffect>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    on_finish: EventHandler<()>,
) -> Element {
    let control = use_signal(|| false);
    let _control_listener = use_hook(move || {
        Rc::new(
            if matches!(
                tool,
                LayoutTransformTool::Stagger
                    | LayoutTransformTool::Splay
                    | LayoutTransformTool::Origin
            ) {
                CtrlModifierListener::install(control)
            } else {
                None
            },
        )
    });
    let _ = surface_size.map(|size| size());
    // Ticket 03: the rail peers share one svg handle per canvas. The key is
    // the shared handle's identity, so every overlay of one selection reads
    // the same mount set however the mounts interleave.
    let svg_key: usize = Rc::as_ptr(&svg.0) as usize;
    use_hook(move || {
        rail_mount_tool(svg_key, tool);
    });
    // Peers mount in one commit, so the first overlay paints before the rest
    // register. This second pass re-reads the full mount set once every peer
    // of the commit has registered, keeping every rail centred.
    let mut mounted = use_signal(|| false);
    let mounted_read = mounted;
    use_effect(move || {
        mounted.set(true);
    });
    let _ = mounted_read();
    use_drop(move || {
        rail_unmount_tool(svg_key, tool);
    });
    let drag = use_hook(|| Rc::new(RefCell::new(None::<TransformDrag>)));
    let cancellation = use_hook(Cancellation::default);
    let snap_guide = use_signal(|| None::<Vec2>);
    // Ticket 03: hover (or keyboard focus) on a rail token previews, in
    // amber, the keys its drag would move — the same transient view state
    // the Inspector number fields show. It never writes.
    let hovered = use_signal(|| false);
    // Keyboard focus previews like hover, matching the Inspector fields.
    let focused = use_signal(|| false);
    // Ticket 06: the value popup and commit toast content (pure reducer;
    // rendering only), the popup's top-left in world units (`None` docks it
    // above the rail), and the toast generation the 1.8 s timer checks.
    let feedback = use_signal(FeedbackState::default);
    let popup_at = use_signal(|| None::<(f64, f64)>);
    let toast_gen = use_signal(|| 0_u64);
    let preview = try_use_context::<FieldPreview>();
    let preview_source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let shown = use_hook(|| Rc::new(RefCell::new(Vec::<String>::new())));
    let show_preview = {
        let shown = shown.clone();
        let context = context.clone();
        let affect = splay_affect;
        move || {
            let (Some(target), Some(source)) = (preview, preview_source.clone()) else {
                return;
            };
            let Some(field) = preview_field_for_tool(tool, &context) else {
                return;
            };
            let Some(facts) = matrix_relations::field_preview(
                &source.document,
                &source.scene.matrix_scenes,
                &context,
                field,
                affect(),
            ) else {
                return;
            };
            *shown.borrow_mut() = facts.parts.clone();
            target.show(FieldPreviewState {
                parts: facts.parts,
                hint: facts.hint,
            });
        }
    };
    let release_preview = {
        let shown = shown.clone();
        move || {
            if let Some(target) = preview {
                target.release(&shown.borrow());
            }
        }
    };
    use_drop({
        let shown = shown.clone();
        move || {
            if let Some(target) = preview {
                target.release(&shown.borrow());
            }
        }
    });
    // True from a Ctrl pointer-down until release: the handle shows the offset glyph.
    let quick_glyph = use_signal(|| false);
    // True from a Ctrl pointer-down on a Splay or Origin handle until release.
    let swing_glyph = use_signal(|| false);
    let runtime_inner = runtime.0.clone();
    let arbiter_for_drop = arbiter.clone();
    let drag_for_drop = drag.clone();
    let cancellation_for_drop = cancellation.clone();
    let mut guide_for_drop = snap_guide;
    use_drop(move || {
        cancel_transform_drag(
            &runtime_inner,
            &drag_for_drop,
            &arbiter_for_drop,
            &cancellation_for_drop,
            &mut guide_for_drop,
            quick_glyph,
            swing_glyph,
            svg_key,
        );
    });
    let escape_listener = use_hook(TransformEscapeListener::default);
    use_effect({
        let listener = escape_listener.clone();
        let runtime = runtime.0.clone();
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let cancellation = cancellation.clone();
        let feedback_for_escape = feedback;
        let popup_at_for_escape = popup_at;
        move || {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let runtime = runtime.clone();
            let drag = drag.clone();
            let arbiter = arbiter.clone();
            let cancellation = cancellation.clone();
            let mut feedback_for_escape = feedback_for_escape;
            let mut popup_at_for_escape = popup_at_for_escape;
            let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() != "Escape" || event.default_prevented() {
                    return;
                }
                if event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|target| {
                        target
                            .closest("input, textarea, select, [contenteditable='true']")
                            .ok()
                            .flatten()
                            .is_some()
                    })
                {
                    return;
                }
                // Another visible handle may own the live gesture. Its listener cancels it.
                if drag.borrow().is_none() && arbiter.current().is_some() {
                    return;
                }
                event.prevent_default();
                let mut guide = snap_guide;
                // Ticket 06: Escape clears the chosen tool and its popup.
                feedback_for_escape.set(advance_feedback(
                    &feedback_for_escape(),
                    FeedbackEvent::Dismiss,
                ));
                popup_at_for_escape.set(None);
                cancel_transform_drag(
                    &runtime,
                    &drag,
                    &arbiter,
                    &cancellation,
                    &mut guide,
                    quick_glyph,
                    swing_glyph,
                    svg_key,
                );
                on_finish.call(());
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
            *listener.borrow_mut() = Some((document, callback));
        }
    });
    use_drop({
        let listener = escape_listener.clone();
        move || {
            if let Some((document, callback)) = listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    });
    let active_owner = owner.clone();
    let active_context = context.clone();
    let active_tool = tool;
    let drag_for_owner = drag.clone();
    let runtime_for_owner = runtime.0.clone();
    let arbiter_for_owner = arbiter.clone();
    let cancellation_for_owner = cancellation.clone();
    let mut guide_for_owner = snap_guide;
    use_effect(use_reactive(
        (&active_owner, &active_context, &active_tool),
        {
            move |(active_owner, active_context, active_tool)| {
                if drag_for_owner.borrow().as_ref().is_some_and(|active| {
                    active.owner != active_owner
                        || active.context != active_context
                        || active.tool != active_tool
                }) {
                    cancel_transform_drag(
                        &runtime_for_owner,
                        &drag_for_owner,
                        &arbiter_for_owner,
                        &cancellation_for_owner,
                        &mut guide_for_owner,
                        quick_glyph,
                        swing_glyph,
                        svg_key,
                    );
                }
            }
        },
    ));

    let mut selected_points = {
        let model = runtime.0.model();
        model.accepted.as_ref().map_or_else(Vec::new, |snapshot| {
            model
                .selected_part_ids
                .iter()
                .filter_map(|id| {
                    snapshot
                        .document
                        .parts
                        .iter()
                        .find(|part| &part.id == id)
                        .map(|part| part.pose.at)
                })
                .collect::<Vec<_>>()
        })
    };
    match &context {
        TreeContext::Column { column, .. } => {
            selected_points = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled && cell.column == *column)
                .map(|cell| cell.pose.at)
                .collect()
        }
        TreeContext::Row { row, .. } => {
            selected_points = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled && cell.row == *row)
                .map(|cell| cell.pose.at)
                .collect()
        }
        _ => {}
    }
    // The matrix Move/Rotate handles share one anchor point on the drawn
    // matrix: the first enabled key's pose. The rotation center captured at
    // pointer-down uses the same point from the live scene, so the two cannot
    // drift apart.
    let matrix_handle_point = projection
        .cells
        .iter()
        .find(|cell| cell.enabled)
        .or(projection.cells.first())
        .map(|cell| cell.pose.at)
        .unwrap_or(matrix.origin);
    let matrix_rotation_followed = matrix
        .anchor
        .as_ref()
        .map(|anchor| anchor.follow.rotation)
        .unwrap_or(true);
    let selected_column = context_column(&context).unwrap_or(0);
    let selected_row = context_row(&context);
    let handle_scale = svg.0.borrow().as_ref().map_or(3.0, |surface| {
        let rect = surface.get_bounding_client_rect();
        if rect.width() > 0.0 {
            (width / rect.width()).max(height / rect.height().max(1.0)) * 3.0
        } else {
            3.0
        }
    });
    // World units per screen px: the overlay draws screen-size chrome from
    // px art scaled by this factor, so tokens stay 18 px at every zoom.
    let world_per_screen = handle_scale / 3.0;
    // Column 0 follows the pitch, so it has no Spread step to drag: a Spread
    // token never joins its rail. Every overlay of the selection filters the
    // same mount set, so the rail stays centred on the tokens that draw.
    let rail_tools: Vec<LayoutTransformTool> = rail_peers(svg_key)
        .into_iter()
        .filter(|peer| {
            !matches!(peer, LayoutTransformTool::Spread)
                || selected_column != 0
                || selected_row.is_some()
        })
        .collect();
    let rail_index = rail_tools.iter().position(|peer| *peer == tool);
    // The Origin point stays on the canvas at the splay origin; it never
    // joins the rail.
    let origin_point = if matches!(
        tool,
        LayoutTransformTool::Splay | LayoutTransformTool::Origin
    ) {
        projection
            .columns
            .iter()
            .find(|basis| basis.column == selected_column)
            .map(|basis| basis.splay_origin)
    } else {
        None
    };
    // The rail centres on the selection: the column or row cells for a
    // Column/Row selection, every enabled cell for a Matrix.
    let rail_points: Vec<Vec2> = match &context {
        TreeContext::Column { column, .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled && cell.column == *column)
            .map(|cell| cell.pose.at)
            .collect(),
        TreeContext::Row { row, .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled && cell.row == *row)
            .map(|cell| cell.pose.at)
            .collect(),
        TreeContext::Key { row, column, .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled && cell.row == *row && cell.column == *column)
            .map(|cell| cell.pose.at)
            .collect(),
        TreeContext::Matrix { .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled)
            .map(|cell| cell.pose.at)
            .collect(),
        _ => Vec::new(),
    };
    // A Matrix with no enabled cell still draws its rail at the shared
    // handle point, so Move and Rotate never lose their tokens.
    let rail_points = if rail_points.is_empty() && matches!(context, TreeContext::Matrix { .. }) {
        vec![matrix_handle_point]
    } else {
        rail_points
    };
    let rail_layout = if rail_index.is_some() && !rail_points.is_empty() {
        let min_x = rail_points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = rail_points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let top = rail_points
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        handle_rail::rail_world(
            (min_x + max_x) / 2.0,
            top,
            rail_tools.len(),
            world_per_screen,
        )
    } else {
        None
    };
    if rail_layout.is_none() && origin_point.is_none() {
        return rsx! {};
    }
    let fallback_angle_point = angle_handle_point(
        &projection,
        selected_column,
        matrix.pitch.y,
        &selected_points,
    );
    let angle_point = if selected_points.is_empty() {
        fallback_angle_point
    } else {
        Vec2 {
            x: selected_points.iter().map(|p| p.x).sum::<f64>() / selected_points.len() as f64,
            y: selected_points
                .iter()
                .map(|p| p.y)
                .fold(f64::INFINITY, f64::min)
                - matrix.pitch.y / 2.0
                - 18.0 * handle_scale,
        }
    };
    let angle_degrees = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(0.0, |basis| {
            column_angle(basis.splay_angle, basis.rotation, &splay_affect())
        });
    let arc = origin_point.map(|origin| splay_arc(origin, angle_point, angle_degrees));
    let current_owner = owner.clone();
    let current_context = context.clone();
    let matrix_for_key = matrix.clone();
    let matrix_for_spread = matrix.clone();
    let offset_parameter = match context_row(&context) {
        Some(row) => PlacementParameter::RowOffset { row },
        None => PlacementParameter::ColumnOffset {
            column: selected_column,
        },
    };
    let offset_now = match placement_current(&matrix, offset_parameter) {
        PlacementValue::Offset(offset) => offset,
        _ => Vec2::default(),
    };
    // Ticket 05: a row or key selection carries its own shift and bend; any
    // other context leaves them absent and the tokens stay hidden.
    let shift_parameter = shift_parameter(&context);
    let shift_now = shift_parameter
        .and_then(|parameter| match placement_current(&matrix, parameter) {
            PlacementValue::Shift(shift) => Some(shift),
            _ => None,
        })
        .unwrap_or_default();
    let bend_parameter = bend_parameter(&context);
    let bend_now = bend_parameter
        .and_then(|parameter| match placement_current(&matrix, parameter) {
            PlacementValue::Bend(bend) => Some(bend),
            _ => None,
        })
        .unwrap_or(0.0);
    let stagger_now = matrix
        .column_layout
        .get(selected_column as usize)
        .map(|record| &record.stagger)
        .copied()
        .unwrap_or(0.0);
    let projection_for_key = projection.clone();
    let projection_for_start = projection.clone();
    let matrix_for_start = matrix.clone();
    let owner_for_key = owner.clone();
    let context_for_key = context.clone();
    let runtime_for_start = runtime.0.clone();
    let svg_for_start = svg.0.clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let cancellation_for_start = cancellation.clone();
    let guide_for_start = snap_guide;
    let popup_at_for_start = popup_at;
    let start_drag = Rc::new(move |event: PointerEvent, gesture: TransformGesture| {
        let mut guide_for_start = guide_for_start;
        let mut quick_for_start = quick_glyph;
        let mut swing_for_start = swing_glyph;
        let svg_key_for_start = svg_key;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        // The modifier is read here and nowhere else, so a drag never switches parameter.
        let gesture = match gesture {
            TransformGesture::Stagger { column, .. } => TransformGesture::Stagger {
                column,
                quick_offset: pointer.ctrl_key(),
            },
            TransformGesture::Splay {
                column,
                start_angle,
                origin,
                ..
            } => TransformGesture::Splay {
                column,
                start_angle,
                origin,
                swing: pointer.ctrl_key(),
            },
            TransformGesture::Origin { column, .. } => TransformGesture::Origin {
                column,
                swing: pointer.ctrl_key(),
            },
            TransformGesture::MatrixMove { .. } => {
                // Capture the anchor context now; the preview and the commit
                // compose this drag against whatever the matrix becomes. An
                // anchored matrix whose anchor target is not laid out refuses
                // the drag before it acquires anything.
                let Some(accepted) = runtime_for_start.model().accepted.clone() else {
                    return;
                };
                let Ok(anchor) = matrix_drag_anchor(&accepted, &matrix_for_start) else {
                    return;
                };
                TransformGesture::MatrixMove { anchor }
            }
            TransformGesture::MatrixRotate { .. } => {
                let Some(accepted) = runtime_for_start.model().accepted.clone() else {
                    return;
                };
                let Ok(anchor) = matrix_drag_anchor(&accepted, &matrix_for_start) else {
                    return;
                };
                TransformGesture::MatrixRotate {
                    anchor,
                    center: Vec2::default(),
                    start_angle: 0.0,
                    start_rotation: 0.0,
                }
            }
            other => other,
        };
        let basis = gesture
            .parameter()
            .and_then(|parameter| SceneBasis::for_parameter(&projection_for_start, parameter));
        if pointer.button() != 0 || drag_for_start.borrow().is_some() {
            return;
        }
        if !transform_owner_is_current(
            &runtime_for_start,
            workspace,
            scope_generation,
            selected_context,
            &current_owner,
            &current_context,
        ) || context_matrix_id(&current_context) != Some(matrix_for_start.id.as_str())
            || !transform_start_is_editable(
                &runtime_for_start,
                &current_owner,
                &matrix_for_start,
                &cancellation_for_start,
            )
            || !arbiter_for_start.try_acquire(CanvasInteractionOwner::MatrixTransform)
        {
            return;
        }
        let Some(capture) = pointer
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest(".m1-transform-handle").ok().flatten())
            .and_then(|target| target.dyn_into::<SvgElement>().ok())
        else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if capture.set_pointer_capture(pointer.pointer_id()).is_err() {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        pointer.prevent_default();
        pointer.stop_propagation();
        let focus_options = web_sys::FocusOptions::new();
        focus_options.set_prevent_scroll(true);
        let _ = capture.focus_with_options(&focus_options);
        let angle_origin = match gesture {
            TransformGesture::Splay { origin, .. } => origin,
            _ => Vec2::default(),
        };
        let Some(start_point) = crate::presentation::coordinates(
            &svg_for_start,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            let _ = capture.release_pointer_capture(pointer.pointer_id());
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        let start_angle = (start_point.y - angle_origin.y).atan2(start_point.x - angle_origin.x);
        let gesture = match gesture {
            TransformGesture::Splay {
                column,
                origin,
                swing,
                ..
            } => TransformGesture::Splay {
                column,
                origin,
                start_angle,
                swing,
            },
            TransformGesture::MatrixRotate { anchor, .. } => {
                // The rotation center is the shared handle point: the first
                // enabled key's pose from the drag's own scene.
                let center = projection_for_start
                    .cells
                    .iter()
                    .find(|cell| cell.enabled)
                    .or(projection_for_start.cells.first())
                    .map(|cell| cell.pose.at)
                    .unwrap_or(matrix_for_start.origin);
                TransformGesture::MatrixRotate {
                    anchor,
                    center,
                    start_angle: (start_point.y - center.y).atan2(start_point.x - center.x),
                    start_rotation: matrix_for_start.rotation.unwrap_or(0.0),
                }
            }
            other => other,
        };
        let operation = runtime_for_start.operation();
        let swing_basis = match gesture {
            TransformGesture::Splay { column, .. } | TransformGesture::Origin { column, .. } => {
                projection_for_start
                    .columns
                    .iter()
                    .find(|basis| basis.column == column)
            }
            _ => None,
        };
        let origin_base = swing_basis.map(|basis| basis.base);
        let start_origin = swing_basis.map(|basis| basis.splay_origin);
        *drag_for_start.borrow_mut() = Some(TransformDrag {
            pointer_id: pointer.pointer_id(),
            capture,
            matrix: matrix_for_start.clone(),
            gesture,
            transaction_id: format!("layout-transform-{}", operation.0),
            owner: current_owner.clone(),
            context: current_context.clone(),
            tool,
            basis,
            origin_base,
            start_origin,
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
            pending: None,
            preview_submitted: false,
            cancellation_dependency: cancellation_for_start.borrow().as_ref().and_then(
                |(owner, outcome)| {
                    (owner == &current_owner && outcome.borrow().is_none()).then(|| outcome.clone())
                },
            ),
        });
        guide_for_start.set(None);
        // Ticket 06: the popup follows the pointer from pointer-down, 18 px
        // off it like the prototype, until the drop re-docks or clears it.
        let mut popup_at_for_start = popup_at_for_start;
        popup_at_for_start.set(Some((
            start_point.x + 18.0 * world_per_screen,
            start_point.y - 18.0 * world_per_screen,
        )));
        rail_set_active(svg_key_for_start, Some(tool));
        quick_for_start.set(matches!(
            gesture,
            TransformGesture::Stagger {
                quick_offset: true,
                ..
            }
        ));
        swing_for_start.set(matches!(
            gesture,
            TransformGesture::Splay { swing: true, .. }
                | TransformGesture::Origin { swing: true, .. }
        ));
    });

    let runtime_for_move = runtime.0.clone();
    let svg_for_move = svg.0.clone();
    let drag_for_move = drag.clone();
    let arbiter_for_move = arbiter.clone();
    let cancellation_for_move = cancellation.clone();
    let guide_for_move = snap_guide;
    let feedback_for_move = feedback;
    let popup_at_for_move = popup_at;
    let snap_settings_for_move = snap_settings.clone();
    let snap_origins_for_move = snap_origins.clone();
    let move_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_move = guide_for_move;
        let mut feedback_for_move = feedback_for_move;
        let mut popup_at_for_move = popup_at_for_move;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let Some(mut active) = drag_for_move
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer.pointer_id())
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_move,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
            || active
                .cancellation_dependency
                .as_ref()
                .is_some_and(|outcome| {
                    outcome
                        .borrow()
                        .as_ref()
                        .is_some_and(|outcome| *outcome != TerminalOutcome::Completed)
                })
        {
            cancel_transform_drag(
                &runtime_for_move,
                &drag_for_move,
                &arbiter_for_move,
                &cancellation_for_move,
                &mut guide_for_move,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            return;
        }
        let Some(point) = crate::presentation::coordinates(
            &svg_for_move,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            return;
        };
        let operation = sample_transform(
            &active,
            &pointer,
            point,
            TransformSampleContext {
                view_width: width,
                view_height: height,
                settings: &snap_settings_for_move,
                origins: &snap_origins_for_move,
                affect: splay_affect(),
                svg: &svg_for_move,
                guide: &mut guide_for_move,
            },
        );
        if let Some(operation) = operation
            && active.pending.as_ref() != Some(&operation)
        {
            submit_transform_edit(&runtime_for_move, &active, operation.clone());
            active.preview_submitted = true;
            // Ticket 06: the popup follows the pointer with the live value
            // and delta; a non-placement gesture keeps its docked content.
            if let Some(TransformDragResult::Placement { delta, .. }) =
                transform_drag_result(&active, &operation)
            {
                let value = placement_target(&operation, &active.matrix)
                    .map(|(_, end)| format_placement_value(end))
                    .unwrap_or_default();
                feedback_for_move.set(advance_feedback(
                    &feedback_for_move(),
                    FeedbackEvent::Drag {
                        name: feedback_tool_name(tool).to_owned(),
                        value,
                        delta: placement_feedback_delta(delta),
                    },
                ));
            }
            popup_at_for_move.set(Some((
                point.x + 18.0 * world_per_screen,
                point.y - 18.0 * world_per_screen,
            )));
            active.pending = Some(operation);
            *drag_for_move.borrow_mut() = Some(active);
        }
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg.0.clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let cancellation_for_end = cancellation.clone();
    let guide_for_end = snap_guide;
    let feedback_for_end = feedback;
    let popup_at_for_end = popup_at;
    let toast_gen_for_end = toast_gen;
    let end_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_end = guide_for_end;
        let mut feedback_for_end = feedback_for_end;
        let mut popup_at_for_end = popup_at_for_end;
        let mut toast_gen_for_end = toast_gen_for_end;
        let svg_key_for_end = svg_key;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let Some(mut active) = drag_for_end
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer.pointer_id())
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_end,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
            || active
                .cancellation_dependency
                .as_ref()
                .is_some_and(|outcome| {
                    outcome
                        .borrow()
                        .as_ref()
                        .is_some_and(|outcome| *outcome != TerminalOutcome::Completed)
                })
        {
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &cancellation_for_end,
                &mut guide_for_end,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            return;
        }
        if pointer.type_() != "pointerup" {
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &cancellation_for_end,
                &mut guide_for_end,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            return;
        }
        let point =
            crate::presentation::coordinates(&svg_for_end, &pointer, view_x, view_y, width, height);
        let operation = point
            .and_then(|point| {
                sample_transform(
                    &active,
                    &pointer,
                    point,
                    TransformSampleContext {
                        view_width: width,
                        view_height: height,
                        settings: &snap_settings,
                        origins: &snap_origins,
                        affect: splay_affect(),
                        svg: &svg_for_end,
                        guide: &mut guide_for_end,
                    },
                )
            })
            .or_else(|| active.pending.clone());
        // The live value behind the toast and the re-docked popup.
        let live_value = operation
            .as_ref()
            .and_then(|operation| placement_target(operation, &active.matrix))
            .map(|(_, end)| format_placement_value(end))
            .unwrap_or_default();
        if let Some(operation) = operation {
            if active.pending.as_ref() != Some(&operation) {
                submit_transform_edit(&runtime_for_end, &active, operation.clone());
                active.preview_submitted = true;
            }
            if !same_transform_as_start(&active, &operation) {
                commit_transform_drag(
                    &runtime_for_end,
                    &active,
                    &operation,
                    workspace,
                    scope_generation,
                    selected_context,
                );
                // Ticket 06: a committed placement drag raises the value
                // toast as one Undo step. The toast writes nothing: it is a
                // signal the 1.8 s timer clears.
                if let Some(TransformDragResult::Placement { delta, .. }) =
                    transform_drag_result(&active, &operation)
                {
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Commit {
                            value: format!("{} {live_value}", feedback_tool_name(tool)),
                            delta: placement_feedback_delta(delta),
                        },
                    ));
                    let seen = {
                        toast_gen_for_end.set(toast_gen_for_end() + 1);
                        toast_gen_for_end()
                    };
                    arm_commit_toast_expiry(feedback_for_end, toast_gen_for_end, seen);
                }
            } else if active.preview_submitted {
                clear_transform_preview(&runtime_for_end, &active);
                feedback_for_end.set(advance_feedback(
                    &feedback_for_end(),
                    FeedbackEvent::Choose {
                        name: feedback_tool_name(tool).to_owned(),
                        value: live_value.clone(),
                    },
                ));
            }
        } else if active.preview_submitted {
            clear_transform_preview(&runtime_for_end, &active);
            feedback_for_end.set(advance_feedback(
                &feedback_for_end(),
                FeedbackEvent::Choose {
                    name: feedback_tool_name(tool).to_owned(),
                    value: live_value,
                },
            ));
        }
        popup_at_for_end.set(None);
        guide_for_end.set(None);
        rail_set_active(svg_key_for_end, None);
        let mut quick_for_end = quick_glyph;
        quick_for_end.set(false);
        let mut swing_for_end = swing_glyph;
        swing_for_end.set(false);
        // Releasing capture fires `lostpointercapture`; retire the drag before it can cancel the drop.
        drag_for_end.borrow_mut().take();
        if active.capture.has_pointer_capture(active.pointer_id) {
            let _ = active.capture.release_pointer_capture(active.pointer_id);
        }
        arbiter_for_end.release(CanvasInteractionOwner::MatrixTransform);
    });

    let runtime_for_cancel = runtime.0.clone();
    let drag_for_cancel = drag.clone();
    let arbiter_for_cancel = arbiter.clone();
    let cancellation_for_cancel = cancellation.clone();
    let guide_for_cancel = snap_guide;
    let feedback_for_cancel = feedback;
    let popup_at_for_cancel = popup_at;
    let cancel_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_cancel = guide_for_cancel;
        let mut feedback_for_cancel = feedback_for_cancel;
        let mut popup_at_for_cancel = popup_at_for_cancel;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if drag_for_cancel
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer_id == pointer.pointer_id())
        {
            pointer.prevent_default();
            pointer.stop_propagation();
            // Ticket 06: a cancelled drag takes its popup with it.
            feedback_for_cancel.set(advance_feedback(
                &feedback_for_cancel(),
                FeedbackEvent::Dismiss,
            ));
            popup_at_for_cancel.set(None);
            cancel_transform_drag(
                &runtime_for_cancel,
                &drag_for_cancel,
                &arbiter_for_cancel,
                &cancellation_for_cancel,
                &mut guide_for_cancel,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
        }
    });

    let runtime_for_key = runtime.0.clone();
    let drag_for_key = drag.clone();
    let arbiter_for_key = arbiter.clone();
    let cancellation_for_key = cancellation.clone();
    let guide_for_key = snap_guide;
    let feedback_for_key = feedback;
    let popup_at_for_key = popup_at;
    let on_key_down = Rc::new(move |event: KeyboardEvent| {
        let mut guide_for_key = guide_for_key;
        let mut feedback_for_key = feedback_for_key;
        let mut popup_at_for_key = popup_at_for_key;
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        if key.key() == "Escape" {
            key.prevent_default();
            key.stop_propagation();
            // Ticket 06: Escape clears the chosen tool and its popup.
            feedback_for_key.set(advance_feedback(
                &feedback_for_key(),
                FeedbackEvent::Dismiss,
            ));
            popup_at_for_key.set(None);
            cancel_transform_drag(
                &runtime_for_key,
                &drag_for_key,
                &arbiter_for_key,
                &cancellation_for_key,
                &mut guide_for_key,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            on_finish.call(());
            return;
        }
        if !matches!(
            key.key().as_str(),
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"
        ) || !transform_nudge_admitted(
            &runtime_for_key,
            workspace,
            scope_generation,
            selected_context,
            &owner_for_key,
            &context_for_key,
        ) {
            return;
        }
        let Some(target) = key
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest(".m1-transform-handle").ok().flatten())
        else {
            return;
        };
        let step = if key.shift_key() { 1.0 } else { 0.1 };
        let delta = Vec2 {
            x: match key.key().as_str() {
                "ArrowLeft" => -step,
                "ArrowRight" => step,
                _ => 0.0,
            },
            y: match key.key().as_str() {
                "ArrowDown" => -step,
                "ArrowUp" => step,
                _ => 0.0,
            },
        };
        let arrow = match key.key().as_str() {
            "ArrowLeft" => ArrowKey::Left,
            "ArrowRight" => ArrowKey::Right,
            "ArrowUp" => ArrowKey::Up,
            _ => ArrowKey::Down,
        };
        let nudge = match target.get_attribute("aria-label").as_deref() {
            Some("Drag to spread") => {
                let parameter = PlacementParameter::ColumnSpread {
                    column: selected_column,
                };
                let big = key.shift_key();
                // The pair across the spread axis is not consumed; column 0 has
                // no Spread handle, so its basis never reaches this mapping.
                let Some(basis) = SceneBasis::for_parameter(&projection_for_key, parameter) else {
                    return;
                };
                if nudge_value(
                    &basis,
                    parameter,
                    placement_current(&matrix_for_key, parameter),
                    arrow,
                    big,
                )
                .is_none()
                {
                    return;
                }
                TransformNudge::Spread {
                    column: selected_column,
                    key: arrow,
                    big,
                }
            }
            Some("Drag to stagger") => {
                let parameter = PlacementParameter::Stagger {
                    column: selected_column,
                };
                let big = key.shift_key();
                // The pair across the stagger axis is not consumed.
                let Some(basis) = SceneBasis::for_parameter(&projection_for_key, parameter) else {
                    return;
                };
                if context_row(&context_for_key).is_some()
                    || nudge_value(
                        &basis,
                        parameter,
                        placement_current(&matrix_for_key, parameter),
                        arrow,
                        big,
                    )
                    .is_none()
                {
                    return;
                }
                TransformNudge::Stagger {
                    column: selected_column,
                    key: arrow,
                    big,
                }
            }
            Some(label @ ("Drag to offset row" | "Drag to offset column")) => {
                let parameter = match (label, context_row(&context_for_key)) {
                    ("Drag to offset row", Some(row)) => PlacementParameter::RowOffset { row },
                    ("Drag to offset column", None) => PlacementParameter::ColumnOffset {
                        column: selected_column,
                    },
                    _ => return,
                };
                TransformNudge::Offset {
                    parameter,
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            // Ticket 05: Shift nudges step through the carried-shift frame
            // like an offset; Bend nudges turn the running frame by degrees,
            // mirroring the Splay angle nudge (Shift steps five).
            Some("Drag to shift row") => {
                let Some(row) = context_row(&context_for_key) else {
                    return;
                };
                TransformNudge::Offset {
                    parameter: PlacementParameter::RowShift { row },
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            Some("Drag to shift key") => {
                let Some((row, column)) = context_key(&context_for_key) else {
                    return;
                };
                TransformNudge::Offset {
                    parameter: PlacementParameter::KeyShift { row, column },
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            Some(label @ ("Drag to bend row" | "Drag to bend key")) if delta.x != 0.0 => {
                let parameter = match (label, &context_for_key) {
                    ("Drag to bend row", TreeContext::Row { row, .. }) => {
                        PlacementParameter::RowBend { row: *row }
                    }
                    ("Drag to bend key", TreeContext::Key { row, column, .. }) => {
                        PlacementParameter::KeyBend {
                            row: *row,
                            column: *column,
                        }
                    }
                    _ => return,
                };
                TransformNudge::Bend {
                    parameter,
                    degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
                }
            }
            Some("Move splay origin") => TransformNudge::SplayOrigin {
                column: selected_column,
            },
            Some("Drag to splay") if delta.x != 0.0 => TransformNudge::SplayAngle {
                column: selected_column,
                degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
                affect: splay_affect(),
            },
            Some("Drag to move matrix") => TransformNudge::MatrixMove,
            Some("Drag to rotate matrix") if delta.x != 0.0 => TransformNudge::MatrixRotate {
                degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
            },
            _ => return,
        };
        key.prevent_default();
        key.stop_propagation();
        // A held key queues one step per repeat; each resolves against the matrix the
        // previous steps produced.
        let resolver = transform_nudge_resolver(matrix_for_key.id.clone(), nudge, delta);
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime_for_key,
            (),
            "layout-transform-nudge",
            Some("transform".into()),
            resolver,
        );
        let runtime = runtime_for_key.clone();
        let owner = owner_for_key.clone();
        let context = context_for_key.clone();
        spawn_local(async move {
            loop {
                let is_live = transform_nudge_admitted(
                    &runtime,
                    workspace,
                    scope_generation,
                    selected_context,
                    &owner,
                    &context,
                );
                if !edits.settle(is_live).is_empty() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(16).await;
            }
        });
    });

    let gesture = match tool {
        LayoutTransformTool::Stagger => TransformGesture::Stagger {
            column: selected_column,
            quick_offset: false,
        },
        LayoutTransformTool::Spread => TransformGesture::Spread {
            column: selected_column,
        },
        LayoutTransformTool::Offset => TransformGesture::Offset {
            parameter: offset_parameter,
        },
        // A Shift drag edits the row's or key's carried shift through the
        // Offset gesture's shift axes; a Bend drag turns the running frame.
        // Without a row or key the tokens stay hidden, so the offsets below
        // never start a drag.
        LayoutTransformTool::Shift => TransformGesture::Offset {
            parameter: shift_parameter.unwrap_or(offset_parameter),
        },
        LayoutTransformTool::Bend => TransformGesture::Bend {
            parameter: bend_parameter.unwrap_or(offset_parameter),
        },
        LayoutTransformTool::Splay | LayoutTransformTool::Origin => TransformGesture::Origin {
            column: selected_column,
            swing: false,
        },
        LayoutTransformTool::Move => TransformGesture::MatrixMove { anchor: None },
        LayoutTransformTool::Rotate => TransformGesture::MatrixRotate {
            anchor: None,
            center: Vec2::default(),
            start_angle: 0.0,
            start_rotation: 0.0,
        },
        // Ticket 08: the Anchor token never mounts this drag overlay (the
        // pick overlay owns it), so no gesture starts here.
        LayoutTransformTool::Anchor => TransformGesture::MatrixMove { anchor: None },
    };
    let stagger_axis = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(Vec2 { x: 0.0, y: 1.0 }, |basis| basis.stagger_axis);
    let spread_now = match placement_current(
        &matrix_for_spread,
        PlacementParameter::ColumnSpread {
            column: selected_column,
        },
    ) {
        PlacementValue::Spread(value) => value,
        _ => matrix_for_spread.pitch.x,
    };
    let handle_label = match (tool, &context) {
        (LayoutTransformTool::Stagger, _) if control() || quick_glyph() => "Drag to offset column",
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => "Drag to offset row",
        (LayoutTransformTool::Offset, _) => "Drag to offset column",
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => "Drag to shift row",
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => "Drag to shift key",
        (LayoutTransformTool::Shift, _) => "Drag to shift",
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => "Drag to bend row",
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => "Drag to bend key",
        (LayoutTransformTool::Bend, _) => "Drag to bend",
        (LayoutTransformTool::Spread, _) => "Drag to spread",
        _ => "Drag to stagger",
    };
    let handle_value = match tool {
        LayoutTransformTool::Offset => {
            format!("X {:.2} mm, Y {:.2} mm", offset_now.x, offset_now.y)
        }
        LayoutTransformTool::Shift => {
            format!("X {:.2} mm, Y {:.2} mm", shift_now.x, shift_now.y)
        }
        LayoutTransformTool::Bend => format!("{bend_now:.1}°"),
        LayoutTransformTool::Spread => format!("{spread_now:.2} mm"),
        _ if control() || quick_glyph() => {
            format!("X {:.2} mm, Y {:.2} mm", offset_now.x, offset_now.y)
        }
        _ => format!("{stagger_now:.2} mm"),
    };
    let quick_offset_shown = quick_glyph() || control() || tool == LayoutTransformTool::Offset;
    let active_peer = rail_active_tool(svg_key);
    let dragging_here = drag.borrow().is_some();
    // A live drag on a peer dims this token to 35%; the active token keeps
    // full opacity. Siblings re-read the shared mount set whenever the
    // Editor re-renders, which every preview submit triggers.
    let rail_dimmed = active_peer.is_some_and(|peer| peer != tool) && rail_index.is_some();
    let token_active = dragging_here;
    let token_lit = hovered() || focused() || token_active;
    // Screen-frame glyph rotation for the column axis: the px groups below
    // are y-down, so the world axis appears mirrored vertically.
    let glyph_degrees = (-stagger_axis.y).atan2(stagger_axis.x).to_degrees();
    let token_fill = if token_active {
        "var(--wb-accent)"
    } else if hovered() || focused() {
        "var(--wb-selection-surface)"
    } else {
        "transparent"
    };
    let glyph_stroke = if token_active {
        "var(--wb-on-accent)"
    } else if hovered() || focused() {
        "var(--wb-selection-ink)"
    } else {
        "var(--wb-muted)"
    };
    let token_opacity = if rail_dimmed { "0.35" } else { "1" };
    let guide_opacity = if token_active { "0.8" } else { "0.35" };
    // Splay, Move and Rotate keep hardcoded labels: the gesture distinguishes
    // them, and the mounted drag tests query these strings.
    let token_aria_label: String = match (tool, &context) {
        (LayoutTransformTool::Splay, _) => "Drag to splay".to_owned(),
        (LayoutTransformTool::Move, _) => "Drag to move matrix".to_owned(),
        (LayoutTransformTool::Rotate, _) => "Drag to rotate matrix".to_owned(),
        (LayoutTransformTool::Anchor, _) => "Pick anchor".to_owned(),
        (LayoutTransformTool::Origin, _) => "Move splay origin".to_owned(),
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => "Drag to shift row".to_owned(),
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => "Drag to shift key".to_owned(),
        (LayoutTransformTool::Shift, _) => "Drag to shift".to_owned(),
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => "Drag to bend row".to_owned(),
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => "Drag to bend key".to_owned(),
        (LayoutTransformTool::Bend, _) => "Drag to bend".to_owned(),
        _ => handle_label.to_owned(),
    };
    let token_description: String = match tool {
        LayoutTransformTool::Spread => handle_value.clone(),
        LayoutTransformTool::Shift | LayoutTransformTool::Bend => handle_value.clone(),
        LayoutTransformTool::Stagger | LayoutTransformTool::Offset => {
            format!("{handle_value}. Hold Ctrl while dragging to offset the column.")
        }
        LayoutTransformTool::Splay => {
            "Hold Ctrl while dragging to swing the origin instead.".to_owned()
        }
        LayoutTransformTool::Move => {
            format!("X {:.2} mm, Y {:.2} mm", matrix.origin.x, matrix.origin.y)
        }
        LayoutTransformTool::Rotate => {
            if matrix_rotation_followed {
                format!("{:.2} degrees", matrix.rotation.unwrap_or(0.0))
            } else {
                "Follow rotation is off, so turning changes nothing.".to_owned()
            }
        }
        LayoutTransformTool::Origin => {
            "Hold Ctrl while dragging to swing the origin without moving the keys.".to_owned()
        }
        LayoutTransformTool::Anchor => {
            "Click a violet ring to anchor here. Esc leaves pick mode.".to_owned()
        }
    };
    let token_title: Option<String> = match tool {
        LayoutTransformTool::Spread => Some(format!("Drag to spread · {handle_value}")),
        LayoutTransformTool::Splay => Some(format!("Drag to splay · {angle_degrees:.0}°")),
        LayoutTransformTool::Move => Some(format!(
            "Drag to move matrix · X {:.2} mm, Y {:.2} mm",
            matrix.origin.x, matrix.origin.y
        )),
        LayoutTransformTool::Rotate if matrix_rotation_followed => Some(format!(
            "Drag to rotate matrix · {:.2} degrees",
            matrix.rotation.unwrap_or(0.0)
        )),
        LayoutTransformTool::Rotate => {
            Some("Rotate matrix (unavailable: Follow rotation is off)".to_owned())
        }
        LayoutTransformTool::Origin => Some("Move splay origin".to_owned()),
        LayoutTransformTool::Anchor => Some("Pick anchor".to_owned()),
        LayoutTransformTool::Stagger
        | LayoutTransformTool::Offset
        | LayoutTransformTool::Shift
        | LayoutTransformTool::Bend => None,
    };
    // The token centre in world units, when this overlay draws a rail token.
    // Splay also needs its origin basis: without it the drag has no frame.
    let token_center: Option<(f64, f64)> = rail_index.and_then(|index| {
        rail_layout
            .as_ref()
            .and_then(|layout| layout.token_centers.get(index).copied())
    });
    // Shift and Bend need their row or key parameter: on any other context
    // the token stays hidden rather than editing the wrong field.
    let token_shown = token_center.is_some()
        && (tool != LayoutTransformTool::Splay || origin_point.is_some())
        && (tool != LayoutTransformTool::Shift || shift_parameter.is_some())
        && (tool != LayoutTransformTool::Bend || bend_parameter.is_some());
    // Axis guide endpoints for the one-dimensional tokens, in world units.
    let (axis_x, axis_y) = match tool {
        LayoutTransformTool::Spread => (1.0, 0.0),
        _ => (stagger_axis.x, stagger_axis.y),
    };
    let axis_len = axis_x.hypot(axis_y).max(1e-9);
    let (guide_dx, guide_dy) = (
        axis_x / axis_len * 17.0 * world_per_screen,
        axis_y / axis_len * 17.0 * world_per_screen,
    );
    let px = world_per_screen;
    // The token drags its own gesture: Splay needs the origin frame the
    // shared `gesture` leaves to the Origin handle.
    let token_gesture = if tool == LayoutTransformTool::Splay {
        origin_point
            .map(|origin| TransformGesture::Splay {
                column: selected_column,
                start_angle: 0.0,
                origin,
                swing: false,
            })
            .unwrap_or(gesture)
    } else {
        gesture
    };
    // Ticket 06: the popup and toast content behind the chrome below.
    let feedback_state = feedback();
    rsx! {
        if let Some(guide) = snap_guide() {
            g { class: "m1-transform-snap-guide", "aria-hidden": "true",
                circle { cx: "{guide.x}", cy: "{guide.y}", r: "{width / 170.0}" }
            }
        }
        if token_shown {
            {
                let (tok_x, tok_y) = token_center.unwrap_or((0.0, 0.0));
                let rail_w = handle_rail::rail_width(rail_tools.len());
                let (rail_cx, rail_cy) = rail_layout.as_ref().map_or((tok_x, tok_y), |layout| {
                    (layout.rail.x + layout.rail.w / 2.0, layout.rail.y + layout.rail.h / 2.0)
                });
                let token_style =
                    format!("cursor:grab;opacity:{token_opacity};touch-action:none");
                let tok_style = format!("fill:{token_fill};stroke:none");
                let hit_style = "fill:transparent;stroke:none;pointer-events:all";
                let glyph = rail_token_glyph(tool, quick_offset_shown, glyph_degrees, glyph_stroke);
                let show_hover = show_preview.clone();
                let hide_hover = release_preview.clone();
                let show_focus = show_preview.clone();
                let hide_focus = release_preview.clone();
                rsx! {
                    if rail_index == Some(0) {
                        g { "aria-hidden": "true",
                            transform: "translate({rail_cx} {rail_cy}) scale({px} {-px})",
                            style: "pointer-events:none",
                            rect {
                                x: "{-rail_w / 2.0}",
                                y: "-11",
                                width: "{rail_w}",
                                height: "22",
                                rx: "6",
                                style: "fill:var(--wb-panel);stroke:var(--wb-rule-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))",
                            }
                        }
                    }
                    if tool == LayoutTransformTool::Rotate && !matrix_rotation_followed {
                        g {
                            class: "m1-transform-handle",
                            transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
                            role: "button", tabindex: "0", "aria-label": "{token_aria_label}",
                            "aria-disabled": "true",
                            "aria-description": "{token_description}",
                            style: "{token_style}",
                            rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "{tok_style}" }
                            rect { x: "-10", y: "-12", width: "20", height: "24", style: "{hit_style}" }
                            {glyph}
                            if let Some(title) = token_title.as_ref() {
                                title { "{title}" }
                            }
                        }
                    } else {
                        g {
                            class: "m1-transform-handle",
                            transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
                            role: "button", tabindex: "0", "aria-label": "{token_aria_label}",
                            "aria-description": "{token_description}",
                            "data-quick-offset": "{quick_offset_shown}",
                            "data-swing": "{swing_glyph()}",
                            style: "{token_style}",
                            onpointerdown: { let start = start_drag.clone(); move |event| start(event, token_gesture) },
                            onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                            onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                            onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                            onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                            onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                            // Ticket 06: hovering or focusing a token docks the
                            // value popup above the rail; leaving clears it.
                            // The content comes from the pure reducer, so the
                            // chrome never writes.
                            onpointerenter: { let mut hovered = hovered; let mut feedback = feedback; let mut popup_at = popup_at; let name = feedback_tool_name(tool).to_owned(); let value = handle_value.clone(); move |_| { hovered.set(true); show_hover(); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: name.clone(), value: value.clone() })); } },
                            onpointerleave: { let mut hovered = hovered; let mut feedback = feedback; move |_| { hovered.set(false); hide_hover(); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                            onfocus: { let mut focused = focused; let mut feedback = feedback; let mut popup_at = popup_at; let name = feedback_tool_name(tool).to_owned(); let value = handle_value.clone(); move |_| { focused.set(true); show_focus(); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: name.clone(), value: value.clone() })); } },
                            onblur: { let mut focused = focused; let mut feedback = feedback; move |_| { focused.set(false); hide_focus(); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                            rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "{tok_style}" }
                            rect { x: "-10", y: "-12", width: "20", height: "24", style: "{hit_style}" }
                            {glyph}
                            if let Some(title) = token_title.as_ref() {
                                title { "{title}" }
                            }
                        }
                    }
                    if token_lit
                        && matches!(
                            tool,
                            LayoutTransformTool::Spread
                                | LayoutTransformTool::Stagger
                                | LayoutTransformTool::Offset
                        )
                    {
                        line {
                            x1: "{tok_x - guide_dx}",
                            y1: "{tok_y - guide_dy}",
                            x2: "{tok_x + guide_dx}",
                            y2: "{tok_y + guide_dy}",
                            class: "m1-rail-guide",
                            style: "fill:none;stroke:var(--wb-accent);stroke-width:1px;stroke-dasharray:3 3;opacity:{guide_opacity};pointer-events:none;vector-effect:non-scaling-stroke",
                        }
                    }
                }
            }
        }
        if tool == LayoutTransformTool::Splay && token_lit {
            if let Some(origin) = origin_point {
                if let Some(arc_path) = arc.as_ref() {
                    path {
                        class: "m1-rail-guide",
                        d: "M{origin.x} {origin.y} L{angle_point.x} {angle_point.y}",
                        style: "fill:none;stroke:var(--wb-accent);stroke-width:1px;stroke-dasharray:3 3;opacity:{guide_opacity};pointer-events:none;vector-effect:non-scaling-stroke",
                    }
                    path {
                        class: "m1-rail-guide",
                        d: "{arc_path}",
                        style: "fill:none;stroke:var(--wb-accent);stroke-width:1px;stroke-dasharray:3 3;opacity:{guide_opacity};pointer-events:none;vector-effect:non-scaling-stroke",
                    }
                }
            }
        }
        if matches!(tool, LayoutTransformTool::Splay | LayoutTransformTool::Origin) {
            if let Some(origin) = origin_point {
                g {
                    class: "m1-transform-handle",
                    transform: "translate({origin.x} {origin.y}) scale({px} {-px})",
                    role: "button", tabindex: "0", "aria-label": "Move splay origin",
                    "aria-description": "Hold Ctrl while dragging to swing the origin without moving the keys.",
                    "data-swing": "{swing_glyph()}",
                    style: "cursor:grab;touch-action:none",
                    onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                    onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                    onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                    onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                    rect {
                        x: "-12",
                        y: "-12",
                        width: "24",
                        height: "24",
                        style: "fill:transparent;stroke:none;pointer-events:all",
                    }
                    circle {
                        r: "3",
                        style: "fill:var(--wb-panel);stroke:var(--wb-accent);stroke-width:1.5px;vector-effect:non-scaling-stroke",
                    }
                    path {
                        d: "M-6 0h12M0 -6v12",
                        style: "fill:none;stroke:var(--wb-accent);stroke-width:1px;vector-effect:non-scaling-stroke",
                    }
                    title { "Move splay origin" }
                }
            }
        }
        // Ticket 06: the value popup — docked 8 px above the rail while a
        // tool is hovered or focused, 18 px off the pointer while dragging —
        // and the commit toast at the top centre of the canvas. Both are
        // view-only: `pointer-events: none`, no tabindex, no document write.
        if let Some(popup) = feedback_state.popup.as_ref() {
            {feedback_popup_panel(
                popup,
                popup_at(),
                rail_layout.as_ref().map(|layout| {
                    (
                        layout.rail.x,
                        layout.rail.y,
                        layout.rail.w,
                        layout.rail.h,
                    )
                }),
                px,
            )}
        }
        if let Some(toast) = feedback_state.toast.as_ref() {
            {feedback_toast_panel(toast, view_x, view_y, width, px)}
        }
    }
}

/// Ticket 07: the part rail peers share one mount set per svg, like the
/// matrix rail, but in part order (Move, Rotate, Spread, Stagger) so the
/// drawn tokens match the `part_rail` tool list.
#[derive(Default)]
struct PartRailMount {
    tools: Vec<LayoutTransformTool>,
    active: Option<LayoutTransformTool>,
}

thread_local! {
    static PART_RAIL_MOUNTS: RefCell<HashMap<usize, PartRailMount>> = RefCell::new(HashMap::new());
}

/// Canonical part rail order, matching the `part_rail` listing order.
const PART_RAIL_ORDER: [LayoutTransformTool; 5] = [
    LayoutTransformTool::Move,
    LayoutTransformTool::Rotate,
    LayoutTransformTool::Spread,
    LayoutTransformTool::Stagger,
    LayoutTransformTool::Anchor,
];

fn part_rail_mount_tool(svg_key: usize, tool: LayoutTransformTool) {
    PART_RAIL_MOUNTS.with(|mounts| {
        let mut mounts = mounts.borrow_mut();
        let mount = mounts.entry(svg_key).or_default();
        if !mount.tools.contains(&tool) {
            mount.tools.push(tool);
        }
    });
}

fn part_rail_unmount_tool(svg_key: usize, tool: LayoutTransformTool) {
    PART_RAIL_MOUNTS.with(|mounts| {
        if let Some(mount) = mounts.borrow_mut().get_mut(&svg_key) {
            mount.tools.retain(|peer| *peer != tool);
            if mount.active == Some(tool) {
                mount.active = None;
            }
        }
    });
}

fn part_rail_set_active(svg_key: usize, tool: Option<LayoutTransformTool>) {
    PART_RAIL_MOUNTS.with(|mounts| {
        if let Some(mount) = mounts.borrow_mut().get_mut(&svg_key) {
            mount.active = tool;
        }
    });
}

/// The part rail peers for one overlay: the mounted part tools in part
/// order. Members, companions and locked parts never mount, so the list only
/// ever holds one independent part's tools.
fn part_rail_peers(svg_key: usize) -> Vec<LayoutTransformTool> {
    PART_RAIL_MOUNTS.with(|mounts| {
        mounts
            .borrow()
            .get(&svg_key)
            .map_or_else(Vec::new, |mount| {
                PART_RAIL_ORDER
                    .iter()
                    .filter(|peer| mount.tools.contains(peer))
                    .copied()
                    .collect()
            })
    })
}

fn part_rail_active_tool(svg_key: usize) -> Option<LayoutTransformTool> {
    PART_RAIL_MOUNTS.with(|mounts| mounts.borrow().get(&svg_key).and_then(|mount| mount.active))
}

/// Ticket 07: which anchor value a part drag writes. Spread and Stagger edit
/// the values carried from the anchor key; Offset and Rotation edit the
/// part's local values. Detached parts have no anchor: Move drives the
/// shipped part-drag gesture, and Rotate commits one `SetPartRotation`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PartAnchorAxis {
    Spread,
    Stagger,
    Offset,
    Rotation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PartRailDragKind {
    AnchorValue(PartAnchorAxis),
    DetachedMove,
    DetachedRotate,
}

fn part_drag_kind(tool: LayoutTransformTool, anchored: bool) -> Option<PartRailDragKind> {
    match (tool, anchored) {
        (LayoutTransformTool::Move, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Offset))
        }
        (LayoutTransformTool::Rotate, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Rotation))
        }
        (LayoutTransformTool::Spread, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Spread))
        }
        (LayoutTransformTool::Stagger, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Stagger))
        }
        (LayoutTransformTool::Move, false) => Some(PartRailDragKind::DetachedMove),
        (LayoutTransformTool::Rotate, false) => Some(PartRailDragKind::DetachedRotate),
        _ => None,
    }
}

/// Ticket 07: the carried (Spread/Stagger) axes rotate with the
/// follow-filtered anchor key; the local Offset frame adds the carried Splay,
/// matching Core's anchor evaluation.
fn part_placement_basis(base_degrees: f64, splay_degrees: f64) -> SceneBasis {
    let base = base_degrees.to_radians();
    let frame = (base_degrees + splay_degrees).to_radians();
    SceneBasis {
        stagger_axis: Vec2 {
            x: -base.sin(),
            y: base.cos(),
        },
        axis_x: Vec2 {
            x: frame.cos(),
            y: frame.sin(),
        },
        axis_y: Vec2 {
            x: -frame.sin(),
            y: frame.cos(),
        },
        shift_axis_x: Vec2 {
            x: frame.cos(),
            y: frame.sin(),
        },
        shift_axis_y: Vec2 {
            x: -frame.sin(),
            y: frame.cos(),
        },
        spread_axis: Some(Vec2 {
            x: base.cos(),
            y: base.sin(),
        }),
    }
}

/// Ticket 07: a part Rotate sweep snaps to half degrees; Alt keeps the raw
/// angle.
fn part_rotate_value(start: f64, swept_degrees: f64, alt: bool) -> f64 {
    let value = start + swept_degrees;
    if alt {
        value
    } else {
        (value / 0.5).round() * 0.5
    }
}

/// Ticket 07: the member part owning an anchor key, so the drag frame follows
/// the evaluated key (frozen members included) exactly like Core.
fn anchor_key_member_id(anchor: &PartAnchor) -> String {
    format!(
        "matrix/{}/r{}c{}",
        anchor.matrix_id, anchor.row, anchor.column
    )
}

/// Ticket 07: the follow-filtered anchor key rotation behind a part drag, or
/// `None` when the anchor key no longer resolves.
fn anchor_key_base_rotation(document: &ProjectDoc, anchor: &PartAnchor) -> Option<f64> {
    let member = document
        .parts
        .iter()
        .find(|part| part.id == anchor_key_member_id(anchor))?;
    let rotation = evaluated_part_pose(document, member).rotation;
    Some(if anchor.follow.rotation {
        rotation
    } else {
        0.0
    })
}

/// Ticket 07: a part drag captured at pointer-down, mirroring the matrix
/// `TransformDrag` lifecycle (preview while moving, one composed commit on
/// drop). The anchor snapshot and frames never move mid-drag; the commit
/// re-solves the net delta against whatever is accepted when it runs.
#[derive(Clone)]
struct PartDrag {
    pointer_id: i64,
    capture: SvgElement,
    part_id: String,
    kind: PartRailDragKind,
    tool: LayoutTransformTool,
    transaction_id: String,
    owner: LayoutOwnerIdentity,
    context: TreeContext,
    anchor: Option<PartAnchor>,
    start_spread: f64,
    start_stagger: f64,
    start_offset: Vec2,
    start_rotation: f64,
    start_at: Vec2,
    base_rotation: f64,
    frame_rotation: f64,
    center: Vec2,
    start_angle: f64,
    start_client_x: f64,
    start_client_y: f64,
    /// A detached Move only begins the session gesture past the shipped body
    /// threshold, so a click never churns Core.
    gesture_begun: bool,
    preview_submitted: bool,
    pending: Option<EditOperation>,
}

/// Ticket 07: the net change a finished anchored drag applies, re-solved
/// against the accepted anchor when the commit runs.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PartAxisDelta {
    Spread(f64),
    Stagger(f64),
    Offset(Vec2),
    Rotation(f64),
}

/// Ticket 07: why an anchored part-drag edit has to retire: the part is gone,
/// locked, driven, a member, or no longer anchored. Mirrors the Inspector's
/// anchor guard so rail drags and typed edits agree.
fn part_anchor_drag_guard(document: &ProjectDoc, part_id: &str) -> Option<String> {
    let part = document.parts.iter().find(|part| part.id == part_id)?;
    if part.locked == Some(true) {
        return Some("This part is locked.".into());
    }
    if part.anchor.is_none() {
        return Some("The part is no longer anchored.".into());
    }
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(&part.id))
    {
        return Some("This part follows its matrix.".into());
    }
    match document.constraint_driving_part(part_id) {
        Ok(Some(_)) => Some("This part is driven by a relationship.".into()),
        Ok(None) => None,
        Err(_) => Some("The saved placement relationship could not be verified.".into()),
    }
}

/// Ticket 07: resolve a finished anchored part drag against the accepted
/// document: the net delta applies to whatever the anchor holds when the
/// edit runs, so a drag queued behind other edits composes.
fn part_anchor_drag_resolver(
    part_id: String,
    transaction_id: String,
    delta: PartAxisDelta,
) -> EditResolver {
    EditResolver::new(
        "layout-part-rail-drag",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if let Some(reason) = part_anchor_drag_guard(document, &part_id) {
                return Resolution::Retire(reason);
            }
            let anchor = document
                .parts
                .iter()
                .find(|part| part.id == part_id)
                .and_then(|part| part.anchor.clone())
                .expect("guarded above");
            let change = match delta {
                PartAxisDelta::Spread(delta) => {
                    PartAnchorValueChange::Spread(anchor.spread + delta)
                }
                PartAxisDelta::Stagger(delta) => {
                    PartAnchorValueChange::Stagger(anchor.stagger + delta)
                }
                PartAxisDelta::Offset(delta) => PartAnchorValueChange::Offset(Vec2 {
                    x: anchor.offset.x + delta.x,
                    y: anchor.offset.y + delta.y,
                }),
                PartAxisDelta::Rotation(delta) => {
                    PartAnchorValueChange::Rotation(anchor.rotation + delta)
                }
            };
            Resolution::submit_with_transaction_id(
                vec![part_id.clone()],
                transaction_id.clone(),
                EditOperation::SetPartAnchorValues {
                    part_id: part_id.clone(),
                    change,
                },
            )
        },
    )
}

fn submit_part_preview(runtime: &Rc<Runtime>, drag: &PartDrag, operation: EditOperation) {
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: drag.transaction_id.clone(),
        target_ids: vec![drag.part_id.clone()],
        operation,
    });
}

fn clear_part_preview(runtime: &Rc<Runtime>, drag: &PartDrag) {
    if !drag.preview_submitted {
        return;
    }
    if let (Some(token), Some(revision), Some(scope)) = (
        drag.owner.token,
        drag.owner.revision,
        drag.owner.scope.as_ref(),
    ) {
        runtime.submit(Event::ClearPreview {
            operation_id: runtime.operation(),
            token,
            revision,
            board_id: scope.board_id.clone(),
            transaction_id: drag.transaction_id.clone(),
        });
    }
}

/// Ticket 07: cancel a part drag (Escape, lost capture, owner change): clear
/// the anchored preview or the detached session gesture, release the pointer
/// and the canvas, and land nothing. Returns whether a drag was active.
fn cancel_part_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<PartDrag>>>,
    arbiter: &CanvasInteractionArbiter,
    svg_key: usize,
) -> bool {
    let Some(active) = drag.borrow_mut().take() else {
        return false;
    };
    match active.kind {
        PartRailDragKind::AnchorValue(_) | PartRailDragKind::DetachedRotate => {
            clear_part_preview(runtime, &active)
        }
        PartRailDragKind::DetachedMove => {
            if active.gesture_begun {
                runtime.submit(Event::GestureCancel {
                    pointer_id: active.pointer_id,
                });
            }
        }
    }
    if active.capture.has_pointer_capture(active.pointer_id as i32) {
        let _ = active
            .capture
            .release_pointer_capture(active.pointer_id as i32);
    }
    arbiter.release(CanvasInteractionOwner::MatrixTransform);
    part_rail_set_active(svg_key, None);
    true
}

fn commit_part_anchor_drag(runtime: &Rc<Runtime>, drag: &PartDrag, delta: PartAxisDelta) {
    let mut edits = PendingEdits::default();
    edits.begin(
        runtime,
        (),
        "layout-part-rail-drag",
        Some("transform".into()),
        part_anchor_drag_resolver(drag.part_id.clone(), drag.transaction_id.clone(), delta),
    );
    spawn_local(async move {
        loop {
            if !edits.settle(true).is_empty() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(16).await;
        }
    });
}

/// Ticket 07: why a detached Rotate edit has to retire: the part is gone,
/// locked, anchored, a member, or driven. Mirrors the Inspector's guards so
/// rail drags and typed edits agree.
fn part_detached_rotate_guard(document: &ProjectDoc, part_id: &str) -> Option<String> {
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return Some("The part no longer exists.".into());
    };
    if part.locked == Some(true) {
        return Some("This part is locked.".into());
    }
    if part.anchor.is_some() {
        return Some("The part is now anchored.".into());
    }
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(&part.id))
    {
        return Some("This part follows its matrix.".into());
    }
    match document.constraint_driving_part(part_id) {
        Ok(Some(_)) => Some("This part is driven by a relationship.".into()),
        Ok(None) => None,
        Err(_) => Some("The saved placement relationship could not be verified.".into()),
    }
}

/// Ticket 07: resolve a finished detached Rotate drag against the accepted
/// document: the net delta applies to whatever the pose holds when the edit
/// runs, so a drag queued behind other edits composes.
fn part_detached_rotate_resolver(
    part_id: String,
    transaction_id: String,
    delta: f64,
) -> EditResolver {
    EditResolver::new(
        "layout-part-rail-drag",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if let Some(reason) = part_detached_rotate_guard(document, &part_id) {
                return Resolution::Retire(reason);
            }
            let rotation = document
                .parts
                .iter()
                .find(|part| part.id == part_id)
                .map(|part| evaluated_part_pose(document, part).rotation)
                .expect("guarded above")
                + delta;
            Resolution::submit_with_transaction_id(
                vec![part_id.clone()],
                transaction_id.clone(),
                EditOperation::SetPartRotation {
                    part_id: part_id.clone(),
                    rotation,
                },
            )
        },
    )
}

fn commit_part_detached_rotate(runtime: &Rc<Runtime>, drag: &PartDrag, delta: f64) {
    let mut edits = PendingEdits::default();
    edits.begin(
        runtime,
        (),
        "layout-part-rail-drag",
        Some("transform".into()),
        part_detached_rotate_resolver(drag.part_id.clone(), drag.transaction_id.clone(), delta),
    );
    spawn_local(async move {
        loop {
            if !edits.settle(true).is_empty() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(16).await;
        }
    });
}

/// Ticket 07: one live sample of an anchored part drag: the absolute
/// `SetPartAnchorValues` change for the preview, the net change for the popup,
/// and the composed delta for the commit.
fn sample_anchor_value(
    drag: &PartDrag,
    axis: PartAnchorAxis,
    world_delta: Vec2,
    point: Vec2,
    alt: bool,
    pitch: Vec2,
    snap_fraction: f64,
) -> Option<(EditOperation, FeedbackDelta, PartAxisDelta, String)> {
    let anchor = drag.anchor.as_ref()?;
    let basis = part_placement_basis(drag.base_rotation, drag.frame_rotation - drag.base_rotation);
    if let PartAnchorAxis::Rotation = axis {
        let angle = (point.y - drag.center.y).atan2(point.x - drag.center.x) - drag.start_angle;
        let swept = angle.sin().atan2(angle.cos()).to_degrees();
        let value = part_rotate_value(drag.start_rotation, swept, alt);
        let delta = value - drag.start_rotation;
        return Some((
            EditOperation::SetPartAnchorValues {
                part_id: drag.part_id.clone(),
                change: PartAnchorValueChange::Rotation(value),
            },
            FeedbackDelta::Deg(delta),
            PartAxisDelta::Rotation(delta),
            format!("{value:.1}°"),
        ));
    }
    let snap = if alt {
        None
    } else {
        grid_step(pitch, snap_fraction)
    };
    // A Stagger drag steps along the dominant screen axis, like the matrix
    // Stagger preview; Spread steps the column pitch directly.
    let snap = match axis {
        PartAnchorAxis::Stagger if basis.stagger_axis.x.abs() > basis.stagger_axis.y.abs() => snap
            .map(|step| Vec2 {
                x: step.x,
                y: step.x,
            }),
        _ => snap,
    };
    let (parameter, start) = match axis {
        PartAnchorAxis::Spread => (
            PlacementParameter::ColumnSpread {
                column: anchor.column,
            },
            PlacementValue::Spread(drag.start_spread),
        ),
        PartAnchorAxis::Stagger => (
            PlacementParameter::Stagger {
                column: anchor.column,
            },
            PlacementValue::Stagger(drag.start_stagger),
        ),
        PartAnchorAxis::Offset => (
            PlacementParameter::ColumnOffset {
                column: anchor.column,
            },
            PlacementValue::Offset(drag.start_offset),
        ),
        PartAnchorAxis::Rotation => return None,
    };
    let value = gesture_value(&basis, parameter, start, world_delta, snap);
    let (change, feedback, delta, value_text) = match (axis, value) {
        (PartAnchorAxis::Spread, PlacementValue::Spread(value)) => (
            PartAnchorValueChange::Spread(value),
            FeedbackDelta::Mm(value - drag.start_spread),
            PartAxisDelta::Spread(value - drag.start_spread),
            format_placement_value(PlacementValue::Spread(value)),
        ),
        (PartAnchorAxis::Stagger, PlacementValue::Stagger(value)) => (
            PartAnchorValueChange::Stagger(value),
            FeedbackDelta::Mm(value - drag.start_stagger),
            PartAxisDelta::Stagger(value - drag.start_stagger),
            format_placement_value(PlacementValue::Stagger(value)),
        ),
        (PartAnchorAxis::Offset, PlacementValue::Offset(value)) => (
            PartAnchorValueChange::Offset(value),
            FeedbackDelta::Mm2(value.x - drag.start_offset.x, value.y - drag.start_offset.y),
            PartAxisDelta::Offset(Vec2 {
                x: value.x - drag.start_offset.x,
                y: value.y - drag.start_offset.y,
            }),
            format_placement_value(PlacementValue::Offset(value)),
        ),
        _ => return None,
    };
    Some((
        EditOperation::SetPartAnchorValues {
            part_id: drag.part_id.clone(),
            change,
        },
        feedback,
        delta,
        value_text,
    ))
}

/// Ticket 07: whether a part rail drag may start: the session is Ready and
/// saved at the owner's revision, no preview or gesture is in flight, and the
/// part is still in the accepted document.
fn part_start_is_editable(
    runtime: &Rc<Runtime>,
    owner: &LayoutOwnerIdentity,
    part_id: &str,
) -> bool {
    let model = runtime.model();
    model.lifecycle == Lifecycle::Ready
        && owner
            .revision
            .is_some_and(|revision| model.durability == (Durability::Saved { revision }))
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot
                .document
                .parts
                .iter()
                .any(|part| part.id == part_id)
        })
}

/// Ticket 07: the rail token for one independent part tool. Move and Rotate
/// edit the part's local Offset and Rotation; Spread and Stagger edit the
/// values carried from its anchor key. A detached part moves through the
/// shipped part-drag gesture; its Rotate sweep previews the popup and commits
/// one `SetPartRotation`.
#[component]
#[allow(clippy::too_many_arguments)]
pub fn LayoutPartToolOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    part_id: String,
    tool: LayoutTransformTool,
    snap: GestureSnapInputs,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    on_finish: EventHandler<()>,
    on_focus_half: EventHandler<()>,
) -> Element {
    let _ = surface_size.map(|size| size());
    let svg_key: usize = Rc::as_ptr(&svg.0) as usize;
    use_hook(move || {
        part_rail_mount_tool(svg_key, tool);
    });
    let mut mounted = use_signal(|| false);
    let mounted_read = mounted;
    use_effect(move || {
        mounted.set(true);
    });
    let _ = mounted_read();
    use_drop(move || {
        part_rail_unmount_tool(svg_key, tool);
    });
    let drag = use_hook(|| Rc::new(RefCell::new(None::<PartDrag>)));
    let hovered = use_signal(|| false);
    let focused = use_signal(|| false);
    // Ticket 06, reused: the value popup and commit toast content (pure
    // reducer; rendering only), the popup's top-left in world units (`None`
    // docks it above the rail), and the toast generation the 1.8 s timer
    // checks.
    let feedback = use_signal(FeedbackState::default);
    let popup_at = use_signal(|| None::<(f64, f64)>);
    let toast_gen = use_signal(|| 0_u64);
    let runtime_inner = runtime.0.clone();
    let drag_for_drop = drag.clone();
    let arbiter_for_drop = arbiter.clone();
    use_drop(move || {
        cancel_part_drag(&runtime_inner, &drag_for_drop, &arbiter_for_drop, svg_key);
    });
    let escape_listener = use_hook(TransformEscapeListener::default);
    use_effect({
        let listener = escape_listener.clone();
        let runtime = runtime.0.clone();
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let feedback_for_escape = feedback;
        let popup_at_for_escape = popup_at;
        move || {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let runtime = runtime.clone();
            let drag = drag.clone();
            let arbiter = arbiter.clone();
            let mut feedback_for_escape = feedback_for_escape;
            let mut popup_at_for_escape = popup_at_for_escape;
            let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() != "Escape" || event.default_prevented() {
                    return;
                }
                if event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|target| {
                        target
                            .closest("input, textarea, select, [contenteditable='true']")
                            .ok()
                            .flatten()
                            .is_some()
                    })
                {
                    return;
                }
                if drag.borrow().is_none() && arbiter.current().is_some() {
                    return;
                }
                event.prevent_default();
                feedback_for_escape.set(advance_feedback(
                    &feedback_for_escape(),
                    FeedbackEvent::Dismiss,
                ));
                popup_at_for_escape.set(None);
                cancel_part_drag(&runtime, &drag, &arbiter, svg_key);
                on_finish.call(());
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
            *listener.borrow_mut() = Some((document, callback));
        }
    });
    use_drop({
        let listener = escape_listener.clone();
        move || {
            if let Some((document, callback)) = listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    });
    // A stale drag (owner, selection or tool moved on) cancels before it
    // can land: the effect re-runs whenever the selection or scope
    // generation changes.
    let owner_for_effect = owner.clone();
    let part_for_effect = part_id.clone();
    let runtime_for_effect = runtime.0.clone();
    let drag_for_effect = drag.clone();
    let arbiter_for_effect = arbiter.clone();
    use_effect(move || {
        let _ = selected_context();
        let _ = scope_generation();
        let context = TreeContext::Component {
            part_id: Some(part_for_effect.clone()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        if drag_for_effect.borrow().as_ref().is_some_and(|active| {
            active.owner != owner_for_effect || active.context != context || active.tool != tool
        }) {
            cancel_part_drag(
                &runtime_for_effect,
                &drag_for_effect,
                &arbiter_for_effect,
                svg_key,
            );
        }
    });

    // The live part behind this token: a missing, locked, member or
    // companion part hides its token instead of editing the wrong field.
    // A detached part keeps its stored pose; the anchor snapshot stays empty
    // and only Move and Rotate reach the drag.
    let live = {
        let model = runtime.0.model();
        model.accepted.as_ref().and_then(|snapshot| {
            let document = &snapshot.document;
            let part = document.parts.iter().find(|part| part.id == part_id)?;
            if part.locked == Some(true) {
                return None;
            }
            if document
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids.contains(&part.id))
            {
                return None;
            }
            let anchor = part.anchor.clone();
            part_drag_kind(tool, anchor.is_some())?;
            let evaluated = evaluated_part_pose(document, part);
            let courtyard_top = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .map(|definition| {
                    definition
                        .courtyard
                        .iter()
                        .map(|point| point.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .filter(|top| top.is_finite())
                .unwrap_or(0.0);
            let base_rotation = anchor
                .as_ref()
                .and_then(|anchor| anchor_key_base_rotation(document, anchor))
                .unwrap_or(0.0);
            Some((anchor, evaluated, courtyard_top, base_rotation))
        })
    };
    let Some((anchor, evaluated, courtyard_top, base_rotation)) = live else {
        return rsx! {};
    };
    let handle_scale = svg.0.borrow().as_ref().map_or(3.0, |surface| {
        let rect = surface.get_bounding_client_rect();
        if rect.width() > 0.0 {
            (width / rect.width()).max(height / rect.height().max(1.0)) * 3.0
        } else {
            3.0
        }
    });
    // World units per screen px: the overlay draws screen-size chrome from
    // px art scaled by this factor, so tokens stay 18 px at every zoom.
    let world_per_screen = handle_scale / 3.0;
    let rail_tools = part_rail_peers(svg_key);
    let rail_index = rail_tools.iter().position(|peer| *peer == tool);
    // The rail centres on the part: its evaluated centre with the top edge
    // of its courtyard, matching the matrix rail's centred row.
    let rail_layout = if rail_index.is_some() {
        handle_rail::rail_world(
            evaluated.at.x,
            evaluated.at.y + courtyard_top,
            rail_tools.len(),
            world_per_screen,
        )
    } else {
        None
    };
    let Some(rail_layout) = rail_layout else {
        return rsx! {};
    };
    let (start_spread, start_stagger, start_offset, start_rotation) = anchor
        .as_ref()
        .map(|anchor| {
            (
                anchor.spread,
                anchor.stagger,
                anchor.offset,
                anchor.rotation,
            )
        })
        .unwrap_or((0.0, 0.0, Vec2::default(), evaluated.rotation));
    let handle_value = match tool {
        LayoutTransformTool::Spread => {
            format!("{:.2} mm", start_spread)
        }
        LayoutTransformTool::Stagger => {
            format!("{:.2} mm", start_stagger)
        }
        LayoutTransformTool::Rotate => format!("{start_rotation:.1}°"),
        // An anchored Move edits the local Offset; a detached Move edits the
        // board position, so each shows the value its drag writes.
        LayoutTransformTool::Move => {
            let at = if anchor.is_some() {
                start_offset
            } else {
                evaluated.at
            };
            format!("X {:.2}, Y {:.2} mm", at.x, at.y)
        }
        _ => format!("{start_stagger:.2} mm"),
    };
    let token_aria_label: String = match tool {
        LayoutTransformTool::Move => "Drag to move part".to_owned(),
        LayoutTransformTool::Rotate => "Drag to rotate part".to_owned(),
        LayoutTransformTool::Spread => "Drag to spread".to_owned(),
        _ => "Drag to stagger".to_owned(),
    };
    let token_description: String = match tool {
        LayoutTransformTool::Rotate => format!("{start_rotation:.2} degrees"),
        _ => handle_value.clone(),
    };
    let token_title: Option<String> = match tool {
        LayoutTransformTool::Move => Some(format!("Drag to move part · {handle_value}")),
        LayoutTransformTool::Rotate => {
            Some(format!("Drag to rotate part · {start_rotation:.2} degrees"))
        }
        LayoutTransformTool::Spread => Some(format!("Drag to spread · {handle_value}")),
        _ => None,
    };
    // Screen-frame glyph rotation for the one-dimensional carried axes: the
    // px groups below are y-down, so the world axis appears mirrored
    // vertically, exactly like the matrix tokens.
    let glyph_degrees = match tool {
        LayoutTransformTool::Spread => (-base_rotation.to_radians().sin())
            .atan2(base_rotation.to_radians().cos())
            .to_degrees(),
        LayoutTransformTool::Stagger => (base_rotation.to_radians().cos())
            .atan2(-base_rotation.to_radians().sin())
            .to_degrees(),
        _ => 0.0,
    };
    let active_peer = part_rail_active_tool(svg_key);
    let dragging_here = drag.borrow().is_some();
    // A live drag on a peer dims this token to 35%; the active token keeps
    // full opacity. Siblings re-read the shared mount set whenever the
    // Editor re-renders, which each preview submit triggers.
    let rail_dimmed = active_peer.is_some_and(|peer| peer != tool) && rail_index.is_some();
    let token_active = dragging_here;
    let token_lit = hovered() || focused() || token_active;
    let token_fill = if token_active {
        "var(--wb-accent)"
    } else if hovered() || focused() {
        "var(--wb-selection-surface)"
    } else {
        "transparent"
    };
    let glyph_stroke = if token_active {
        "var(--wb-on-accent)"
    } else if hovered() || focused() {
        "var(--wb-selection-ink)"
    } else {
        "var(--wb-muted)"
    };
    let token_opacity = if rail_dimmed { "0.35" } else { "1" };
    let guide_opacity = if token_active { "0.8" } else { "0.35" };
    let (axis_x, axis_y) = match tool {
        LayoutTransformTool::Spread => {
            let radians = base_rotation.to_radians();
            (radians.cos(), radians.sin())
        }
        _ => {
            let radians = base_rotation.to_radians();
            (-radians.sin(), radians.cos())
        }
    };
    let axis_len = axis_x.hypot(axis_y).max(1e-9);
    let (guide_dx, guide_dy) = (
        axis_x / axis_len * 17.0 * world_per_screen,
        axis_y / axis_len * 17.0 * world_per_screen,
    );
    let px = world_per_screen;
    let token_center: (f64, f64) = rail_index
        .and_then(|index| rail_layout.token_centers.get(index).copied())
        .unwrap_or((evaluated.at.x, evaluated.at.y + courtyard_top));
    let feedback_state = feedback();

    let part_context = TreeContext::Component {
        part_id: Some(part_id.clone()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    let runtime_for_start = runtime.0.clone();
    let svg_for_start = svg.0.clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let context_for_start = part_context.clone();
    let owner_for_start = owner.clone();
    let part_for_start = part_id.clone();
    let start_drag = Rc::new(move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        // A token press focuses the part's half, like a body click, so a
        // drag on the dimmed half keeps editing that half.
        on_focus_half.call(());
        if pointer.button() != 0 || drag_for_start.borrow().is_some() {
            return;
        }
        if !transform_owner_is_current(
            &runtime_for_start,
            workspace,
            scope_generation,
            selected_context,
            &owner_for_start,
            &context_for_start,
        ) || !part_start_is_editable(&runtime_for_start, &owner_for_start, &part_for_start)
            || runtime_for_start.model().gesture.is_some()
            || !arbiter_for_start.try_acquire(CanvasInteractionOwner::MatrixTransform)
        {
            return;
        }
        let Some(accepted) = runtime_for_start.model().accepted.clone() else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        let document = &accepted.document;
        let Some(part) = document.parts.iter().find(|part| part.id == part_for_start) else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if part.locked == Some(true)
            || document
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids.contains(&part.id))
        {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        let anchor = part.anchor.clone();
        let Some(kind) = part_drag_kind(tool, anchor.is_some()) else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if let PartRailDragKind::AnchorValue(_) = kind
            && part_anchor_drag_guard(document, &part_for_start).is_some()
        {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        let Some(capture) = pointer
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest(".m1-transform-handle").ok().flatten())
            .and_then(|target| target.dyn_into::<SvgElement>().ok())
        else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if capture.set_pointer_capture(pointer.pointer_id()).is_err() {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        pointer.prevent_default();
        pointer.stop_propagation();
        let focus_options = web_sys::FocusOptions::new();
        focus_options.set_prevent_scroll(true);
        let _ = capture.focus_with_options(&focus_options);
        let Some(start_point) = crate::presentation::coordinates(
            &svg_for_start,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            let _ = capture.release_pointer_capture(pointer.pointer_id());
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        let evaluated = evaluated_part_pose(document, part);
        let base_rotation = anchor
            .as_ref()
            .and_then(|anchor| anchor_key_base_rotation(document, anchor))
            .unwrap_or(0.0);
        let splay = anchor.as_ref().map_or(0.0, |anchor| anchor.splay);
        let (start_spread, start_stagger, start_offset, start_rotation) = anchor
            .as_ref()
            .map(|anchor| {
                (
                    anchor.spread,
                    anchor.stagger,
                    anchor.offset,
                    anchor.rotation,
                )
            })
            .unwrap_or((0.0, 0.0, Vec2::default(), evaluated.rotation));
        let operation = runtime_for_start.operation();
        *drag_for_start.borrow_mut() = Some(PartDrag {
            pointer_id: i64::from(pointer.pointer_id()),
            capture,
            part_id: part_for_start.clone(),
            kind,
            tool,
            transaction_id: format!("layout-part-rail-{}", operation.0),
            owner: owner_for_start.clone(),
            context: context_for_start.clone(),
            anchor,
            start_spread,
            start_stagger,
            start_offset,
            start_rotation,
            start_at: evaluated.at,
            base_rotation,
            frame_rotation: base_rotation + splay,
            center: evaluated.at,
            start_angle: (start_point.y - evaluated.at.y).atan2(start_point.x - evaluated.at.x),
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
            gesture_begun: false,
            preview_submitted: false,
            pending: None,
        });
        let mut popup_at_for_start = popup_at;
        popup_at_for_start.set(Some((
            start_point.x + 18.0 * world_per_screen,
            start_point.y - 18.0 * world_per_screen,
        )));
        part_rail_set_active(svg_key, Some(tool));
    });

    let runtime_for_move = runtime.0.clone();
    let svg_for_move = svg.0.clone();
    let drag_for_move = drag.clone();
    let arbiter_for_move = arbiter.clone();
    let feedback_for_move = feedback;
    let popup_at_for_move = popup_at;
    let move_drag = Rc::new(move |event: PointerEvent| {
        let mut feedback_for_move = feedback_for_move;
        let mut popup_at_for_move = popup_at_for_move;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let pointer_id = i64::from(pointer.pointer_id());
        let Some(mut active) = drag_for_move
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer_id)
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_move,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
        {
            cancel_part_drag(
                &runtime_for_move,
                &drag_for_move,
                &arbiter_for_move,
                svg_key,
            );
            return;
        }
        let Some(rect) = svg_for_move
            .borrow()
            .as_ref()
            .map(|surface| surface.get_bounding_client_rect())
        else {
            return;
        };
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        let world_delta = Vec2 {
            x: (f64::from(pointer.client_x()) - active.start_client_x) * width / rect.width(),
            y: -(f64::from(pointer.client_y()) - active.start_client_y) * height / rect.height(),
        };
        let alt = pointer.alt_key();
        let tool_name = feedback_tool_name(tool).to_owned();
        match active.kind {
            PartRailDragKind::AnchorValue(axis) => {
                let Some(point) = crate::presentation::coordinates(
                    &svg_for_move,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                ) else {
                    return;
                };
                let Some((operation, delta_feedback, _, value_text)) = sample_anchor_value(
                    &active,
                    axis,
                    world_delta,
                    point,
                    alt,
                    snap.pitch,
                    snap.snap_fraction,
                ) else {
                    return;
                };
                if active.pending.as_ref() != Some(&operation) {
                    submit_part_preview(&runtime_for_move, &active, operation.clone());
                    active.preview_submitted = true;
                    // The popup follows the pointer with the live value and
                    // delta, like the matrix rail.
                    feedback_for_move.set(advance_feedback(
                        &feedback_for_move(),
                        FeedbackEvent::Drag {
                            name: tool_name,
                            value: value_text,
                            delta: delta_feedback,
                        },
                    ));
                    popup_at_for_move.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
                    active.pending = Some(operation);
                    *drag_for_move.borrow_mut() = Some(active);
                }
            }
            PartRailDragKind::DetachedMove => {
                // The shipped body threshold keeps clicks out of the session
                // gesture; only real motion begins it.
                if !crate::presentation::canvas_interaction::pending_part_drag_threshold_reached(
                    (active.start_client_x, active.start_client_y),
                    (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                ) {
                    return;
                }
                let at = Vec2 {
                    x: active.start_at.x + world_delta.x,
                    y: active.start_at.y + world_delta.y,
                };
                let positions = vec![Position {
                    id: active.part_id.clone(),
                    at,
                }];
                if !active.gesture_begun {
                    let operation = runtime_for_move.operation();
                    runtime_for_move.submit(Event::GestureBegin {
                        operation_id: operation,
                        pointer_id: active.pointer_id,
                        target_ids: vec![active.part_id.clone()],
                        transaction_id: active.transaction_id.clone(),
                        start: vec![Position {
                            id: active.part_id.clone(),
                            at: active.start_at,
                        }],
                        pitch: snap.pitch,
                        snap_fraction: snap.snap_fraction,
                        geometry_snap: snap.geometry_snap,
                        gap: snap.gap,
                        alt,
                    });
                    let fresh = runtime_for_move.model();
                    let admitted = fresh.gesture.as_ref().is_some_and(|gesture| {
                        gesture.pointer_id == active.pointer_id
                            && gesture.target_ids == vec![active.part_id.clone()]
                    });
                    if !admitted {
                        cancel_part_drag(
                            &runtime_for_move,
                            &drag_for_move,
                            &arbiter_for_move,
                            svg_key,
                        );
                        return;
                    }
                    active.gesture_begun = true;
                } else {
                    runtime_for_move.submit(Event::GestureSample {
                        pointer_id: active.pointer_id,
                        positions: positions.clone(),
                        alt,
                    });
                }
                feedback_for_move.set(advance_feedback(
                    &feedback_for_move(),
                    FeedbackEvent::Drag {
                        name: tool_name,
                        value: format!("X {:.2}, Y {:.2} mm", at.x, at.y),
                        delta: FeedbackDelta::Mm2(
                            at.x - active.start_at.x,
                            at.y - active.start_at.y,
                        ),
                    },
                ));
                if let Some(point) = crate::presentation::coordinates(
                    &svg_for_move,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                ) {
                    popup_at_for_move.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
                }
                *drag_for_move.borrow_mut() = Some(active);
            }
            PartRailDragKind::DetachedRotate => {
                let Some(point) = crate::presentation::coordinates(
                    &svg_for_move,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                ) else {
                    return;
                };
                let angle = (point.y - active.center.y).atan2(point.x - active.center.x)
                    - active.start_angle;
                let swept = angle.sin().atan2(angle.cos()).to_degrees();
                let value = part_rotate_value(active.start_rotation, swept, alt);
                let delta = value - active.start_rotation;
                let operation = EditOperation::SetPartRotation {
                    part_id: active.part_id.clone(),
                    rotation: value,
                };
                if active.pending.as_ref() != Some(&operation) {
                    submit_part_preview(&runtime_for_move, &active, operation.clone());
                    active.preview_submitted = true;
                    feedback_for_move.set(advance_feedback(
                        &feedback_for_move(),
                        FeedbackEvent::Drag {
                            name: tool_name,
                            value: format!("{value:.1}°"),
                            delta: FeedbackDelta::Deg(delta),
                        },
                    ));
                    popup_at_for_move.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
                    active.pending = Some(operation);
                    *drag_for_move.borrow_mut() = Some(active);
                }
            }
        }
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg.0.clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let feedback_for_end = feedback;
    let popup_at_for_end = popup_at;
    let toast_gen_for_end = toast_gen;
    let handle_value_for_end = handle_value.clone();
    let end_drag = Rc::new(move |event: PointerEvent| {
        let mut feedback_for_end = feedback_for_end;
        let mut popup_at_for_end = popup_at_for_end;
        let mut toast_gen_for_end = toast_gen_for_end;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let pointer_id = i64::from(pointer.pointer_id());
        let Some(active) = drag_for_end
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer_id)
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_end,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
        {
            cancel_part_drag(&runtime_for_end, &drag_for_end, &arbiter_for_end, svg_key);
            return;
        }
        if pointer.type_() != "pointerup" {
            cancel_part_drag(&runtime_for_end, &drag_for_end, &arbiter_for_end, svg_key);
            return;
        }
        let alt = pointer.alt_key();
        let tool_name = feedback_tool_name(tool).to_owned();
        // Landing one revision, with a toast unless the drag netted nothing.
        let mut landed = |feedback: FeedbackDelta, value_text: String| {
            if feedback.is_zero() {
                return false;
            }
            feedback_for_end.set(advance_feedback(
                &feedback_for_end(),
                FeedbackEvent::Commit {
                    value: format!("{tool_name} {value_text}"),
                    delta: feedback,
                },
            ));
            let seen = {
                toast_gen_for_end.set(toast_gen_for_end() + 1);
                toast_gen_for_end()
            };
            arm_commit_toast_expiry(feedback_for_end, toast_gen_for_end, seen);
            true
        };
        match active.kind {
            PartRailDragKind::AnchorValue(axis) => {
                let sample = crate::presentation::coordinates(
                    &svg_for_end,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                )
                .and_then(|point| {
                    let rect = svg_for_end
                        .borrow()
                        .as_ref()
                        .map(|surface| surface.get_bounding_client_rect())?;
                    if rect.width() <= 0.0 || rect.height() <= 0.0 {
                        return None;
                    }
                    let world_delta = Vec2 {
                        x: (f64::from(pointer.client_x()) - active.start_client_x) * width
                            / rect.width(),
                        y: -(f64::from(pointer.client_y()) - active.start_client_y) * height
                            / rect.height(),
                    };
                    sample_anchor_value(
                        &active,
                        axis,
                        world_delta,
                        point,
                        alt,
                        snap.pitch,
                        snap.snap_fraction,
                    )
                })
                .or_else(|| {
                    active.pending.as_ref().and_then(|operation| {
                        let EditOperation::SetPartAnchorValues { change, .. } = operation else {
                            return None;
                        };
                        let (feedback, delta, value_text) = match (axis, change) {
                            (PartAnchorAxis::Spread, PartAnchorValueChange::Spread(value)) => (
                                FeedbackDelta::Mm(value - active.start_spread),
                                PartAxisDelta::Spread(value - active.start_spread),
                                format_placement_value(PlacementValue::Spread(*value)),
                            ),
                            (PartAnchorAxis::Stagger, PartAnchorValueChange::Stagger(value)) => (
                                FeedbackDelta::Mm(value - active.start_stagger),
                                PartAxisDelta::Stagger(value - active.start_stagger),
                                format_placement_value(PlacementValue::Stagger(*value)),
                            ),
                            (PartAnchorAxis::Offset, PartAnchorValueChange::Offset(value)) => (
                                FeedbackDelta::Mm2(
                                    value.x - active.start_offset.x,
                                    value.y - active.start_offset.y,
                                ),
                                PartAxisDelta::Offset(Vec2 {
                                    x: value.x - active.start_offset.x,
                                    y: value.y - active.start_offset.y,
                                }),
                                format_placement_value(PlacementValue::Offset(*value)),
                            ),
                            (PartAnchorAxis::Rotation, PartAnchorValueChange::Rotation(value)) => (
                                FeedbackDelta::Deg(value - active.start_rotation),
                                PartAxisDelta::Rotation(value - active.start_rotation),
                                format!("{value:.1}°"),
                            ),
                            _ => return None,
                        };
                        Some((operation.clone(), feedback, delta, value_text))
                    })
                });
                if let Some((operation, feedback, delta, value_text)) = sample {
                    if active.pending.as_ref() != Some(&operation) {
                        submit_part_preview(&runtime_for_end, &active, operation);
                    }
                    if feedback.is_zero() {
                        clear_part_preview(&runtime_for_end, &active);
                        feedback_for_end.set(advance_feedback(
                            &feedback_for_end(),
                            FeedbackEvent::Choose {
                                name: tool_name,
                                value: value_text,
                            },
                        ));
                    } else {
                        commit_part_anchor_drag(&runtime_for_end, &active, delta);
                        landed(feedback, value_text);
                    }
                } else if active.preview_submitted {
                    clear_part_preview(&runtime_for_end, &active);
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Choose {
                            name: tool_name,
                            value: handle_value_for_end.clone(),
                        },
                    ));
                }
            }
            PartRailDragKind::DetachedMove => {
                if active.gesture_begun {
                    let positions = crate::presentation::coordinates(
                        &svg_for_end,
                        &pointer,
                        view_x,
                        view_y,
                        width,
                        height,
                    )
                    .and_then(|_| {
                        let rect = svg_for_end
                            .borrow()
                            .as_ref()
                            .map(|surface| surface.get_bounding_client_rect())?;
                        if rect.width() <= 0.0 || rect.height() <= 0.0 {
                            return None;
                        }
                        Some(vec![Position {
                            id: active.part_id.clone(),
                            at: Vec2 {
                                x: active.start_at.x
                                    + (f64::from(pointer.client_x()) - active.start_client_x)
                                        * width
                                        / rect.width(),
                                y: active.start_at.y
                                    - (f64::from(pointer.client_y()) - active.start_client_y)
                                        * height
                                        / rect.height(),
                            },
                        }])
                    })
                    .unwrap_or_else(|| {
                        vec![Position {
                            id: active.part_id.clone(),
                            at: active.start_at,
                        }]
                    });
                    runtime_for_end.submit(Event::GestureEnd {
                        pointer_id: active.pointer_id,
                        final_positions: positions.clone(),
                        alt,
                    });
                    let at = &positions[0].at;
                    let delta =
                        FeedbackDelta::Mm2(at.x - active.start_at.x, at.y - active.start_at.y);
                    let value_text = format!("X {:.2}, Y {:.2} mm", at.x, at.y);
                    if !landed(delta, value_text.clone()) {
                        feedback_for_end.set(advance_feedback(
                            &feedback_for_end(),
                            FeedbackEvent::Choose {
                                name: tool_name,
                                value: value_text,
                            },
                        ));
                    }
                } else {
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Dismiss,
                    ));
                }
            }
            PartRailDragKind::DetachedRotate => {
                let sample = crate::presentation::coordinates(
                    &svg_for_end,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                )
                .map(|point| {
                    let angle = (point.y - active.center.y).atan2(point.x - active.center.x)
                        - active.start_angle;
                    let swept = angle.sin().atan2(angle.cos()).to_degrees();
                    let value = part_rotate_value(active.start_rotation, swept, alt);
                    let delta = value - active.start_rotation;
                    (
                        EditOperation::SetPartRotation {
                            part_id: active.part_id.clone(),
                            rotation: value,
                        },
                        FeedbackDelta::Deg(delta),
                        delta,
                        format!("{value:.1}°"),
                    )
                })
                .or_else(|| {
                    active.pending.as_ref().and_then(|operation| {
                        let EditOperation::SetPartRotation { rotation, .. } = operation else {
                            return None;
                        };
                        let delta = rotation - active.start_rotation;
                        Some((
                            operation.clone(),
                            FeedbackDelta::Deg(delta),
                            delta,
                            format!("{rotation:.1}°"),
                        ))
                    })
                });
                if let Some((operation, feedback, delta, value_text)) = sample {
                    if active.pending.as_ref() != Some(&operation) {
                        submit_part_preview(&runtime_for_end, &active, operation);
                    }
                    if feedback.is_zero() {
                        clear_part_preview(&runtime_for_end, &active);
                        feedback_for_end.set(advance_feedback(
                            &feedback_for_end(),
                            FeedbackEvent::Choose {
                                name: tool_name,
                                value: value_text,
                            },
                        ));
                    } else {
                        commit_part_detached_rotate(&runtime_for_end, &active, delta);
                        landed(feedback, value_text);
                    }
                } else if active.preview_submitted {
                    clear_part_preview(&runtime_for_end, &active);
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Choose {
                            name: tool_name,
                            value: handle_value_for_end.clone(),
                        },
                    ));
                }
            }
        }
        popup_at_for_end.set(None);
        part_rail_set_active(svg_key, None);
        drag_for_end.borrow_mut().take();
        if active.capture.has_pointer_capture(pointer.pointer_id()) {
            let _ = active.capture.release_pointer_capture(pointer.pointer_id());
        }
        arbiter_for_end.release(CanvasInteractionOwner::MatrixTransform);
    });

    let runtime_for_cancel = runtime.0.clone();
    let drag_for_cancel = drag.clone();
    let arbiter_for_cancel = arbiter.clone();
    let feedback_for_cancel = feedback;
    let popup_at_for_cancel = popup_at;
    let cancel_drag = Rc::new(move |event: PointerEvent| {
        let mut feedback_for_cancel = feedback_for_cancel;
        let mut popup_at_for_cancel = popup_at_for_cancel;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if drag_for_cancel
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer_id == i64::from(pointer.pointer_id()))
        {
            pointer.prevent_default();
            pointer.stop_propagation();
            // A cancelled drag takes its popup with it.
            feedback_for_cancel.set(advance_feedback(
                &feedback_for_cancel(),
                FeedbackEvent::Dismiss,
            ));
            popup_at_for_cancel.set(None);
            cancel_part_drag(
                &runtime_for_cancel,
                &drag_for_cancel,
                &arbiter_for_cancel,
                svg_key,
            );
        }
    });

    let runtime_for_key = runtime.0.clone();
    let drag_for_key = drag.clone();
    let arbiter_for_key = arbiter.clone();
    let feedback_for_key = feedback;
    let popup_at_for_key = popup_at;
    let on_key_down = Rc::new(move |event: KeyboardEvent| {
        let mut feedback_for_key = feedback_for_key;
        let mut popup_at_for_key = popup_at_for_key;
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        // Escape clears the drag and its popup. Arrow keys do not nudge
        // parts: the ticket carries no part nudge, so the rail leaves them
        // to the canvas.
        if key.key() != "Escape" {
            return;
        }
        key.prevent_default();
        key.stop_propagation();
        feedback_for_key.set(advance_feedback(
            &feedback_for_key(),
            FeedbackEvent::Dismiss,
        ));
        popup_at_for_key.set(None);
        cancel_part_drag(&runtime_for_key, &drag_for_key, &arbiter_for_key, svg_key);
        on_finish.call(());
    });

    let (tok_x, tok_y) = token_center;
    let rail_w = handle_rail::rail_width(rail_tools.len());
    let (rail_cx, rail_cy) = (
        rail_layout.rail.x + rail_layout.rail.w / 2.0,
        rail_layout.rail.y + rail_layout.rail.h / 2.0,
    );
    let token_style = format!("cursor:grab;opacity:{token_opacity};touch-action:none");
    let tok_style = format!("fill:{token_fill};stroke:none");
    let hit_style = "fill:transparent;stroke:none;pointer-events:all";
    let glyph = rail_token_glyph(tool, false, glyph_degrees, glyph_stroke);
    let show_hover_name = feedback_tool_name(tool).to_owned();
    let show_hover_value = handle_value.clone();
    let show_focus_name = feedback_tool_name(tool).to_owned();
    let show_focus_value = handle_value.clone();
    rsx! {
        if token_lit
            && matches!(
                tool,
                LayoutTransformTool::Spread | LayoutTransformTool::Stagger
            )
        {
            line {
                x1: "{tok_x - guide_dx}",
                y1: "{tok_y - guide_dy}",
                x2: "{tok_x + guide_dx}",
                y2: "{tok_y + guide_dy}",
                class: "m1-rail-guide",
                style: "fill:none;stroke:var(--wb-accent);stroke-width:1px;stroke-dasharray:3 3;opacity:{guide_opacity};pointer-events:none;vector-effect:non-scaling-stroke",
            }
        }
        {
            let start = start_drag.clone();
            let move_handler = move_drag.clone();
            let end_handler = end_drag.clone();
            let cancel_handler = cancel_drag.clone();
            let lost_capture_handler = cancel_drag.clone();
            let key_handler = on_key_down.clone();
            rsx! {
                if rail_index == Some(0) {
                    g { "aria-hidden": "true",
                        transform: "translate({rail_cx} {rail_cy}) scale({px} {-px})",
                        style: "pointer-events:none",
                        rect {
                            x: "{-rail_w / 2.0}",
                            y: "-11",
                            width: "{rail_w}",
                            height: "22",
                            rx: "6",
                            style: "fill:var(--wb-panel);stroke:var(--wb-rule-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))",
                        }
                    }
                }
                g {
                    class: "m1-transform-handle",
                    transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
                    role: "button", tabindex: "0", "aria-label": "{token_aria_label}",
                    "aria-description": "{token_description}",
                    style: "{token_style}",
                    onpointerdown: move |event| start(event),
                    onpointermove: move |event| move_handler(event),
                    onpointerup: move |event| end_handler(event),
                    onpointercancel: move |event| cancel_handler(event),
                    onlostpointercapture: move |event| lost_capture_handler(event),
                    onkeydown: move |event| key_handler(event),
                    // Hovering or focusing a token docks the value popup
                    // above the rail; leaving clears it. The content comes
                    // from the pure reducer, so the chrome never writes.
                    onpointerenter: { let mut hovered = hovered; let mut feedback = feedback; let mut popup_at = popup_at; move |_| { hovered.set(true); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: show_hover_name.clone(), value: show_hover_value.clone() })); } },
                    onpointerleave: { let mut hovered = hovered; let mut feedback = feedback; move |_| { hovered.set(false); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                    onfocus: { let mut focused = focused; let mut feedback = feedback; let mut popup_at = popup_at; move |_| { focused.set(true); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: show_focus_name.clone(), value: show_focus_value.clone() })); } },
                    onblur: { let mut focused = focused; let mut feedback = feedback; move |_| { focused.set(false); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                    rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "{tok_style}" }
                    rect { x: "-10", y: "-12", width: "20", height: "24", style: "{hit_style}" }
                    {glyph}
                    if let Some(title) = token_title.as_ref() {
                        title { "{title}" }
                    }
                }
            }
        }
        // The value popup — docked 8 px above the rail while a token is
        // hovered or focused, 18 px off the pointer while dragging — and the
        // commit toast at the top centre of the canvas. Both are view-only:
        // `pointer-events: none`, no tabindex, no document write.
        if let Some(popup) = feedback_state.popup.as_ref() {
            {feedback_popup_panel(
                popup,
                popup_at(),
                Some((
                    rail_layout.rail.x,
                    rail_layout.rail.y,
                    rail_layout.rail.w,
                    rail_layout.rail.h,
                )),
                px,
            )}
        }
        if let Some(toast) = feedback_state.toast.as_ref() {
            {feedback_toast_panel(toast, view_x, view_y, width, px)}
        }
    }
}

/// Ticket 08: which selection a canvas Anchor pick retargets: an anchored
/// matrix (with its scene for the rail centre) or an independent part.
#[derive(Clone, PartialEq)]
pub enum AnchorPickOwner {
    Matrix {
        matrix: Rc<Matrix>,
        projection: Rc<MatrixScene>,
        context: TreeContext,
    },
    Part {
        part_id: String,
    },
}

impl AnchorPickOwner {
    fn context(&self) -> TreeContext {
        match self {
            Self::Matrix { context, .. } => context.clone(),
            Self::Part { part_id } => TreeContext::Component {
                part_id: Some(part_id.clone()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
        }
    }

    fn mount_tool(&self, svg_key: usize) {
        match self {
            Self::Matrix { .. } => rail_mount_tool(svg_key, LayoutTransformTool::Anchor),
            Self::Part { .. } => part_rail_mount_tool(svg_key, LayoutTransformTool::Anchor),
        }
    }

    fn unmount_tool(&self, svg_key: usize) {
        match self {
            Self::Matrix { .. } => rail_unmount_tool(svg_key, LayoutTransformTool::Anchor),
            Self::Part { .. } => part_rail_unmount_tool(svg_key, LayoutTransformTool::Anchor),
        }
    }

    fn set_active(&self, svg_key: usize, tool: Option<LayoutTransformTool>) {
        match self {
            Self::Matrix { .. } => rail_set_active(svg_key, tool),
            Self::Part { .. } => part_rail_set_active(svg_key, tool),
        }
    }
}

/// Ticket 08: the owner's Layout id behind the half-origin major target.
fn anchor_pick_layout(document: &ProjectDoc, context: &TreeContext) -> Option<String> {
    match context {
        TreeContext::Matrix { matrix_id } => document
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
            .map(|layout| layout.id.clone()),
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => document
            .layouts
            .iter()
            .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
            .map(|layout| layout.id.clone()),
        _ => None,
    }
}

/// Ticket 08: one candidate key's canvas pose from the evaluated scenes.
fn anchor_candidate_pose(scenes: &[MatrixScene], target: &PickTarget) -> Option<Vec2> {
    scenes
        .iter()
        .find(|scene| scene.matrix_id == target.matrix_id)
        .and_then(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| cell.row == target.row && cell.column == target.column)
        })
        .map(|cell| cell.pose.at)
}

/// Ticket 08: the current anchor in words for the armed popup: the live key,
/// or the half origin while detached.
fn anchor_pick_current_label(document: &ProjectDoc, projected: &AnchorPickCandidates) -> String {
    projected
        .keys
        .iter()
        .find(|key| key.current)
        .map(|key| anchor_key_label(document, &key.matrix_id, key.row, key.column))
        .unwrap_or_else(|| "half origin".to_owned())
}

/// Ticket 08: resolve one pick against the accepted document: owner and
/// target both live, so Core's refusal stays for genuinely driven keys.
fn anchor_pick_resolver(context: TreeContext, target: PickTarget) -> EditResolver {
    EditResolver::new(
        "layout-anchor-pick",
        move |accepted: &AcceptedSnapshot| match pick_commit(&accepted.document, &context, &target)
        {
            Some(commit) => {
                let owner_id = match &context {
                    TreeContext::Matrix { matrix_id } => matrix_id.clone(),
                    TreeContext::Component {
                        part_id: Some(part_id),
                        ..
                    } => part_id.clone(),
                    _ => {
                        return Resolution::Retire("The selection can no longer anchor.".into());
                    }
                };
                Resolution::submit(vec![owner_id], commit.operation)
            }
            None => Resolution::Retire("The anchor target is no longer available.".into()),
        },
    )
}

/// Ticket 08: whether the owner's live anchor names the target after the
/// settle loop.
fn anchor_pick_landed(document: &ProjectDoc, context: &TreeContext, target: &PickTarget) -> bool {
    crate::anchor_pick::anchor_is_target(document, context, target)
}

/// Ticket 08: the canvas Anchor pick mode for one anchored matrix or
/// independent part. The Anchor token toggles the armed state; while armed
/// every committable key draws a violet ring (loop-forming targets draw no
/// ring), hovering previews the Core-evaluated post-commit pose in amber,
/// and clicking commits one `SetMatrixAnchor` / `SetPartAnchor` Undo entry
/// with the carried toast. Esc, a second token press or a selection change
/// leaves pick mode with no revision, and the selection and focus stay put.
#[component]
pub fn LayoutAnchorPickOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    pick_owner: AnchorPickOwner,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Element {
    let _ = surface_size.map(|size| size());
    let svg_key: usize = Rc::as_ptr(&svg.0) as usize;
    let pick_owner_hook = pick_owner.clone();
    use_hook(move || {
        pick_owner_hook.mount_tool(svg_key);
    });
    let pick_owner_drop = pick_owner.clone();
    use_drop(move || {
        pick_owner_drop.unmount_tool(svg_key);
    });
    // Peers mount in one commit, so the first overlay paints before the rest
    // register. This second pass re-reads the full mount set once every peer
    // of the commit has registered, keeping every rail centred.
    let mut mounted = use_signal(|| false);
    let mounted_read = mounted;
    use_effect(move || {
        mounted.set(true);
    });
    let _ = mounted_read();
    let armed = use_signal(|| false);
    let hovered_target = use_signal(|| None::<PickTarget>);
    let preview = use_signal(Vec::<(String, Pose2)>::new);
    // Ticket 06, reused: the value popup and commit toast content (pure
    // reducer; rendering only) plus the toast generation the 1.8 s timer
    // checks. While armed the popup renders pinned to the canvas's top-right
    // corner; otherwise it docks above the rail like every other token.
    let feedback = use_signal(FeedbackState::default);
    let toast_gen = use_signal(|| 0_u64);

    // Leaving pick mode clears the armed peer, the hover preview and the
    // popup. The selection and focus stay put: nothing here writes them.
    let disarm = {
        let pick_owner = pick_owner.clone();
        Rc::new(move || {
            let mut armed = armed;
            let mut hovered_target = hovered_target;
            let mut preview = preview;
            let mut feedback = feedback;
            armed.set(false);
            hovered_target.set(None);
            preview.set(Vec::new());
            // peek: the stale-arm effect calls this, and subscribing it to
            // the feedback it writes would re-run it forever.
            let dismissed = advance_feedback(&feedback.peek(), FeedbackEvent::Dismiss);
            feedback.set(dismissed);
            pick_owner.set_active(svg_key, None);
        })
    };
    let escape_listener = use_hook(TransformEscapeListener::default);
    use_effect({
        let listener = escape_listener.clone();
        let disarm = disarm.clone();
        let armed_for_escape = armed;
        move || {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let disarm = disarm.clone();
            let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() != "Escape" || event.default_prevented() {
                    return;
                }
                if !armed_for_escape() {
                    return;
                }
                event.prevent_default();
                disarm();
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
            *listener.borrow_mut() = Some((document, callback));
        }
    });
    use_drop({
        let listener = escape_listener.clone();
        move || {
            if let Some((document, callback)) = listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    });
    // A stale arm (owner, selection or scope moved on) drops without writing.
    {
        let disarm = disarm.clone();
        let pick_owner = pick_owner.clone();
        let owner_for_effect = owner.clone();
        use_effect(move || {
            let _ = selected_context();
            let _ = scope_generation();
            let _ = pick_owner;
            let _ = owner_for_effect;
            disarm();
        });
    }

    let model = runtime.0.model();
    let accepted = model.accepted.clone();
    let context = pick_owner.context();
    let document = accepted.as_ref().map(|snapshot| snapshot.document.clone());
    let scenes: Vec<MatrixScene> = try_use_context::<Option<FieldPreviewSource>>()
        .flatten()
        .map(|source| source.scene.matrix_scenes.clone())
        .unwrap_or_default();
    let projected: Option<AnchorPickCandidates> = document
        .as_ref()
        .and_then(|document| anchor_pick_candidates(document, &[], &context));
    let Some(document) = document else {
        return rsx! {};
    };
    let Some(projected) = projected else {
        return rsx! {};
    };
    let handle_scale = svg.0.borrow().as_ref().map_or(3.0, |surface| {
        let rect = surface.get_bounding_client_rect();
        if rect.width() > 0.0 {
            (width / rect.width()).max(height / rect.height().max(1.0)) * 3.0
        } else {
            3.0
        }
    });
    // World units per screen px: rings and labels stay screen-size at every
    // zoom, matching the badge and rail chrome.
    let world_per_screen = handle_scale / 3.0;
    let px = world_per_screen;
    // The rail peers share one mount set per canvas; the Anchor token sits
    // last in canonical order. The matrix rail drops Spread on column 0,
    // like the drag overlay, so the token stays centred on what draws.
    let rail_tools: Vec<LayoutTransformTool> = match &pick_owner {
        AnchorPickOwner::Matrix { .. } => {
            let selected_column = context_column(&context).unwrap_or(0);
            let selected_row = context_row(&context);
            rail_peers(svg_key)
                .into_iter()
                .filter(|peer| {
                    !matches!(peer, LayoutTransformTool::Spread)
                        || selected_column != 0
                        || selected_row.is_some()
                })
                .collect()
        }
        AnchorPickOwner::Part { .. } => part_rail_peers(svg_key),
    };
    let rail_index = rail_tools
        .iter()
        .position(|peer| *peer == LayoutTransformTool::Anchor);
    let rail_points: Vec<Vec2> = match &pick_owner {
        AnchorPickOwner::Matrix { projection, .. } => {
            let mut points: Vec<Vec2> = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled)
                .map(|cell| cell.pose.at)
                .collect();
            if points.is_empty() {
                points = projection
                    .cells
                    .first()
                    .map(|cell| vec![cell.pose.at])
                    .unwrap_or_default();
            }
            points
        }
        AnchorPickOwner::Part { part_id } => {
            let Some(part) = document.parts.iter().find(|part| part.id == *part_id) else {
                return rsx! {};
            };
            let evaluated = evaluated_part_pose(&document, part);
            let courtyard_top = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .map(|definition| {
                    definition
                        .courtyard
                        .iter()
                        .map(|point| point.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .filter(|top| top.is_finite())
                .unwrap_or(0.0);
            vec![Vec2 {
                x: evaluated.at.x,
                y: evaluated.at.y + courtyard_top,
            }]
        }
    };
    if rail_index.is_none() || rail_points.is_empty() {
        return rsx! {};
    }
    let min_x = rail_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = rail_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let top = rail_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let rail_layout = if rail_index.is_some() && !rail_points.is_empty() {
        handle_rail::rail_world(
            (min_x + max_x) / 2.0,
            top,
            rail_tools.len(),
            world_per_screen,
        )
    } else {
        None
    };
    let Some(rail_layout) = rail_layout else {
        return rsx! {};
    };
    let (tok_x, tok_y) = rail_layout
        .token_centers
        .get(rail_index.expect("the Anchor token draws"))
        .copied()
        .unwrap_or((rail_layout.rail.x, rail_layout.rail.y));
    let rail_w = handle_rail::rail_width(rail_tools.len());
    let (rail_cx, rail_cy) = (
        rail_layout.rail.x + rail_layout.rail.w / 2.0,
        rail_layout.rail.y + rail_layout.rail.h / 2.0,
    );
    let token_active = armed();
    let token_fill = if token_active {
        "var(--wb-accent)"
    } else {
        "transparent"
    };
    let glyph_stroke = if token_active {
        "var(--wb-on-accent)"
    } else {
        "var(--wb-muted)"
    };
    let current_label = anchor_pick_current_label(&document, &projected);
    let item_label: String = match &pick_owner {
        AnchorPickOwner::Matrix { matrix, .. } => matrix_display_name(&document, &matrix.id),
        AnchorPickOwner::Part { part_id } => document
            .parts
            .iter()
            .find(|part| part.id == *part_id)
            .map(|part| {
                if part.reference.trim().is_empty() {
                    part.id.clone()
                } else {
                    part.reference.clone()
                }
            })
            .unwrap_or_else(|| part_id.clone()),
    };
    // The pinned popup rows: the item against the current or hovered target.
    // Width and pin recompute with the value, so the panel hugs the corner.
    let popup_value = match hovered_target() {
        Some(target) => format!(
            "{item_label} → {}",
            anchor_key_label(&document, &target.matrix_id, target.row, target.column)
        ),
        None => format!("{item_label} · {current_label}"),
    };
    let popup_rows = vec![
        ("Anchor".to_owned(), 11.0, "var(--wb-muted)", false),
        (popup_value, 15.0, "inherit", true),
        (
            "Click to anchor · Esc: cancel".to_owned(),
            10.5,
            "var(--wb-muted)",
            false,
        ),
    ];
    // The pinned panel measures its own rows; see anchor_pick_popup_panel.
    // Arming toggles on the token; the peers dim while one is armed, like a
    // drag. Nothing writes the selection or focus.
    let toggle_arm = {
        let pick_owner = pick_owner.clone();
        let disarm = disarm.clone();
        let current_label = current_label.clone();
        Rc::new(move |_| {
            let mut armed = armed;
            let mut feedback = feedback;
            if armed() {
                disarm();
                return;
            }
            armed.set(true);
            pick_owner.set_active(svg_key, Some(LayoutTransformTool::Anchor));
            feedback.set(advance_feedback(
                &feedback(),
                FeedbackEvent::Choose {
                    name: "Anchor".to_owned(),
                    value: current_label.clone(),
                },
            ));
        })
    };
    // Hovering a ring previews the Core-evaluated post-commit pose in amber;
    // leaving restores the current-target popup.
    let hover_ring = {
        let document = document.clone();
        let context = context.clone();
        let current_label = current_label.clone();
        let item_label = item_label.clone();
        Rc::new(move |target: Option<PickTarget>| {
            let mut hovered_target = hovered_target;
            let mut preview = preview;
            let mut feedback = feedback;
            hovered_target.set(target.clone());
            match target {
                Some(target) => {
                    preview.set(preview_anchor_poses(&document, &context, &target));
                    feedback.set(advance_feedback(
                        &feedback(),
                        FeedbackEvent::Choose {
                            name: "Anchor".to_owned(),
                            value: format!(
                                "{item_label} → {}",
                                anchor_key_label(
                                    &document,
                                    &target.matrix_id,
                                    target.row,
                                    target.column
                                )
                            ),
                        },
                    ));
                }
                None => {
                    preview.set(Vec::new());
                    feedback.set(advance_feedback(
                        &feedback(),
                        FeedbackEvent::Choose {
                            name: "Anchor".to_owned(),
                            value: current_label.clone(),
                        },
                    ));
                }
            }
        })
    };
    // Clicking a ring commits one Undo entry through the shared resolver and
    // raises the carried toast only once the live anchor names the target.
    // The selection and focus stay put; a stale pick reports and disarms.
    let commit_pick = {
        let runtime_for_commit = runtime.0.clone();
        let feedback_for_commit = feedback;
        let toast_gen_for_commit = toast_gen;
        let disarm = disarm.clone();
        let context = context.clone();
        let owner_for_commit = owner.clone();
        let arbiter_for_commit = arbiter.clone();
        Rc::new(move |target: PickTarget| {
            if arbiter_for_commit.current().is_some()
                || runtime_for_commit.model().gesture.is_some()
            {
                return;
            }
            if !transform_owner_is_current(
                &runtime_for_commit,
                workspace,
                scope_generation,
                selected_context,
                &owner_for_commit,
                &context,
            ) {
                runtime_for_commit.report("The selection changed. Pick the anchor again.");
                disarm();
                return;
            }
            let Some(accepted) = runtime_for_commit.model().accepted.clone() else {
                return;
            };
            let Some(commit) = pick_commit(&accepted.document, &context, &target) else {
                return;
            };
            let (item_label, old_anchor_label, new_anchor_label) = (
                commit.item_label.clone(),
                commit.old_anchor_label.clone(),
                commit.new_anchor_label.clone(),
            );
            let mut edits = PendingEdits::default();
            edits.begin(
                &runtime_for_commit,
                (),
                "layout-anchor-pick",
                Some("transform".into()),
                anchor_pick_resolver(context.clone(), target.clone()),
            );
            // Leave pick mode at the click: the rings go, and a second click
            // cannot queue a second commit while this one settles.
            disarm();
            let runtime_for_settle = runtime_for_commit.clone();
            let context_for_settle = context.clone();
            let target_for_settle = target.clone();
            let mut feedback_for_commit = feedback_for_commit;
            let mut toast_gen_for_commit = toast_gen_for_commit;
            spawn_local(async move {
                loop {
                    if !edits.settle(true).is_empty() {
                        break;
                    }
                    gloo_timers::future::TimeoutFuture::new(16).await;
                }
                let landed = runtime_for_settle
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|snapshot| {
                        anchor_pick_landed(
                            &snapshot.document,
                            &context_for_settle,
                            &target_for_settle,
                        )
                    });
                if !landed {
                    runtime_for_settle.report("The anchor target is no longer available.");
                    return;
                }
                // The overlay remounts on scope and selection changes; its
                // signals die together, so one liveness probe covers the writes.
                if toast_gen_for_commit.try_peek().is_err() {
                    return;
                }
                feedback_for_commit.set(advance_feedback(
                    &feedback_for_commit(),
                    FeedbackEvent::Commit {
                        value: format!("{item_label} anchored to {new_anchor_label}"),
                        delta: FeedbackDelta::Text(format!("was {old_anchor_label}")),
                    },
                ));
                let seen = {
                    toast_gen_for_commit.set(toast_gen_for_commit() + 1);
                    toast_gen_for_commit()
                };
                arm_commit_toast_expiry(feedback_for_commit, toast_gen_for_commit, seen);
            });
        })
    };
    // The live poses behind the amber comparison: the owner's own scene
    // cells for a matrix, the evaluated part for a part.
    let current_poses: std::collections::HashMap<String, Pose2> = match &pick_owner {
        AnchorPickOwner::Matrix { projection, .. } => projection
            .cells
            .iter()
            .filter_map(|cell| cell.member_id.clone().map(|id| (id, cell.pose)))
            .collect(),
        AnchorPickOwner::Part { part_id } => document
            .parts
            .iter()
            .find(|part| part.id == *part_id)
            .map(|part| (part.id.clone(), evaluated_part_pose(&document, part)))
            .into_iter()
            .collect(),
    };
    let active_peer = match &pick_owner {
        AnchorPickOwner::Matrix { .. } => rail_active_tool(svg_key),
        AnchorPickOwner::Part { .. } => part_rail_active_tool(svg_key),
    };
    let token_opacity = if active_peer.is_some_and(|peer| peer != LayoutTransformTool::Anchor) {
        "0.35"
    } else {
        "1"
    };
    let token_style = format!("cursor:pointer;opacity:{token_opacity};touch-action:none");
    let tok_fill = format!("fill:{token_fill};stroke:none");
    let hit_style = "fill:transparent;stroke:none;pointer-events:all";
    let glyph = rail_token_glyph(LayoutTransformTool::Anchor, false, 0.0, glyph_stroke);
    let feedback_state = feedback();
    let layout_id = anchor_pick_layout(&document, &context);
    rsx! {
        if rail_index == Some(0) {
            g { "aria-hidden": "true",
                transform: "translate({rail_cx} {rail_cy}) scale({px} {-px})",
                style: "pointer-events:none",
                rect {
                    x: "{-rail_w / 2.0}",
                    y: "-11",
                    width: "{rail_w}",
                    height: "22",
                    rx: "6",
                    style: "fill:var(--wb-panel);stroke:var(--wb-rule-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))",
                }
            }
        }
        g {
            class: "m1-transform-handle",
            transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
            role: "button", tabindex: "0", "aria-label": "Pick anchor",
            "data-pick": "anchor",
            style: "{token_style}",
            onpointerdown: { let toggle = toggle_arm.clone(); move |event| {
                let Some(pointer) = event.data().try_as_web_event() else { return; };
                pointer.prevent_default();
                pointer.stop_propagation();
                toggle(());
            } },
            onkeydown: { let toggle = toggle_arm.clone(); move |event: KeyboardEvent| {
                let Some(key) = event.data().try_as_web_event() else { return; };
                if key.key() != "Enter" && key.key() != " " { return; }
                key.prevent_default();
                toggle(());
            } },
            onpointerenter: { let mut feedback = feedback; let value = current_label.clone(); move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: "Anchor".to_owned(), value: value.clone() }));
            } },
            onpointerleave: { let mut feedback = feedback; move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
            } },
            onfocus: { let mut feedback = feedback; let value = current_label.clone(); move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: "Anchor".to_owned(), value: value.clone() }));
            } },
            onblur: { let mut feedback = feedback; move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
            } },
            rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "{tok_fill}" }
            rect { x: "-10", y: "-12", width: "20", height: "24", style: "{hit_style}" }
            {glyph}
            title { "Pick anchor" }
        }
        if armed() {
            for key in projected.keys.iter().filter_map(|key| {
                let target = PickTarget { matrix_id: key.matrix_id.clone(), row: key.row, column: key.column };
                anchor_candidate_pose(&scenes, &target).map(|pose| (key, target, pose))
            }) {
                {
                    let (key, target, pose) = (key.0, key.1, key.2);
                    let hovered = hovered_target().is_some_and(|hover| hover == target);
                    let ring_r = if hovered { CANDIDATE_RING_HOVER_PX } else { CANDIDATE_RING_PX } / 2.0;
                    let label = anchor_key_label(&document, &target.matrix_id, target.row, target.column);
                    let hover_ring = hover_ring.clone();
                    let commit_pick = commit_pick.clone();
                    let enter_target = target.clone();
                    let commit_target_down = target.clone();
                    let commit_target_key = target.clone();
                    rsx! {
                        g {
                            class: "m1-anchor-pick-candidate",
                            transform: "translate({pose.x} {pose.y}) scale({px} {-px})",
                            "data-anchor": "{target.matrix_id}:{target.row}:{target.column}",
                            role: "button", tabindex: "0",
                            "aria-label": "Anchor to {label}",
                            onpointerenter: { let hover = hover_ring.clone(); move |_| hover(Some(enter_target.clone())) },
                            onpointerleave: { let hover = hover_ring.clone(); move |_| hover(None) },
                            onpointerdown: { let commit = commit_pick.clone(); move |event| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                pointer.prevent_default();
                                pointer.stop_propagation();
                                commit(commit_target_down.clone());
                            } },
                            onkeydown: { let commit = commit_pick.clone(); move |event: KeyboardEvent| {
                                let Some(key) = event.data().try_as_web_event() else { return; };
                                if key.key() != "Enter" { return; }
                                key.prevent_default();
                                commit(commit_target_key.clone());
                            } },
                            circle { r: "{CANDIDATE_HIT_PX / 2.0}", style: "fill:transparent;stroke:none;pointer-events:all" }
                            circle { r: "{ring_r}", style: "fill:var(--wb-panel);stroke:var(--wb-accent-secondary);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
                            if key.current {
                                circle { r: "1.6", style: "fill:#fff;stroke:none;pointer-events:none" }
                            }
                        }
                    }
                }
            }
            // Prototype majors are display-only under the current Core
            // contract (keys only): dimmed rings with labels, no pointer
            // events, so they never commit. A detached owner's half origin
            // keeps the white current dot.
            for major in projected.majors.iter().filter_map(|major| {
                let (pose, label) = match &major.kind {
                    AnchorMajorKind::Part(id) => {
                        let part = document.parts.iter().find(|part| part.id == *id)?;
                        let label = if part.reference.trim().is_empty() { part.id.clone() } else { part.reference.clone() };
                        (evaluated_part_pose(&document, part).at, label)
                    }
                    AnchorMajorKind::HalfOrigin => {
                        let layout = layout_id.clone()?;
                        // An unsplit layout has no half offset: its half frame
                        // sits at the board origin.
                        let origin = layout_half_offset(&document, &layout)
                            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                        (origin, "Half origin".to_owned())
                    }
                };
                Some((major, pose, label))
            }) {
                {
                    let (major, pose, label) = (major.0, major.1, major.2);
                    let ring_r = MAJOR_RING_PX / 2.0;
                    rsx! {
                        g {
                            class: "m1-anchor-pick-major",
                            transform: "translate({pose.x} {pose.y}) scale({px} {-px})",
                            opacity: "0.55",
                            style: "pointer-events:none",
                            "aria-hidden": "true",
                            circle { r: "{ring_r}", style: "fill:var(--wb-panel);stroke:var(--wb-accent-secondary);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
                            path { d: "M{-ring_r - 4.0} 0h{ring_r * 2.0 + 8.0}M0 {-ring_r - 4.0}v{ring_r * 2.0 + 8.0}", style: "fill:none;stroke:var(--wb-accent-secondary);stroke-width:1px;vector-effect:non-scaling-stroke" }
                            text { x: "{ring_r + 6.0}", y: "{0.0 - ring_r - 2.0}", style: "font-size:11px;fill:var(--wb-muted)", "{label}" }
                            if major.current {
                                circle { r: "2.2", style: "fill:#fff;stroke:none;pointer-events:none" }
                            }
                        }
                    }
                }
            }
            // Amber marks the Core-evaluated post-commit poses that move.
            for moved in preview().into_iter().filter_map(|(id, pose)| {
                let current = current_poses.get(&id).copied()?;
                (current != pose).then_some((id, current, pose))
            }) {
                {
                    let (_, current, pose) = moved;
                    let amber_r = CANDIDATE_RING_PX / 2.0 * px;
                    rsx! {
                        circle {
                            class: "m1-anchor-pick-preview",
                            cx: "{pose.at.x}", cy: "{pose.at.y}", r: "{amber_r}",
                            style: "fill:none;stroke:var(--wb-status-warning);stroke-width:{1.5 * px};pointer-events:none",
                            "aria-hidden": "true",
                        }
                        line {
                            class: "m1-anchor-pick-preview",
                            x1: "{current.at.x}", y1: "{current.at.y}",
                            x2: "{pose.at.x}", y2: "{pose.at.y}",
                            style: "stroke:var(--wb-status-warning);stroke-width:{1.0 * px};stroke-dasharray:{3.0 * px} {2.0 * px};pointer-events:none",
                            "aria-hidden": "true",
                        }
                    }
                }
            }
            {anchor_pick_popup_panel(popup_rows, view_x, view_y, width, px)}
        } else {
            if let Some(popup) = feedback_state.popup.as_ref() {
                {feedback_popup_panel(
                    popup,
                    None,
                    Some((
                        rail_layout.rail.x,
                        rail_layout.rail.y,
                        rail_layout.rail.w,
                        rail_layout.rail.h,
                    )),
                    px,
                )}
            }
        }
        if let Some(toast) = feedback_state.toast.as_ref() {
            {feedback_toast_panel(toast, view_x, view_y, width, px)}
        }
    }
}

/// Ticket 08: the armed Anchor popup, pinned to the canvas's top-right
/// corner so it never covers a candidate. Same panel treatment as the
/// value popup, with the pick hint instead of the drag hint.
fn anchor_pick_popup_panel(
    rows: Vec<(String, f64, &'static str, bool)>,
    view_x: f64,
    view_y: f64,
    view_width: f64,
    px: f64,
) -> Element {
    let panel_w = rows
        .iter()
        .map(|(text, size, _, _)| feedback_line_width(text, *size))
        .fold(0.0_f64, f64::max)
        + 16.0;
    let panel_x = view_x + view_width - panel_w * px - 12.0 * px;
    let panel_y = -view_y - 12.0 * px;
    rsx! {
        g {
            class: "m1-feedback-popup",
            transform: "translate({panel_x} {panel_y}) scale({px} {-px})",
            style: "pointer-events:none",
            "aria-hidden": "true",
            {feedback_panel_contents(panel_w, rows)}
        }
    }
}

/// What a handle key press moves, captured as intent. The step is a delta: the resolver
/// applies it to the matrix accepted when the edit runs.
#[derive(Clone, Debug, PartialEq)]
enum TransformNudge {
    Stagger {
        column: u32,
        key: ArrowKey,
        big: bool,
    },
    Spread {
        column: u32,
        key: ArrowKey,
        big: bool,
    },
    /// One arrow step of a row or column offset, in screen directions.
    Offset {
        parameter: PlacementParameter,
        key: ArrowKey,
        big: bool,
    },
    /// One arrow step of a row or key bend, in degrees. Only the horizontal
    /// pair is consumed, mirroring the Splay angle nudge.
    Bend {
        parameter: PlacementParameter,
        degrees: f64,
    },
    SplayOrigin {
        column: u32,
    },
    SplayAngle {
        column: u32,
        degrees: f64,
        affect: MatrixSplayAffect,
    },
    /// One arrow step of the selected matrix's Origin; the step arrives as
    /// the nudge delta in world directions.
    MatrixMove,
    /// One arrow step of the selected matrix's stored rotation.
    MatrixRotate {
        degrees: f64,
    },
}

fn accepted_matrix<'a>(accepted: &'a AcceptedSnapshot, matrix_id: &str) -> Option<&'a Matrix> {
    accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
}

/// The anchor context a matrix drag inverse-solves through, or `None` for an
/// unanchored matrix. Refuses when the matrix is anchored but its anchor
/// target is not laid out, so the drag never solves through a stale frame.
fn matrix_drag_anchor(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
) -> Result<Option<MatrixDragAnchor>, String> {
    let Some(anchor) = matrix.anchor.as_ref() else {
        return Ok(None);
    };
    let Some(key_pose) = matrix_drag_anchor_pose(&accepted.scene.matrix_scenes, anchor) else {
        return Err(format!(
            "Matrix {} is anchored to {}.r{}c{}, which is not laid out.",
            matrix.id, anchor.matrix_id, anchor.row, anchor.column
        ));
    };
    Ok(Some(MatrixDragAnchor {
        key_pose,
        follow: anchor.follow,
        transport: matrix_drag_transport(&accepted.document, &matrix.id),
    }))
}

fn accepted_basis<'a>(
    accepted: &'a AcceptedSnapshot,
    matrix_id: &str,
    column: u32,
) -> Option<&'a boardstudio_core::model::MatrixColumnBasis> {
    accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?
        .columns
        .iter()
        .find(|basis| basis.column == column)
}

fn stagger_parameter(column: u32, quick_offset: bool) -> PlacementParameter {
    if quick_offset {
        PlacementParameter::ColumnOffset { column }
    } else {
        PlacementParameter::Stagger { column }
    }
}

/// The parameter's value in `matrix`; an index past the saved vector reads as zero,
/// and an absent Spread or Padding reads the pitch so a drag that starts from a
/// default writes an explicit step.
fn placement_current(matrix: &Matrix, parameter: PlacementParameter) -> PlacementValue {
    match parameter {
        PlacementParameter::Stagger { column } => PlacementValue::Stagger(
            matrix
                .column_layout
                .get(column as usize)
                .map(|record| &record.stagger)
                .copied()
                .unwrap_or(0.0),
        ),
        PlacementParameter::ColumnOffset { column } => PlacementValue::Offset(
            matrix
                .column_layout
                .get(column as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::RowOffset { row } => PlacementValue::Offset(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::RowShift { row } => PlacementValue::Shift(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.shift)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::KeyShift { row, column } => PlacementValue::Shift(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .and_then(|cell| cell.shift)
                .unwrap_or_default(),
        ),
        PlacementParameter::RowBend { row } => PlacementValue::Bend(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
        ),
        PlacementParameter::KeyBend { row, column } => PlacementValue::Bend(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .and_then(|cell| cell.bend)
                .unwrap_or(0.0),
        ),
        PlacementParameter::ColumnSpread { column } => PlacementValue::Spread(
            matrix
                .column_layout
                .get(column as usize)
                .and_then(|record| record.spread)
                .unwrap_or(matrix.pitch.x),
        ),
        PlacementParameter::RowPadding { row } => PlacementValue::Padding(
            matrix
                .row_layout
                .get(row as usize)
                .and_then(|record| record.padding)
                .unwrap_or(matrix.pitch.y),
        ),
    }
}

/// The parameter and absolute value a placement operation sets. Absent Spread
/// and Padding resolve against the matrix pitch, matching `placement_current`,
/// so a drag that lands on a default still compares and deltas exactly.
fn placement_target(
    operation: &EditOperation,
    matrix: &Matrix,
) -> Option<(PlacementParameter, PlacementValue)> {
    let EditOperation::SetMatrixPlacement {
        matrix_id: target,
        change,
    } = operation
    else {
        return None;
    };
    if target != &matrix.id {
        return None;
    }
    match change {
        MatrixPlacementChange::ColumnStagger { column, value } => Some((
            PlacementParameter::Stagger { column: *column },
            PlacementValue::Stagger(*value),
        )),
        MatrixPlacementChange::ColumnOffset { column, value } => Some((
            PlacementParameter::ColumnOffset { column: *column },
            PlacementValue::Offset(*value),
        )),
        MatrixPlacementChange::RowOffset { row, value } => Some((
            PlacementParameter::RowOffset { row: *row },
            PlacementValue::Offset(*value),
        )),
        MatrixPlacementChange::RowShift { row, value } => Some((
            PlacementParameter::RowShift { row: *row },
            PlacementValue::Shift(*value),
        )),
        MatrixPlacementChange::KeyShift { row, column, value } => Some((
            PlacementParameter::KeyShift {
                row: *row,
                column: *column,
            },
            PlacementValue::Shift(*value),
        )),
        MatrixPlacementChange::RowBend { row, value } => Some((
            PlacementParameter::RowBend { row: *row },
            PlacementValue::Bend(*value),
        )),
        MatrixPlacementChange::KeyBend { row, column, value } => Some((
            PlacementParameter::KeyBend {
                row: *row,
                column: *column,
            },
            PlacementValue::Bend(*value),
        )),
        MatrixPlacementChange::ColumnSpread { column, value } => Some((
            PlacementParameter::ColumnSpread { column: *column },
            PlacementValue::Spread(value.unwrap_or(matrix.pitch.x)),
        )),
        MatrixPlacementChange::RowPadding { row, value } => Some((
            PlacementParameter::RowPadding { row: *row },
            PlacementValue::Padding(value.unwrap_or(matrix.pitch.y)),
        )),
        _ => None,
    }
}

/// The net change of a placement parameter over a drag, from start to snapped value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PlacementDelta {
    Stagger(f64),
    Offset(Vec2),
    Shift(Vec2),
    Bend(f64),
    Spread(f64),
    Padding(f64),
}

fn placement_delta(start: PlacementValue, end: PlacementValue) -> Option<PlacementDelta> {
    match (start, end) {
        (PlacementValue::Stagger(start), PlacementValue::Stagger(end)) => {
            Some(PlacementDelta::Stagger(end - start))
        }
        (PlacementValue::Offset(start), PlacementValue::Offset(end)) => {
            Some(PlacementDelta::Offset(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }))
        }
        (PlacementValue::Shift(start), PlacementValue::Shift(end)) => {
            Some(PlacementDelta::Shift(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }))
        }
        (PlacementValue::Bend(start), PlacementValue::Bend(end)) => {
            Some(PlacementDelta::Bend(end - start))
        }
        (PlacementValue::Spread(start), PlacementValue::Spread(end)) => {
            Some(PlacementDelta::Spread(end - start))
        }
        (PlacementValue::Padding(start), PlacementValue::Padding(end)) => {
            Some(PlacementDelta::Padding(end - start))
        }
        _ => None,
    }
}

fn apply_placement_delta(current: PlacementValue, delta: PlacementDelta) -> Option<PlacementValue> {
    match (current, delta) {
        (PlacementValue::Stagger(value), PlacementDelta::Stagger(delta)) => {
            Some(PlacementValue::Stagger(value + delta))
        }
        (PlacementValue::Offset(value), PlacementDelta::Offset(delta)) => {
            Some(PlacementValue::Offset(Vec2 {
                x: value.x + delta.x,
                y: value.y + delta.y,
            }))
        }
        (PlacementValue::Shift(value), PlacementDelta::Shift(delta)) => {
            Some(PlacementValue::Shift(Vec2 {
                x: value.x + delta.x,
                y: value.y + delta.y,
            }))
        }
        (PlacementValue::Bend(value), PlacementDelta::Bend(delta)) => {
            Some(PlacementValue::Bend(value + delta))
        }
        (PlacementValue::Spread(value), PlacementDelta::Spread(delta)) => {
            Some(PlacementValue::Spread(value + delta))
        }
        (PlacementValue::Padding(value), PlacementDelta::Padding(delta)) => {
            Some(PlacementValue::Padding(value + delta))
        }
        _ => None,
    }
}

fn transform_commit(
    transaction_id: String,
    matrix_id: &str,
    operation: EditOperation,
) -> Resolution {
    Resolution::submit_with_transaction_id(vec![matrix_id.to_owned()], transaction_id, operation)
}

/// The accepted frame of a placement parameter, or why the edit has to retire: the column
/// (or, for a row offset, the row) has gone from the accepted matrix.
fn accepted_placement_basis(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
    parameter: PlacementParameter,
) -> Result<SceneBasis, &'static str> {
    if let PlacementParameter::RowOffset { row }
    | PlacementParameter::RowShift { row }
    | PlacementParameter::RowBend { row }
    | PlacementParameter::RowPadding { row } = parameter
        && row >= matrix.rows
    {
        return Err("The selected row no longer exists.");
    }
    if let PlacementParameter::Stagger { column }
    | PlacementParameter::ColumnOffset { column }
    | PlacementParameter::ColumnSpread { column }
    | PlacementParameter::KeyShift { column, .. }
    | PlacementParameter::KeyBend { column, .. } = parameter
        && column >= matrix.columns
    {
        return Err("The selected column no longer exists.");
    }
    if let PlacementParameter::KeyShift { row, .. } | PlacementParameter::KeyBend { row, .. } =
        parameter
        && row >= matrix.rows
    {
        return Err("The selected key no longer exists.");
    }
    accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix.id)
        .and_then(|scene| SceneBasis::for_parameter(scene, parameter))
        .ok_or("The selected column no longer exists.")
}

/// Why an arrow step makes no edit.
enum NudgeStop {
    /// The column or row has gone from the accepted matrix.
    Gone(&'static str),
    /// The key is across the stagger axis and is not consumed.
    NotConsumed,
}

impl NudgeStop {
    fn resolution(self) -> Resolution {
        match self {
            Self::Gone(reason) => Resolution::Retire(reason.into()),
            Self::NotConsumed => Resolution::Unchanged,
        }
    }
}

/// One arrow step against the accepted matrix.
fn nudge_placement(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
    parameter: PlacementParameter,
    key: ArrowKey,
    big: bool,
) -> Result<EditOperation, NudgeStop> {
    let basis = accepted_placement_basis(accepted, matrix, parameter).map_err(NudgeStop::Gone)?;
    let value = nudge_value(
        &basis,
        parameter,
        placement_current(matrix, parameter),
        key,
        big,
    )
    .ok_or(NudgeStop::NotConsumed)?;
    Ok(placement_operation(&matrix.id, parameter, value))
}

/// Resolve a handle key press: the step is added to the accepted stagger along the accepted
/// stagger axis, or to the accepted column basis (splay origin and angle). `delta` serves splay.
fn transform_nudge_resolver(matrix_id: String, nudge: TransformNudge, delta: Vec2) -> EditResolver {
    EditResolver::new(
        "layout-transform-nudge",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = accepted_matrix(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let operation = match &nudge {
                TransformNudge::Stagger { column, key, big } => {
                    let parameter = PlacementParameter::Stagger { column: *column };
                    // The step follows the accepted stagger axis, so a queued press lands on
                    // whatever the earlier presses produced.
                    match nudge_placement(accepted, matrix, parameter, *key, *big) {
                        Ok(operation) => operation,
                        Err(stop) => return stop.resolution(),
                    }
                }
                TransformNudge::Spread { column, key, big } => {
                    let parameter = PlacementParameter::ColumnSpread { column: *column };
                    match nudge_placement(accepted, matrix, parameter, *key, *big) {
                        Ok(operation) => operation,
                        Err(stop) => return stop.resolution(),
                    }
                }
                TransformNudge::Offset {
                    parameter,
                    key,
                    big,
                } => match nudge_placement(accepted, matrix, *parameter, *key, *big) {
                    Ok(operation) => operation,
                    Err(stop) => return stop.resolution(),
                },
                TransformNudge::SplayOrigin { column } => {
                    let Some(basis) = accepted_basis(accepted, &matrix_id, *column) else {
                        return Resolution::Retire("The selected column no longer exists.".into());
                    };
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::Origin {
                            world: Some(Vec2 {
                                x: basis.splay_origin.x + delta.x,
                                y: basis.splay_origin.y + delta.y,
                            }),
                        },
                    }
                }
                TransformNudge::Bend { parameter, degrees } => {
                    if let Err(reason) = accepted_placement_basis(accepted, matrix, *parameter) {
                        return Resolution::Retire(reason.into());
                    }
                    let PlacementValue::Bend(current) = placement_current(matrix, *parameter)
                    else {
                        return Resolution::Retire(
                            "This transform no longer matches the selected matrix.".into(),
                        );
                    };
                    placement_operation(
                        &matrix_id,
                        *parameter,
                        PlacementValue::Bend(current + degrees),
                    )
                }
                TransformNudge::SplayAngle {
                    column,
                    degrees,
                    affect,
                } => {
                    let Some(basis) = accepted_basis(accepted, &matrix_id, *column) else {
                        return Resolution::Retire("The selected column no longer exists.".into());
                    };
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::Angle {
                            angle: column_angle(basis.splay_angle, basis.rotation, affect)
                                + degrees,
                            affect: affect.clone(),
                        },
                    }
                }
                TransformNudge::MatrixMove => {
                    let anchor = match matrix_drag_anchor(accepted, matrix) {
                        Ok(anchor) => anchor,
                        Err(reason) => return Resolution::Retire(reason),
                    };
                    let mut next = matrix.clone();
                    next.origin = solve_matrix_origin(matrix, anchor, delta);
                    if next.origin == matrix.origin {
                        return Resolution::Retire(
                            "The nudge stays outside the followed directions.".into(),
                        );
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
                TransformNudge::MatrixRotate { degrees } => {
                    let follow_rotation = matrix
                        .anchor
                        .as_ref()
                        .map(|anchor| anchor.follow.rotation)
                        .unwrap_or(true);
                    let Some(rotation) = solve_matrix_rotation(
                        matrix.rotation.unwrap_or(0.0),
                        follow_rotation,
                        *degrees,
                    ) else {
                        return Resolution::Retire(
                            "Rotation is not followed, so turning it changes nothing.".into(),
                        );
                    };
                    let mut next = matrix.clone();
                    next.rotation = Some(rotation);
                    if next.rotation == matrix.rotation {
                        return Resolution::Retire("The nudge turns nothing.".into());
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
            };
            transform_commit(String::new(), &matrix_id, operation)
        },
    )
}

/// The net transform a drag produced, captured when it ends: a stagger or offset and the
/// splay angle as deltas from the drag's start, a splay origin as the point it was dropped on.
/// A matrix move is the constrained world delta from the drag's start, recovered through the
/// inverse of the drag's anchor frame; a matrix rotation is the swept degrees.
#[derive(Clone, Debug, PartialEq)]
enum TransformDragResult {
    Placement {
        parameter: PlacementParameter,
        delta: PlacementDelta,
    },
    SplayAngle {
        column: u32,
        delta_degrees: f64,
        affect: MatrixSplayAffect,
    },
    SplayOrigin {
        column: u32,
        world: Option<Vec2>,
    },
    SplaySwing {
        column: u32,
        world: Vec2,
    },
    MatrixMove {
        delta: Vec2,
    },
    MatrixRotate {
        delta_degrees: f64,
    },
}

fn transform_drag_result(
    drag: &TransformDrag,
    operation: &EditOperation,
) -> Option<TransformDragResult> {
    if let Some(gesture_parameter) = drag.gesture.parameter() {
        let (parameter, end) = placement_target(operation, &drag.matrix)?;
        if parameter != gesture_parameter {
            return None;
        }
        let delta = placement_delta(placement_current(&drag.matrix, parameter), end)?;
        return Some(TransformDragResult::Placement { parameter, delta });
    }
    match (&drag.gesture, operation) {
        (
            TransformGesture::Splay {
                column,
                swing: true,
                ..
            },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
                ..
            },
        ) => Some(TransformDragResult::SplaySwing {
            column: *column,
            world: *world,
        }),
        (
            TransformGesture::Splay { column, .. },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::Angle { angle, affect },
                ..
            },
        ) => Some(TransformDragResult::SplayAngle {
            column: *column,
            delta_degrees: angle - stored_column_angle(&drag.matrix, *column, affect),
            affect: affect.clone(),
        }),
        (
            TransformGesture::Origin {
                column,
                swing: true,
            },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
                ..
            },
        ) => Some(TransformDragResult::SplaySwing {
            column: *column,
            world: *world,
        }),
        (
            TransformGesture::Origin { column, .. },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::Origin { world },
                ..
            },
        ) => Some(TransformDragResult::SplayOrigin {
            column: *column,
            world: *world,
        }),
        (
            TransformGesture::MatrixMove { anchor },
            EditOperation::SetMatrix { matrix: next, .. },
        ) => {
            // Recover the constrained world delta the preview applied: the
            // commit re-solves it against whatever is accepted when the edit
            // runs, so queued edits compose instead of restoring this pose.
            let origin_delta = Vec2 {
                x: next.origin.x - drag.matrix.origin.x,
                y: next.origin.y - drag.matrix.origin.y,
            };
            let (sin, cos) = anchor
                .map(|anchor| anchor.key_pose.rotation.to_radians().sin_cos())
                .unwrap_or((0.0, 1.0));
            Some(TransformDragResult::MatrixMove {
                delta: Vec2 {
                    x: origin_delta.x * cos - origin_delta.y * sin,
                    y: origin_delta.x * sin + origin_delta.y * cos,
                },
            })
        }
        (TransformGesture::MatrixRotate { .. }, EditOperation::SetMatrix { matrix: next, .. }) => {
            Some(TransformDragResult::MatrixRotate {
                delta_degrees: next.rotation.unwrap_or(0.0) - drag.matrix.rotation.unwrap_or(0.0),
            })
        }
        _ => None,
    }
}

/// Resolve a finished drag against the accepted matrix: the drag's net transform is applied
/// to whatever is accepted when the edit runs, so a drag queued behind other edits composes.
fn transform_drag_resolver(
    matrix_id: String,
    transaction_id: String,
    result: TransformDragResult,
) -> EditResolver {
    EditResolver::new(
        "layout-transform-drag",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = accepted_matrix(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let operation = match &result {
                TransformDragResult::Placement { parameter, delta } => {
                    // A column or row that has gone from the accepted matrix retires the
                    // drag; Core is never asked to edit it.
                    if let Err(reason) = accepted_placement_basis(accepted, matrix, *parameter) {
                        return Resolution::Retire(reason.into());
                    }
                    let Some(value) =
                        apply_placement_delta(placement_current(matrix, *parameter), *delta)
                    else {
                        return Resolution::Retire(
                            "This transform no longer matches the selected matrix.".into(),
                        );
                    };
                    placement_operation(&matrix_id, *parameter, value)
                }
                TransformDragResult::SplayAngle {
                    column,
                    delta_degrees,
                    affect,
                } => EditOperation::SetMatrixSplay {
                    matrix_id: matrix_id.clone(),
                    column: *column,
                    change: MatrixSplayChange::Angle {
                        angle: stored_column_angle(matrix, *column, affect) + delta_degrees,
                        affect: affect.clone(),
                    },
                },
                TransformDragResult::SplayOrigin { column, world } => {
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::Origin { world: *world },
                    }
                }
                TransformDragResult::SplaySwing { column, world } => {
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::OriginSwing {
                            world: Some(*world),
                        },
                    }
                }
                TransformDragResult::MatrixMove { delta } => {
                    // Re-solve the drag's constrained world delta against the
                    // live matrix: an anchor added, retargeted or detached
                    // while the drag was in flight resolves through the live
                    // frame, so the gesture composes instead of restoring.
                    let anchor = match matrix_drag_anchor(accepted, matrix) {
                        Ok(anchor) => anchor,
                        Err(reason) => return Resolution::Retire(reason),
                    };
                    let mut next = matrix.clone();
                    next.origin = solve_matrix_origin(matrix, anchor, *delta);
                    if next.origin == matrix.origin {
                        return Resolution::Retire(
                            "The drag stays outside the followed directions.".into(),
                        );
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
                TransformDragResult::MatrixRotate { delta_degrees } => {
                    let follow_rotation = matrix
                        .anchor
                        .as_ref()
                        .map(|anchor| anchor.follow.rotation)
                        .unwrap_or(true);
                    let Some(rotation) = solve_matrix_rotation(
                        matrix.rotation.unwrap_or(0.0),
                        follow_rotation,
                        *delta_degrees,
                    ) else {
                        return Resolution::Retire(
                            "Rotation is not followed, so turning it changes nothing.".into(),
                        );
                    };
                    let mut next = matrix.clone();
                    next.rotation = Some(rotation);
                    if next.rotation == matrix.rotation {
                        return Resolution::Retire("The drag turns nothing.".into());
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
            };
            transform_commit(transaction_id.clone(), &matrix_id, operation)
        },
    )
}

fn commit_transform_drag(
    runtime: &Rc<Runtime>,
    drag: &TransformDrag,
    operation: &EditOperation,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
) {
    let Some(result) = transform_drag_result(drag, operation) else {
        return;
    };
    let resolver =
        transform_drag_resolver(drag.matrix.id.clone(), drag.transaction_id.clone(), result);
    let resolver = if let Some(dependency) = &drag.cancellation_dependency {
        let dependency = dependency.clone();
        let owner = drag.owner.clone();
        let context = drag.context.clone();
        EditResolver::new("layout-transform-drag", move |accepted| {
            if *dependency.borrow() != Some(TerminalOutcome::Completed)
                || owner.token != Some(accepted.token)
                || owner.revision != Some(accepted.document.revision)
                || owner.scope.as_ref().is_none_or(|scope| {
                    scope.document_id != accepted.document.id
                        || scope.session_epoch != accepted.session_epoch
                })
                || !workspace
                    .try_read()
                    .is_ok_and(|current| *current == owner.workspace)
                || !generation
                    .try_read()
                    .is_ok_and(|current| *current == owner.generation)
                || !selected_context.try_read().is_ok_and(|current| {
                    current.as_ref().is_some_and(|selected| {
                        Some(&selected.scope) == owner.scope.as_ref() && selected.context == context
                    })
                })
            {
                return Resolution::Retire(
                    "The cancelled drag changed before this drag could commit.".into(),
                );
            }
            resolver.resolve(accepted)
        })
    } else {
        resolver
    };
    let mut edits = PendingEdits::default();
    edits.begin(
        runtime,
        (),
        "layout-transform-drag",
        Some("transform".into()),
        resolver,
    );
    spawn_local(async move {
        loop {
            if !edits.settle(true).is_empty() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(16).await;
        }
    });
}

/// A handle key press is a delta intent, so it stays admitted while earlier presses are
/// still applying: only the selection, scope and workspace must be the live owner.
fn transform_nudge_admitted(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
) -> bool {
    let model = runtime.model();
    owner.workspace == "Layout"
        && workspace() == owner.workspace
        && generation() == owner.generation
        && runtime.scope() == owner.scope
        && matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && owner.scope.as_ref().is_some_and(|scope| {
            model.active_board_id == scope.board_id
                && model.active_instance_id == scope.instance_id
                && selected_context().is_some_and(|selected| {
                    selected.scope == *scope && selected.context == *context
                })
        })
}

fn context_matrix_id(context: &TreeContext) -> Option<&str> {
    match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => Some(matrix_id),
        TreeContext::Component { matrix_id, .. } => matrix_id.as_deref(),
        _ => None,
    }
}

fn context_column(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Column { column, .. } | TreeContext::Key { column, .. } => Some(*column),
        TreeContext::Component { column, .. } => *column,
        _ => None,
    }
}

fn context_row(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Row { row, .. } => Some(*row),
        _ => None,
    }
}

/// The selected key's (row, column), when the selection is a key.
fn context_key(context: &TreeContext) -> Option<(u32, u32)> {
    match context {
        TreeContext::Key { row, column, .. } => Some((*row, *column)),
        _ => None,
    }
}

/// The carried-shift parameter for a row or key selection, or `None` when the
/// context has no row or key (a hidden Shift token on a column or matrix
/// selection draws nothing rather than editing the wrong field).
fn shift_parameter(context: &TreeContext) -> Option<PlacementParameter> {
    match context {
        TreeContext::Row { row, .. } => Some(PlacementParameter::RowShift { row: *row }),
        TreeContext::Key { row, column, .. } => Some(PlacementParameter::KeyShift {
            row: *row,
            column: *column,
        }),
        _ => None,
    }
}

/// The carried-bend parameter for a row or key selection, or `None` when the
/// context has no row or key.
fn bend_parameter(context: &TreeContext) -> Option<PlacementParameter> {
    match context {
        TreeContext::Row { row, .. } => Some(PlacementParameter::RowBend { row: *row }),
        TreeContext::Key { row, column, .. } => Some(PlacementParameter::KeyBend {
            row: *row,
            column: *column,
        }),
        _ => None,
    }
}

fn transform_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
) -> bool {
    let model = runtime.model();
    let current = LayoutOwnerIdentity {
        scope: runtime.scope(),
        token: model.accepted.as_ref().map(|snapshot| snapshot.token),
        revision: model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
        generation: generation(),
        workspace: workspace(),
    };
    current == *owner
        && owner.workspace == "Layout"
        && owner.scope.as_ref().is_some_and(|scope| {
            model.active_board_id == scope.board_id
                && model.active_instance_id == scope.instance_id
                && model.accepted.as_ref().is_some_and(|snapshot| {
                    snapshot.document.id == scope.document_id
                        && snapshot.session_epoch == scope.session_epoch
                })
                && selected_context().is_some_and(|selected| {
                    selected.scope == *scope && selected.context == *context
                })
        })
}

fn transform_start_is_editable(
    runtime: &Runtime,
    owner: &LayoutOwnerIdentity,
    matrix: &Matrix,
    cancellation: &Cancellation,
) -> bool {
    let model = runtime.model();
    let cancelling = cancellation
        .borrow()
        .as_ref()
        .is_some_and(|(cancelled_owner, outcome)| {
            cancelled_owner == owner && outcome.borrow().is_none()
        });
    (model.lifecycle == Lifecycle::Ready || (model.lifecycle == Lifecycle::Applying && cancelling))
        && owner
            .revision
            .is_some_and(|revision| model.durability == (Durability::Saved { revision }))
        && (model.display_preview.is_none() || cancelling)
        && model.gesture.is_none()
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot
                .document
                .matrices
                .iter()
                .any(|accepted| accepted == matrix)
        })
}

/// The grid step: `snap_fraction` of the pitch (a negative fraction is an absolute step in
/// millimetres). `None` when snapping is off.
fn grid_step(pitch: Vec2, fraction: f64) -> Option<Vec2> {
    if fraction == 0.0 {
        return None;
    }
    let step_x = if fraction < 0.0 {
        -fraction
    } else {
        pitch.x * fraction
    }
    .max(0.001);
    let step_y = if fraction < 0.0 {
        -fraction
    } else {
        pitch.y * fraction
    }
    .max(0.001);
    Some(Vec2 {
        x: step_x,
        y: step_y,
    })
}

fn snap_delta(delta: Vec2, pitch: Vec2, fraction: f64) -> Vec2 {
    let Some(step) = grid_step(pitch, fraction) else {
        return delta;
    };
    Vec2 {
        x: (delta.x / step.x).round() * step.x,
        y: (delta.y / step.y).round() * step.y,
    }
}

fn snap_tolerance(svg: &Rc<RefCell<Option<SvgElement>>>, view_width: f64) -> f64 {
    svg.borrow().as_ref().map_or(1.0, |surface| {
        let width = surface.get_bounding_client_rect().width();
        if width > 0.0 {
            view_width / width * 8.0
        } else {
            1.0
        }
    })
}

/// The snap inputs for an origin drop: part geometry, the dragged column's
/// world base and the grid fraction. The base is checked first: Ctrl-dragging
/// the origin onto it is the ergogen-style reset.
struct OriginSnapTarget<'a> {
    settings: &'a super::layout_toolbar::LayoutSnapSettings,
    origins: &'a [super::super::outline_snapping::Origin],
    svg: &'a Rc<RefCell<Option<SvgElement>>>,
    view_width: f64,
    column_base: Option<Vec2>,
}

fn snap_world_origin(
    point: Vec2,
    matrix: &Matrix,
    pointer: &web_sys::PointerEvent,
    target: &OriginSnapTarget<'_>,
) -> (Vec2, Option<Vec2>) {
    if pointer.alt_key() {
        return (point, None);
    }
    if let Some(base) = target.column_base
        && (point.x - base.x).hypot(point.y - base.y)
            <= snap_tolerance(target.svg, target.view_width)
    {
        return (base, Some(base));
    }
    if target.settings.geometry_snap {
        let tolerance = snap_tolerance(target.svg, target.view_width);
        if let Some((at, _)) =
            super::super::outline_snapping::snap_origin(point, target.origins, tolerance)
        {
            return (at, Some(at));
        }
    }
    (
        snap_delta(point, matrix.pitch, target.settings.snap_fraction),
        None,
    )
}

struct TransformSampleContext<'a> {
    view_width: f64,
    view_height: f64,
    settings: &'a super::layout_toolbar::LayoutSnapSettings,
    origins: &'a [super::super::outline_snapping::Origin],
    affect: MatrixSplayAffect,
    svg: &'a Rc<RefCell<Option<SvgElement>>>,
    guide: &'a mut Signal<Option<Vec2>>,
}

/// The preview of a Stagger, Offset, Shift or Bend gesture: an absolute placement value
/// for the gesture's parameter, built by the placement module from the world delta since
/// pointer-down. A Bend ignores the grid step's magnitude (it snaps to half degrees) and
/// only reads whether snapping is on, so Alt still keeps the raw angle.
///
/// A stagger snaps to the grid step of the stagger axis' dominant component: pitch.y for an
/// upright axis, pitch.x when a splay has turned the axis nearer to horizontal. The gesture
/// value reads `y` of the step it is given, so the dominant step is handed over there.
fn placement_preview(
    drag: &TransformDrag,
    world_delta: Vec2,
    snap: Option<Vec2>,
) -> Option<EditOperation> {
    let parameter = drag.gesture.parameter()?;
    let basis = drag.basis?;
    let snap = match parameter {
        PlacementParameter::Stagger { .. }
            if basis.stagger_axis.x.abs() > basis.stagger_axis.y.abs() =>
        {
            snap.map(|step| Vec2 {
                x: step.x,
                y: step.x,
            })
        }
        _ => snap,
    };
    let value = gesture_value(
        &basis,
        parameter,
        placement_current(&drag.matrix, parameter),
        world_delta,
        snap,
    );
    Some(placement_operation(&drag.matrix.id, parameter, value))
}

fn sample_transform(
    drag: &TransformDrag,
    pointer: &web_sys::PointerEvent,
    point: Vec2,
    context: TransformSampleContext<'_>,
) -> Option<EditOperation> {
    let TransformSampleContext {
        view_width,
        view_height,
        settings,
        origins,
        affect,
        svg,
        guide,
    } = context;
    let matrix = &drag.matrix;
    match drag.gesture {
        TransformGesture::Stagger { .. }
        | TransformGesture::Spread { .. }
        | TransformGesture::Offset { .. }
        | TransformGesture::Bend { .. } => {
            let rect = svg.borrow().as_ref()?.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return None;
            }
            let world_delta = Vec2 {
                x: (f64::from(pointer.client_x()) - drag.start_client_x) * view_width
                    / rect.width(),
                y: -(f64::from(pointer.client_y()) - drag.start_client_y) * view_height
                    / rect.height(),
            };
            let snap = if pointer.alt_key() {
                None
            } else {
                grid_step(matrix.pitch, settings.snap_fraction)
            };
            guide.set(None);
            placement_preview(drag, world_delta, snap)
        }
        TransformGesture::Splay {
            column,
            start_angle,
            origin,
            swing,
        } => {
            if swing {
                // A Ctrl pointer-down swings the keys: the origin moves with no
                // hold-still compensation, exactly like a Ctrl-click pick.
                let (world, snap) = snap_world_origin(
                    point,
                    matrix,
                    pointer,
                    &OriginSnapTarget {
                        settings,
                        origins,
                        svg,
                        view_width,
                        column_base: drag.origin_base,
                    },
                );
                guide.set(snap);
                return Some(EditOperation::SetMatrixSplay {
                    matrix_id: matrix.id.clone(),
                    column,
                    change: MatrixSplayChange::OriginSwing { world: Some(world) },
                });
            }
            let angle = (point.y - origin.y).atan2(point.x - origin.x) - start_angle;
            let mut delta = angle.sin().atan2(angle.cos()).to_degrees();
            if matches!(
                matrix.mirror,
                Some(boardstudio_core::model::Mirror::X | boardstudio_core::model::Mirror::Y)
            ) {
                delta = -delta;
            }
            let base = stored_column_angle(matrix, column, &affect);
            let value = base + delta;
            let value = if pointer.alt_key() {
                value
            } else {
                value.round()
            };
            guide.set(None);
            Some(EditOperation::SetMatrixSplay {
                matrix_id: matrix.id.clone(),
                column,
                change: MatrixSplayChange::Angle {
                    angle: value,
                    affect,
                },
            })
        }
        TransformGesture::Origin { column, swing } => {
            let (world, snap) = snap_world_origin(
                point,
                matrix,
                pointer,
                &OriginSnapTarget {
                    settings,
                    origins,
                    svg,
                    view_width,
                    column_base: drag.origin_base,
                },
            );
            guide.set(snap);
            let change = if swing {
                MatrixSplayChange::OriginSwing { world: Some(world) }
            } else {
                MatrixSplayChange::Origin { world: Some(world) }
            };
            Some(EditOperation::SetMatrixSplay {
                matrix_id: matrix.id.clone(),
                column,
                change,
            })
        }
        TransformGesture::MatrixMove { anchor } => {
            let rect = svg.borrow().as_ref()?.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return None;
            }
            let world_delta = Vec2 {
                x: (f64::from(pointer.client_x()) - drag.start_client_x) * view_width
                    / rect.width(),
                y: -(f64::from(pointer.client_y()) - drag.start_client_y) * view_height
                    / rect.height(),
            };
            guide.set(None);
            let origin = solve_matrix_origin(matrix, anchor, world_delta);
            if origin == matrix.origin {
                // A drag outside every Follow direction changes nothing.
                return None;
            }
            let mut next = matrix.as_ref().clone();
            next.origin = origin;
            Some(EditOperation::SetMatrix {
                matrix: next,
                definitions: None,
            })
        }
        TransformGesture::MatrixRotate {
            anchor,
            center,
            start_angle,
            start_rotation,
        } => {
            let rotation_followed = match anchor {
                Some(anchor) => anchor.follow.rotation,
                None => true,
            };
            if !rotation_followed {
                return None;
            }
            let angle = (point.y - center.y).atan2(point.x - center.x) - start_angle;
            let mut delta = angle.sin().atan2(angle.cos()).to_degrees();
            if matches!(matrix.mirror, Some(Mirror::X | Mirror::Y)) {
                delta = -delta;
            }
            let mut rotation = start_rotation + delta;
            if !pointer.alt_key() {
                rotation = rotation.round();
            }
            if Some(rotation) == matrix.rotation {
                return None;
            }
            let mut next = matrix.as_ref().clone();
            next.rotation = Some(rotation);
            Some(EditOperation::SetMatrix {
                matrix: next,
                definitions: None,
            })
        }
    }
}

fn stored_column_angle(matrix: &Matrix, column: u32, affect: &MatrixSplayAffect) -> f64 {
    matrix
        .column_layout
        .get(column as usize)
        .map_or(0.0, |record| {
            column_angle(record.splay, record.rotation, affect)
        })
}

fn same_transform_as_start(drag: &TransformDrag, operation: &EditOperation) -> bool {
    if let Some(gesture_parameter) = drag.gesture.parameter() {
        return placement_target(operation, &drag.matrix).is_some_and(|(parameter, value)| {
            parameter == gesture_parameter && value == placement_current(&drag.matrix, parameter)
        });
    }
    match (&drag.gesture, operation) {
        (
            TransformGesture::Splay {
                column,
                swing: true,
                ..
            },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
            },
        )
        | (
            TransformGesture::Origin {
                column,
                swing: true,
            },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
            },
        ) => matrix_id == &drag.matrix.id && target == column && Some(*world) == drag.start_origin,
        (
            TransformGesture::Splay { column, .. },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::Angle { angle, affect },
            },
        ) => {
            matrix_id == &drag.matrix.id
                && target == column
                && *angle == stored_column_angle(&drag.matrix, *column, affect)
        }
        (
            TransformGesture::Origin { column, .. },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::Origin { world },
            },
        ) => {
            matrix_id == &drag.matrix.id
                && target == column
                && world.is_none()
                && drag
                    .matrix
                    .column_layout
                    .get(*column as usize)
                    .is_none_or(|record| record.splay_origin == Vec2::default())
        }
        (TransformGesture::MatrixMove { .. }, EditOperation::SetMatrix { matrix: next, .. }) => {
            next.id == drag.matrix.id && next.origin == drag.matrix.origin
        }
        (TransformGesture::MatrixRotate { .. }, EditOperation::SetMatrix { matrix: next, .. }) => {
            next.id == drag.matrix.id && next.rotation == drag.matrix.rotation
        }
        _ => false,
    }
}

fn submit_transform_edit(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: EditOperation) {
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: drag.transaction_id.clone(),
        target_ids: vec![drag.matrix.id.clone()],
        operation,
    });
}

fn clear_transform_preview(
    runtime: &Rc<Runtime>,
    drag: &TransformDrag,
) -> Option<Rc<RefCell<Option<TerminalOutcome>>>> {
    if !drag.preview_submitted {
        return None;
    }
    if let (Some(token), Some(revision), Some(scope)) = (
        drag.owner.token,
        drag.owner.revision,
        drag.owner.scope.as_ref(),
    ) {
        let operation_id = runtime.operation();
        let outcome = runtime.observe_operation(operation_id);
        runtime.submit(Event::ClearPreview {
            operation_id,
            token,
            revision,
            board_id: scope.board_id.clone(),
            transaction_id: drag.transaction_id.clone(),
        });
        return Some(outcome);
    }
    None
}

fn cancel_transform_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<TransformDrag>>>,
    arbiter: &CanvasInteractionArbiter,
    cancellation: &Cancellation,
    guide: &mut Signal<Option<Vec2>>,
    mut quick_glyph: Signal<bool>,
    mut swing_glyph: Signal<bool>,
    svg_key: usize,
) {
    if abandon_transform_drag(runtime, drag, arbiter, cancellation) {
        guide.set(None);
        quick_glyph.set(false);
        swing_glyph.set(false);
    }
    rail_set_active(svg_key, None);
}

/// Cancel, Escape, lost capture or an owner change: clear the preview, release the pointer
/// and the canvas, and land nothing. Returns whether a drag was active.
fn abandon_transform_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<TransformDrag>>>,
    arbiter: &CanvasInteractionArbiter,
    cancellation: &Cancellation,
) -> bool {
    let Some(active) = drag.borrow_mut().take() else {
        return false;
    };
    if let Some(outcome) = clear_transform_preview(runtime, &active) {
        *cancellation.borrow_mut() = Some((active.owner.clone(), outcome));
    }
    if active.capture.has_pointer_capture(active.pointer_id) {
        let _ = active.capture.release_pointer_capture(active.pointer_id);
    }
    arbiter.release(CanvasInteractionOwner::MatrixTransform);
    true
}

// Presentation-only angular annotation: the accepted Core scene supplies origin and angle.
fn splay_arc(origin: Vec2, end: Vec2, degrees: f64) -> String {
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

fn angle_handle_point(
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

#[component]
pub fn LayoutContextActionToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
) -> Element {
    let property_action = mount.pointer_tools_visible
        && mount.properties_available
        && (matches!(
            selection_kind,
            LayoutSelectionKind::Matrix
                | LayoutSelectionKind::Column
                | LayoutSelectionKind::Key
                | LayoutSelectionKind::Part
        ) || (selection_kind == LayoutSelectionKind::Row && mount.row_available));
    let selection_label = match selection_kind {
        LayoutSelectionKind::Matrix => "Matrix",
        LayoutSelectionKind::Row => "Row",
        LayoutSelectionKind::Column => "Column",
        LayoutSelectionKind::Key => "Key",
        LayoutSelectionKind::Part => "Part",
    };
    let rotate_followed = mount.matrix_rotate_followed.unwrap_or(true);
    let rotate_accessible_label = if rotate_followed {
        "Activate Rotate for the selected matrix"
    } else {
        "Rotate matrix (unavailable: Follow rotation is off)"
    };
    let tools = match selection_kind {
        LayoutSelectionKind::Matrix => vec![
            (
                LayoutTransformTool::Move,
                "Move matrix",
                "Activate Move for the selected matrix",
                true,
                false,
            ),
            (
                LayoutTransformTool::Rotate,
                "Rotate matrix",
                rotate_accessible_label,
                true,
                !rotate_followed,
            ),
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Offset,
                "Offset column",
                "Activate Offset column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread column; selection changes to Column",
                mount.column_available,
                false,
            ),
        ],
        LayoutSelectionKind::Key | LayoutSelectionKind::Part => vec![
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Offset,
                "Offset column",
                "Activate Offset column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread column; selection changes to Column",
                mount.column_available,
                false,
            ),
        ],
        LayoutSelectionKind::Row => vec![
            (
                LayoutTransformTool::Offset,
                "Offset row",
                "Activate Offset for the selected row",
                mount.row_available,
                false,
            ),
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread column; selection changes to Column",
                mount.column_available,
                false,
            ),
        ],
        LayoutSelectionKind::Column => vec![
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger for the selected column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Offset,
                "Offset column",
                "Activate Offset for the selected column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread for the selected column",
                mount.column_available,
                false,
            ),
        ],
    };
    let mut visible_tools = if mount.pointer_tools_visible && mount.pointer_tools_available {
        tools
            .into_iter()
            .filter(|(_, _, _, available, _)| *available)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if mount.pointer_tools_visible
        && mount.pointer_tools_available
        && mount.column_available
        && matches!(
            selection_kind,
            LayoutSelectionKind::Column | LayoutSelectionKind::Matrix
        )
    {
        visible_tools.push((
            LayoutTransformTool::Splay,
            "Splay",
            "Activate Splay for the selected column",
            true,
            false,
        ));
    }
    if !property_action && visible_tools.is_empty() {
        return rsx! {};
    }
    let on_show_properties = mount.on_show_properties;
    let on_selection_kind = mount.on_selection_kind;
    let position_menu = open_menu;
    let property_label = if selection_kind == LayoutSelectionKind::Row {
        "Row offsets"
    } else {
        "Position"
    };
    let property_accessible_label = if selection_kind == LayoutSelectionKind::Row {
        "Open Row offsets properties for Row".to_owned()
    } else {
        format!("Open Position and rotation properties for {selection_label}")
    };

    rsx! {
        BoardToolbar {
            class: "layout-context-actions flex items-center gap-1 px-2",
            label: "Actions for {selection_label}",
            if property_action {
                BoardToolbarButton {
                    index: 0,
                    class: "layout-context-action",
                    aria_label: property_accessible_label.clone(),
                    title: property_accessible_label,
                    on_activate: move |_| {
                        on_show_properties.call(());
                        if selection_kind == LayoutSelectionKind::Row {
                            on_selection_kind.call(LayoutSelectionKind::Row);
                        }
                        close_layout_command_menu(position_menu, LayoutCommandMenu::Transform);
                    },
                    svg { class: "m1-context-tool-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                        path { d: "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3M5 7l-3 3 3 3M15 7l3 3-3 3" }
                    }
                    span { class: "layout-action-label", "{property_label}" }
                }
            }
            for (index, (tool, label, accessible_label, _, disabled)) in visible_tools.into_iter().enumerate() {
                LayoutContextToolButton {
                    index: index + usize::from(property_action),
                    mount: mount.clone(),
                    open_menu,
                    tool,
                    label: label.to_owned(),
                    accessible_label: accessible_label.to_owned(),
                    pressed: mount.active_tool == Some(tool),
                    disabled,
                }
            }
        }
    }
}

#[component]
fn LayoutContextToolButton(
    index: usize,
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    tool: LayoutTransformTool,
    label: String,
    accessible_label: String,
    pressed: bool,
    #[props(default)] disabled: bool,
) -> Element {
    let on_transform_tool = mount.on_transform_tool;
    let glyph = match tool {
        LayoutTransformTool::Stagger => "M10 2v16M6 6l4-4 4 4M6 14l4 4 4-4",
        LayoutTransformTool::Spread => "M2 10h16M4 6l-2 4 2 4M16 6l2 4-2 4",
        LayoutTransformTool::Offset => "M2 10h16M6 6l-4 4 4 4M14 6l4 4-4 4",
        // Ticket 05: Shift shares the four-way arrow; Bend is a curved arrow.
        // The Row/Key context bars do not list them yet (rail-only ticket).
        LayoutTransformTool::Shift => "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3",
        LayoutTransformTool::Bend => "M4 16C4 9 8 5 15 5M12 2l3 3-3 3",
        LayoutTransformTool::Splay => "M4 16h13M4 16l8-12M7 16a7 7 0 0 0 3-6",
        LayoutTransformTool::Origin => "M10 1v18M1 10h18M5 5h10v10H5z",
        LayoutTransformTool::Move => "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3",
        LayoutTransformTool::Rotate => "M17 10a7 7 0 1 1-2-4.9M17 2v5h-5",
        LayoutTransformTool::Anchor => {
            "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0"
        }
    };
    rsx! {
        BoardToolbarButton {
            index,
            class: "layout-context-action",
            aria_label: accessible_label.clone(),
            title: accessible_label,
            pressed,
            disabled,
            on_activate: move |_| {
                on_transform_tool.call(tool);
                close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
            },
            svg { class: "m1-context-tool-icon", view_box: "0 0 20 20", "aria-hidden": "true", path { d: glyph } }
            span { class: "layout-action-label", {match label.as_str() {
                "Stagger column" => "Stagger",
                "Offset column" | "Offset row" => "Offset",
                "Spread column" => "Spread",
                "Move matrix" => "Move",
                "Rotate matrix" => "Rotate",
                _ => label.as_str(),
            }} }
        }
    }
}

#[component]
pub fn LayoutTransformToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
) -> Element {
    let is_open = open_menu() == Some(LayoutCommandMenu::Transform);
    let position_label = "Position & rotation";

    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                let mut open_menu = open_menu;
                if next {
                    open_menu.set(Some(LayoutCommandMenu::Transform));
                } else if open_menu() == Some(LayoutCommandMenu::Transform) {
                    open_menu.set(None);
                }
            },
            trigger_id: "m1-layout-transform-trigger",
            content_id: "m1-layout-transform-menu",
            trigger_label: "Transform",
            root_class: "m1-layout-command-menu m1-layout-transform-menu",
            menu_kind: "transform",
            content_class: "m1-layout-transform-popover grid gap-1",
            content_role: "group",
            content_label: "Transform tools and properties",
            trigger: rsx! {
                svg { class: "m1-layout-command-trigger-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    circle { cx: "4", cy: "10", r: "1.2", fill: "currentColor" }
                    circle { cx: "10", cy: "10", r: "1.2", fill: "currentColor" }
                    circle { cx: "16", cy: "10", r: "1.2", fill: "currentColor" }
                }
                span { class: "layout-action-label", "Transform" }
            },
            super::layout_toolbar::LayoutCommandMenuHeader {
                label: "Transform".to_owned(),
                close_label: "Close Transform".to_owned(),
                open_menu,
                menu: LayoutCommandMenu::Transform,
            }
            if mount.pointer_tools_visible {
                BoardButton {
                    density: BoardButtonDensity::Compact,
                    disabled: !mount.pointer_tools_available || !mount.column_available,
                    pressed: mount.active_tool == Some(LayoutTransformTool::Origin),
                    onclick: move |_| {
                        mount.on_transform_tool.call(LayoutTransformTool::Origin);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "Origin"
                }
            } else {
            for (tool, label) in LayoutTransformTool::ALL {
                if mount.pointer_tools_visible {
                    BoardButton {
                        density: BoardButtonDensity::Compact,
                        disabled: !mount.pointer_tools_available,
                        pressed: mount.active_tool == Some(tool),
                        onclick: move |_| {
                            mount.on_transform_tool.call(tool);
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                        },
                        "{label}"
                    }
                }
            }
            BoardButton {
                density: BoardButtonDensity::Compact,
                disabled: !mount.properties_available,
                onclick: move |_| {
                    mount.on_show_properties.call(());
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                },
                "{position_label}"
            }
            BoardButton {
                density: BoardButtonDensity::Compact,
                disabled: !mount.column_available,
                onclick: move |_| {
                    mount.on_show_properties.call(());
                    mount.on_selection_kind.call(LayoutSelectionKind::Column);
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                },
                "Splay & origin"
            }
            BoardButton {
                density: BoardButtonDensity::Compact,
                disabled: !mount.row_available,
                onclick: move |_| {
                    mount.on_show_properties.call(());
                    mount.on_selection_kind.call(LayoutSelectionKind::Row);
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                },
                "Row offsets"
            }
            }
            if !mount.properties_available {
                p { "Select a matrix, row, column, key or part to open its current transform properties." }
            }
        }
    }
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn shortcut_test_host() -> Element {
        let mut properties_tab = use_signal(|| false);
        let mut owner_current = use_signal(|| true);
        let mut selection_changes = use_signal(|| 0_u32);
        let mut open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
        let on_show_properties = EventHandler::new(move |()| {
            if owner_current() {
                properties_tab.set(true);
            }
        });
        let on_selection_kind = EventHandler::new(move |_: LayoutSelectionKind| {
            owner_current.set(false);
            selection_changes += 1;
        });
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: false,
            pointer_tools_available: false,
            active_tool: None,
            matrix_rotate_followed: None,
            on_selection_kind,
            on_show_properties,
            on_transform_tool: EventHandler::new(|_: LayoutTransformTool| {}),
        };

        rsx! {
            LayoutTransformToolbar { mount, open_menu }
            button {
                id: "reset-shortcut-probe",
                onclick: move |_| {
                    properties_tab.set(false);
                    owner_current.set(true);
                    selection_changes.set(0);
                    open_menu.set(Some(LayoutCommandMenu::Transform));
                },
                "Reset"
            }
            output { id: "shortcut-probe-state", "{properties_tab()}:{selection_changes()}" }
        }
    }

    #[derive(Props, Clone, PartialEq)]
    struct ContextHostProps {
        selection_kind: LayoutSelectionKind,
        pointer_tools_visible: bool,
        pointer_tools_available: bool,
    }

    #[component]
    fn context_test_host(props: ContextHostProps) -> Element {
        let mut active_tool = use_signal(|| None::<LayoutTransformTool>);
        let mut selection_kind = use_signal(|| props.selection_kind);
        let open_menu = use_signal(|| None::<LayoutCommandMenu>);
        let on_transform_tool = EventHandler::new(move |tool| {
            let context = match selection_kind() {
                LayoutSelectionKind::Row => TreeContext::Row {
                    matrix_id: "matrix-1".to_owned(),
                    row: 0,
                },
                _ => TreeContext::Column {
                    matrix_id: "matrix-1".to_owned(),
                    column: 0,
                },
            };
            let activation = activate_transform_tool(active_tool(), tool, &context);
            active_tool.set(activation.active);
            if let Some(next_kind) = activation.selection_kind {
                selection_kind.set(next_kind);
            }
        });
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: props.pointer_tools_visible,
            pointer_tools_available: props.pointer_tools_available,
            active_tool: active_tool(),
            matrix_rotate_followed: None,
            on_selection_kind: EventHandler::new(move |kind| selection_kind.set(kind)),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool,
        };
        let selection_label = match selection_kind() {
            LayoutSelectionKind::Matrix => "Matrix",
            LayoutSelectionKind::Row => "Row",
            LayoutSelectionKind::Column => "Column",
            LayoutSelectionKind::Key => "Key",
            LayoutSelectionKind::Part => "Part",
        };
        let active_label = match active_tool() {
            Some(LayoutTransformTool::Stagger) => "Stagger",
            Some(LayoutTransformTool::Spread) => "Spread",
            Some(LayoutTransformTool::Offset) => "Offset",
            Some(LayoutTransformTool::Shift) => "Shift",
            Some(LayoutTransformTool::Bend) => "Bend",
            Some(LayoutTransformTool::Splay) => "Splay",
            Some(LayoutTransformTool::Origin) => "Origin",
            Some(LayoutTransformTool::Move) => "Move",
            Some(LayoutTransformTool::Rotate) => "Rotate",
            Some(LayoutTransformTool::Anchor) => "Anchor",
            None => "None",
        };

        rsx! {
            LayoutContextActionToolbar { mount, open_menu, selection_kind: selection_kind() }
            output { id: "context-action-state", "{selection_label}:{active_label}" }
        }
    }

    /// Ticket 23: the Handles toggle flips view state and reports the intent,
    /// so hiding handles never touches the document. The host owns no Runtime
    /// and no edit machinery: the intent carries a plain bool to workspace state.
    #[component]
    fn snap_dock_host() -> Element {
        let mut handles = use_signal(|| true);
        let mut toggled = use_signal(|| None::<bool>);
        rsx! {
            super::super::layout_toolbar::LayoutSnapDock {
                settings: super::super::layout_toolbar::LayoutSnapSettings::default(),
                open_menu: use_signal(|| None),
                on_snap_intent: EventHandler::new(|_| {}),
                handles_visible: handles(),
                on_toggle_handles: EventHandler::new(move |visible| {
                    toggled.set(Some(visible));
                    handles.set(visible);
                }),
            }
            output { id: "snap-dock-toggled", "{toggled():?}" }
        }
    }

    fn handles_checkbox(root: &web_sys::Element) -> web_sys::HtmlInputElement {
        let labels = root.query_selector_all("label").unwrap();
        let toggle: web_sys::Element = (0..labels.length())
            .filter_map(|index| labels.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .find(|node| node.text_content().as_deref() == Some("Canvas handles"))
            .expect("the dock names the Canvas handles toggle");
        toggle
            .query_selector("input")
            .unwrap()
            .expect("the toggle is a checkbox")
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn handles_toggle_reports_intent_and_flips_view_only() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(snap_dock_host),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root.query_selector("#m1-layout-snap-trigger")
            .unwrap()
            .expect("the snap dock trigger is mounted")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert!(handles_checkbox(&root).checked(), "handles start visible");
        handles_checkbox(&root).click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#snap-dock-toggled")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Some(false)"),
            "the toggle reports the flipped view state"
        );
        assert!(
            !handles_checkbox(&root).checked(),
            "the checkbox follows view state"
        );
        handles_checkbox(&root).click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#snap-dock-toggled")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Some(true)"),
            "toggling back restores the handles"
        );
        root.remove();
    }

    async fn mount_context_actions(
        selection_kind: LayoutSelectionKind,
        pointer_tools_visible: bool,
    ) -> web_sys::Element {
        mount_context_actions_with_availability(selection_kind, pointer_tools_visible, true).await
    }

    async fn mount_context_actions_with_availability(
        selection_kind: LayoutSelectionKind,
        pointer_tools_visible: bool,
        pointer_tools_available: bool,
    ) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                context_test_host,
                ContextHostProps {
                    selection_kind,
                    pointer_tools_visible,
                    pointer_tools_available,
                },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[wasm_bindgen_test]
    async fn contextual_tools_show_for_layout_selection_and_follow_existing_activation_scope() {
        let root = mount_context_actions(LayoutSelectionKind::Row, true).await;
        let buttons = root.query_selector_all(".layout-context-action").unwrap();
        let labels = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .map(|button| button.text_content().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(labels, ["Row offsets", "Offset", "Stagger", "Spread"]);
        root.query_selector(
            "button[aria-label='Activate Stagger column; selection changes to Column']",
        )
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#context-action-state")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Column:Stagger")
        );
        assert_eq!(
            root.query_selector("button[aria-label='Activate Stagger for the selected column']")
                .unwrap()
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        let stagger = root
            .query_selector("button[aria-label='Activate Stagger for the selected column']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        stagger.focus().unwrap();
        let key = web_sys::KeyboardEventInit::new();
        key.set_key("ArrowRight");
        key.set_bubbles(true);
        stagger
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .get_attribute("aria-label")
                .as_deref(),
            Some("Activate Offset for the selected column"),
            "toolbar keyboard navigation must follow the updated selection's action order"
        );
        root.remove();

        let pcb = mount_context_actions(LayoutSelectionKind::Matrix, false).await;
        assert!(
            pcb.query_selector(".layout-context-actions")
                .unwrap()
                .is_none(),
            "PCB must not inherit Layout-only contextual quick actions"
        );
        pcb.remove();
    }

    #[wasm_bindgen_test]
    async fn all_transform_property_shortcuts_route_to_properties_before_selection_changes_owner() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(shortcut_test_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;

        for (label, expected) in [
            ("Position & rotation", "true:0"),
            ("Splay & origin", "true:1"),
            ("Row offsets", "true:1"),
        ] {
            let button = root.query_selector_all("button").unwrap();
            let target = (0..button.length())
                .filter_map(|index| button.item(index))
                .find(|element| {
                    element
                        .text_content()
                        .is_some_and(|text| text.trim() == label)
                })
                .expect("transform shortcut button exists")
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            target.click();
            gloo_timers::future::TimeoutFuture::new(40).await;
            assert_eq!(
                root.query_selector("#shortcut-probe-state")
                    .unwrap()
                    .unwrap()
                    .text_content()
                    .as_deref(),
                Some(expected),
                "{label} must select Properties before any selection transition"
            );
            root.query_selector("#reset-shortcut-probe")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click();
            gloo_timers::future::TimeoutFuture::new(40).await;
        }

        root.remove();
    }

    /// Ticket 07: a part selection keeps the Position strip without canvas
    /// tools, like production (the anchor pick arrives with ticket 08).
    #[wasm_bindgen_test]
    async fn part_selection_keeps_the_position_strip_without_canvas_tools() {
        let root =
            mount_context_actions_with_availability(LayoutSelectionKind::Part, true, false).await;
        let buttons = root.query_selector_all(".layout-context-action").unwrap();
        let labels = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .map(|button| button.text_content().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(labels, ["Position"]);
        assert_eq!(
            root.query_selector("#context-action-state")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Part:None")
        );
        root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod nudge_tests {
    use super::*;
    use crate::matrix_transform_operation::{
        PlacementParameter, PlacementValue, placement_operation,
    };
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::Mirror;
    use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    const TRANSACTION: &str = "layout-transform-test";

    fn nudge_edit(runtime: &Rc<Runtime>, key: ArrowKey, big: bool) -> PendingEdits<()> {
        nudge_with(
            runtime,
            TransformNudge::Stagger {
                column: 0,
                key,
                big,
            },
        )
    }

    fn nudge_with(runtime: &Rc<Runtime>, nudge: TransformNudge) -> PendingEdits<()> {
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            (),
            "layout-transform-nudge",
            Some("transform".into()),
            transform_nudge_resolver(fixture::MATRIX_ID.into(), nudge, Vec2::default()),
        );
        edits
    }

    fn stagger_result(column: u32, along: f64) -> TransformDragResult {
        TransformDragResult::Placement {
            parameter: PlacementParameter::Stagger { column },
            delta: PlacementDelta::Stagger(along),
        }
    }

    #[wasm_bindgen_test]
    async fn column_angle_nudges_compose_without_overwriting_splay() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(1, Default::default);
        document.matrices[0].column_layout[0].splay = 7.0;
        document.matrices[0].column_layout[0].rotation = 10.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let nudge = TransformNudge::SplayAngle {
            column: 0,
            degrees: 1.0,
            affect: MatrixSplayAffect::Column,
        };
        let mut first = nudge_with(&runtime, nudge.clone());
        settle_edit(&runtime, &mut first).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].rotation,
            11.0
        );
        let mut second = nudge_with(&runtime, nudge);
        settle_edit(&runtime, &mut second).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].rotation, 12.0);
        assert_eq!(matrix.column_layout[0].splay, 7.0);
        let mut following = nudge_with(
            &runtime,
            TransformNudge::SplayAngle {
                column: 0,
                degrees: 1.0,
                affect: MatrixSplayAffect::Following,
            },
        );
        settle_edit(&runtime, &mut following).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].splay, 8.0);
        assert_eq!(matrix.column_layout[0].rotation, 12.0);
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].splay,
            7.0
        );
    }

    #[wasm_bindgen_test]
    async fn column_angle_drag_delta_composes_with_the_latest_rotation() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(1, Default::default);
        document.matrices[0].column_layout[0].splay = 7.0;
        document.matrices[0].column_layout[0].rotation = 10.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            TransformGesture::Splay {
                column: 0,
                start_angle: 0.0,
                origin: Vec2::default(),
                swing: false,
            },
        );
        let operation = EditOperation::SetMatrixSplay {
            matrix_id: fixture::MATRIX_ID.into(),
            column: 0,
            change: MatrixSplayChange::Angle {
                angle: 12.0,
                affect: MatrixSplayAffect::Column,
            },
        };
        let result = transform_drag_result(&drag, &operation).unwrap();
        assert_eq!(
            result,
            TransformDragResult::SplayAngle {
                column: 0,
                delta_degrees: 2.0,
                affect: MatrixSplayAffect::Column,
            }
        );
        assert!(same_transform_as_start(
            &drag,
            &EditOperation::SetMatrixSplay {
                matrix_id: fixture::MATRIX_ID.into(),
                column: 0,
                change: MatrixSplayChange::Angle {
                    angle: 10.0,
                    affect: MatrixSplayAffect::Column
                },
            }
        ));
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut other = nudge_with(
            &runtime,
            TransformNudge::SplayAngle {
                column: 0,
                degrees: 1.0,
                affect: MatrixSplayAffect::Column,
            },
        );
        support::drive_pending(&runtime);
        entered.await.expect("the nudge reached Core");
        let mut queued = drag_edit(&runtime, fixture::MATRIX_ID, result);
        release.send(()).unwrap();
        settle_edit(&runtime, &mut other).await;
        settle_edit(&runtime, &mut queued).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].rotation, 13.0);
        assert_eq!(matrix.column_layout[0].splay, 7.0);
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].rotation,
            11.0
        );
        redo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].rotation,
            13.0
        );
    }

    /// The first terminal result of an edit, for tests that care how it ended.
    async fn settle_outcome(
        runtime: &Rc<Runtime>,
        edits: &mut PendingEdits<()>,
    ) -> PendingEditResult<()> {
        for _ in 0..100 {
            support::run_pending(runtime).await;
            if let Some(result) = edits.settle(true).into_iter().find(|result| {
                matches!(
                    result,
                    PendingEditResult::Landed { .. }
                        | PendingEditResult::Failed { .. }
                        | PendingEditResult::Retired { .. }
                )
            }) {
                return result;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("transform edit did not settle");
    }

    fn drag_edit(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        result: TransformDragResult,
    ) -> PendingEdits<()> {
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            (),
            "layout-transform-drag",
            Some("transform".into()),
            transform_drag_resolver(matrix_id.into(), TRANSACTION.into(), result),
        );
        edits
    }

    /// An absolute placement edit, as the Inspector sends it.
    fn placement_edit(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        parameter: PlacementParameter,
        value: PlacementValue,
    ) -> PendingEdits<()> {
        let matrix_id = matrix_id.to_owned();
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            (),
            "inspector-placement",
            Some("transform".into()),
            EditResolver::new("inspector-placement", move |_: &AcceptedSnapshot| {
                transform_commit(
                    String::new(),
                    &matrix_id,
                    placement_operation(&matrix_id, parameter, value),
                )
            }),
        );
        edits
    }

    async fn settle_edit(runtime: &Rc<Runtime>, edits: &mut PendingEdits<()>) {
        for _ in 0..100 {
            support::run_pending(runtime).await;
            if edits.settle(true).into_iter().any(|result| {
                matches!(
                    result,
                    PendingEditResult::Landed { .. }
                        | PendingEditResult::Failed { .. }
                        | PendingEditResult::Retired { .. }
                )
            }) {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("transform edit did not settle");
    }

    async fn redo(runtime: &Rc<Runtime>) {
        runtime.submit(Event::Redo {
            operation_id: runtime.operation(),
        });
        support::run_pending(runtime).await;
    }

    fn matrix_named(runtime: &Runtime, id: &str) -> Matrix {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == id)
            .cloned()
            .expect("the matrix exists")
    }

    fn matrix_and_poses(runtime: &Rc<Runtime>) -> (Matrix, Vec<(String, Vec2, f64)>) {
        let accepted = runtime.model().accepted.expect("an accepted document");
        let mut poses = accepted
            .document
            .parts
            .iter()
            .map(|part| (part.id.clone(), part.pose.at, part.pose.rotation))
            .collect::<Vec<_>>();
        poses.sort_by(|a, b| a.0.cmp(&b.0));
        (fixture::accepted_matrix(runtime), poses)
    }

    fn revision(runtime: &Runtime) -> u64 {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision
    }

    /// A drag as pointer-down would record it, for the matrix as it is accepted now.
    fn begin_drag(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        column: u32,
        quick_offset: bool,
    ) -> TransformDrag {
        begin_gesture(
            runtime,
            matrix_id,
            TransformGesture::Stagger {
                column,
                quick_offset,
            },
        )
    }

    fn begin_gesture(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        gesture: TransformGesture,
    ) -> TransformDrag {
        let column = match gesture {
            TransformGesture::Stagger { column, .. } => column,
            TransformGesture::Offset {
                parameter: PlacementParameter::ColumnOffset { column },
            } => column,
            _ => 0,
        };
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let scene = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
            .expect("the matrix has a scene");
        let capture = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| {
                document
                    .create_element_ns(Some("http://www.w3.org/2000/svg"), "g")
                    .ok()
            })
            .and_then(|element| element.dyn_into::<SvgElement>().ok())
            .expect("an SVG element");
        TransformDrag {
            pointer_id: 1,
            capture,
            matrix: Rc::new(matrix_named(runtime, matrix_id)),
            gesture,
            transaction_id: TRANSACTION.into(),
            owner: LayoutOwnerIdentity {
                scope: runtime.scope(),
                token: Some(accepted.token),
                revision: Some(accepted.document.revision),
                generation: 0,
                workspace: "Layout",
            },
            context: TreeContext::Column {
                matrix_id: matrix_id.into(),
                column,
            },
            tool: LayoutTransformTool::Stagger,
            basis: gesture
                .parameter()
                .and_then(|parameter| SceneBasis::for_parameter(scene, parameter)),
            origin_base: None,
            start_origin: None,
            start_client_x: 0.0,
            start_client_y: 0.0,
            pending: None,
            preview_submitted: false,
            cancellation_dependency: None,
        }
    }

    fn preview(runtime: &Rc<Runtime>, drag: &mut TransformDrag, world: Vec2) -> EditOperation {
        let operation = placement_preview(drag, world, None).expect("a preview");
        submit_transform_edit(runtime, drag, operation.clone());
        drag.preview_submitted = true;
        operation
    }

    async fn commit(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: &EditOperation) {
        let result = transform_drag_result(drag, operation).expect("a drag result");
        let mut edits = drag_edit(runtime, &drag.matrix.id, result);
        settle_edit(runtime, &mut edits).await;
    }

    fn offset_of(parameter: PlacementParameter) -> TransformGesture {
        TransformGesture::Offset { parameter }
    }

    #[wasm_bindgen_test]
    async fn an_offset_drag_moves_one_column_by_the_pointer_delta() {
        let runtime = fixture::open_matrix_runtime().await;
        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            offset_of(PlacementParameter::ColumnOffset { column: 0 }),
        );
        let last = preview(&runtime, &mut drag, Vec2 { x: 2.0, y: 1.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.column_layout.first().map(|record| &record.offset),
            Some(&Vec2 { x: 2.0, y: 1.0 })
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0)
        );
        assert!(
            matrix
                .row_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default())
        );
    }

    #[wasm_bindgen_test]
    async fn a_row_offset_drag_and_nudge_change_only_that_rows_offset() {
        let mut document = fixture::matrix_document();
        document.matrices[0].rows = 2;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let row = PlacementParameter::RowOffset { row: 1 };
        let mut drag = begin_gesture(&runtime, fixture::MATRIX_ID, offset_of(row));
        let last = preview(&runtime, &mut drag, Vec2 { x: 1.0, y: 2.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.offset),
            Some(&Vec2 { x: 1.0, y: 2.0 })
        );

        let mut nudge = nudge_with(
            &runtime,
            TransformNudge::Offset {
                parameter: row,
                key: ArrowKey::Up,
                big: false,
            },
        );
        settle_edit(&runtime, &mut nudge).await;
        let matrix = fixture::accepted_matrix(&runtime);
        let offset = matrix.row_layout[1].offset;
        assert!(
            (offset.x - 1.0).abs() < 1e-9 && (offset.y - 2.1).abs() < 1e-9,
            "ArrowUp raises the row by 0.1 mm: {offset:?}"
        );
        assert_eq!(
            matrix
                .row_layout
                .first()
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
            Vec2::default()
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default())
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0)
        );
    }

    #[wasm_bindgen_test]
    async fn a_column_offset_on_a_splayed_rotated_matrix_follows_the_pointer_in_world_space() {
        let mut document = fixture::matrix_document();
        document.matrices[0].rotation = Some(30.0);
        {
            let values = vec![0.0, 20.0, 0.0];
            let count = values.len().max(document.matrices[0].column_layout.len());
            document.matrices[0]
                .column_layout
                .resize_with(count, Default::default);
            for record in &mut document.matrices[0].column_layout {
                record.splay = 0.0;
            }
            for (record, value) in document.matrices[0].column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        // The first edit syncs the fixture's unlaid poses; the drag is measured after it.
        let mut sync = placement_edit(
            &runtime,
            fixture::MATRIX_ID,
            PlacementParameter::ColumnOffset { column: 2 },
            PlacementValue::Offset(Vec2 { x: 0.5, y: 0.0 }),
        );
        settle_edit(&runtime, &mut sync).await;
        let (before_matrix, before) = matrix_and_poses(&runtime);

        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            offset_of(PlacementParameter::ColumnOffset { column: 1 }),
        );
        let world = Vec2 { x: 3.0, y: 2.0 };
        let last = preview(&runtime, &mut drag, world);
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let (after_matrix, after) = matrix_and_poses(&runtime);
        for ((id, from, _), (_, to, _)) in before.iter().zip(&after) {
            let expected = if id == "matrix/matrix/r0c1" {
                Vec2 {
                    x: world.x,
                    y: world.y,
                }
            } else {
                Vec2::default()
            };
            assert!(
                (to.x - from.x - expected.x).abs() < 1e-6
                    && (to.y - from.y - expected.y).abs() < 1e-6,
                "{id} moved by ({}, {}), expected {expected:?}",
                to.x - from.x,
                to.y - from.y
            );
        }
        assert_eq!(
            after_matrix
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>(),
            before_matrix
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            after_matrix
                .row_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>(),
            before_matrix
                .row_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>()
        );
    }

    #[wasm_bindgen_test]
    async fn a_drag_for_a_column_or_row_that_has_gone_is_refused_before_core() {
        let runtime = fixture::open_matrix_runtime().await;
        let revision_before = revision(&runtime);
        for parameter in [
            PlacementParameter::ColumnOffset { column: 9 },
            PlacementParameter::Stagger { column: 9 },
            PlacementParameter::RowOffset { row: 5 },
        ] {
            let delta = match parameter {
                PlacementParameter::Stagger { .. } => PlacementDelta::Stagger(1.0),
                _ => PlacementDelta::Offset(Vec2 { x: 1.0, y: 0.0 }),
            };
            let mut edits = drag_edit(
                &runtime,
                fixture::MATRIX_ID,
                TransformDragResult::Placement { parameter, delta },
            );
            let outcome = settle_outcome(&runtime, &mut edits).await;
            assert!(
                matches!(
                    &outcome,
                    PendingEditResult::Failed { message, .. } if message.contains("no longer exists")
                ),
                "{parameter:?} must be refused by the resolver, got {outcome:?}"
            );
        }
        assert_eq!(revision(&runtime), revision_before, "nothing was written");
    }

    #[wasm_bindgen_test]
    async fn a_stagger_snaps_to_the_grid_step_of_its_axis_dominant_component() {
        let runtime = fixture::open_matrix_runtime().await;
        let mut drag = begin_drag(&runtime, fixture::MATRIX_ID, 1, false);
        let step = Some(Vec2 { x: 4.0, y: 9.0 });
        let value = |drag: &TransformDrag, world: Vec2| {
            let Some(EditOperation::SetMatrixPlacement {
                change: MatrixPlacementChange::ColumnStagger { value, .. },
                ..
            }) = placement_preview(drag, world, step)
            else {
                panic!("a ColumnStagger preview");
            };
            value
        };
        assert!(
            (value(&drag, Vec2 { x: 0.0, y: 5.0 }) - 9.0).abs() < 1e-9,
            "an upright axis snaps to the y step"
        );
        drag.basis = Some(SceneBasis {
            stagger_axis: Vec2 { x: 1.0, y: 0.0 },
            axis_x: Vec2 { x: 1.0, y: 0.0 },
            axis_y: Vec2 { x: 0.0, y: 1.0 },
            shift_axis_x: Vec2 { x: 1.0, y: 0.0 },
            shift_axis_y: Vec2 { x: 0.0, y: 1.0 },
            spread_axis: None,
        });
        assert!(
            (value(&drag, Vec2 { x: 5.0, y: 0.0 }) - 4.0).abs() < 1e-9,
            "a horizontal axis snaps to the x step"
        );
    }

    #[wasm_bindgen_test]
    async fn three_handle_nudges_queued_behind_a_gated_reply_move_the_matrix_three_steps() {
        let runtime = fixture::open_matrix_runtime().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut first = nudge_edit(&runtime, ArrowKey::Up, false);
        support::drive_pending(&runtime);
        entered.await.expect("the first nudge reached Core");
        let mut second = nudge_edit(&runtime, ArrowKey::Up, false);
        let mut third = nudge_edit(&runtime, ArrowKey::Up, false);
        support::drive_pending(&runtime);
        release.send(()).expect("release the held reply");
        settle_edit(&runtime, &mut third).await;
        settle_edit(&runtime, &mut first).await;
        settle_edit(&runtime, &mut second).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert!(
            (matrix.column_layout[0].stagger - 0.3).abs() < 1e-9,
            "three steps of 0.1 compose to a stagger of 0.3, got {}",
            matrix.column_layout[0].stagger
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default()),
            "the offsets are untouched"
        );
    }

    #[wasm_bindgen_test]
    async fn a_drag_commit_applies_its_net_transform_to_the_accepted_matrix() {
        let runtime = fixture::open_matrix_runtime().await;
        // Another stagger edit lands while the drag was in flight; the drag's net change
        // composes with it instead of restoring the matrix the drag started from.
        let mut other = placement_edit(
            &runtime,
            fixture::MATRIX_ID,
            PlacementParameter::Stagger { column: 0 },
            PlacementValue::Stagger(1.0),
        );
        settle_edit(&runtime, &mut other).await;
        let mut drag = drag_edit(&runtime, fixture::MATRIX_ID, stagger_result(0, 2.0));
        settle_edit(&runtime, &mut drag).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].stagger, 3.0);
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default())
        );
    }

    #[wasm_bindgen_test]
    async fn cancel_and_escape_land_nothing() {
        let runtime = fixture::open_matrix_runtime().await;
        let mut settled = nudge_edit(&runtime, ArrowKey::Up, true);
        settle_edit(&runtime, &mut settled).await;
        let before = matrix_and_poses(&runtime);
        let revision_before = revision(&runtime);

        let mut drag = begin_drag(&runtime, fixture::MATRIX_ID, 1, false);
        preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 3.0 });
        support::run_pending(&runtime).await;
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag is previewing"
        );
        let arbiter = CanvasInteractionArbiter::default();
        assert!(arbiter.try_acquire(CanvasInteractionOwner::MatrixTransform));
        // Escape, pointer cancel and lost capture all end in this.
        let cell = Rc::new(RefCell::new(Some(drag)));
        assert!(abandon_transform_drag(
            &runtime,
            &cell,
            &arbiter,
            &Cancellation::default()
        ));
        support::run_pending(&runtime).await;

        assert!(runtime.model().display_preview.is_none());
        assert_eq!(arbiter.current(), None, "the canvas is released");
        assert_eq!(revision(&runtime), revision_before);
        assert_eq!(matrix_and_poses(&runtime), before);
        // The Undo depth is unchanged: one Undo reverts only the earlier nudge.
        fixture::undo(&runtime).await;
        assert!(
            fixture::accepted_matrix(&runtime)
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0),
            "the cancelled drag left no Undo step behind"
        );
    }

    #[wasm_bindgen_test]
    async fn one_drag_is_one_undo_step_and_redo_restores_it() {
        let runtime = fixture::open_matrix_runtime().await;
        let before = matrix_and_poses(&runtime);
        let revision_before = revision(&runtime);
        let mut drag = begin_drag(&runtime, fixture::MATRIX_ID, 1, false);
        preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 1.0 });
        support::run_pending(&runtime).await;
        let last = preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 4.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let after = matrix_and_poses(&runtime);
        assert_eq!(
            after.0.column_layout.get(1).map(|record| &record.stagger),
            Some(&4.0)
        );
        assert_eq!(revision(&runtime), revision_before + 1, "one accepted edit");

        fixture::undo(&runtime).await;
        assert_eq!(
            matrix_and_poses(&runtime)
                .0
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>(),
            before
                .0
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>()
        );
        redo(&runtime).await;
        assert_eq!(matrix_and_poses(&runtime), after, "Redo restores the drag");
    }

    #[wasm_bindgen_test]
    async fn a_drag_composes_with_an_unrelated_edit_held_in_flight() {
        let runtime = fixture::open_matrix_runtime().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut unrelated = placement_edit(
            &runtime,
            fixture::MATRIX_ID,
            PlacementParameter::ColumnOffset { column: 2 },
            PlacementValue::Offset(Vec2 { x: 1.0, y: 0.0 }),
        );
        support::drive_pending(&runtime);
        entered.await.expect("the unrelated edit reached Core");
        let mut drag = drag_edit(&runtime, fixture::MATRIX_ID, stagger_result(1, 2.0));
        support::drive_pending(&runtime);
        release.send(()).expect("release the held reply");
        settle_edit(&runtime, &mut drag).await;
        settle_edit(&runtime, &mut unrelated).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.column_layout.get(1).map(|record| &record.stagger),
            Some(&2.0)
        );
        assert_eq!(
            matrix.column_layout.get(2).map(|record| &record.offset),
            Some(&Vec2 { x: 1.0, y: 0.0 })
        );
    }

    #[wasm_bindgen_test]
    async fn arrow_up_moves_the_column_up_on_screen_on_a_mirror_y_matrix() {
        let mut document = fixture::matrix_document();
        document.matrices[0].mirror = Some(Mirror::Y);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let mut up = nudge_edit(&runtime, ArrowKey::Up, false);
        settle_edit(&runtime, &mut up).await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let key_y = accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == "matrix/matrix/r0c1")
            .expect("the second key")
            .pose
            .at
            .y;
        assert!(
            (key_y - 0.1).abs() < 1e-9,
            "ArrowUp raises the column by 0.1 mm on screen, got {key_y}"
        );
        assert!(
            fixture::accepted_matrix(&runtime).column_layout[0].stagger < 0.0,
            "the stagger value falls because the matrix is mirrored"
        );
        // The pair across the stagger axis is not consumed.
        let scene = &accepted.scene.matrix_scenes[0];
        let basis = SceneBasis::for_parameter(scene, PlacementParameter::Stagger { column: 0 })
            .expect("column 0 has a basis");
        assert_eq!(
            nudge_value(
                &basis,
                PlacementParameter::Stagger { column: 0 },
                PlacementValue::Stagger(0.0),
                ArrowKey::Left,
                false,
            ),
            None
        );
    }

    /// A linked left/right pair on the fixture board, for the mirror-sync
    /// drag tests.
    async fn open_mirrored_pair_runtime() -> Rc<Runtime> {
        let mut document = fixture::matrix_document();
        document.matrices.clear();
        document.parts.clear();
        document.boards[0].part_ids.clear();
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let operation: EditOperation = serde_json::from_value(serde_json::json!({
            "kind": "create-mirrored-pair",
            "matrix": {
                "id": "left-matrix", "rows": 1, "columns": 3,
                "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": -20.0, "y": 0.0},
                "mirror": "x", "definitionId": "switch:base",
                "boardId": fixture::BOARD_ID, "partIds": []
            },
            "left": {"id": "left-layout", "name": "Left", "boardId": fixture::BOARD_ID,
                     "matrixIds": ["left-matrix"], "partIds": []},
            "right": {"id": "right-layout", "name": "Right", "boardId": fixture::BOARD_ID,
                      "matrixIds": ["right-matrix"], "partIds": [],
                      "mirrorLink": {"sourceId": "left-layout", "axisX": 10.0}}
        }))
        .expect("a mirrored pair operation");
        let mut pair = PendingEdits::default();
        pair.begin(
            &runtime,
            (),
            "pair",
            Some("pair".into()),
            EditResolver::new("pair", move |_: &AcceptedSnapshot| {
                Resolution::submit(vec![], operation.clone())
            }),
        );
        settle_edit(&runtime, &mut pair).await;
        runtime
    }

    #[wasm_bindgen_test]
    async fn a_drag_on_the_right_half_of_a_linked_pair_updates_both_halves_equally() {
        let runtime = open_mirrored_pair_runtime().await;
        let mut drag = begin_drag(&runtime, "right-matrix", 1, false);
        let last = preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 2.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let (left, right) = (
            matrix_named(&runtime, "left-matrix"),
            matrix_named(&runtime, "right-matrix"),
        );
        assert_eq!(
            right.column_layout.get(1).map(|record| &record.stagger),
            Some(&2.0)
        );
        assert_eq!(
            left.column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>(),
            right
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            left.column_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>(),
            right
                .column_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>()
        );
    }

    /// Ticket 05: a row Shift drag carries the row through the shift axes
    /// and a row Bend drag turns its running frame; a Bend nudge adds degrees
    /// like the Splay nudge. Only the dragged row changes.
    #[wasm_bindgen_test]
    async fn a_row_shift_drag_and_a_row_bend_drag_change_only_that_row() {
        let mut document = fixture::matrix_document();
        document.matrices[0].rows = 2;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let shift = PlacementParameter::RowShift { row: 1 };
        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            TransformGesture::Offset { parameter: shift },
        );
        // The resolver-level preview carries no grid step, so the raw world
        // delta lands; the mounted pointer path covers snapping.
        let last = preview(&runtime, &mut drag, Vec2 { x: 8.0, y: 4.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.shift),
            Some(&Vec2 { x: 8.0, y: 4.0 })
        );
        assert_eq!(
            matrix
                .row_layout
                .first()
                .map(|record| &record.shift)
                .copied()
                .unwrap_or_default(),
            Vec2::default()
        );

        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            TransformGesture::Bend {
                parameter: PlacementParameter::RowBend { row: 1 },
            },
        );
        // Ten millimetres right turns the running frame five degrees.
        let last = preview(&runtime, &mut drag, Vec2 { x: 10.0, y: 0.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.bend),
            Some(&-5.0)
        );
        assert_eq!(
            matrix
                .row_layout
                .first()
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
            0.0
        );

        // A Bend nudge adds degrees without touching the shift.
        let mut nudge = nudge_with(
            &runtime,
            TransformNudge::Bend {
                parameter: PlacementParameter::RowBend { row: 1 },
                degrees: 1.0,
            },
        );
        settle_edit(&runtime, &mut nudge).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.bend),
            Some(&-4.0)
        );
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.shift),
            Some(&Vec2 { x: 8.0, y: 4.0 })
        );
    }

    /// Ticket 05: a row Shift drag on the right half mirrors Core's sync
    /// rules onto the left half, like the stagger drag above.
    #[wasm_bindgen_test]
    async fn a_row_shift_drag_on_the_right_half_mirrors_the_left_half() {
        let runtime = open_mirrored_pair_runtime().await;
        let mut drag = begin_gesture(
            &runtime,
            "right-matrix",
            TransformGesture::Offset {
                parameter: PlacementParameter::RowShift { row: 0 },
            },
        );
        let last = preview(&runtime, &mut drag, Vec2 { x: 4.7625, y: 0.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let (left, right) = (
            matrix_named(&runtime, "left-matrix"),
            matrix_named(&runtime, "right-matrix"),
        );
        assert_eq!(
            right.row_layout.first().map(|record| &record.shift),
            Some(&Vec2 { x: 4.7625, y: 0.0 })
        );
        assert_eq!(
            left.row_layout
                .iter()
                .map(|record| &record.shift)
                .collect::<Vec<_>>(),
            right
                .row_layout
                .iter()
                .map(|record| &record.shift)
                .collect::<Vec<_>>(),
            "Core mirrors the carried shift onto the linked half"
        );
    }

    /// Ticket 05: a key Shift drag moves only that key's carried shift.
    #[wasm_bindgen_test]
    async fn a_key_shift_drag_moves_only_that_keys_shift() {
        let runtime = fixture::open_matrix_runtime().await;
        let parameter = PlacementParameter::KeyShift { row: 0, column: 1 };
        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            TransformGesture::Offset { parameter },
        );
        let last = preview(&runtime, &mut drag, Vec2 { x: 8.0, y: 4.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.shift),
            Some(Vec2 { x: 8.0, y: 4.0 })
        );
        assert!(
            matrix
                .cells
                .iter()
                .filter(|cell| !(cell.row == 0 && cell.column == 1))
                .all(|cell| cell.shift.is_none()),
            "only the dragged key carries a shift"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tool_tests {
    use super::*;
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::Scope;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn context_of(kind: &str) -> TreeContext {
        let matrix_id = "matrix".to_owned();
        match kind {
            "row" => TreeContext::Row { matrix_id, row: 1 },
            "column" => TreeContext::Column {
                matrix_id,
                column: 2,
            },
            "key" => TreeContext::Key {
                matrix_id,
                row: 1,
                column: 2,
            },
            _ => TreeContext::Matrix { matrix_id },
        }
    }

    #[wasm_bindgen_test]
    fn activation_changes_the_selection_kind_per_tool() {
        use LayoutSelectionKind::{Column, Key, Row};
        use LayoutTransformTool::{
            Anchor, Bend, Move, Offset, Origin, Rotate, Shift, Splay, Stagger,
        };
        for (tool, selection, expected) in [
            // Offset keeps a Row and turns the rest into their Column.
            (Offset, "row", Row),
            (Offset, "column", Column),
            (Offset, "key", Column),
            (Offset, "matrix", Column),
            // Ticket 05: Shift and Bend keep a Row or a Key and turn the
            // rest into their Column.
            (Shift, "row", Row),
            (Shift, "key", Key),
            (Shift, "column", Column),
            (Shift, "matrix", Column),
            (Bend, "row", Row),
            (Bend, "key", Key),
            (Bend, "column", Column),
            (Bend, "matrix", Column),
            // Stagger is a column parameter, so a Row asks for a Column selection. Which
            // column it resolves to (the retained key's, or column 1) is the selection
            // conversion's rule (`context_for_selection_kind`), not covered here.
            (Stagger, "row", Column),
            (Stagger, "column", Column),
            (Stagger, "key", Column),
            (Splay, "row", Column),
            (Origin, "key", Column),
        ] {
            let activation = activate_transform_tool(None, tool, &context_of(selection));
            assert_eq!(
                activation,
                ToolActivation {
                    active: Some(tool),
                    selection_kind: Some(expected),
                },
                "{tool:?} on a {selection} selection"
            );
        }
        // Move, Rotate and Anchor edit or retarget the selection itself, so
        // every selection stays put: no selection change is requested.
        for tool in [Move, Rotate, Anchor] {
            for selection in ["row", "column", "key", "matrix"] {
                assert_eq!(
                    activate_transform_tool(None, tool, &context_of(selection)),
                    ToolActivation {
                        active: Some(tool),
                        selection_kind: None,
                    },
                    "{tool:?} on a {selection} selection"
                );
            }
        }
        // Choosing the active tool again turns it off and leaves the selection alone.
        assert_eq!(
            activate_transform_tool(
                Some(LayoutTransformTool::Offset),
                LayoutTransformTool::Offset,
                &context_of("row")
            ),
            ToolActivation {
                active: None,
                selection_kind: None
            }
        );
        // Switching tool replaces the active one.
        assert_eq!(
            activate_transform_tool(
                Some(LayoutTransformTool::Stagger),
                LayoutTransformTool::Offset,
                &context_of("row")
            )
            .active,
            Some(LayoutTransformTool::Offset)
        );
    }

    #[derive(Props, Clone, PartialEq)]
    struct MenuHostProps {
        available: bool,
    }

    #[component]
    fn menu_host(props: MenuHostProps) -> Element {
        let open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
        let mut chosen = use_signal(String::new);
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: true,
            pointer_tools_available: props.available,
            active_tool: Some(LayoutTransformTool::Offset),
            matrix_rotate_followed: None,
            on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool: EventHandler::new(move |tool: LayoutTransformTool| {
                chosen.set(format!("{tool:?}"));
            }),
        };
        rsx! {
            LayoutTransformToolbar { mount, open_menu }
            output { id: "chosen-tool", "{chosen()}" }
        }
    }

    async fn mount_menu(available: bool) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(menu_host, MenuHostProps { available }),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[component]
    fn context_menu_host(props: MenuHostProps) -> Element {
        let open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
        let mut chosen = use_signal(String::new);
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: true,
            pointer_tools_available: props.available,
            active_tool: Some(LayoutTransformTool::Offset),
            matrix_rotate_followed: None,
            on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool: EventHandler::new(move |tool: LayoutTransformTool| {
                chosen.set(format!("{tool:?}"));
            }),
        };
        rsx! {
            LayoutContextActionToolbar {
                mount,
                open_menu,
                selection_kind: LayoutSelectionKind::Column,
            }
            output { id: "chosen-tool", "{chosen()}" }
        }
    }

    async fn mount_context_menu(available: bool) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(context_menu_host, MenuHostProps { available }),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[wasm_bindgen_test]
    async fn transform_popover_preserves_escape_and_outside_focus() {
        let root = mount_menu(true).await;
        let document = web_sys::window().unwrap().document().unwrap();
        let trigger = root
            .query_selector("#m1-layout-transform-trigger")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let content = root
            .query_selector("#m1-layout-transform-menu")
            .unwrap()
            .unwrap();
        assert_eq!(content.get_attribute("role").as_deref(), Some("group"));
        let label_id = content.get_attribute("aria-labelledby").unwrap();
        assert_eq!(
            document
                .get_element_by_id(&label_id)
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Transform tools and properties")
        );
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        content
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(document.active_element().unwrap().id(), trigger.id());

        trigger.click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        let outside = document.create_element("button").unwrap();
        outside.set_id("transform-outside-target");
        root.append_child(&outside).unwrap();
        outside
            .dyn_ref::<web_sys::HtmlElement>()
            .unwrap()
            .focus()
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(
            document.active_element().unwrap().id(),
            "transform-outside-target"
        );
        root.remove();
    }

    #[derive(Props, Clone, PartialEq)]
    struct MatrixMenuHostProps {
        followed: bool,
    }

    #[component]
    fn matrix_menu_host(props: MatrixMenuHostProps) -> Element {
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: true,
            pointer_tools_available: true,
            active_tool: None,
            matrix_rotate_followed: Some(props.followed),
            on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool: EventHandler::new(|_: LayoutTransformTool| {}),
        };
        rsx! {
            LayoutContextActionToolbar {
                mount,
                open_menu: Signal::new(Some(LayoutCommandMenu::Transform)),
                selection_kind: LayoutSelectionKind::Matrix,
            }
        }
    }

    /// Ticket 23 slice: the Matrix contextual tools offer Move always, and Rotate
    /// enabled only when the matrix follows rotation; otherwise Rotate stays
    /// visible but disabled with the reason. The Transform popover no longer
    /// duplicates these tools, so the host mounts them directly at their
    /// owning seam.
    #[wasm_bindgen_test]
    async fn the_matrix_menu_disables_rotate_without_follow_rotation() {
        for (followed, rotate_enabled) in [(true, true), (false, false)] {
            let document = web_sys::window().unwrap().document().unwrap();
            let root = document.create_element("div").unwrap();
            document.body().unwrap().append_child(&root).unwrap();
            dioxus_web::launch::launch_virtual_dom(
                VirtualDom::new_with_props(matrix_menu_host, MatrixMenuHostProps { followed }),
                dioxus_web::Config::new().rootnode(root.clone().into()),
            );
            gloo_timers::future::TimeoutFuture::new(40).await;
            let buttons = root.query_selector_all("button").unwrap();
            let labels = (0..buttons.length())
                .filter_map(|index| buttons.item(index))
                .filter_map(|node| node.text_content())
                .collect::<Vec<_>>();
            assert!(
                labels.iter().any(|label| label == "Move"),
                "the Matrix menu offers Move, got {labels:?}"
            );
            let rotate = (0..buttons.length())
                .filter_map(|index| buttons.item(index))
                .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
                .find(|button| button.text_content().as_deref() == Some("Rotate"))
                .expect("the Matrix menu offers Rotate");
            assert_eq!(
                rotate.has_attribute("disabled"),
                !rotate_enabled,
                "Rotate follows the rotation Follow"
            );
            assert_eq!(
                rotate.get_attribute("aria-label").as_deref(),
                Some(if rotate_enabled {
                    "Activate Rotate for the selected matrix"
                } else {
                    "Rotate matrix (unavailable: Follow rotation is off)"
                }),
                "the Rotate entry explains itself"
            );
            root.remove();
        }
    }

    /// The Transform popover no longer duplicates the inline column tools, so the
    /// availability/pressed/dispatch contract is asserted at the owning seam:
    /// the contextual toolbar omits unavailable tools instead of disabling them.
    #[wasm_bindgen_test]
    async fn contextual_column_tools_follow_availability_and_dispatch() {
        let root = mount_context_menu(true).await;
        let buttons = root.query_selector_all("button").unwrap();
        let tools = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .filter(|button| {
                matches!(
                    button.text_content().as_deref(),
                    Some("Position" | "Stagger" | "Spread" | "Offset" | "Splay")
                )
            })
            .collect::<Vec<_>>();
        let labels = tools
            .iter()
            .map(|button| button.text_content().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(labels, ["Position", "Stagger", "Offset", "Spread", "Splay"]);
        for button in &tools {
            assert!(!button.has_attribute("disabled"));
            let pressed = button.get_attribute("aria-pressed");
            let expected = match button.text_content().as_deref() {
                Some("Offset") => Some("true"),
                Some("Position") => None,
                _ => Some("false"),
            };
            assert_eq!(
                pressed.as_deref(),
                expected,
                "pressed follows the active tool; the Position action carries no pressed state"
            );
        }
        tools
            .iter()
            .find(|button| button.text_content().as_deref() == Some("Offset"))
            .expect("the Offset tool stays listed")
            .clone()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#chosen-tool")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Offset")
        );
        root.remove();

        let root = mount_context_menu(false).await;
        let buttons = root.query_selector_all("button").unwrap();
        let labels = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .filter_map(|node| node.text_content())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            ["Position"],
            "unavailable pointer tools are omitted, leaving only the Position action"
        );
        root.remove();
    }

    #[derive(Props, Clone, PartialEq)]
    struct OverlayHostProps {
        runtime: LayoutTransformRuntime,
        svg: LayoutTransformSvg,
        matrix: Matrix,
        projection: MatrixScene,
        owner: LayoutOwnerIdentity,
        scope: Scope,
        tool: LayoutTransformTool,
        context: TreeContext,
        /// Receives the host's visibility signal, so a test can unmount the
        /// overlay like a production scope switch.
        show_slot: Option<Rc<RefCell<Option<Signal<bool>>>>>,
    }

    #[component]
    fn overlay_host(props: OverlayHostProps) -> Element {
        let context = props.context.clone();
        let selected_context = use_signal(|| {
            Some(ScopedTreeContext {
                scope: props.scope.clone(),
                context: context.clone(),
            })
        });
        let scope_generation = use_signal(|| props.owner.generation);
        let workspace = use_signal(|| "Layout");
        let splay_affect = use_signal(|| MatrixSplayAffect::Column);
        let arbiter = use_hook(CanvasInteractionArbiter::default);
        let show = use_signal(|| true);
        if let Some(slot) = &props.show_slot {
            *slot.borrow_mut() = Some(show);
        }
        rsx! {
            if show() {
                svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
                    LayoutTransformToolOverlay {
                        runtime: props.runtime.clone(),
                        svg: props.svg.clone(),
                        arbiter,
                        owner: props.owner.clone(),
                        selected_context,
                        scope_generation,
                        workspace,
                        matrix: props.matrix.clone(),
                        projection: props.projection.clone(),
                        context: props.context.clone(),
                        tool: props.tool,
                        snap_settings: super::super::layout_toolbar::LayoutSnapSettings::default(),
                        snap_origins: Vec::new(),
                        splay_affect,
                        view_x: 0.0,
                        view_y: 0.0,
                        width: 100.0,
                        height: 100.0,
                        on_finish: EventHandler::new(|()| {}),
                    }
                }
            }
        }
    }

    /// Mount the overlay for the runtime's accepted matrix as it is now.
    async fn mount_overlay(
        runtime: &Rc<Runtime>,
        tool: LayoutTransformTool,
    ) -> (web_sys::Element, web_sys::Element) {
        let context = TreeContext::Column {
            matrix_id: fixture::MATRIX_ID.into(),
            column: 1,
        };
        mount_context_overlay(runtime, tool, context).await
    }

    /// Ticket 05: mount one tool overlay for an explicit selection context,
    /// for the row and key rails.
    async fn mount_context_overlay(
        runtime: &Rc<Runtime>,
        tool: LayoutTransformTool,
        context: TreeContext,
    ) -> (web_sys::Element, web_sys::Element) {
        let (root, handle, _) = mount_toggle_overlay(runtime, tool, context).await;
        (root, handle)
    }

    /// Mount a tool overlay and return the host's visibility toggle, so a test
    /// can unmount the overlay exactly like a production scope switch while a
    /// commit-toast timer is still pending.
    async fn mount_toggle_overlay(
        runtime: &Rc<Runtime>,
        tool: LayoutTransformTool,
        context: TreeContext,
    ) -> (web_sys::Element, web_sys::Element, Signal<bool>) {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let matrix = fixture::accepted_matrix(runtime);
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
            .cloned()
            .expect("the matrix scene");
        let owner = LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        };
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let show_slot: Rc<RefCell<Option<Signal<bool>>>> = Rc::new(RefCell::new(None));
        let props = OverlayHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            matrix,
            projection,
            scope: owner.scope.clone().expect("a scope"),
            owner,
            tool,
            context,
            show_slot: Some(show_slot.clone()),
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(overlay_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#overlay-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        let handle = root
            .query_selector(".m1-transform-handle")
            .unwrap()
            .expect("the tool draws a handle");
        // A synthetic pointer id is not an active pointer, so the browser refuses real capture.
        // The handle's capture methods are stubbed on this element; everything else is the
        // production pointer path.
        let stub = js_sys::Function::new_no_args("return true;");
        for method in [
            "setPointerCapture",
            "hasPointerCapture",
            "releasePointerCapture",
        ] {
            js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
        }
        let show = show_slot
            .borrow()
            .expect("the overlay host shares its visibility signal");
        (root, handle, show)
    }

    /// Mount the overlay for a named matrix with a Matrix selection, for the
    /// matrix Move and Rotate tools.
    async fn mount_matrix_overlay(
        runtime: &Rc<Runtime>,
        tool: LayoutTransformTool,
        matrix_id: &str,
    ) -> (web_sys::Element, web_sys::Element) {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .cloned()
            .expect("the named matrix");
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
            .cloned()
            .expect("the matrix scene");
        let owner = LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        };
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let context = TreeContext::Matrix {
            matrix_id: matrix.id.clone(),
        };
        let props = OverlayHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            matrix,
            projection,
            scope: owner.scope.clone().expect("a scope"),
            owner,
            tool,
            context,
            show_slot: None,
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(overlay_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#overlay-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        // Every handle needs the capture stubs: synthetic pointer events are
        // never real active pointers, so the unstubbed capture throws and the
        // drag refuses to start.
        let handles = root.query_selector_all(".m1-transform-handle").unwrap();
        let stub = js_sys::Function::new_no_args("return true;");
        for index in 0..handles.length() {
            let handle = handles.item(index).expect("the stubbed handle");
            for method in [
                "setPointerCapture",
                "hasPointerCapture",
                "releasePointerCapture",
            ] {
                js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
            }
        }
        let handle = root
            .query_selector(".m1-transform-handle")
            .unwrap()
            .expect("the tool draws a handle");
        (root, handle)
    }

    /// Ticket 07: one part rail token, mounted like the production mount
    /// site mounts it: a Component context for the part, the anchor matrix
    /// pitch for drag snap, and a counter for the half-focus callback.
    #[derive(Props, Clone, PartialEq)]
    struct PartOverlayHostProps {
        runtime: LayoutTransformRuntime,
        svg: LayoutTransformSvg,
        owner: LayoutOwnerIdentity,
        scope: Scope,
        part_id: String,
        tool: LayoutTransformTool,
        snap: GestureSnapInputs,
        focus_half: Rc<RefCell<u32>>,
    }

    #[component]
    fn part_overlay_host(props: PartOverlayHostProps) -> Element {
        let context = TreeContext::Component {
            part_id: Some(props.part_id.clone()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        let selected_context = use_signal(|| {
            Some(ScopedTreeContext {
                scope: props.scope.clone(),
                context,
            })
        });
        let scope_generation = use_signal(|| props.owner.generation);
        let workspace = use_signal(|| "Layout");
        let surface_size = use_signal(|| (800.0, 600.0));
        let arbiter = use_hook(CanvasInteractionArbiter::default);
        let focus_half = props.focus_half.clone();
        rsx! {
            svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
                LayoutPartToolOverlay {
                    runtime: props.runtime.clone(),
                    svg: props.svg.clone(),
                    arbiter,
                    owner: props.owner.clone(),
                    selected_context,
                    scope_generation,
                    workspace,
                    surface_size: Some(surface_size),
                    part_id: props.part_id.clone(),
                    tool: props.tool,
                    snap: props.snap,
                    view_x: 0.0,
                    view_y: 0.0,
                    width: 100.0,
                    height: 100.0,
                    on_finish: EventHandler::new(|()| {}),
                    on_focus_half: EventHandler::new(move |_| {
                        *focus_half.borrow_mut() += 1;
                    }),
                }
            }
        }
    }

    /// Ticket 07: the rail host mounts one overlay per tool into a shared
    /// svg, like the production rail, so the drawn tokens form one rail.
    #[derive(Props, Clone, PartialEq)]
    struct PartRailHostProps {
        runtime: LayoutTransformRuntime,
        svg: LayoutTransformSvg,
        owner: LayoutOwnerIdentity,
        scope: Scope,
        part_id: String,
        tools: Vec<LayoutTransformTool>,
        snap: GestureSnapInputs,
    }

    #[component]
    fn part_rail_host(props: PartRailHostProps) -> Element {
        let context = TreeContext::Component {
            part_id: Some(props.part_id.clone()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        let selected_context = use_signal(|| {
            Some(ScopedTreeContext {
                scope: props.scope.clone(),
                context,
            })
        });
        let scope_generation = use_signal(|| props.owner.generation);
        let workspace = use_signal(|| "Layout");
        let surface_size = use_signal(|| (800.0, 600.0));
        let arbiter = use_hook(CanvasInteractionArbiter::default);
        rsx! {
            svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
                for tool in props.tools.clone() {
                    LayoutPartToolOverlay {
                        key: "{tool:?}",
                        runtime: props.runtime.clone(),
                        svg: props.svg.clone(),
                        arbiter: arbiter.clone(),
                        owner: props.owner.clone(),
                        selected_context,
                        scope_generation,
                        workspace,
                        surface_size: Some(surface_size),
                        part_id: props.part_id.clone(),
                        tool,
                        snap: props.snap,
                        view_x: 0.0,
                        view_y: 0.0,
                        width: 100.0,
                        height: 100.0,
                        on_finish: EventHandler::new(|()| {}),
                        on_focus_half: EventHandler::new(|()| {}),
                    }
                }
            }
        }
    }

    fn part_owner(runtime: &Rc<Runtime>) -> LayoutOwnerIdentity {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        }
    }

    fn part_snap() -> GestureSnapInputs {
        super::super::layout_toolbar::gesture_snap_inputs(
            &super::super::layout_toolbar::LayoutSnapSettings::default(),
            Some(Vec2 { x: 19.05, y: 19.05 }),
            None,
        )
    }

    /// Mount one part tool overlay and stub pointer capture on its token, if
    /// it draws one. Locked parts, anchors without the tool, and matrix
    /// members draw nothing: those mounts return `None` for the handle.
    async fn mount_part_overlay(
        runtime: &Rc<Runtime>,
        part_id: &str,
        tool: LayoutTransformTool,
    ) -> (web_sys::Element, Option<web_sys::Element>, Rc<RefCell<u32>>) {
        let owner = part_owner(runtime);
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let focus_half = Rc::new(RefCell::new(0_u32));
        let props = PartOverlayHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            owner: owner.clone(),
            scope: owner.scope.clone().expect("a scope"),
            part_id: part_id.to_owned(),
            tool,
            snap: part_snap(),
            focus_half: focus_half.clone(),
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(part_overlay_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#overlay-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        stub_part_capture(&root);
        let handle = root.query_selector(".m1-transform-handle").unwrap();
        (root, handle, focus_half)
    }

    /// Mount one part tool overlay that must draw a token, for the drag and
    /// hover tests.
    async fn mount_part_overlay_expect(
        runtime: &Rc<Runtime>,
        part_id: &str,
        tool: LayoutTransformTool,
    ) -> (web_sys::Element, web_sys::Element, Rc<RefCell<u32>>) {
        let (root, handle, focus_half) = mount_part_overlay(runtime, part_id, tool).await;
        let handle = handle.expect("the tool draws a handle");
        (root, handle, focus_half)
    }

    fn stub_part_capture(root: &web_sys::Element) {
        let handles = root.query_selector_all(".m1-transform-handle").unwrap();
        let stub = js_sys::Function::new_no_args("return true;");
        for index in 0..handles.length() {
            let handle = handles.item(index).expect("the stubbed handle");
            for method in [
                "setPointerCapture",
                "hasPointerCapture",
                "releasePointerCapture",
            ] {
                js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
            }
        }
    }

    /// Mount a whole part rail (one overlay per tool) into one svg.
    async fn mount_part_rail(
        runtime: &Rc<Runtime>,
        part_id: &str,
        tools: Vec<LayoutTransformTool>,
    ) -> web_sys::Element {
        let owner = part_owner(runtime);
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let props = PartRailHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            owner: owner.clone(),
            scope: owner.scope.clone().expect("a scope"),
            part_id: part_id.to_owned(),
            tools,
            snap: part_snap(),
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(part_rail_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#overlay-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        stub_part_capture(&root);
        root
    }

    fn part_anchor_of(runtime: &Rc<Runtime>, part_id: &str) -> boardstudio_core::model::PartAnchor {
        runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .document
            .parts
            .iter()
            .find(|part| part.id == part_id)
            .expect("the part exists")
            .anchor
            .clone()
            .expect("the part is anchored")
    }

    /// Dispatch a script-created pointer event. The returned event is never
    /// trusted: mounted drag coverage proves wiring and math through this
    /// seam, while real trusted-pointer proof lives in the packaged-page
    /// mouse smoke.
    fn dispatch_pointer(
        target: &web_sys::Element,
        kind: &str,
        shift: bool,
        alt: bool,
        buttons: u16,
        client: (i32, i32),
    ) -> web_sys::PointerEvent {
        let init = js_sys::Object::new();
        for (name, value) in [
            ("bubbles", wasm_bindgen::JsValue::TRUE),
            ("cancelable", wasm_bindgen::JsValue::TRUE),
            ("pointerId", 17.into()),
            ("pointerType", "mouse".into()),
            ("isPrimary", wasm_bindgen::JsValue::TRUE),
            ("button", 0.into()),
            ("buttons", buttons.into()),
            ("ctrlKey", shift.into()),
            ("altKey", alt.into()),
            ("clientX", client.0.into()),
            ("clientY", client.1.into()),
        ] {
            js_sys::Reflect::set(init.as_ref(), &name.into(), &value).unwrap();
        }
        let constructor = js_sys::Reflect::get(&js_sys::global(), &"PointerEvent".into())
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap();
        let arguments = js_sys::Array::new();
        arguments.push(&kind.into());
        arguments.push(init.as_ref());
        let event = js_sys::Reflect::construct(&constructor, &arguments)
            .unwrap()
            .dyn_into::<web_sys::PointerEvent>()
            .unwrap();
        target.dispatch_event(&event).unwrap();
        event
    }

    /// Ticket 23: the mounted drag harness dispatches script-created pointer
    /// events, which the platform never trusts. This pins the coverage split:
    /// mounted tests prove drag wiring and math, and the packaged-page mouse
    /// smoke (`scripts/test-drag-interactions.cjs`) proves the same gestures
    /// with real trusted pointer events.
    #[wasm_bindgen_test]
    async fn mounted_pointer_dispatch_is_synthetic_never_trusted() {
        let document = web_sys::window().unwrap().document().unwrap();
        let target = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&target).unwrap();
        for kind in ["pointerdown", "pointermove", "pointerup"] {
            let event = dispatch_pointer(&target, kind, false, false, 1, (100, 100));
            assert!(
                !event.is_trusted(),
                "{kind} dispatched from the harness is synthetic, never trusted"
            );
        }
        target.remove();
    }

    async fn wait_until_editable(runtime: &Rc<Runtime>) {
        for _ in 0..200 {
            support::run_pending(runtime).await;
            let model = runtime.model();
            if model.lifecycle == Lifecycle::Ready
                && model.accepted.as_ref().is_some_and(|accepted| {
                    model.durability
                        == (Durability::Saved {
                            revision: accepted.document.revision,
                        })
                })
            {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("the runtime did not become editable");
    }

    /// Ticket 03: two overlays sharing one svg, so the rail registry sees the
    /// whole peer set. The host also provides the transient amber preview
    /// contexts the Editor owns in production.
    #[derive(Clone, PartialEq)]
    struct RailProbe {
        preview: Rc<RefCell<Option<Signal<Option<FieldPreviewState>>>>>,
    }

    #[derive(Props, Clone, PartialEq)]
    struct RailPairProps {
        runtime: LayoutTransformRuntime,
        svg: LayoutTransformSvg,
        matrix: Matrix,
        projection: MatrixScene,
        owner: LayoutOwnerIdentity,
        scope: Scope,
        context: TreeContext,
        tools: Vec<LayoutTransformTool>,
        probe: RailProbe,
    }

    #[component]
    fn rail_pair_host(props: RailPairProps) -> Element {
        let preview_state = use_signal(|| None::<FieldPreviewState>);
        *props.probe.preview.borrow_mut() = Some(preview_state);
        use_context_provider(|| FieldPreview(preview_state));
        let model = props.runtime.0.model();
        let accepted = model.accepted.clone().expect("an accepted document");
        use_context_provider(|| {
            Some(FieldPreviewSource {
                document: accepted.document.clone(),
                scene: accepted.scene.clone(),
            })
        });
        let splay_affect = use_signal(|| MatrixSplayAffect::Column);
        let arbiter = use_hook(CanvasInteractionArbiter::default);
        let selected_context = use_signal(|| {
            Some(ScopedTreeContext {
                scope: props.scope.clone(),
                context: props.context.clone(),
            })
        });
        let scope_generation = use_signal(|| props.owner.generation);
        let workspace = use_signal(|| "Layout");
        rsx! {
            svg { id: "rail-pair-svg", view_box: "0 0 100 100", width: "400", height: "400",
                for tool in props.tools.clone() {
                    LayoutTransformToolOverlay {
                        key: "{tool:?}",
                        runtime: props.runtime.clone(),
                        svg: props.svg.clone(),
                        arbiter: arbiter.clone(),
                        owner: props.owner.clone(),
                        selected_context,
                        scope_generation,
                        workspace,
                        matrix: props.matrix.clone(),
                        projection: props.projection.clone(),
                        context: props.context.clone(),
                        tool,
                        snap_settings: super::super::layout_toolbar::LayoutSnapSettings::default(),
                        snap_origins: Vec::new(),
                        splay_affect,
                        view_x: 0.0,
                        view_y: 0.0,
                        width: 100.0,
                        height: 100.0,
                        on_finish: EventHandler::new(|_| {}),
                    }
                }
            }
        }
    }

    /// Mount the rail's peer overlays in one svg, sharing one svg handle so
    /// the registry lays out the real peer set. Returns the root and the
    /// preview probe for the amber assertions.
    async fn mount_rail_pair(
        runtime: &Rc<Runtime>,
        tools: Vec<LayoutTransformTool>,
    ) -> (web_sys::Element, RailProbe) {
        let context = TreeContext::Column {
            matrix_id: fixture::MATRIX_ID.into(),
            column: 1,
        };
        mount_context_rail_pair(runtime, tools, context).await
    }

    /// Ticket 05: mount a rail's peer overlays for an explicit selection
    /// context, for the row and key hover-preview assertions.
    async fn mount_context_rail_pair(
        runtime: &Rc<Runtime>,
        tools: Vec<LayoutTransformTool>,
        context: TreeContext,
    ) -> (web_sys::Element, RailProbe) {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let matrix = fixture::accepted_matrix(runtime);
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
            .cloned()
            .expect("the matrix scene");
        let owner = LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        };
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let probe = RailProbe {
            preview: Rc::new(RefCell::new(None)),
        };
        let props = RailPairProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            matrix,
            projection,
            scope: owner.scope.clone().expect("a scope"),
            owner,
            context,
            tools,
            probe: probe.clone(),
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(rail_pair_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#rail-pair-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        // Every token needs the capture stubs: synthetic pointer events are
        // never real active pointers, so the unstubbed capture throws and the
        // drag refuses to start.
        let handles = root.query_selector_all(".m1-transform-handle").unwrap();
        let stub = js_sys::Function::new_no_args("return true;");
        for index in 0..handles.length() {
            let handle = handles.item(index).expect("the stubbed handle");
            for method in [
                "setPointerCapture",
                "hasPointerCapture",
                "releasePointerCapture",
            ] {
                js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
            }
        }
        (root, probe)
    }

    fn token_rect_style(root: &web_sys::Element, label: &str) -> String {
        let token = root
            .query_selector(&format!("[aria-label='{label}']"))
            .unwrap()
            .unwrap_or_else(|| panic!("the {label} token draws"));
        let rect = token
            .query_selector("rect")
            .unwrap()
            .expect("the token draws its box");
        rect.get_attribute("style").unwrap_or_default()
    }

    fn token_opacity(root: &web_sys::Element, label: &str) -> String {
        let token = root
            .query_selector(&format!("[aria-label='{label}']"))
            .unwrap()
            .unwrap_or_else(|| panic!("the {label} token draws"));
        token
            .get_attribute("style")
            .unwrap_or_default()
            .split(';')
            .find_map(|part| {
                let (name, value) = part.split_once(':')?;
                (name.trim() == "opacity").then(|| value.trim().to_owned())
            })
            .unwrap_or_else(|| "1".to_owned())
    }

    /// Ticket 03: the rail lays its tokens out side by side above the
    /// selection at screen size; hover previews the movers in amber with a
    /// selection-surface token and leaves nothing behind; while one token
    /// drags, the token stays accent and its peers dim to 35%.
    #[wasm_bindgen_test]
    async fn rail_tokens_share_one_screen_size_row_with_hover_and_dimming() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let before = fixture_revision(&runtime);
        let (root, probe) = mount_rail_pair(
            &runtime,
            vec![LayoutTransformTool::Stagger, LayoutTransformTool::Splay],
        )
        .await;
        // The rail draws one backdrop for two 18 px tokens: 2·18 + 2 + 2·2.
        // Peers converge on the post-mount pass, so poll briefly.
        let mut backdrop_width = None;
        for _ in 0..40 {
            backdrop_width = root
                .query_selector("g[aria-hidden] rect")
                .unwrap()
                .and_then(|backdrop| backdrop.get_attribute("width"));
            if backdrop_width.as_deref() == Some("42") {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(50).await;
        }
        assert_eq!(
            backdrop_width.as_deref(),
            Some("42"),
            "two tokens share one 42 px rail"
        );
        // Both tokens sit on one row through the rail centre, symmetric about
        // it, in the peers' canonical order. World offsets scale with the
        // harness fallback until the first hover re-render, so these assert
        // the scale-free invariants; absolute px geometry lives in the
        // native `handle_rail` tests.
        let stagger = root
            .query_selector("[aria-label='Drag to stagger']")
            .unwrap()
            .expect("the stagger token draws");
        let splay = root
            .query_selector("[aria-label='Drag to splay']")
            .unwrap()
            .expect("the splay token draws");
        let offset = |element: &web_sys::Element| {
            let transform = element.get_attribute("transform").unwrap_or_default();
            let inner = transform
                .strip_prefix("translate(")
                .unwrap_or_default()
                .split([' ', ')'])
                .collect::<Vec<_>>();
            (
                inner
                    .first()
                    .and_then(|x| x.parse::<f64>().ok())
                    .unwrap_or(f64::NAN),
                inner
                    .get(1)
                    .and_then(|y| y.parse::<f64>().ok())
                    .unwrap_or(f64::NAN),
            )
        };
        let (stagger_x, stagger_y) = offset(&stagger);
        let (splay_x, splay_y) = offset(&splay);
        let backdrop_group = root
            .query_selector("g[aria-hidden]")
            .unwrap()
            .expect("the rail backdrop group draws");
        let (rail_cx, rail_cy) = offset(&backdrop_group);
        assert!(
            (stagger_y - rail_cy).abs() < 1e-9 && (splay_y - rail_cy).abs() < 1e-9,
            "the tokens share the rail row, got {stagger_y}, {splay_y} and {rail_cy}"
        );
        assert!(
            ((stagger_x + splay_x) / 2.0 - rail_cx).abs() < 1e-9,
            "the tokens centre on the rail, got {stagger_x}, {splay_x} and {rail_cx}"
        );
        assert!(
            splay_x > stagger_x,
            "the splay token follows the stagger token in canonical order"
        );
        // Token chrome is px art: an 18 px box plus a 20 by 24 hit area.
        for label in ["Drag to stagger", "Drag to splay"] {
            let token = root
                .query_selector(&format!("[aria-label='{label}']"))
                .unwrap()
                .unwrap_or_else(|| panic!("the {label} token draws"));
            let boxes = token.query_selector_all("rect").unwrap();
            assert_eq!(
                boxes.length(),
                2,
                "the {label} token draws box and hit area"
            );
        }
        // Hover previews the movers in amber on a selection-surface token
        // and leaves with no document revision.
        dispatch_pointer(&stagger, "pointerenter", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        assert!(
            token_rect_style(&root, "Drag to stagger").contains("var(--wb-selection-surface)"),
            "hover fills the token with the selection surface"
        );
        let preview = probe
            .preview
            .borrow()
            .as_ref()
            .and_then(|signal| signal.read().clone());
        assert!(
            preview
                .as_ref()
                .is_some_and(|state| !state.parts.is_empty()),
            "hover previews the keys the drag would move, got {preview:?}"
        );
        dispatch_pointer(&stagger, "pointerleave", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        assert!(
            probe
                .preview
                .borrow()
                .as_ref()
                .is_none_or(|signal| signal.read().is_none()),
            "leave clears the amber preview"
        );
        assert!(
            !token_rect_style(&root, "Drag to stagger").contains("var(--wb-selection-surface)"),
            "leave restores the muted token"
        );
        assert_eq!(
            fixture_revision(&runtime),
            before,
            "hover and leave write nothing"
        );
        // While the stagger token drags, it stays accent and the splay peer
        // dims to 35%: the drag re-render marks the token, and the peer's
        // own hover pass reads the shared mount set.
        dispatch_pointer(&stagger, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&stagger, "pointermove", false, false, 1, (140, 100));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the token drag previews before commit"
        );
        assert!(
            token_rect_style(&root, "Drag to stagger").contains("var(--wb-accent)"),
            "the active token keeps the accent fill"
        );
        dispatch_pointer(&splay, "pointerenter", false, false, 0, (140, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&splay, "pointerleave", false, false, 0, (140, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            token_opacity(&root, "Drag to splay"),
            "0.35",
            "the peer dims while the stagger token drags"
        );
        assert_eq!(
            token_opacity(&root, "Drag to stagger"),
            "1",
            "the active token keeps full opacity"
        );
        dispatch_pointer(&stagger, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > before {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        // The peer undims once the drag lands.
        dispatch_pointer(&splay, "pointerenter", false, false, 0, (140, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&splay, "pointerleave", false, false, 0, (140, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            token_opacity(&root, "Drag to splay"),
            "1",
            "the peer undims after the drag lands"
        );
        root.remove();
    }

    /// One synthetic pointer drag on the handle: down, move, up. The dispatched
    /// events are script-created, so `isTrusted` is always false here; real
    /// trusted-pointer proof for the drags lives in the packaged-page mouse
    /// smoke (`scripts/test-drag-interactions.cjs`, `check.py drag-smoke`).
    /// Returns the handle's `data-quick-offset` after the down and after the
    /// move.
    async fn drag_handle(
        runtime: &Rc<Runtime>,
        handle: &web_sys::Element,
        shift_down: bool,
        to: (i32, i32),
    ) -> (Option<String>, Option<String>) {
        let revision = fixture_revision(runtime);
        dispatch_pointer(handle, "pointerdown", shift_down, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        let after_down = handle.get_attribute("data-quick-offset");
        // Shift is released before the first move; the latch must hold.
        dispatch_pointer(handle, "pointermove", false, false, 1, to);
        gloo_timers::future::TimeoutFuture::new(40).await;
        let after_move = handle.get_attribute("data-quick-offset");
        dispatch_pointer(handle, "pointerup", false, false, 0, to);
        for _ in 0..200 {
            support::run_pending(runtime).await;
            if fixture_revision(runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        (after_down, after_move)
    }

    fn fixture_revision(runtime: &Runtime) -> u64 {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision
    }

    #[wasm_bindgen_test]
    async fn column_angle_pointer_preview_starts_at_rotation_and_preserves_mirror_direction() {
        for (mirror, expected) in [
            (None, 100.0),
            (Some(boardstudio_core::model::Mirror::X), -80.0),
        ] {
            let mut document = fixture::matrix_document();
            let matrix = &mut document.matrices[0];
            matrix.mirror = mirror;
            matrix.column_layout.resize_with(2, Default::default);
            matrix.column_layout[1].splay = 7.0;
            matrix.column_layout[1].rotation = 10.0;
            let runtime = support::new_runtime();
            support::open_document(&runtime, document).await;
            let before = fixture_revision(&runtime);
            let (root, _) = mount_overlay(&runtime, LayoutTransformTool::Splay).await;
            let handle = root
                .query_selector("[aria-label='Drag to splay']")
                .unwrap()
                .unwrap();
            assert_eq!(
                handle
                    .query_selector("title")
                    .unwrap()
                    .unwrap()
                    .text_content()
                    .as_deref(),
                Some("Drag to splay · 10°")
            );
            let stub = js_sys::Function::new_no_args("return true;");
            for method in [
                "setPointerCapture",
                "hasPointerCapture",
                "releasePointerCapture",
            ] {
                js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
            }
            let accepted = runtime.model().accepted.unwrap();
            let origin = accepted.scene.matrix_scenes[0].columns[1].splay_origin;
            let rect = root
                .query_selector("svg")
                .unwrap()
                .unwrap()
                .get_bounding_client_rect();
            let client = |x: f64, y: f64| {
                (
                    (rect.left() + x * 4.0).round() as i32,
                    (rect.top() - y * 4.0).round() as i32,
                )
            };
            let start = client(origin.x + 20.0, origin.y);
            let end = client(origin.x, origin.y + 20.0);
            dispatch_pointer(&handle, "pointerdown", false, false, 1, start);
            dispatch_pointer(&handle, "pointermove", false, false, 1, start);
            support::run_pending(&runtime).await;
            assert_eq!(
                fixture_revision(&runtime),
                before,
                "initial preview never commits"
            );
            assert_eq!(
                runtime
                    .model()
                    .display_preview
                    .as_ref()
                    .unwrap()
                    .matrix_scenes[0]
                    .columns[1]
                    .rotation,
                10.0,
                "zero pointer delta previews the stored Column rotation"
            );
            dispatch_pointer(&handle, "pointermove", false, false, 1, end);
            support::run_pending(&runtime).await;
            assert_eq!(
                runtime
                    .model()
                    .display_preview
                    .as_ref()
                    .unwrap()
                    .matrix_scenes[0]
                    .columns[1]
                    .rotation,
                expected,
                "the pointer preview uses the selected angle and mirror direction"
            );
            assert_eq!(
                fixture::accepted_matrix(&runtime).column_layout[1].rotation,
                10.0
            );
            dispatch_pointer(&handle, "pointerup", false, false, 0, end);
            wait_until_editable(&runtime).await;
            let matrix = fixture::accepted_matrix(&runtime);
            assert_eq!(matrix.column_layout[1].rotation, expected);
            assert_eq!(matrix.column_layout[1].splay, 7.0);
            fixture::undo(&runtime).await;
            assert_eq!(
                fixture::accepted_matrix(&runtime).column_layout[1].rotation,
                10.0
            );
            root.remove();
        }
    }

    #[wasm_bindgen_test]
    async fn real_pointer_events_with_stubbed_capture_latch_ctrl_at_pointer_down_and_show_the_offset_glyph()
     {
        let runtime = fixture::open_matrix_runtime().await;
        // The first edit syncs the fixture's unlaid poses.
        let mut sync = PendingEdits::<()>::default();
        sync.begin(
            &runtime,
            (),
            "sync",
            Some("transform".into()),
            EditResolver::new("sync", |_: &AcceptedSnapshot| {
                transform_commit(
                    String::new(),
                    fixture::MATRIX_ID,
                    placement_operation(
                        fixture::MATRIX_ID,
                        PlacementParameter::ColumnOffset { column: 2 },
                        PlacementValue::Offset(Vec2 { x: 0.5, y: 0.0 }),
                    ),
                )
            }),
        );
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if !sync.settle(true).is_empty() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        wait_until_editable(&runtime).await;

        // Shift at pointer-down: a free 2-D column offset with the offset glyph, though Shift
        // is released before the first move. The grid step is 4.7625 mm; 4 px is 1 mm.
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        assert_eq!(
            handle.get_attribute("data-quick-offset").as_deref(),
            Some("false")
        );
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to stagger")
        );
        let (down, moved) = drag_handle(&runtime, &handle, true, (140, 80)).await;
        assert_eq!(
            down.as_deref(),
            Some("true"),
            "Shift at pointer-down shows the glyph"
        );
        assert_eq!(
            moved.as_deref(),
            Some("true"),
            "releasing Shift does not switch it"
        );
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.column_layout.get(1).map(|record| &record.offset),
            Some(&Vec2 {
                x: 9.525,
                y: 4.7625
            })
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0)
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        assert_eq!(
            handle.get_attribute("data-quick-offset").as_deref(),
            Some("false"),
            "the glyph returns when the drag ends"
        );
        root.remove();
        wait_until_editable(&runtime).await;

        // No Shift: the same gesture is a one-dimensional stagger.
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let (down, moved) = drag_handle(&runtime, &handle, false, (140, 60)).await;
        assert_eq!(
            (down.as_deref(), moved.as_deref()),
            (Some("false"), Some("false"))
        );
        let matrix = fixture::accepted_matrix(&runtime);
        assert!((matrix.column_layout[1].stagger - 9.525).abs() < 1e-9);
        assert_eq!(
            matrix.column_layout.get(1).map(|record| &record.offset),
            Some(&Vec2 {
                x: 9.525,
                y: 4.7625
            }),
            "the offset is untouched"
        );
        root.remove();
    }

    /// Ticket 23, RED: the Spread tool draws its own horizontal handle that
    /// commits an explicit Spread through the real Runtime/Core seam.
    /// A 40 px drag is 10 world mm; the 4.7625 mm grid lands six steps out,
    /// so column 1 steps 28.575 mm and column 2 follows.
    #[wasm_bindgen_test]
    async fn spread_handle_drag_commits_an_explicit_spread_with_one_undo() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Spread).await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to spread"),
            "the Spread tool draws its own horizontal handle"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag previews actual later-column movement before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let matrix = fixture::accepted_matrix(&runtime);
        assert!(
            matrix
                .column_layout
                .get(1)
                .and_then(|record| record.spread)
                .is_some_and(|spread| (spread - 28.575).abs() < 1e-9),
            "the drag commits an explicit Spread"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let scene = runtime.model().accepted.unwrap().scene.clone();
        let at = |column| {
            scene.matrix_scenes[0]
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == column)
                .expect("the key is laid out")
                .pose
                .at
        };
        assert!(
            (at(1).x - 28.575).abs() < 1e-9 && (at(2).x - 47.625).abs() < 1e-9,
            "later columns step out the full Spread"
        );
        assert!(
            at(0).x.abs() < 1e-9,
            "column 0 does not follow a later Spread"
        );
        fixture::undo(&runtime).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert!(
            matrix
                .column_layout
                .get(1)
                .and_then(|record| record.spread)
                .is_none(),
            "one Undo restores the default step"
        );
        root.remove();
    }

    /// Ticket 05: the row Bend token drags the running frame through the
    /// real pointer path and commits one Undo entry. Forty pixels are ten
    /// world millimetres, so the frame turns five degrees.
    #[wasm_bindgen_test]
    async fn row_bend_token_drag_turns_only_that_row_with_one_undo() {
        let mut document = fixture::matrix_document();
        document.matrices[0].rows = 2;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Row {
            matrix_id: fixture::MATRIX_ID.into(),
            row: 1,
        };
        let (root, handle) =
            mount_context_overlay(&runtime, LayoutTransformTool::Bend, context).await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to bend row"),
            "the row Bend tool draws its own token"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.bend),
            Some(&-5.0),
            "the drag turns the row's running frame"
        );
        assert_eq!(
            matrix
                .row_layout
                .first()
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
            0.0,
            "the other row holds still"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime)
                .row_layout
                .get(1)
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
            0.0,
            "one Undo restores the running frame"
        );
        root.remove();
    }

    /// Ticket 05: the key Shift token drags the key's carried shift through
    /// the real pointer path and commits one Undo entry.
    #[wasm_bindgen_test]
    async fn key_shift_token_drag_moves_only_that_key_with_one_undo() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Key {
            matrix_id: fixture::MATRIX_ID.into(),
            row: 0,
            column: 1,
        };
        let (root, handle) =
            mount_context_overlay(&runtime, LayoutTransformTool::Shift, context).await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to shift key"),
            "the key Shift tool draws its own token"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.shift),
            Some(Vec2 { x: 9.525, y: 0.0 }),
            "forty pixels step the shift two grid steps"
        );
        assert!(
            matrix
                .cells
                .iter()
                .filter(|cell| !(cell.row == 0 && cell.column == 1))
                .all(|cell| cell.shift.is_none()),
            "only the dragged key carries a shift"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime)
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.shift)
                .unwrap_or_default(),
            Vec2::default(),
            "one Undo restores the key's shift"
        );
        root.remove();
    }

    /// Ticket 06: the value popup docks above the rail on hover, follows the
    /// pointer during a drag with the live value and delta, and the drop
    /// raises one commit toast with no extra revision; the toast is gone
    /// after its timeout. Sixty pixels up are fifteen world millimetres, so
    /// the snapped stagger lands three grid steps out.
    #[wasm_bindgen_test]
    async fn value_popup_follows_the_drag_and_commit_toast_expires() {
        fn translate_of(element: &web_sys::Element) -> (f64, f64) {
            let transform = element.get_attribute("transform").unwrap_or_default();
            let inner = transform
                .strip_prefix("translate(")
                .and_then(|rest| rest.split(')').next())
                .unwrap_or("");
            let mut parts = inner.split(' ');
            (
                parts.next().and_then(|x| x.parse().ok()).unwrap_or(0.0),
                parts.next().and_then(|y| y.parse().ok()).unwrap_or(0.0),
            )
        }
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Column {
            matrix_id: fixture::MATRIX_ID.into(),
            column: 1,
        };
        let (root, _) =
            mount_context_overlay(&runtime, LayoutTransformTool::Stagger, context).await;
        let handle = root
            .query_selector("[aria-label='Drag to stagger']")
            .unwrap()
            .expect("the stagger token draws");
        dispatch_pointer(&handle, "pointerenter", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root
            .query_selector(".m1-feedback-popup")
            .unwrap()
            .expect("hover docks the value popup");
        let (_, token_y) = translate_of(&handle);
        let (_, popup_y) = translate_of(&popup);
        assert!(
            popup_y > token_y,
            "the docked popup sits above the rail token"
        );
        assert!(
            popup.text_content().unwrap_or_default().contains("Stagger"),
            "the popup names the field"
        );

        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let (_, following_start) =
            translate_of(&root.query_selector(".m1-feedback-popup").unwrap().unwrap());
        dispatch_pointer(&handle, "pointermove", false, false, 1, (100, 40));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root.query_selector(".m1-feedback-popup").unwrap().unwrap();
        let (_, following_end) = translate_of(&popup);
        assert!(
            (following_end - following_start - 15.0).abs() < 1.5,
            "the popup follows the pointer up by the dragged world distance"
        );
        assert!(
            popup
                .text_content()
                .unwrap_or_default()
                .contains("+14.29 mm"),
            "the popup shows the live delta"
        );

        dispatch_pointer(&handle, "pointerup", false, false, 0, (100, 40));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let toast = root
            .query_selector(".m1-feedback-toast")
            .unwrap()
            .expect("the commit raises a toast");
        assert_eq!(toast.get_attribute("role").as_deref(), Some("status"));
        assert_eq!(
            toast.get_attribute("tabindex"),
            None,
            "the toast never takes focus"
        );
        assert_eq!(
            toast.text_content().unwrap_or_default(),
            "Stagger 14.29 mm (+14.29 mm) · 1 Undo step"
        );
        gloo_timers::future::TimeoutFuture::new(300).await;
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the toast writes no extra revision"
        );
        gloo_timers::future::TimeoutFuture::new(2200).await;
        assert!(
            root.query_selector(".m1-feedback-toast").unwrap().is_none(),
            "the toast is gone after its timeout"
        );
        root.remove();
    }

    /// A commit-toast timer that fires after the overlay unmounts must stay
    /// silent: scope switches remount the overlay constantly, and the 1.8 s
    /// timer touching dropped signals panicked the page (WASM `unreachable`),
    /// leaving the Dioxus panic toast over the toolbar.
    #[wasm_bindgen_test]
    async fn commit_toast_timer_survives_overlay_unmount() {
        // A wasm panic aborts instead of unwinding, so no drop guard can restore
        // the hook. The recorder forwards to the previous hook, so a panic still
        // reaches the console, and the hook is only re-installed on success.
        let previous = std::sync::Arc::new(std::panic::take_hook());
        let panics = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        {
            let (previous, recorded) = (previous.clone(), panics.clone());
            std::panic::set_hook(Box::new(move |info| {
                recorded.lock().unwrap().push(info.to_string());
                (**previous)(info);
            }));
        }
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Column {
            matrix_id: fixture::MATRIX_ID.into(),
            column: 1,
        };
        let (root, handle, mut show) =
            mount_toggle_overlay(&runtime, LayoutTransformTool::Stagger, context).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (100, 40));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointerup", false, false, 0, (100, 40));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        assert!(
            root.query_selector(".m1-feedback-toast").unwrap().is_some(),
            "the commit arms its toast timer"
        );
        // A production scope switch unmounts the overlay while the timer is
        // still pending; the timer must not touch the dropped signals.
        show.set(false);
        gloo_timers::future::TimeoutFuture::new(120).await;
        assert!(
            root.query_selector(".m1-transform-handle")
                .unwrap()
                .is_none(),
            "the toggle unmounted the overlay"
        );
        gloo_timers::future::TimeoutFuture::new(2200).await;
        let recorded = panics.lock().unwrap().clone();
        assert!(
            recorded.is_empty(),
            "no expired commit-toast timer panicked after unmount, got {recorded:?}"
        );
        // Only after the assert: set_hook panics once a panic is already counted.
        std::panic::set_hook(Box::new(move |info| (**previous)(info)));
        root.remove();
    }

    /// Ticket 05: hovering a row or key token previews, in amber, the same
    /// key set the Inspector field preview carries for that field.
    #[wasm_bindgen_test]
    async fn row_and_key_tokens_preview_the_same_movers_as_the_inspector_fields() {
        use crate::matrix_transform_operation::MatrixTransformField;
        // The fixture's parts all sit in row 0, so the row rail previews row
        // 0: a part-less row would give the Inspector field no movers either.
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Row {
            matrix_id: fixture::MATRIX_ID.into(),
            row: 0,
        };
        let (root, probe) = mount_context_rail_pair(
            &runtime,
            vec![
                LayoutTransformTool::Offset,
                LayoutTransformTool::Shift,
                LayoutTransformTool::Bend,
            ],
            context.clone(),
        )
        .await;
        for (label, field) in [
            ("Drag to offset row", MatrixTransformField::RowOffsetX),
            ("Drag to shift row", MatrixTransformField::RowShiftX),
            ("Drag to bend row", MatrixTransformField::RowBend),
        ] {
            let token = root
                .query_selector(&format!("[aria-label='{label}']"))
                .unwrap()
                .unwrap_or_else(|| panic!("the {label} token draws"));
            dispatch_pointer(&token, "pointerenter", false, false, 0, (100, 100));
            gloo_timers::future::TimeoutFuture::new(60).await;
            let preview = probe
                .preview
                .borrow()
                .as_ref()
                .and_then(|signal| signal.read().clone());
            let accepted = runtime.model().accepted.expect("an accepted document");
            let expected = matrix_relations::field_preview(
                &accepted.document,
                &accepted.scene.matrix_scenes,
                &context,
                field,
                MatrixSplayAffect::Column,
            )
            .expect("the Inspector field previews its movers");
            assert_eq!(
                preview.as_ref().map(|state| &state.parts),
                Some(&expected.parts),
                "{label} previews the {field:?} movers"
            );
            assert!(
                !expected.parts.is_empty(),
                "{label} previews a non-empty mover set"
            );
            dispatch_pointer(&token, "pointerleave", false, false, 0, (100, 100));
            gloo_timers::future::TimeoutFuture::new(60).await;
            assert!(
                probe
                    .preview
                    .borrow()
                    .as_ref()
                    .is_none_or(|signal| signal.read().is_none()),
                "{label} clears the preview on leave"
            );
        }
        root.remove();

        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Key {
            matrix_id: fixture::MATRIX_ID.into(),
            row: 0,
            column: 1,
        };
        let (root, probe) = mount_context_rail_pair(
            &runtime,
            vec![LayoutTransformTool::Shift, LayoutTransformTool::Bend],
            context.clone(),
        )
        .await;
        for (label, field) in [
            ("Drag to shift key", MatrixTransformField::KeyShiftX),
            ("Drag to bend key", MatrixTransformField::KeyBend),
        ] {
            let token = root
                .query_selector(&format!("[aria-label='{label}']"))
                .unwrap()
                .unwrap_or_else(|| panic!("the {label} token draws"));
            dispatch_pointer(&token, "pointerenter", false, false, 0, (100, 100));
            gloo_timers::future::TimeoutFuture::new(60).await;
            let preview = probe
                .preview
                .borrow()
                .as_ref()
                .and_then(|signal| signal.read().clone());
            let accepted = runtime.model().accepted.expect("an accepted document");
            let expected = matrix_relations::field_preview(
                &accepted.document,
                &accepted.scene.matrix_scenes,
                &context,
                field,
                MatrixSplayAffect::Column,
            )
            .expect("the Inspector field previews its movers");
            assert_eq!(
                preview.as_ref().map(|state| &state.parts),
                Some(&expected.parts),
                "{label} previews the {field:?} movers"
            );
            assert!(
                !expected.parts.is_empty(),
                "{label} previews a non-empty mover set"
            );
            dispatch_pointer(&token, "pointerleave", false, false, 0, (100, 100));
            gloo_timers::future::TimeoutFuture::new(60).await;
            assert!(
                probe
                    .preview
                    .borrow()
                    .as_ref()
                    .is_none_or(|signal| signal.read().is_none()),
                "{label} clears the preview on leave"
            );
        }
        root.remove();
    }

    /// A second 1x1 matrix anchored to the fixture matrix's first key, with a
    /// live key part, for the matrix Move and Rotate drags.
    fn anchored_drag_document(
        follow: boardstudio_core::model::Follow,
    ) -> boardstudio_core::model::ProjectDoc {
        use boardstudio_core::model::*;
        let mut document = fixture::matrix_document();
        document.layouts.push(Layout {
            id: "layout".into(),
            name: "Main".into(),
            board_id: fixture::BOARD_ID.into(),
            matrix_ids: vec![fixture::MATRIX_ID.into(), "dep".into()],
            part_ids: vec![],
            mirror_link: None,
        });
        let key_id = "matrix/dep/r0c0";
        document.boards[0].part_ids.push(key_id.into());
        document.parts.push(Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: key_id.into(),
            definition_id: "switch:base".into(),
            reference: "K3".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: -19.05 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
        document.matrices.push(Matrix {
            id: "dep".into(),
            name: Some("Dependent".into()),
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 0.0, y: -19.05 },
            definition_id: "switch:base".into(),
            part_ids: vec![key_id.into()],
            board_id: Some(fixture::BOARD_ID.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![ColumnLayout::default()],
            anchor: Some(MatrixAnchor {
                matrix_id: fixture::MATRIX_ID.into(),
                row: 0,
                column: 0,
                follow,
                aggregate: None,
            }),
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: Some("switch:base".into()),
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }],
        });
        document
    }

    async fn open_anchored_drag_runtime(follow: boardstudio_core::model::Follow) -> Rc<Runtime> {
        let runtime = support::new_runtime();
        support::open_document(&runtime, anchored_drag_document(follow)).await;
        wait_until_editable(&runtime).await;
        runtime
    }

    /// Ticket 23 slice, RED: dragging an anchored matrix writes its Origin
    /// through the inverse of the anchor frame as one atomic edit.
    #[wasm_bindgen_test]
    async fn anchored_matrix_move_drag_writes_origin_through_anchor_inverse() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let (root, handle) = mount_matrix_overlay(&runtime, LayoutTransformTool::Move, "dep").await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to move matrix"),
            "the Move tool draws its own handle"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag previews actual matrix movement before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = runtime.model().accepted.expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dragged matrix");
        assert!(
            (matrix.origin.x - 10.0).abs() < 1e-9 && (matrix.origin.y + 19.05).abs() < 1e-9,
            "the drag writes the Origin through the anchor inverse, got {:?}",
            matrix.origin
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        fixture::undo(&runtime).await;
        let matrix = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .cloned()
            .expect("the dragged matrix");
        assert!(
            matrix.origin.x.abs() < 1e-9 && (matrix.origin.y + 19.05).abs() < 1e-9,
            "one Undo restores the dragged Origin"
        );
        root.remove();
    }

    /// Ticket 23 slice, RED: the matrix drag is constrained to the Follow
    /// directions, so an unfollowed component never moves the frame.
    #[wasm_bindgen_test]
    async fn anchored_matrix_move_drag_ignores_unfollowed_directions() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: false,
            y: true,
            rotation: true,
        })
        .await;
        let (root, handle) = mount_matrix_overlay(&runtime, LayoutTransformTool::Move, "dep").await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_none(),
            "a drag outside every followed direction previews nothing"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision,
            "a drag outside every followed direction commits nothing"
        );
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 80));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the followed direction previews before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 80));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let matrix = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .cloned()
            .expect("the dragged matrix");
        assert!(
            matrix.origin.x.abs() < 1e-9 && (matrix.origin.y + 14.05).abs() < 1e-9,
            "only the followed direction moves the frame, got {:?}",
            matrix.origin
        );
        root.remove();
    }

    /// Ticket 23 follow-up: each matrix tool mount draws only its own
    /// handle. The packaged-page smoke caught the Move and Rotate mounts
    /// each rendering both handles, so two identical "Drag to move matrix"
    /// copies fought over the gesture.
    #[wasm_bindgen_test]
    async fn matrix_tool_mounts_draw_only_their_own_handle() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let (move_root, _) = mount_matrix_overlay(&runtime, LayoutTransformTool::Move, "dep").await;
        assert!(
            move_root
                .query_selector("[aria-label='Drag to move matrix']")
                .unwrap()
                .is_some(),
            "the Move mount draws the move handle"
        );
        assert!(
            move_root
                .query_selector("[aria-label='Drag to rotate matrix']")
                .unwrap()
                .is_none(),
            "the Move mount draws no rotate handle"
        );
        move_root.remove();
        let (rotate_root, _) =
            mount_matrix_overlay(&runtime, LayoutTransformTool::Rotate, "dep").await;
        assert!(
            rotate_root
                .query_selector("[aria-label='Drag to rotate matrix']")
                .unwrap()
                .is_some(),
            "the Rotate mount draws the rotate handle"
        );
        assert!(
            rotate_root
                .query_selector("[aria-label='Drag to move matrix']")
                .unwrap()
                .is_none(),
            "the Rotate mount draws no move handle"
        );
        rotate_root.remove();
    }

    /// Ticket 23 slice, RED: the Rotate tool turns the selected matrix, and
    /// the menu disables Rotate with an explanation when Follow rotation is
    /// off.
    #[wasm_bindgen_test]
    async fn matrix_rotate_drag_turns_the_matrix() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let (root, _) = mount_matrix_overlay(&runtime, LayoutTransformTool::Rotate, "dep").await;
        let rotate = root
            .query_selector("[aria-label='Drag to rotate matrix']")
            .unwrap()
            .expect("the Rotate tool draws its own handle");
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let accepted = runtime.model().accepted.expect("an accepted document");
        let center = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == "dep")
            .expect("the dragged scene")
            .cells
            .iter()
            .find(|cell| cell.enabled)
            .expect("the key is laid out")
            .pose
            .at;
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(
            &rotate,
            "pointerdown",
            false,
            false,
            1,
            client(center.x + 20.0, center.y),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(
            &rotate,
            "pointermove",
            false,
            false,
            1,
            client(center.x, center.y + 20.0),
        );
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the rotation previews before commit"
        );
        dispatch_pointer(
            &rotate,
            "pointerup",
            false,
            false,
            0,
            client(center.x, center.y + 20.0),
        );
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let rotation = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dragged matrix")
            .rotation;
        assert_eq!(rotation, Some(90.0), "a quarter turn sweeps 90 degrees");
        root.remove();
    }

    /// Ticket 23, RED: hovering the Spread handle neither replaces its SVG
    /// target nor lets a passive hint intercept the pointer-down that follows.
    #[wasm_bindgen_test]
    async fn spread_handle_hover_keeps_its_svg_target() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Spread).await;
        dispatch_pointer(&handle, "pointerover", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        let live = root
            .query_selector(".m1-transform-handle")
            .unwrap()
            .expect("the handle survives hover");
        assert!(
            live == handle,
            "hover does not replace the handle's SVG target"
        );
        let rect = handle.get_bounding_client_rect();
        // No passive hint may cover the handle: every painted element in the
        // overlay that contains the handle's center and paints above it must
        // opt out of pointer events. Box math is viewport-independent, so the
        // check holds wherever the canvas scrolled.
        let center = (
            rect.x() + rect.width() / 2.0,
            rect.y() + rect.height() / 2.0,
        );
        let window = web_sys::window().unwrap();
        let painted = root.query_selector_all("svg *").unwrap();
        let mut interceptors = Vec::new();
        for index in 0..painted.length() {
            let element: web_sys::Element = painted.item(index).unwrap().dyn_into().unwrap();
            if element == handle || handle.contains(Some(element.unchecked_ref())) {
                continue;
            }
            let position = handle.compare_document_position(element.unchecked_ref());
            if position & web_sys::Node::DOCUMENT_POSITION_FOLLOWING == 0 {
                continue;
            }
            let area = element.get_bounding_client_rect();
            if center.0 < area.x()
                || center.0 > area.x() + area.width()
                || center.1 < area.y()
                || center.1 > area.y() + area.height()
            {
                continue;
            }
            let events = window
                .get_computed_style(&element)
                .ok()
                .flatten()
                .and_then(|style| style.get_property_value("pointer-events").ok())
                .unwrap_or_default();
            if events != "none" {
                interceptors.push(format!("{}.{}", element.tag_name(), element.class_name()));
            }
        }
        assert!(
            interceptors.is_empty(),
            "no passive hint intercepts the handle: {interceptors:?}"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision || runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            fixture_revision(&runtime) > revision || runtime.model().display_preview.is_some(),
            "the pointer-down after hover still starts the drag"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        root.remove();
    }

    /// Ticket 23 slice 11: releasing the pointer outside the handle still
    /// commits the drag. Capture retargets the up to the handle, so the drop
    /// path must not re-hit-test the handle geometry: an up far outside it
    /// commits the previewed operation with one Undo step.
    #[wasm_bindgen_test]
    async fn pointerup_outside_the_handle_still_commits_the_drag() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(2, Default::default);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag is previewing before the outside release"
        );
        // The pointer left the handle: the up arrives with a client point far
        // outside the handle geometry, as capture retargeting delivers it.
        dispatch_pointer(&handle, "pointerup", false, false, 0, (900, 900));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the outside release commits one accepted edit"
        );
        assert!(
            runtime.model().display_preview.is_none(),
            "the preview clears on release"
        );
        let stagger = fixture::accepted_matrix(&runtime).column_layout[1].stagger;
        assert!(stagger != 0.0, "the committed drag moved the column");
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].stagger,
            0.0,
            "one Undo restores the column"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn unrelated_applying_work_does_not_admit_transform_drag() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        assert_eq!(runtime.model().lifecycle, Lifecycle::Applying);
        let press = dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100));
        assert!(
            !press.default_prevented(),
            "unrelated work must not admit a drag"
        );
        support::run_pending(&runtime).await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn escape_then_immediate_drag_is_remembered_while_preview_is_applying() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(2, Default::default);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100));
        dispatch_pointer(&handle, "pointermove", true, false, 1, (140, 60));
        assert_eq!(runtime.model().lifecycle, Lifecycle::Applying);
        let init = web_sys::KeyboardEventInit::new();
        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_key("Escape");
        let escape =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
        handle.dispatch_event(&escape).unwrap();
        let second = dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100));
        assert!(
            second.default_prevented(),
            "the immediate second press is admitted"
        );
        dispatch_pointer(&handle, "pointermove", true, false, 1, (140, 100));
        dispatch_pointer(&handle, "pointerup", true, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "only the second drag commits"
        );
        assert_ne!(
            fixture::accepted_matrix(&runtime).column_layout[1].offset,
            Vec2::default()
        );
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].offset,
            Vec2::default()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn escape_after_queued_edit_retires_immediate_second_drag() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(2, Default::default);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100));
        dispatch_pointer(&handle, "pointermove", true, false, 1, (140, 60));
        let mut matrix = fixture::accepted_matrix(&runtime);
        matrix.origin.x += 5.0;
        runtime.submit(Event::ResolveEdit {
            operation_id: runtime.operation(),
            label: "unrelated edit".into(),
            resolver: EditResolver::new("unrelated edit", move |_| {
                Resolution::submit(
                    vec![matrix.id.clone()],
                    EditOperation::SetMatrix {
                        matrix: matrix.clone(),
                        definitions: None,
                    },
                )
            }),
        });
        let init = web_sys::KeyboardEventInit::new();
        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_key("Escape");
        handle
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        let second = dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100));
        assert!(second.default_prevented());
        dispatch_pointer(&handle, "pointermove", true, false, 1, (140, 100));
        dispatch_pointer(&handle, "pointerup", true, false, 0, (140, 100));
        support::run_pending(&runtime).await;
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "only the unrelated edit lands"
        );
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].offset,
            Vec2::default()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn escape_unmoved_second_drag_keeps_first_clear_for_third_press() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100));
        dispatch_pointer(&handle, "pointermove", true, false, 1, (140, 60));
        let init = web_sys::KeyboardEventInit::new();
        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_key("Escape");
        let escape =
            || web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
        handle.dispatch_event(&escape()).unwrap();
        assert!(
            dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100))
                .default_prevented()
        );
        handle.dispatch_event(&escape()).unwrap();
        assert!(
            dispatch_pointer(&handle, "pointerdown", true, false, 1, (100, 100))
                .default_prevented(),
            "third press is admitted while the first clear remains pending"
        );
        handle.dispatch_event(&escape()).unwrap();
        support::run_pending(&runtime).await;
        root.remove();
    }

    /// Ticket 23 slice 12: losing pointer capture mid-drag cancels the
    /// preview with nothing committed, and a subsequent drag on the same
    /// handle recovers and commits.
    #[wasm_bindgen_test]
    async fn lost_capture_cancels_and_a_subsequent_drag_recovers() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(2, Default::default);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag is previewing before capture is lost"
        );
        dispatch_pointer(&handle, "lostpointercapture", false, false, 1, (140, 60));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_none() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_none(),
            "lost capture clears the preview"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision,
            "the cancelled drag commits nothing"
        );
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].stagger,
            0.0,
            "the cancelled drag moves nothing"
        );
        // A subsequent drag on the same handle recovers and commits.
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 60));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the subsequent drag commits one accepted edit"
        );
        assert!(
            fixture::accepted_matrix(&runtime).column_layout[1].stagger != 0.0,
            "the subsequent drag moves the column"
        );
        root.remove();
    }

    /// Ticket 23 slice 6, RED: Ctrl at pointer-down on the Splay handle latches a
    /// swing. The drag writes only the origin through `OriginSwing` (no step
    /// compensation), never the angle; one Undo restores exactly.
    #[wasm_bindgen_test]
    async fn ctrl_splay_drag_swings_the_origin_without_step_compensation() {
        let mut document = fixture::matrix_document();
        let matrix = &mut document.matrices[0];
        matrix.column_layout.resize_with(2, Default::default);
        matrix.column_layout[1].splay = 17.0;
        matrix.column_layout[1].stagger = 3.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let before = fixture_revision(&runtime);
        let origin = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .scene
            .matrix_scenes[0]
            .columns[1]
            .splay_origin;
        let (root, _) = mount_overlay(&runtime, LayoutTransformTool::Splay).await;
        let handle = root
            .query_selector("[aria-label='Drag to splay']")
            .unwrap()
            .unwrap();
        let stub = js_sys::Function::new_no_args("return true;");
        for method in [
            "setPointerCapture",
            "hasPointerCapture",
            "releasePointerCapture",
        ] {
            js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
        }
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let start = client(origin.x + 20.0, origin.y);
        let end = client(origin.x, origin.y + 20.0);
        // Ctrl is down for the pointer-down only; the latch must hold the swing.
        dispatch_pointer(&handle, "pointerdown", true, false, 1, start);
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            handle.get_attribute("data-swing").as_deref(),
            Some("true"),
            "Ctrl at pointer-down shows the swing"
        );
        dispatch_pointer(&handle, "pointermove", false, false, 1, end);
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            before,
            "the swing preview never commits"
        );
        let preview = runtime
            .model()
            .display_preview
            .as_ref()
            .expect("the swing previews")
            .matrix_scenes[0]
            .columns[1]
            .clone();
        assert_eq!(
            preview.splay_angle, 17.0,
            "the swing preview never turns the angle"
        );
        assert!(
            preview.splay_origin != origin,
            "the swing preview moves the origin"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, end);
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > before {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(fixture_revision(&runtime), before + 1, "one accepted edit");
        let matrix = fixture::accepted_matrix(&runtime);
        let record = &matrix.column_layout[1];
        assert_eq!(record.splay, 17.0, "the swing writes no angle");
        assert!(
            record.spread.is_none(),
            "the swing writes no compensating Spread"
        );
        assert_eq!(
            record.stagger, 3.0,
            "the swing writes no compensating Stagger"
        );
        assert!(
            record.splay_origin != Vec2::default(),
            "the swing moves the origin"
        );
        fixture::undo(&runtime).await;
        let matrix = fixture::accepted_matrix(&runtime);
        let record = &matrix.column_layout[1];
        assert_eq!(
            record.splay_origin,
            Vec2::default(),
            "one Undo restores the origin exactly"
        );
        assert_eq!(record.splay, 17.0, "one Undo keeps the angle");
        assert!(
            record.spread.is_none() && record.stagger == 3.0,
            "one Undo keeps the steps uncompensated"
        );
        root.remove();
    }

    /// Ticket 23 slice 6, RED: an Origin drag snaps to its column base (the
    /// ergogen-style reset target). Alt disables the snap, and a subsequent
    /// drag recovers.
    #[wasm_bindgen_test]
    async fn origin_drag_snaps_to_the_column_base_without_alt() {
        let mut document = fixture::matrix_document();
        let matrix = &mut document.matrices[0];
        matrix.column_layout.resize_with(2, Default::default);
        // An off-grid base: the grid alone would land this drop at 23.8125,
        // never exactly on the base.
        matrix.column_layout[1].spread = Some(22.0);
        matrix.column_layout[1].splay_origin = Vec2 { x: 5.0, y: 0.0 };
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let scene = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .scene
            .clone();
        let basis = &scene.matrix_scenes[0].columns[1];
        assert_eq!(basis.base, Vec2 { x: 22.0, y: 0.0 });
        assert_eq!(basis.splay_origin, Vec2 { x: 27.0, y: 0.0 });
        let before = fixture_revision(&runtime);
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Origin).await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Move splay origin")
        );
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        // Raw drop (23, 1): 1.4 mm from the base, off the grid.
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(27.0, 0.0));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, client(23.0, 1.0));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the origin drag previews before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, client(23.0, 1.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > before {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let record = &fixture::accepted_matrix(&runtime).column_layout[1];
        assert!(
            record.splay_origin.x.abs() < 1e-9 && record.splay_origin.y.abs() < 1e-9,
            "the drag snaps to the column base, not the grid: {:?}",
            record.splay_origin
        );
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].splay_origin,
            Vec2 { x: 5.0, y: 0.0 },
            "one Undo restores the custom origin"
        );

        // Alt disables the snap: the same gesture lands on the raw drop, and
        // the recovered drag still commits. The overlay remounts because the
        // commit moved the owner revision on.
        root.remove();
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Origin).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(27.0, 0.0));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, client(23.0, 1.0));
        dispatch_pointer(&handle, "pointerup", false, true, 0, client(23.0, 1.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let record = &fixture::accepted_matrix(&runtime).column_layout[1];
        // One client pixel is a quarter world mm, and Alt skips the snap that
        // would absorb the roundtrip: the drop lands on the raw point, which
        // is neither the base nor a grid step.
        assert!(
            (record.splay_origin.x - 1.0).abs() < 0.25
                && (record.splay_origin.y - 1.0).abs() < 0.25,
            "Alt lands the raw drop without snapping: {:?}",
            record.splay_origin
        );
        root.remove();
    }

    /// Ticket 07: an anchored part's rail draws Move, Rotate, Spread and
    /// Stagger left to right whatever the mount order, and hovering the
    /// Spread token docks the carried value in the popup.
    #[wasm_bindgen_test]
    async fn part_rail_orders_move_rotate_spread_stagger_with_live_values() {
        use LayoutTransformTool::{Move, Rotate, Spread, Stagger};
        let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
        wait_until_editable(&runtime).await;
        let root = mount_part_rail(&runtime, "mcu", vec![Stagger, Move, Spread, Rotate]).await;
        let handles = root.query_selector_all(".m1-transform-handle").unwrap();
        assert_eq!(handles.length(), 4, "the anchored rail draws four tokens");
        let mut placed: Vec<(f64, String)> = Vec::new();
        for index in 0..handles.length() {
            let handle = handles
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::Element>()
                .unwrap();
            let transform = handle.get_attribute("transform").unwrap_or_default();
            let x: f64 = transform
                .strip_prefix("translate(")
                .and_then(|rest| rest.split(' ').next())
                .and_then(|x| x.parse().ok())
                .unwrap_or(f64::NAN);
            placed.push((x, handle.get_attribute("aria-label").unwrap_or_default()));
        }
        placed.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap());
        assert_eq!(
            placed
                .iter()
                .map(|(_, label)| label.as_str())
                .collect::<Vec<_>>(),
            [
                "Drag to move part",
                "Drag to rotate part",
                "Drag to spread",
                "Drag to stagger",
            ]
        );
        let spread = root
            .query_selector("[aria-label='Drag to spread']")
            .unwrap()
            .expect("the Spread token draws");
        dispatch_pointer(&spread, "pointerenter", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root
            .query_selector(".m1-feedback-popup")
            .unwrap()
            .expect("hover docks the value popup");
        let text = popup.text_content().unwrap_or_default();
        assert!(
            text.contains("Spread") && text.contains("19.00 mm"),
            "the popup names the carried value: {text}"
        );
        root.remove();
    }

    /// Ticket 07 (RED): dragging the anchored part's Stagger token +2mm
    /// commits one revision whose anchor Stagger grew by 2, with Offset
    /// unchanged; one Undo restores the anchor.
    #[wasm_bindgen_test]
    async fn part_stagger_drag_commits_carried_stagger_with_offset_unchanged() {
        let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
        wait_until_editable(&runtime).await;
        let (root, handle, _) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Stagger).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        // One client pixel is a quarter world mm; Alt skips the grid so the
        // +2mm drag lands exact.
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(38.05, 0.0));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, client(38.05, 2.0));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root.query_selector(".m1-feedback-popup").unwrap().unwrap();
        assert!(
            popup
                .text_content()
                .unwrap_or_default()
                .contains("+2.00 mm"),
            "the popup follows the drag with the live delta"
        );
        dispatch_pointer(&handle, "pointerup", false, true, 0, client(38.05, 2.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(anchor.stagger, 2.0, "the carried Stagger grew by 2");
        assert_eq!(anchor.spread, 19.0, "Spread is untouched");
        assert_eq!(
            anchor.offset,
            Vec2::default(),
            "the local Offset is untouched"
        );
        let toast = root
            .query_selector(".m1-feedback-toast")
            .unwrap()
            .expect("the commit raises a toast");
        assert_eq!(
            toast.text_content().unwrap_or_default(),
            "Stagger 2.00 mm (+2.00 mm) · 1 Undo step"
        );
        fixture::undo(&runtime).await;
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(anchor.stagger, 0.0, "one Undo restores the anchor");
        root.remove();
    }

    /// Ticket 07: dragging the anchored part's Move token edits the local
    /// Offset; one Undo restores it.
    #[wasm_bindgen_test]
    async fn part_move_drag_commits_local_offset() {
        let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
        wait_until_editable(&runtime).await;
        let (root, handle, _) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Move).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(38.05, 0.0));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, client(40.05, 2.0));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointerup", false, true, 0, client(40.05, 2.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(
            anchor.offset,
            Vec2 { x: 2.0, y: 2.0 },
            "the local Offset moved with the drag"
        );
        assert_eq!(anchor.stagger, 0.0, "the carried values are untouched");
        let toast = root
            .query_selector(".m1-feedback-toast")
            .unwrap()
            .expect("the commit raises a toast");
        assert_eq!(
            toast.text_content().unwrap_or_default(),
            "Move X 2.00, Y 2.00 mm (+2.00, +2.00 mm) · 1 Undo step"
        );
        fixture::undo(&runtime).await;
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(
            anchor.offset,
            Vec2::default(),
            "one Undo restores the Offset"
        );
        root.remove();
    }

    /// Ticket 07: dragging the anchored part's Rotate token edits the local
    /// Rotation; the commit matches the live popup value and one Undo
    /// restores it.
    #[wasm_bindgen_test]
    async fn part_rotate_drag_commits_local_rotation() {
        let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
        wait_until_editable(&runtime).await;
        let (root, handle, _) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Rotate).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(38.05, 0.0));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, client(48.05, 10.0));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root.query_selector(".m1-feedback-popup").unwrap().unwrap();
        let popup_text = popup.text_content().unwrap_or_default();
        assert!(
            popup_text.contains("Rotate") && popup_text.contains('°'),
            "the popup follows the sweep: {popup_text}"
        );
        // The value renders as `45.0°`, possibly without surrounding spaces,
        // so scan for the first degree mark instead of splitting words.
        let live: f64 = {
            let end = popup_text.find('°').expect("the swept degrees");
            let start = popup_text[..end]
                .rfind(|cell: char| {
                    !(cell.is_ascii_digit()
                        || cell == '.'
                        || cell == '-'
                        || cell == '+'
                        || cell == '−')
                })
                .map(|index| index + 1)
                .unwrap_or(0);
            popup_text[start..end]
                .parse()
                .expect("the popup names the swept degrees")
        };
        assert!(
            live.abs() > 1.0,
            "the sweep moved the rotation: {popup_text}"
        );
        dispatch_pointer(&handle, "pointerup", false, true, 0, client(48.05, 10.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let anchor = part_anchor_of(&runtime, "mcu");
        assert!(
            (anchor.rotation - live).abs() < 0.06,
            "the commit matches the live popup value within its tenth-degree display: {anchor:?} vs {live}"
        );
        fixture::undo(&runtime).await;
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(anchor.rotation, 0.0, "one Undo restores the Rotation");
        root.remove();
    }

    /// Ticket 07: a detached part draws Move and Rotate but no Spread or
    /// Stagger; a matrix member draws no part token at all and keeps its
    /// matrix's tools.
    #[wasm_bindgen_test]
    async fn detached_parts_and_members_draw_only_their_own_tokens() {
        let runtime = fixture::open_detached_part_runtime().await;
        wait_until_editable(&runtime).await;
        for tool in [LayoutTransformTool::Move, LayoutTransformTool::Rotate] {
            let (root, handle, _) = mount_part_overlay(&runtime, "mcu", tool).await;
            assert!(handle.is_some(), "{tool:?} draws on a detached part");
            root.remove();
        }
        let (root, handle, _) =
            mount_part_overlay(&runtime, "mcu", LayoutTransformTool::Spread).await;
        assert!(handle.is_none(), "Spread never draws without an anchor");
        root.remove();
        let matrix_runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&matrix_runtime).await;
        let (root, handle, _) = mount_part_overlay(
            &matrix_runtime,
            "matrix/matrix/r0c1",
            LayoutTransformTool::Move,
        )
        .await;
        assert!(
            handle.is_none(),
            "a matrix member keeps its matrix's tools, not the part rail"
        );
        root.remove();
    }

    /// Ticket 07: a detached Move token drives the shipped part-drag path:
    /// the gesture commits one revision that moves the pose, raises a toast,
    /// and one Undo restores it.
    #[wasm_bindgen_test]
    async fn detached_move_drag_drives_the_shipped_part_drag_path() {
        let runtime = fixture::open_detached_part_runtime().await;
        wait_until_editable(&runtime).await;
        let start = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .document
            .parts
            .iter()
            .find(|part| part.id == "mcu")
            .expect("the mcu part")
            .pose
            .at;
        let (root, handle, _) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Move).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(
            &handle,
            "pointerdown",
            false,
            false,
            1,
            client(start.x, start.y),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        // Past the shipped body threshold, so the session gesture begins.
        dispatch_pointer(
            &handle,
            "pointermove",
            false,
            false,
            1,
            client(start.x + 3.0, start.y),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        // The token drag drives the shipped session gesture, like a body drag.
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            if runtime.model().gesture.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().gesture.is_some(),
            "the token drag drives the session gesture"
        );
        dispatch_pointer(
            &handle,
            "pointerup",
            false,
            false,
            0,
            client(start.x + 3.0, start.y),
        );
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        assert!(
            runtime.model().gesture.is_none(),
            "the session gesture settles on drop"
        );
        let toast = root
            .query_selector(".m1-feedback-toast")
            .unwrap()
            .expect("the commit raises a toast");
        assert!(
            toast
                .text_content()
                .unwrap_or_default()
                .starts_with("Move X "),
            "the toast names the move: {}",
            toast.text_content().unwrap_or_default()
        );
        fixture::undo(&runtime).await;
        let restored = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .document
            .parts
            .iter()
            .find(|part| part.id == "mcu")
            .expect("the mcu part")
            .pose
            .at;
        assert!(
            (restored.x - start.x).abs() < 1e-9 && (restored.y - start.y).abs() < 1e-9,
            "one Undo restores the pose"
        );
        root.remove();
    }

    /// Ticket 07: a detached Rotate sweep previews the popup and commits one
    /// `SetPartRotation`: the exact rotation persists, one toast, one Undo
    /// restores it. User approval keeps the token visible and working.
    #[wasm_bindgen_test]
    async fn detached_rotate_drag_commits_exact_rotation() {
        let runtime = fixture::open_detached_part_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle, _) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Rotate).await;
        dispatch_pointer(&handle, "pointerenter", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root
            .query_selector(".m1-feedback-popup")
            .unwrap()
            .expect("hover docks the value popup");
        assert!(
            popup.text_content().unwrap_or_default().contains("0.0°"),
            "the popup shows the detached rotation"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, (140, 60));
        gloo_timers::future::TimeoutFuture::new(60).await;
        let popup = root.query_selector(".m1-feedback-popup").unwrap().unwrap();
        let popup_text = popup.text_content().unwrap_or_default();
        assert!(
            popup_text.contains("Rotate") && popup_text.contains('°'),
            "the popup follows the sweep: {popup_text}"
        );
        let live: f64 = {
            let end = popup_text.find('°').expect("the swept degrees");
            let start = popup_text[..end]
                .rfind(|cell: char| {
                    !(cell.is_ascii_digit()
                        || cell == '.'
                        || cell == '-'
                        || cell == '+'
                        || cell == '−')
                })
                .map(|index| index + 1)
                .unwrap_or(0);
            popup_text[start..end]
                .parse()
                .expect("the popup names the swept degrees")
        };
        assert!(
            live.abs() > 1.0,
            "the sweep moved the rotation: {popup_text}"
        );
        dispatch_pointer(&handle, "pointerup", false, true, 0, (140, 60));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let rotation = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .document
            .parts
            .iter()
            .find(|part| part.id == "mcu")
            .expect("the mcu part")
            .pose
            .rotation;
        assert!(
            (rotation - live).abs() < 0.06,
            "the commit matches the live popup value within its tenth-degree display: {rotation} vs {live}"
        );
        let toast = root
            .query_selector(".m1-feedback-toast")
            .unwrap()
            .expect("the commit raises a toast");
        assert!(
            toast.text_content().unwrap_or_default().contains("Rotate"),
            "the toast names the rotation: {}",
            toast.text_content().unwrap_or_default()
        );
        fixture::undo(&runtime).await;
        let restored = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .document
            .parts
            .iter()
            .find(|part| part.id == "mcu")
            .expect("the mcu part")
            .pose
            .rotation;
        assert_eq!(restored, 0.0, "one Undo restores the Rotation");
        root.remove();
    }

    /// Ticket 07: a drag on one half of a mirrored pair updates both halves
    /// per ADR-0038: the Splay-carrying Stagger edit fans out to the partner.
    #[wasm_bindgen_test]
    async fn part_stagger_drag_on_one_half_updates_both_halves() {
        let runtime = fixture::open_part_pair_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle, _) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Stagger).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        // The left mcu evaluates at its anchor key plus the carried spread.
        let center = {
            let model = runtime.model();
            let accepted = model.accepted.as_ref().expect("accepted");
            let part = accepted
                .document
                .parts
                .iter()
                .find(|part| part.id == "mcu")
                .unwrap();
            boardstudio_core::evaluated_part_pose(&accepted.document, part).at
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(
            &handle,
            "pointerdown",
            false,
            false,
            1,
            client(center.x, center.y),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(
            &handle,
            "pointermove",
            false,
            true,
            1,
            client(center.x, center.y + 2.0),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        dispatch_pointer(
            &handle,
            "pointerup",
            false,
            true,
            0,
            client(center.x, center.y + 2.0),
        );
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let source = part_anchor_of(&runtime, "mcu");
        let partner = part_anchor_of(&runtime, "mcu-mirror");
        assert_eq!(source.stagger, 2.0, "the dragged half carries +2");
        assert_eq!(partner.stagger, 2.0, "the partner mirrors the edit");
        assert_eq!(source.offset, Vec2::default(), "Offset is untouched");
        fixture::undo(&runtime).await;
        assert_eq!(
            part_anchor_of(&runtime, "mcu").stagger,
            0.0,
            "one Undo restores the source"
        );
        assert_eq!(
            part_anchor_of(&runtime, "mcu-mirror").stagger,
            0.0,
            "one Undo restores the partner"
        );
        root.remove();
    }

    /// Ticket 07: pressing a part token focuses the part's half, like a body
    /// click (split-board-editing 14), so a drag on the dimmed half keeps
    /// editing that half.
    #[wasm_bindgen_test]
    async fn part_token_press_focuses_the_parts_half() {
        let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
        wait_until_editable(&runtime).await;
        let (root, handle, focus_half) =
            mount_part_overlay_expect(&runtime, "mcu", LayoutTransformTool::Stagger).await;
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(60).await;
        assert_eq!(
            *focus_half.borrow(),
            1,
            "the token press focuses the part's half"
        );
        root.remove();
    }

    /// Ticket 07: locked parts draw no part tokens.
    #[wasm_bindgen_test]
    async fn locked_parts_draw_no_part_tokens() {
        let mut document = fixture::part_document((19.0, 0.0));
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "mcu")
            .expect("the mcu part")
            .locked = Some(true);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        for tool in [
            LayoutTransformTool::Move,
            LayoutTransformTool::Rotate,
            LayoutTransformTool::Spread,
            LayoutTransformTool::Stagger,
        ] {
            let (root, handle, _) = mount_part_overlay(&runtime, "mcu", tool).await;
            assert!(handle.is_none(), "{tool:?} hides on a locked part");
            root.remove();
        }
    }

    /// Ticket 08: the pick host mounts one Anchor overlay with the
    /// production preview context, so rings position on evaluated scenes.
    #[derive(Props, Clone, PartialEq)]
    struct PickHostProps {
        runtime: LayoutTransformRuntime,
        svg: LayoutTransformSvg,
        owner: LayoutOwnerIdentity,
        scope: Scope,
        pick_owner: AnchorPickOwner,
        context: TreeContext,
    }

    #[component]
    fn pick_host(props: PickHostProps) -> Element {
        let selected_context = use_signal(|| {
            Some(ScopedTreeContext {
                scope: props.scope.clone(),
                context: props.context.clone(),
            })
        });
        let scope_generation = use_signal(|| props.owner.generation);
        let workspace = use_signal(|| "Layout");
        let surface_size = use_signal(|| (800.0, 600.0));
        let arbiter = use_hook(CanvasInteractionArbiter::default);
        let model = props.runtime.0.model();
        let accepted = model.accepted.clone().expect("an accepted document");
        use_context_provider(|| {
            Some(FieldPreviewSource {
                document: accepted.document.clone(),
                scene: accepted.scene.clone(),
            })
        });
        rsx! {
            svg { id: "pick-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
                LayoutAnchorPickOverlay {
                    runtime: props.runtime.clone(),
                    svg: props.svg.clone(),
                    arbiter,
                    owner: props.owner.clone(),
                    selected_context,
                    scope_generation,
                    workspace,
                    surface_size: Some(surface_size),
                    pick_owner: props.pick_owner.clone(),
                    view_x: 0.0,
                    view_y: 0.0,
                    width: 100.0,
                    height: 100.0,
                }
            }
        }
    }

    async fn mount_pick_overlay(
        runtime: &Rc<Runtime>,
        pick_owner: AnchorPickOwner,
        context: TreeContext,
    ) -> web_sys::Element {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let owner = LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        };
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let props = PickHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            scope: owner.scope.clone().expect("a scope"),
            owner,
            pick_owner,
            context,
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(pick_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#pick-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        root
    }

    fn pick_token(root: &web_sys::Element) -> Option<web_sys::Element> {
        root.query_selector("[data-pick=\"anchor\"]").unwrap()
    }

    fn pick_ring_values(root: &web_sys::Element) -> Vec<String> {
        let list = root.query_selector_all("[data-anchor]").unwrap();
        let mut values = Vec::new();
        for index in 0..list.length() {
            let element: web_sys::Element = list.item(index).unwrap().dyn_into().unwrap();
            values.push(element.get_attribute("data-anchor").unwrap_or_default());
        }
        values.sort();
        values
    }

    fn pick_ring(root: &web_sys::Element, anchor: &str) -> web_sys::Element {
        root.query_selector(&format!("[data-anchor=\"{anchor}\"]"))
            .unwrap()
            .unwrap_or_else(|| panic!("the {anchor} ring draws"))
    }

    fn ring_circle_radii(ring: &web_sys::Element) -> Vec<String> {
        let circles = ring.query_selector_all("circle").unwrap();
        let mut radii = Vec::new();
        for index in 0..circles.length() {
            let element: web_sys::Element = circles.item(index).unwrap().dyn_into().unwrap();
            radii.push(element.get_attribute("r").unwrap_or_default());
        }
        radii
    }

    /// The toast raises once the commit settles, a few frames after the
    /// revision lands, so tests wait for it instead of reading it at once.
    async fn wait_for_pick_toast(
        runtime: &Rc<Runtime>,
        root: &web_sys::Element,
    ) -> web_sys::Element {
        for _ in 0..100 {
            support::run_pending(runtime).await;
            if let Some(toast) = root.query_selector(".m1-feedback-toast").unwrap() {
                return toast;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("the commit toast");
    }

    async fn arm_pick_via_token(root: &web_sys::Element) {
        let token = pick_token(root).expect("the Anchor token draws");
        dispatch_pointer(&token, "pointerdown", false, false, 1, (50, 50));
        gloo_timers::future::TimeoutFuture::new(80).await;
    }

    /// Ticket 08: arming the anchored matrix lists its committable keys in
    /// variant-D ring sizes, marks the current anchor, and clicking a ring
    /// commits one Undo entry with the carried toast; Undo restores.
    #[wasm_bindgen_test]
    async fn matrix_pick_commits_one_undo_entry_with_the_carried_toast() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the anchored matrix")
            .clone();
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == "dep")
            .expect("the dep scene")
            .clone();
        let context = TreeContext::Matrix {
            matrix_id: "dep".into(),
        };
        let root = mount_pick_overlay(
            &runtime,
            AnchorPickOwner::Matrix {
                matrix: Rc::new(matrix),
                projection: Rc::new(projection),
                context: context.clone(),
            },
            context,
        )
        .await;
        assert!(
            pick_token(&root).is_some(),
            "the anchored matrix draws the Anchor token"
        );
        assert!(
            pick_ring_values(&root).is_empty(),
            "rings wait for the token press"
        );
        arm_pick_via_token(&root).await;
        assert_eq!(
            pick_ring_values(&root),
            vec![
                "matrix:0:0".to_owned(),
                "matrix:0:1".to_owned(),
                "matrix:0:2".to_owned(),
            ],
            "the dep lists the main matrix's keys"
        );
        assert!(
            root.query_selector("[data-anchor=\"dep:0:0\"]")
                .unwrap()
                .is_none(),
            "the dep's own key draws no ring"
        );
        assert_eq!(
            ring_circle_radii(&pick_ring(&root, "matrix:0:0")),
            vec!["12".to_owned(), "2.25".to_owned(), "1.6".to_owned()],
            "the current anchor fills a white dot inside the 4.5 px ring"
        );
        assert!(
            root.query_selector(".m1-feedback-popup").unwrap().is_some(),
            "the anchor popup pins while armed"
        );
        // Hovering previews the Core-evaluated post-commit pose in amber.
        let target = pick_ring(&root, "matrix:0:2");
        dispatch_pointer(&target, "pointerenter", false, false, 0, (60, 60));
        gloo_timers::future::TimeoutFuture::new(120).await;
        assert_eq!(
            ring_circle_radii(&pick_ring(&root, "matrix:0:2"))[1],
            "3",
            "the hovered ring grows to 6 px"
        );
        let amber = root
            .query_selector_all(".m1-anchor-pick-preview")
            .unwrap()
            .length();
        assert!(amber > 0, "hovering previews the moved keys in amber");
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&target, "pointerdown", false, false, 1, (60, 60));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = runtime.model().accepted.expect("an accepted document");
        let anchor = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dep")
            .anchor
            .clone()
            .expect("still anchored");
        assert_eq!(
            (anchor.matrix_id.as_str(), anchor.row, anchor.column),
            ("matrix", 0, 2),
            "the pick retargets the anchor"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the pick lands as one revision"
        );
        assert!(
            pick_ring_values(&root).is_empty(),
            "committing leaves pick mode"
        );
        let toast = wait_for_pick_toast(&runtime, &root).await;
        let text = toast.text_content().unwrap_or_default();
        assert!(
            text.contains("Dependent anchored to matrix R1C3")
                && text.contains("(was matrix R1C1)")
                && text.contains("1 Undo step"),
            "the toast carries the item and both anchors, got {text:?}"
        );
        fixture::undo(&runtime).await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let anchor = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dep")
            .anchor
            .clone()
            .expect("still anchored");
        assert_eq!(
            (anchor.matrix_id.as_str(), anchor.row, anchor.column),
            ("matrix", 0, 0),
            "Undo restores the anchor"
        );
        root.remove();
    }

    /// Ticket 08: a matrix whose anchor chain reaches the owner draws no
    /// ring: re-anchoring there would close a loop.
    #[wasm_bindgen_test]
    async fn matrix_pick_hides_loop_forming_targets() {
        use boardstudio_core::model::*;
        let mut document = anchored_drag_document(Follow {
            x: true,
            y: true,
            rotation: true,
        });
        document
            .layouts
            .iter_mut()
            .find(|layout| layout.id == "layout")
            .expect("the pick layout")
            .matrix_ids
            .push("looped".into());
        let looped_key = "matrix/looped/r0c0";
        document.boards[0].part_ids.push(looped_key.into());
        document.parts.push(Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: looped_key.into(),
            definition_id: "switch:base".into(),
            reference: "K9".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: -38.1 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
        document.matrices.push(Matrix {
            id: "looped".into(),
            name: Some("Looped".into()),
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 0.0, y: -38.1 },
            definition_id: "switch:base".into(),
            part_ids: vec![looped_key.into()],
            board_id: Some(fixture::BOARD_ID.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![ColumnLayout::default()],
            anchor: Some(MatrixAnchor {
                matrix_id: "dep".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            }),
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: Some("switch:base".into()),
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }],
        });
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dep")
            .clone();
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == "dep")
            .expect("the dep scene")
            .clone();
        let context = TreeContext::Matrix {
            matrix_id: "dep".into(),
        };
        let root = mount_pick_overlay(
            &runtime,
            AnchorPickOwner::Matrix {
                matrix: Rc::new(matrix),
                projection: Rc::new(projection),
                context: context.clone(),
            },
            context,
        )
        .await;
        arm_pick_via_token(&root).await;
        assert_eq!(
            pick_ring_values(&root),
            vec![
                "matrix:0:0".to_owned(),
                "matrix:0:1".to_owned(),
                "matrix:0:2".to_owned(),
            ],
            "only the loop-free keys draw rings"
        );
        assert!(
            root.query_selector("[data-anchor=\"looped:0:0\"]")
                .unwrap()
                .is_none(),
            "the loop-forming matrix draws no ring"
        );
        root.remove();
    }

    /// Ticket 08: Escape leaves pick mode with no revision and no toast.
    #[wasm_bindgen_test]
    async fn matrix_pick_escape_leaves_no_revision() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dep")
            .clone();
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == "dep")
            .expect("the dep scene")
            .clone();
        let context = TreeContext::Matrix {
            matrix_id: "dep".into(),
        };
        let root = mount_pick_overlay(
            &runtime,
            AnchorPickOwner::Matrix {
                matrix: Rc::new(matrix),
                projection: Rc::new(projection),
                context: context.clone(),
            },
            context,
        )
        .await;
        let revision = fixture_revision(&runtime);
        arm_pick_via_token(&root).await;
        assert!(
            !pick_ring_values(&root).is_empty(),
            "the token press arms pick mode"
        );
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        root.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert!(
            pick_ring_values(&root).is_empty(),
            "Escape leaves pick mode"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision,
            "Escape commits nothing"
        );
        assert!(
            root.query_selector(".m1-feedback-toast").unwrap().is_none(),
            "Escape raises no toast"
        );
        root.remove();
    }

    /// Ticket 08: the anchored part's rail ends in the Anchor token; picking
    /// a key moves the part with its carried values and Undo restores.
    #[wasm_bindgen_test]
    async fn part_pick_commits_with_carried_values_and_undo_restores() {
        let runtime = fixture::open_part_runtime((5.0, 2.0)).await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Component {
            part_id: Some("mcu".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        let root = mount_pick_overlay(
            &runtime,
            AnchorPickOwner::Part {
                part_id: "mcu".into(),
            },
            context,
        )
        .await;
        assert!(
            pick_token(&root).is_some(),
            "the part rail ends in the Anchor token"
        );
        arm_pick_via_token(&root).await;
        assert_eq!(
            pick_ring_values(&root),
            vec![
                "matrix:0:0".to_owned(),
                "matrix:0:1".to_owned(),
                "matrix:0:2".to_owned(),
            ],
            "the mcu lists its layout keys"
        );
        assert_eq!(
            ring_circle_radii(&pick_ring(&root, "matrix:0:1")),
            vec!["12".to_owned(), "2.25".to_owned(), "1.6".to_owned()],
            "the part's current anchor fills a white dot"
        );
        let target = pick_ring(&root, "matrix:0:0");
        dispatch_pointer(&target, "pointerenter", false, false, 0, (60, 60));
        gloo_timers::future::TimeoutFuture::new(120).await;
        assert!(
            root.query_selector_all(".m1-anchor-pick-preview")
                .unwrap()
                .length()
                > 0,
            "hovering previews where the part would move"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&target, "pointerdown", false, false, 1, (60, 60));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(
            (anchor.matrix_id.as_str(), anchor.row, anchor.column),
            ("matrix", 0, 0),
            "the pick retargets the part anchor"
        );
        assert_eq!(
            (anchor.spread, anchor.stagger),
            (5.0, 2.0),
            "the carried values ride along"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the pick lands as one revision"
        );
        let toast = wait_for_pick_toast(&runtime, &root).await;
        let text = toast.text_content().unwrap_or_default();
        assert!(
            text.contains("U1 anchored to matrix R1C1")
                && text.contains("(was matrix R1C2)")
                && text.contains("1 Undo step"),
            "the toast carries the part and both anchors, got {text:?}"
        );
        fixture::undo(&runtime).await;
        assert_eq!(
            part_anchor_of(&runtime, "mcu").column,
            1,
            "Undo restores the part anchor"
        );
        root.remove();
    }

    /// Ticket 08: a detached part arms pick with the half origin current and
    /// anchors from the first ring click.
    #[wasm_bindgen_test]
    async fn detached_part_pick_anchors_from_the_half_origin() {
        let runtime = fixture::open_detached_part_runtime().await;
        wait_until_editable(&runtime).await;
        let context = TreeContext::Component {
            part_id: Some("mcu".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        let root = mount_pick_overlay(
            &runtime,
            AnchorPickOwner::Part {
                part_id: "mcu".into(),
            },
            context,
        )
        .await;
        arm_pick_via_token(&root).await;
        assert_eq!(
            pick_ring_values(&root),
            vec![
                "matrix:0:0".to_owned(),
                "matrix:0:1".to_owned(),
                "matrix:0:2".to_owned(),
            ],
            "a detached part can anchor to any layout key"
        );
        let majors = root.query_selector_all(".m1-anchor-pick-major").unwrap();
        let half_origin = (0..majors.length())
            .filter_map(|index| majors.item(index))
            .find(|major| major.text_content().as_deref() == Some("Half origin"))
            .expect("the half origin major is rendered")
            .dyn_into::<web_sys::Element>()
            .unwrap();
        assert!(
            half_origin
                .query_selector("circle[r='2.2'][style*='fill:#fff']")
                .unwrap()
                .is_some(),
            "the half origin marks the current placement with a white dot"
        );
        let revision = fixture_revision(&runtime);
        let target = pick_ring(&root, "matrix:0:0");
        dispatch_pointer(&target, "pointerdown", false, false, 1, (60, 60));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(
            (anchor.matrix_id.as_str(), anchor.row, anchor.column),
            ("matrix", 0, 0),
            "the detached part anchors"
        );
        root.remove();
    }

    /// Ticket 08: an unanchored matrix draws no Anchor token: there is no
    /// anchor to retarget.
    #[wasm_bindgen_test]
    async fn unanchored_matrix_draws_no_anchor_token() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == fixture::MATRIX_ID)
            .expect("the main matrix")
            .clone();
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == fixture::MATRIX_ID)
            .expect("the main scene")
            .clone();
        let context = TreeContext::Matrix {
            matrix_id: fixture::MATRIX_ID.into(),
        };
        let root = mount_pick_overlay(
            &runtime,
            AnchorPickOwner::Matrix {
                matrix: Rc::new(matrix),
                projection: Rc::new(projection),
                context: context.clone(),
            },
            context,
        )
        .await;
        assert!(
            pick_token(&root).is_none(),
            "an unanchored matrix offers no Anchor token"
        );
        root.remove();
    }
}

// The modifier changes presentation immediately; pointer-down latches the edit intent.
struct CtrlModifierListener {
    document: web_sys::Document,
    window: web_sys::Window,
    down: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    up: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    blur: Closure<dyn FnMut(web_sys::Event)>,
}
impl CtrlModifierListener {
    fn install(mut pressed: Signal<bool>) -> Option<Self> {
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
