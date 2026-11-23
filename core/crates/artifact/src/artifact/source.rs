//! Source-preserving KiCad footprint import and placement patching.
//!
//! Imported source is the editable artifact. Geometry returned by the importer
//! is only a preview projection; callers must retain `KicadSource.source` for
//! export and must not regenerate an imported footprint from that projection.

use super::sexpr::{self, format_decimal as num, is_back_layer};
use super::source_geometry::project_source_geometry;
use crate::model::{
    ArtifactDiagnostic, ArtifactDiagnosticKind, ArtifactError, ArtifactErrorCode,
    CompiledFootprint, EnvelopeOrigin, EnvelopeSource, FootprintGeometry, FootprintPatch,
    PartDefinition, PartKind, Side, Vec2,
};
use kiutils_sexpr::{Node, Span};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceError(String);

impl SourceError {
    fn artifact(self, code: ArtifactErrorCode) -> ArtifactError {
        ArtifactError::new(code, self.0)
    }
}

/// Imports the footprint and retains the complete original source.
pub(super) fn import_footprint(source: &str, id: &str) -> Result<CompiledFootprint, ArtifactError> {
    let parse_error = |message: String| ArtifactError::new(ArtifactErrorCode::ParseError, message);
    let doc = kiutils_sexpr::parse_one(source).map_err(|error| parse_error(error.to_string()))?;
    let root = doc
        .nodes
        .first()
        .ok_or_else(|| parse_error("Expected a KiCad footprint".into()))?;
    if !matches!(sexpr::head(root), Some("footprint" | "module")) {
        return Err(parse_error("Expected a KiCad footprint".into()));
    }
    let root_items = sexpr::items(root).expect("footprint root is a list");
    let name_node = root_items
        .get(1)
        .ok_or_else(|| parse_error("Footprint needs a name".into()))?;
    let name = sexpr::decoded_text(source, name_node)
        .ok_or_else(|| parse_error("Footprint needs a name".into()))?;

    let mut diagnostics = Vec::new();
    for model in sexpr::children(root, "model") {
        let model_name = sexpr::items(model)
            .and_then(|parts| parts.get(1))
            .and_then(|node| sexpr::decoded_text(source, node))
            .unwrap_or_else(|| "external 3D model".into());
        diagnostics.push(ArtifactDiagnostic {
            kind: ArtifactDiagnosticKind::UnavailableModel,
            message: format!("3D model reference preserved; asset is not bundled: {model_name}"),
            target_id: None,
            source_start: None,
            source_end: None,
        });
    }

    let projection = project_source_geometry(source, root)?;
    diagnostics.extend(projection.diagnostics);
    let fallback = projection.courtyard.is_empty();
    let envelope = if fallback {
        projection.conservative_bounds
    } else {
        projection.courtyard
    };
    let definition = PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.to_owned(),
        name,
        kind: PartKind::Custom,
        keycap: None,
        envelope_source: Some(EnvelopeSource {
            courtyard: Some(if fallback {
                EnvelopeOrigin::Generated
            } else {
                EnvelopeOrigin::Authored
            }),
            keycap: None,
        }),
        terminals: BTreeMap::new(),
        matrix_terminals: None,
        envelope_notice: fallback.then(|| {
            if envelope.is_empty() {
                "Add an authored layout envelope; source has no reliable footprint bounds".into()
            } else {
                "Preview envelope uses conservative physical bounds".into()
            }
        }),
        courtyard: envelope.clone(),
        pads: projection.pads.clone(),
        models: None,
        generator: None,
        mechanical_profile: None,
        kicad_source: Some(crate::model::KicadSource {
            format_version: 1,
            source: source.to_owned(),
        }),
    };
    Ok(CompiledFootprint {
        definition,
        geometry: FootprintGeometry {
            side: Side::Front,
            courtyard: envelope,
            pads: projection.pads,
            traces: Vec::new(),
            vias: Vec::new(),
        },
        diagnostics,
        preview_svg: None,
    })
}

pub(super) fn render_form(form: &str) -> Result<String, ArtifactError> {
    let doc = kiutils_sexpr::parse_one(form)
        .map_err(|error| SourceError(error.to_string()).artifact(ArtifactErrorCode::ParseError))?;
    let rendered = doc.to_canonical_string();
    Ok(rendered.trim_end().to_owned())
}

pub(super) fn patch_footprint(
    source: &str,
    patch: &FootprintPatch,
) -> Result<String, ArtifactError> {
    let doc = kiutils_sexpr::parse_one(source)
        .map_err(|error| ArtifactError::new(ArtifactErrorCode::ParseError, error.to_string()))?;
    let root = doc
        .nodes
        .first()
        .ok_or_else(|| ArtifactError::new(ArtifactErrorCode::ParseError, "Expected footprint"))?;
    let source_back = is_back_layer(root);
    let target_back = matches!(patch.side, Side::Back);
    let normalized = if patch.placement.is_some() || source_back != target_back {
        upgrade_legacy_arcs(source)?
    } else {
        source.to_owned()
    };
    let normalized = remove_text_render_caches(&normalized)?;
    patch_footprint_inner(&normalized, patch)
        .map_err(|error| error.artifact(ArtifactErrorCode::Unsupported))
}

// Cached glyph polygons depend on the old text and pose. KiCad regenerates
// them from the retained text/effects; transforming stale caches is unsafe.
fn remove_text_render_caches(source: &str) -> Result<String, ArtifactError> {
    fn visit(node: &Node, edits: &mut Vec<(Span, String)>) {
        if sexpr::head(node) == Some("render_cache") {
            edits.push((sexpr::span(node), String::new()));
        } else if let Some(items) = sexpr::items(node) {
            for child in items {
                visit(child, edits);
            }
        }
    }
    let document = kiutils_sexpr::parse_one(source)
        .map_err(|error| ArtifactError::new(ArtifactErrorCode::ParseError, error.to_string()))?;
    let mut edits = Vec::new();
    for node in &document.nodes {
        visit(node, &mut edits);
    }
    sexpr::replace_spans(source, edits).ok_or_else(|| {
        ArtifactError::new(
            ArtifactErrorCode::ParseError,
            "Overlapping text render caches",
        )
    })
}

pub fn upgrade_legacy_arcs(source: &str) -> Result<String, ArtifactError> {
    fn visit(node: &Node, edits: &mut Vec<(Span, String)>) -> Result<(), ArtifactError> {
        let Some(items) = sexpr::items(node) else {
            return Ok(());
        };
        let head = sexpr::head(node).unwrap_or("");
        if matches!(head, "fp_arc" | "gr_arc")
            && let (Some(center_node), Some(endpoint_node), Some(angle_node)) = (
                sexpr::child(node, "start"),
                sexpr::child(node, "end"),
                sexpr::child(node, "angle"),
            )
        {
            let center = raw_point(center_node)?;
            let endpoint = raw_point(endpoint_node)?;
            let angle = sexpr::items(angle_node)
                .and_then(|parts| parts.get(1))
                .and_then(sexpr::atom)
                .ok_or_else(|| {
                    ArtifactError::new(ArtifactErrorCode::ParseError, "Legacy arc needs angle")
                })?
                .parse::<f64>()
                .map_err(|_| {
                    ArtifactError::new(ArtifactErrorCode::ParseError, "Invalid legacy arc angle")
                })?;
            if !angle.is_finite() {
                return Err(ArtifactError::new(
                    ArtifactErrorCode::ParseError,
                    "Invalid non-finite legacy arc angle",
                ));
            }
            let dx = endpoint.x - center.x;
            let dy = endpoint.y - center.y;
            let radius = dx.hypot(dy);
            if radius <= 0.0 {
                return Err(ArtifactError::new(
                    ArtifactErrorCode::ParseError,
                    "Degenerate legacy arc",
                ));
            }
            let start_angle = dy.atan2(dx);
            let sweep = angle.to_radians();
            let at_angle = |fraction: f64| {
                let angle = start_angle + sweep * fraction;
                Vec2 {
                    x: center.x + radius * angle.cos(),
                    y: center.y + radius * angle.sin(),
                }
            };
            let start_span = sexpr::span(center_node);
            let end_span = sexpr::span(endpoint_node);
            edits.push((
                start_span,
                format!("(start {} {})", num(endpoint.x), num(endpoint.y)),
            ));
            edits.push((
                Span {
                    start: start_span.end,
                    end: start_span.end,
                },
                format!(" (mid {} {})", num(at_angle(0.5).x), num(at_angle(0.5).y)),
            ));
            let end = at_angle(1.0);
            edits.push((end_span, format!("(end {} {})", num(end.x), num(end.y))));
            edits.push((sexpr::span(angle_node), String::new()));
        }
        for child in items.iter().skip(1) {
            visit(child, edits)?;
        }
        Ok(())
    }

    fn raw_point(node: &Node) -> Result<Vec2, ArtifactError> {
        let parse = |index| {
            sexpr::atom_at(node, index)
                .and_then(|value| sexpr::parse_finite(value).ok())
                .ok_or_else(|| {
                    ArtifactError::new(ArtifactErrorCode::ParseError, "Invalid legacy arc point")
                })
        };
        Ok(Vec2 {
            x: parse(1)?,
            y: parse(2)?,
        })
    }

    let doc = kiutils_sexpr::parse_one(source)
        .map_err(|error| ArtifactError::new(ArtifactErrorCode::ParseError, error.to_string()))?;
    let mut edits = Vec::new();
    for node in &doc.nodes {
        visit(node, &mut edits)?;
    }
    if edits.is_empty() {
        return Ok(source.to_owned());
    }
    sexpr::replace_spans(source, edits).ok_or_else(|| {
        ArtifactError::new(
            ArtifactErrorCode::ParseError,
            "Overlapping legacy arc source spans",
        )
    })
}

fn patch_footprint_inner(source: &str, patch: &FootprintPatch) -> Result<String, SourceError> {
    let doc = kiutils_sexpr::parse_one(source).map_err(|error| SourceError(error.to_string()))?;
    let root = doc
        .nodes
        .first()
        .ok_or_else(|| SourceError("Expected footprint".into()))?;
    if !matches!(sexpr::head(root), Some("footprint" | "module")) {
        return Err(SourceError("Expected footprint".into()));
    }
    let source_back = is_back_layer(root);
    let target_back = matches!(patch.side, Side::Back);
    let needs_side_flip = source_back != target_back;
    let source_rotation = sexpr::child(root, "at")
        .and_then(|at| sexpr::atom_at(at, 3))
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let mut edits: Vec<(Span, String)> = Vec::new();
    let children = sexpr::items(root).expect("list");
    if let (Some(name), Some(source_name)) = (patch.footprint_name.as_deref(), children.get(1)) {
        edits.push((sexpr::span(source_name), sexpr::quote(name)));
    }
    if let Some(placement) = patch.placement
        && (!placement.at.x.is_finite()
            || !placement.at.y.is_finite()
            || !placement.rotation.is_finite()
            || placement.at.x.abs() > boardstudio_contracts::MAX_COORDINATE_MM
            || placement.at.y.abs() > boardstudio_contracts::MAX_COORDINATE_MM)
    {
        return Err(SourceError(
            "Footprint placement is outside the supported coordinate range".into(),
        ));
    }
    let theta = patch
        .placement
        .map(|placement| placement.rotation)
        .unwrap_or(0.0);
    if patch.placement.is_some() || sexpr::child(root, "at").is_some() || target_back {
        let (x, y) = patch
            .placement
            .map(|placement| (placement.at.x, placement.at.y))
            .unwrap_or((0.0, 0.0));
        let rotation = if target_back { theta - 180.0 } else { theta };
        let at_text = format!("(at {} {} {})", num(x), num(-y), num(rotation));
        if let Some(at) = sexpr::child(root, "at") {
            edits.push((sexpr::span(at), at_text));
        } else {
            let name = children
                .get(1)
                .ok_or_else(|| SourceError("Footprint needs a name".into()))?;
            let name_span = sexpr::span(name);
            edits.push((
                Span {
                    start: name_span.end,
                    end: name_span.end,
                },
                format!(" {at_text}"),
            ));
        }
    }
    let mut patched_reference = false;
    let mut patched_value = false;
    for prop in children
        .iter()
        .filter(|node| matches!(sexpr::head(node), Some("property" | "fp_text")))
    {
        let p = sexpr::items(prop).unwrap_or_default();
        let key_node = p.get(1);
        let key = key_node
            .and_then(|node| sexpr::decoded_text(source, node))
            .unwrap_or_default();
        let (replacement, patched) = match key.as_str() {
            "Reference" | "reference" => (patch.reference.as_deref(), &mut patched_reference),
            "Value" | "value" => (patch.value.as_deref(), &mut patched_value),
            _ => (None, &mut patched_reference),
        };
        if let (Some(value), Some(node)) = (replacement, p.get(2)) {
            edits.push((sexpr::span(node), sexpr::quote(value)));
            *patched = true;
        }
    }
    for (name, value, patched, layer) in [
        (
            "Reference",
            patch.reference.as_deref(),
            patched_reference,
            "F.SilkS",
        ),
        ("Value", patch.value.as_deref(), patched_value, "F.Fab"),
    ] {
        if let Some(value) = value.filter(|_| !patched) {
            let end = sexpr::span(root).end - 1;
            edits.push((Span { start: end, end }, format!(" (property \"{name}\" {} (at 0 0 0) (layer \"{layer}\") (effects (font (size 1 1) (thickness 0.15))))", sexpr::quote(value))));
        }
    }
    let source_pads: Vec<_> = sexpr::children(root, "pad").collect();
    let mut ids = BTreeMap::new();
    let mut numbers = BTreeMap::<String, Option<(u32, String)>>::new();
    for (index, pad) in source_pads.iter().enumerate() {
        let id = format!("pad-{index}");
        ids.insert(id.clone(), true);
        let number_node = sexpr::items(pad).and_then(|items| items.get(1));
        let number = number_node
            .and_then(|node| sexpr::decoded_text(source, node))
            .unwrap_or_default();
        let net = patch.pad_nets.get(&id).cloned();
        if let Some(previous) = numbers.insert(number.clone(), net.clone())
            && previous != net
        {
            return Err(SourceError(format!(
                "Repeated logical pad number {number} has conflicting net assignments"
            )));
        }
        if let Some(old_net) = sexpr::child(pad, "net") {
            if let Some((code, name)) = net {
                edits.push((
                    sexpr::span(old_net),
                    format!("(net {code} {})", sexpr::quote(&name)),
                ));
            } else {
                edits.push((sexpr::span(old_net), String::new()));
            }
        } else if let Some((code, name)) = net {
            let span = sexpr::span(pad);
            edits.push((
                Span {
                    start: span.end - 1,
                    end: span.end - 1,
                },
                format!(" (net {code} {})", sexpr::quote(&name)),
            ));
        }
    }
    if patch.pad_nets.keys().any(|id| !ids.contains_key(id)) {
        return Err(SourceError(
            "Net assignment references an unknown physical pad id".into(),
        ));
    }
    if patch.placement.is_some() || needs_side_flip {
        reject_unknown_spatial(root)?;
    }
    patch_placement(
        root,
        theta,
        source_rotation,
        source_back,
        target_back,
        &mut edits,
    );
    if needs_side_flip {
        patch_layers(root, &mut edits);
    }
    remap_uuids(root, &patch.uuid_scope, &mut edits);
    let mut patched = sexpr::replace_spans(source, edits).ok_or_else(|| {
        SourceError("Overlapping or invalid source spans while patching footprint".into())
    })?;
    if !patch.model_forms.is_empty() {
        let rendered = patch
            .model_forms
            .iter()
            .map(|form| render_form(form).map_err(|error| SourceError(error.message)))
            .collect::<Result<Vec<_>, _>>()?;
        let insert_at = patched
            .rfind(')')
            .ok_or_else(|| SourceError("Footprint has no closing parenthesis".into()))?;
        let insertion = rendered
            .iter()
            .map(|form| format!("\n  {form}"))
            .collect::<String>();
        patched.insert_str(insert_at, &insertion);
    }
    Ok(patched)
}

fn reject_unknown_spatial(root: &Node) -> Result<(), SourceError> {
    let known = [
        "footprint",
        "module",
        "property",
        "fp_text",
        "fp_line",
        "fp_arc",
        "fp_circle",
        "fp_rect",
        "fp_poly",
        "fp_curve",
        "gr_line",
        "gr_arc",
        "gr_circle",
        "gr_rect",
        "gr_poly",
        "gr_curve",
        "pad",
        "model",
        "at",
        "layer",
        "layers",
        "uuid",
        "tstamp",
        "descr",
        "tags",
        "attr",
        "path",
        "locked",
        "solder_mask_margin",
        "solder_paste_margin",
        "solder_paste_margin_ratio",
        "clearance",
        "thermal_bridge_width",
        "thermal_gap",
        "options",
        "primitives",
        "roundrect_rratio",
        "rect_delta",
        "chamfer_ratio",
        "drill",
        "offset",
        "size",
        "start",
        "end",
        "mid",
        "center",
        "pts",
        "xy",
        "stroke",
        "width",
        "type",
        "fill",
        "effects",
        "font",
        "thickness",
        "justify",
        "hide",
        "yes",
        "no",
        "xyz",
        "scale",
        "rotate",
        "net",
        "pinfunction",
        "pintype",
        "zone_connect",
        "thermal_width",
        "remove_unused_layers",
        "keep_end_layers",
        "chamfer",
        "ki_locked",
    ];
    fn visit(node: &Node, known: &[&str]) -> Result<(), SourceError> {
        if let Some(items) = sexpr::items(node) {
            if let Some(head) = items.first().and_then(sexpr::atom)
                && !known.contains(&head)
                && (spatial_form(head) || contains_spatial_coordinates(node))
            {
                return Err(SourceError(format!(
                    "Cannot transform unsupported spatial form: {head}"
                )));
            }
            for child in items {
                visit(child, known)?;
            }
        }
        Ok(())
    }
    visit(root, &known)
}

fn spatial_form(name: &str) -> bool {
    [
        "at", "start", "end", "mid", "center", "xy", "pts", "offset", "rotate", "scale", "xyz",
        "size", "drill",
    ]
    .contains(&name)
        || name.starts_with("fp_")
        || name == "pad"
}

fn contains_spatial_coordinates(node: &Node) -> bool {
    sexpr::items(node).is_some_and(|items| {
        items.iter().skip(1).any(|child| {
            sexpr::head(child).is_some_and(|name| {
                matches!(
                    name,
                    "at" | "start"
                        | "end"
                        | "mid"
                        | "center"
                        | "xy"
                        | "pts"
                        | "offset"
                        | "rotate"
                        | "xyz"
                        | "scale"
                        | "size"
                        | "drill"
                        | "rect_delta"
                        | "chamfer"
                        | "primitives"
                        | "polygon"
                        | "bezier"
                        | "curve"
                )
            }) || contains_spatial_coordinates(child)
        })
    })
}

fn patch_layers(root: &Node, edits: &mut Vec<(Span, String)>) {
    fn visit(node: &Node, edits: &mut Vec<(Span, String)>) {
        if let Some(items) = sexpr::items(node) {
            if matches!(sexpr::head(node), Some("layer" | "layers")) {
                for atom in items.iter().skip(1) {
                    if let Some(value) = sexpr::atom(atom) {
                        let mapped = flip_layer(value);
                        if mapped != value {
                            edits.push((sexpr::span(atom), sexpr::quote(&mapped)));
                        }
                    }
                }
            }
            if sexpr::head(node) == Some("effects") {
                let justifies: Vec<_> = sexpr::children(node, "justify").collect();
                if let Some(justify) = justifies.first() {
                    if let Some(mirror) = sexpr::items(justify).and_then(|parts| {
                        parts
                            .iter()
                            .skip(1)
                            .find(|item| sexpr::atom(item) == Some("mirror"))
                    }) {
                        edits.push((sexpr::span(mirror), String::new()));
                    } else {
                        let span = sexpr::span(justify);
                        edits.push((
                            Span {
                                start: span.end - 1,
                                end: span.end - 1,
                            },
                            " mirror".into(),
                        ));
                    }
                } else {
                    let span = sexpr::span(node);
                    edits.push((
                        Span {
                            start: span.end - 1,
                            end: span.end - 1,
                        },
                        " (justify mirror)".into(),
                    ));
                }
            }
            if sexpr::head(node) == Some("chamfer") {
                for item in items.iter().skip(1) {
                    let Some(value) = sexpr::atom(item) else {
                        continue;
                    };
                    let mapped = match value {
                        "top_left" => "bottom_left",
                        "bottom_left" => "top_left",
                        "top_right" => "bottom_right",
                        "bottom_right" => "top_right",
                        _ => value,
                    };
                    if mapped != value {
                        edits.push((sexpr::span(item), mapped.into()));
                    }
                }
            }
            for child in items {
                visit(child, edits);
            }
        }
    }
    visit(root, edits);
}

/// Rewrite an absolute `(at x y angle)` placement for the target side and
/// rotation, appending the angle when the source omits it.
fn patch_at_angle(
    parts: &[Node],
    theta: f64,
    source_rotation: f64,
    source_back: bool,
    target_back: bool,
    edits: &mut Vec<(Span, String)>,
) {
    let source_absolute = parts
        .get(3)
        .and_then(sexpr::atom)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let relative = source_absolute - source_rotation;
    let local = if source_back { -relative } else { relative };
    let absolute = if target_back {
        theta + 180.0 - local
    } else {
        theta + local
    };
    if let Some(angle) = parts.get(3) {
        edits.push((sexpr::span(angle), num(absolute)));
    } else if let Some(y) = parts.get(2) {
        let span = sexpr::span(y);
        edits.push((
            Span {
                start: span.end,
                end: span.end,
            },
            format!(" {}", num(absolute)),
        ));
    }
}

fn patch_placement(
    root: &Node,
    theta: f64,
    source_rotation: f64,
    source_back: bool,
    target_back: bool,
    edits: &mut Vec<(Span, String)>,
) {
    fn visit(
        node: &Node,
        reflect: bool,
        patch_angle: &impl Fn(&[Node], &mut Vec<(Span, String)>),
        edits: &mut Vec<(Span, String)>,
    ) {
        let Some(items) = sexpr::items(node) else {
            return;
        };
        let name = sexpr::head(node).unwrap_or("");
        if name == "model" {
            return;
        }
        let absolute_at = matches!(name, "pad" | "fp_text" | "property");
        if reflect
            && matches!(
                name,
                "pad"
                    | "fp_line"
                    | "fp_arc"
                    | "fp_circle"
                    | "fp_rect"
                    | "fp_poly"
                    | "fp_curve"
                    | "fp_text"
                    | "property"
                    | "gr_line"
                    | "gr_arc"
                    | "gr_circle"
                    | "gr_rect"
                    | "gr_poly"
                    | "gr_curve"
                    | "drill"
                    | "primitives"
                    | "pts"
                    | "xy"
            )
        {
            for child in items.iter().skip(1) {
                let form = sexpr::head(child);
                if !matches!(
                    form,
                    Some(
                        "at" | "start" | "end" | "mid" | "center" | "xy" | "offset" | "rect_delta"
                    )
                ) {
                    continue;
                }
                let parts = sexpr::items(child).expect("coordinate form is a list");
                negate_coordinate(parts, 2, edits);
                if form == Some("at") && absolute_at {
                    patch_angle(parts, edits);
                }
            }
            if matches!(name, "fp_arc" | "gr_arc")
                && let Some(angle) = sexpr::child(node, "angle").and_then(sexpr::items)
            {
                negate_coordinate(angle, 1, edits);
            }
        } else if !reflect
            && absolute_at
            && let Some(parts) = sexpr::child(node, "at").and_then(sexpr::items)
        {
            patch_angle(parts, edits);
        }
        for child in items.iter().skip(1) {
            visit(child, reflect, patch_angle, edits);
        }
    }

    visit(
        root,
        source_back != target_back,
        &|parts, edits| {
            patch_at_angle(
                parts,
                theta,
                source_rotation,
                source_back,
                target_back,
                edits,
            );
        },
        edits,
    );
}

fn negate_coordinate(parts: &[Node], index: usize, edits: &mut Vec<(Span, String)>) {
    if let Some(token) = parts.get(index)
        && let Some(value) = sexpr::atom(token).and_then(|value| value.parse::<f64>().ok())
    {
        edits.push((sexpr::span(token), num(-value)));
    }
}

fn flip_layer(value: &str) -> String {
    if let Some(rest) = value.strip_prefix('F') {
        format!("B{rest}")
    } else if let Some(rest) = value.strip_prefix('B') {
        format!("F{rest}")
    } else {
        value.to_owned()
    }
}

fn remap_uuids(root: &Node, scope: &str, edits: &mut Vec<(Span, String)>) {
    fn collect(
        node: &Node,
        scope: &str,
        in_group: bool,
        path: &str,
        mapping: &mut BTreeMap<String, String>,
        edits: &mut Vec<(Span, String)>,
    ) {
        if let Some(items) = sexpr::items(node) {
            let name = sexpr::head(node).unwrap_or("");
            let group = in_group || name == "group";
            if (matches!(name, "uuid" | "tstamp") || (group && name == "id"))
                && let Some(value) = items.get(1).and_then(sexpr::atom)
            {
                let mapped = deterministic_uuid(&format!("{scope}:{}", value));
                mapping.insert(value.to_owned(), mapped.clone());
                edits.push((sexpr::span(&items[1]), sexpr::quote(&mapped)));
            }
            let is_object = matches!(
                name,
                "footprint"
                    | "module"
                    | "pad"
                    | "fp_line"
                    | "fp_arc"
                    | "fp_circle"
                    | "fp_rect"
                    | "fp_poly"
                    | "fp_curve"
                    | "fp_text"
                    | "property"
                    | "group"
            );
            if is_object
                && sexpr::child(node, "uuid").is_none()
                && sexpr::child(node, "tstamp").is_none()
                && !(group && sexpr::child(node, "id").is_some())
            {
                let generated = deterministic_uuid(&format!("{scope}:object:{path}:{name}"));
                let span = sexpr::span(node);
                edits.push((
                    Span {
                        start: span.end - 1,
                        end: span.end - 1,
                    },
                    format!(" (uuid {})", sexpr::quote(&generated)),
                ));
            }
            for (index, child) in items.iter().enumerate() {
                collect(
                    child,
                    scope,
                    group,
                    &format!("{path}/{index}"),
                    mapping,
                    edits,
                );
            }
        }
    }
    fn refs(node: &Node, mapping: &BTreeMap<String, String>, edits: &mut Vec<(Span, String)>) {
        if let Some(items) = sexpr::items(node) {
            if sexpr::head(node) == Some("members") {
                for child in items.iter().skip(1) {
                    if let Some(old) = sexpr::atom(child)
                        && let Some(new) = mapping.get(old)
                    {
                        edits.push((sexpr::span(child), sexpr::quote(new)));
                    }
                }
            }
            for child in items {
                refs(child, mapping, edits);
            }
        }
    }
    let mut mapping = BTreeMap::new();
    collect(root, scope, false, "root", &mut mapping, edits);
    refs(root, &mapping, edits);
}

fn deterministic_uuid(value: &str) -> String {
    let mut words = [0x811c9dc5u32, 0x91f832ec, 0xd1f5ab43, 0x67ed3a21];
    for ch in value.chars() {
        let code = ch as u32;
        for (index, word) in words.iter_mut().enumerate() {
            *word = (*word ^ code.wrapping_add(index as u32)).wrapping_mul(0x01000193);
        }
    }
    let hex: String = words.iter().map(|word| format!("{word:08x}")).collect();
    format!(
        "{}-{}-5{}-a{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Vec2;

    const SOURCE: &str = "(footprint \"Part\" (layer \"F.Cu\") (at 0 0 0) (uuid \"original-root-id\")\n  (property \"Reference\" \"REF**\" (at 0 1 0) (layer \"F.SilkS\") (effects (font (size 1 1) (thickness 0.15))))\n  (property \"Value\" \"Part\" (at 0 -1 0) (layer \"F.Fab\") (effects (font (size 1 1) (thickness 0.15))))\n  (pad \"1\" smd rect (at 1 2 30) (size 1 2) (layers \"F.Cu\" \"F.Paste\" \"F.Mask\") (net 4 \"old\") (uuid \"pad-one-id\"))\n  (pad \"1\" np_thru_hole oval (at 3 4) (size 1 2) (drill oval 0.4 0.8 (offset 0.1 0.2)) (layers \"*.Cu\" \"*.Mask\") (uuid \"pad-two-id\"))\n  (mystery \"keep this source text verbatim\")\n)";

    fn patch(side: Side) -> FootprintPatch {
        FootprintPatch {
            reference: Some("U1".into()),
            value: Some("café \"Q\"\\line\nnext".into()),
            footprint_name: None,
            placement: Some(crate::model::Pose2 {
                at: Vec2 { x: 12.5, y: -4.25 },
                rotation: 90.0,
            }),
            side,
            pad_nets: BTreeMap::new(),
            uuid_scope: "board:part:u1".into(),
            model_forms: Vec::new(),
        }
    }

    #[test]
    fn import_preserves_source_and_projects_duplicate_physical_pads() {
        let compiled = import_footprint(SOURCE, "definition").expect("footprint imports");
        assert_eq!(
            compiled.definition.kicad_source.as_ref().unwrap().source,
            SOURCE
        );
        assert_eq!(
            compiled
                .geometry
                .pads
                .iter()
                .map(|pad| pad.id.as_str())
                .collect::<Vec<_>>(),
            ["pad-0", "pad-1"]
        );
        assert_eq!(
            compiled.geometry.pads[0].number,
            compiled.geometry.pads[1].number
        );
        assert_eq!(compiled.geometry.pads[0].rotation, Some(30.0));
        assert_eq!(compiled.geometry.pads[1].plated, Some(false));
    }

    #[test]
    fn quoted_footprint_names_and_pad_numbers_use_kicad_escapes() {
        let source = "(footprint \"café\\npart\" (layer \"F.Cu\") (pad \"A\\t1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\")))";
        let imported = import_footprint(source, "escaped").expect("escaped strings project");
        assert_eq!(imported.definition.name, "café\npart");
        assert_eq!(imported.definition.pads[0].number, "A\t1");
        assert_eq!(
            imported.definition.kicad_source.as_ref().unwrap().source,
            source
        );
    }

    #[test]
    fn placed_front_and_canonical_back_match_kicad_rotation_oracle() {
        let source = "(footprint \"Asymmetric\" (layer \"F.Cu\") (pad \"1\" smd rect (at 2 3 30) (size 1 2) (layers \"F.Cu\" \"F.Mask\")))";
        let mut front = patch(Side::Front);
        front.placement = Some(crate::model::Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 37.0,
        });
        let front_output = patch_footprint(source, &front).unwrap();
        assert!(front_output.contains("(at 11 -13 37)"));
        assert!(front_output.contains("(at 2 3 67)"));

        let mut back = front;
        back.side = Side::Back;
        let back_output = patch_footprint(source, &back).unwrap();
        assert!(back_output.contains("(at 11 -13 -143)"));
        assert!(back_output.contains("(at 2 -3 187)"));
    }

    #[test]
    fn absolute_source_angles_are_normalized_from_existing_root_rotation() {
        let source = "(footprint \"Asymmetric\" (layer \"F.Cu\") (at 0 0 20) (pad \"1\" smd rect (at 2 3 50) (size 1 2) (layers \"F.Cu\" \"F.Mask\")))";
        let mut front = patch(Side::Front);
        front.placement = Some(crate::model::Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 37.0,
        });
        let output = patch_footprint(source, &front).unwrap();
        assert!(output.contains("(at 11 -13 37)"));
        assert!(output.contains("(at 2 3 67)"));
    }

    #[test]
    fn omitted_source_pad_angle_defaults_to_absolute_zero() {
        let source = "(footprint \"A\" (layer \"F.Cu\") (at 0 0 37) (pad \"1\" smd rect (at 2 3) (size 1 1) (layers \"F.Cu\")))";
        let mut options = patch(Side::Front);
        options.placement = None;
        let output = patch_footprint(source, &options).unwrap();
        assert!(output.contains("(at 2 3 -37)"), "{output}");

        let mut placed = patch(Side::Front);
        placed.placement = Some(crate::model::Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 37.0,
        });
        let front = patch_footprint(source, &placed).unwrap();
        assert!(front.contains("(at 2 3 0)"), "{front}");
        placed.side = Side::Back;
        let back = patch_footprint(source, &placed).unwrap();
        assert!(back.contains("(at 2 -3 254)"), "{back}");
    }

    #[test]
    fn backside_reflection_negates_legacy_arc_sweep() {
        let source = "(footprint \"A\" (layer \"F.Cu\") (fp_arc (start 0 0) (end 2 0) (angle 90) (layer \"F.SilkS\") (width 0.15)))";
        let mut front = patch(Side::Front);
        front.placement = Some(crate::model::Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        });
        let front_output = patch_footprint(source, &front).unwrap();
        assert!(front_output.contains("(start 2 0)"), "{front_output}");
        assert!(
            front_output.contains("(mid 1.414214 1.414214)"),
            "{front_output}"
        );
        assert!(front_output.contains("(end 0 2)"), "{front_output}");

        let mut options = patch(Side::Back);
        options.placement = None;
        let output = patch_footprint(source, &options).unwrap();
        assert!(output.contains("(start 2 0)"), "{output}");
        assert!(output.contains("(mid 1.414214 -1.414214)"), "{output}");
        assert!(output.contains("(end 0 -2)"), "{output}");
        assert!(!output.contains("(angle "), "{output}");
    }

    #[test]
    fn backside_reflection_negates_legacy_custom_pad_arc_sweep() {
        let source = "(footprint \"A\" (layer \"F.Cu\") (pad \"1\" smd custom (at 0 0) (size 2 2) (layers \"F.Cu\") (primitives (gr_arc (start 0 0) (end 2 0) (angle 90) (width 0.1)))))";
        let mut options = patch(Side::Back);
        options.placement = None;
        let output = patch_footprint(source, &options).unwrap();
        assert!(output.contains("(start 2 0)"), "{output}");
        assert!(output.contains("(mid 1.414214 -1.414214)"), "{output}");
        assert!(output.contains("(end 0 -2)"), "{output}");
        assert!(!output.contains("(angle "), "{output}");
    }

    #[test]
    fn flips_are_based_on_root_side_and_include_mixed_layer_geometry() {
        let back_source = "(footprint \"BackPart\" (layer \"B.Cu\") (at 0 0 20) (pad \"1\" smd rect (at 2 3 50) (size 1 2) (layers \"B.Cu\" \"B.Mask\")) (property \"Reference\" \"U1\" (at 0 1 20) (layer \"B.SilkS\") (effects (font (size 1 1)) (justify mirror))))";
        let mut front = patch(Side::Front);
        front.placement = Some(crate::model::Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 37.0,
        });
        let output = patch_footprint(back_source, &front).unwrap();
        assert!(output.contains("(layer \"F.Cu\")"));
        assert!(output.contains("(at 2 -3 7)"));
        assert!(output.contains("(justify "));
        assert!(!output.contains("mirror"));

        let mixed = "(footprint \"Mixed\" (layer \"F.Cu\") (pad \"1\" smd rect (at 2 3 30) (size 1 2) (layers \"B.Cu\" \"B.Mask\")))";
        let mut back = patch(Side::Back);
        back.placement = Some(crate::model::Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 37.0,
        });
        let output = patch_footprint(mixed, &back).unwrap();
        assert!(output.contains("(at 2 -3 187)"));
        assert!(output.contains("(layers \"F.Cu\" \"F.Mask\")"));
    }

    #[test]
    fn standalone_back_source_is_normalized_to_front_local_frame() {
        let source = "(footprint \"BackPart\" (layer \"B.Cu\") (at 4 5 20) (pad \"1\" smd rect (at 2 3 50) (size 1 2) (layers \"B.Cu\" \"B.Mask\")))";
        let mut options = patch(Side::Front);
        options.placement = None;
        let output = patch_footprint(source, &options).unwrap();
        assert!(output.contains("(at 0 0 0)"));
        assert!(output.contains("(at 2 -3 -30)"), "{output}");
        assert!(output.contains("(layer \"F.Cu\")"));
    }

    #[test]
    fn placement_patch_keeps_unknown_source_and_rewrites_strings_and_uuids() {
        let output = patch_footprint(SOURCE, &patch(Side::Front)).expect("patches footprint");
        assert!(output.contains("(mystery \"keep this source text verbatim\")"));
        assert!(output.contains("(property \"Reference\" \"U1\""));
        assert!(output.contains("(property \"Value\" \"café \\\"Q\\\"\\\\line\\nnext\""));
        assert!(output.contains("(at 12.5 4.25 90)"));
        assert!(!output.contains("original-root-id"));
        assert!(output.contains("\"F.Cu\""));
    }

    #[test]
    fn backside_patch_swaps_layers_and_maps_duplicate_pad_nets() {
        let mut options = patch(Side::Back);
        options.pad_nets.insert("pad-0".into(), (3, "GND".into()));
        options.pad_nets.insert("pad-1".into(), (3, "GND".into()));
        let output = patch_footprint(SOURCE, &options).expect("backside patch");
        assert!(output.contains("(layers \"B.Cu\" \"B.Paste\" \"B.Mask\")"));
        assert!(output.contains("(net 3 \"GND\")"));
        assert!(output.contains("(justify mirror)"));
        assert!(output.contains("(at 1 -2 240)"));
        assert!(output.contains("(at 3 -4 270)"));
        assert!(output.contains("(offset 0.1 -0.2)"));
    }

    #[test]
    fn repeated_logical_pad_with_conflicting_nets_is_rejected() {
        let mut options = patch(Side::Front);
        options.pad_nets.insert("pad-0".into(), (1, "A".into()));
        options.pad_nets.insert("pad-1".into(), (2, "B".into()));
        assert!(
            patch_footprint(SOURCE, &options)
                .unwrap_err()
                .message
                .contains("conflicting net")
        );
    }

    #[test]
    fn malformed_source_is_reported_as_parse_error() {
        let error = patch_footprint("(footprint \"broken\"", &patch(Side::Front)).unwrap_err();
        assert!(matches!(error.code, ArtifactErrorCode::ParseError));
    }

    #[test]
    fn unknown_spatial_forms_are_rejected_when_placement_changes() {
        let source = "(footprint \"Part\" (layer \"F.Cu\") (future_shape (at 1 2)))";
        let error = patch_footprint(source, &patch(Side::Front)).unwrap_err();
        assert!(matches!(error.code, ArtifactErrorCode::Unsupported));
        assert!(error.message.contains("future_shape"));
    }

    #[test]
    fn optional_footprint_name_replaces_only_the_root_name_token() {
        let mut options = patch(Side::Front);
        options.footprint_name = Some("Renamed café".into());
        let output = patch_footprint(SOURCE, &options).unwrap();
        assert!(output.starts_with("(footprint \"Renamed café\""));
        assert!(output.contains("(mystery \"keep this source text verbatim\")"));
    }

    #[test]
    fn group_member_uuids_are_remapped_per_instance() {
        let source = "(footprint \"Part\" (layer \"F.Cu\") (uuid \"root\") (pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\") (uuid \"pad\")) (group \"g\" (id \"group-id\") (members \"pad\")))";
        let first = patch_footprint(source, &patch(Side::Front)).unwrap();
        let mut other = patch(Side::Front);
        other.uuid_scope = "board:part:other".into();
        let second = patch_footprint(source, &other).unwrap();
        let first_pad = first
            .split("(uuid ")
            .nth(2)
            .unwrap()
            .split('"')
            .nth(1)
            .unwrap();
        let first_member = first
            .split("(members ")
            .nth(1)
            .unwrap()
            .split('"')
            .nth(1)
            .unwrap();
        let second_pad = second
            .split("(uuid ")
            .nth(2)
            .unwrap()
            .split('"')
            .nth(1)
            .unwrap();
        assert_eq!(first_pad, first_member);
        assert_ne!(first_pad, second_pad);
    }

    #[test]
    fn unresolved_models_and_custom_pads_remain_in_source_and_models_can_be_appended() {
        let rich = "(footprint \"Rich\" (layer \"F.Cu\") (property \"Value\" \"line\\nquote: \\\"\") (pad \"A\" smd custom (at 0 0) (size 1 1) (layers \"F.Cu\") (options (clearance outline)) (primitives (gr_line (start 0 0) (end 2 1) (width 0.2))) (uuid \"custom-pad\")) (model \"${KIPRJMOD}/missing.pretty/a.step\" (offset (xyz 1 2 3)) (scale (xyz 1 1 1)) (rotate (xyz 4 5 6))))";
        let imported = import_footprint(rich, "rich").expect("source stays importable");
        assert!(
            imported
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.kind == ArtifactDiagnosticKind::UnavailableModel)
        );
        assert!(
            imported
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.kind == ArtifactDiagnosticKind::Approximation)
        );
        let output = patch_footprint(rich, &patch(Side::Back)).expect("patch with model preserved");
        assert!(output.contains("(rotate (xyz 4 5 6))"));
        assert!(output.contains("(gr_line (start 0 0) (end 2 -1)"));
        let mut with_managed = patch(Side::Front);
        with_managed
            .model_forms
            .push("(model \"${KIPRJMOD}/models/managed.step\" (offset (xyz 0 0 0)))".into());
        let output = patch_footprint(rich, &with_managed).expect("append managed model");
        assert!(output.contains("${KIPRJMOD}/models/managed.step"));
    }

    /// Arc forms in `source`, whitespace-normalised, in document order.
    fn arcs(source: &str) -> Vec<String> {
        fn visit(raw: &str, node: &Node, out: &mut Vec<String>) {
            if matches!(sexpr::head(node), Some("fp_arc" | "gr_arc")) {
                let span = sexpr::span(node);
                let text = raw[span.start..span.end]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                out.push(text.replace("( ", "(").replace(" )", ")"));
            }
            for child in sexpr::items(node).into_iter().flatten() {
                visit(raw, child, out);
            }
        }
        let document = kiutils_sexpr::parse_one(source).unwrap();
        let mut out = Vec::new();
        for node in &document.nodes {
            visit(source, node, &mut out);
        }
        out
    }

    // The footprint generators moved from JavaScript to Rust (see
    // docs/investigations/footprint-generators-rust.md). The old worker upgraded
    // legacy arcs before export; this checks Core's upgrade produces the same arcs
    // from the recorded generator output, so the Rust port can leave arcs to Core.
    #[test]
    fn upgrade_legacy_arcs_matches_the_recorded_javascript_export() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../footprints/tests/golden/generators");
        let mut compared = 0;
        for path in std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
        {
            let fixture: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            for case in fixture["cases"].as_array().unwrap() {
                let (Some(forms), Some(export)) = (
                    case["output"]["forms"]["ok"]["forms"].as_array(),
                    case["output"]["export"]["ok"].as_object(),
                ) else {
                    continue;
                };
                let mut before = Vec::new();
                for form in forms
                    .iter()
                    .map(|form| form.as_str().unwrap())
                    .filter(|form| form.contains("(angle "))
                {
                    before.extend(arcs(&upgrade_legacy_arcs(form).unwrap()));
                }
                if before.is_empty() {
                    continue;
                }
                let mut recorded = Vec::new();
                for text in export["footprints"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .chain(export["objects"].as_array().unwrap())
                {
                    recorded.extend(arcs(text.as_str().unwrap()));
                }
                before.sort();
                recorded.sort();
                assert_eq!(before, recorded, "{} {}", fixture["source"], case["id"]);
                compared += 1;
            }
        }
        assert!(compared >= 20, "only {compared} cases with legacy arcs");
    }
}
