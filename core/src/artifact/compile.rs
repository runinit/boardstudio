use crate::model::{CompiledFootprint, FootprintGeometry, PadShape, PartDefinition, Side, Vec2};
use std::collections::BTreeSet;

/// Compile authored geometry. Generated parts must use their generator pipeline.
pub fn compile_authored(
    definition: &PartDefinition,
    side: Side,
) -> Result<CompiledFootprint, String> {
    if definition.kicad_source.is_some() {
        return Err(format!(
            "Footprint '{}' has authoritative KiCad source and must use source projection",
            definition.id
        ));
    }
    if let Some(generator) = &definition.generator {
        return Err(format!(
            "Unsupported authored-footprint generator: {}",
            generator.source
        ));
    }
    let mut geometry = FootprintGeometry {
        side,
        courtyard: definition.courtyard.clone(),
        pads: definition.pads.clone(),
        traces: vec![],
        vias: vec![],
    };
    for pad in &mut geometry.pads {
        pad.net_id = None;
    }
    validate_geometry(definition, &geometry)?;
    let preview_svg = Some(preview_svg(&geometry)?);
    Ok(CompiledFootprint {
        definition: definition.clone(),
        geometry,
        diagnostics: vec![],
        preview_svg,
    })
}

pub fn validate_geometry(
    definition: &PartDefinition,
    geometry: &FootprintGeometry,
) -> Result<(), String> {
    if geometry.courtyard.len() < 3 {
        return Err(format!(
            "Footprint {} needs a courtyard polygon",
            definition.id
        ));
    }
    if geometry.courtyard.iter().any(|point| !finite_point(*point)) {
        return Err(format!("Invalid courtyard coordinate: {}", definition.id));
    }

    let mut ids = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    for pad in &geometry.pads {
        if !ids.insert(pad.id.as_str()) {
            return Err(format!("Duplicate footprint pad id: {}", pad.id));
        }
        if (pad.number.trim().is_empty() && pad.plated != Some(false))
            || (!pad.number.trim().is_empty() && !numbers.insert(pad.number.as_str()))
        {
            return Err(format!("Duplicate or empty pad number: {}", pad.number));
        }
        if !finite_point(pad.at)
            || !finite_point(pad.size)
            || pad.size.x <= 0.0
            || pad.size.y <= 0.0
            || pad
                .drill
                .is_some_and(|drill| !drill.is_finite() || drill <= 0.0)
            || pad.rotation.is_some_and(|rotation| !rotation.is_finite())
        {
            return Err(format!("Invalid pad geometry: {}", pad.id));
        }
    }

    for trace in &geometry.traces {
        if !finite_point(trace.start)
            || !finite_point(trace.end)
            || !trace.width.is_finite()
            || trace.width <= 0.0
            || trace
                .pad_id
                .as_ref()
                .is_some_and(|id| !ids.contains(id.as_str()))
        {
            return Err(format!("Invalid footprint trace: {}", trace.id));
        }
    }
    for via in &geometry.vias {
        if !finite_point(via.at)
            || !via.size.is_finite()
            || !via.drill.is_finite()
            || via.size <= 0.0
            || via.drill <= 0.0
            || via.drill >= via.size
            || via
                .pad_id
                .as_ref()
                .is_some_and(|id| !ids.contains(id.as_str()))
        {
            return Err(format!("Invalid footprint via: {}", via.id));
        }
    }
    Ok(())
}

pub fn preview_svg(geometry: &FootprintGeometry) -> Result<String, String> {
    validate_geometry_for_preview(geometry)?;
    let mut points = geometry.courtyard.clone();
    for pad in &geometry.pads {
        points.push(Vec2 {
            x: pad.at.x - pad.size.x / 2.0,
            y: pad.at.y - pad.size.y / 2.0,
        });
        points.push(Vec2 {
            x: pad.at.x + pad.size.x / 2.0,
            y: pad.at.y + pad.size.y / 2.0,
        });
    }
    for trace in &geometry.traces {
        points.extend([trace.start, trace.end]);
    }
    for via in &geometry.vias {
        points.push(Vec2 {
            x: via.at.x - via.size / 2.0,
            y: via.at.y - via.size / 2.0,
        });
        points.push(Vec2 {
            x: via.at.x + via.size / 2.0,
            y: via.at.y + via.size / 2.0,
        });
    }

    let min_x = points.iter().map(|point| point.x).fold(0.0_f64, f64::min) - 1.0;
    let max_x = points.iter().map(|point| point.x).fold(0.0_f64, f64::max) + 1.0;
    let min_y = points.iter().map(|point| point.y).fold(0.0_f64, f64::min) - 1.0;
    let max_y = points.iter().map(|point| point.y).fold(0.0_f64, f64::max) + 1.0;
    let courtyard = geometry
        .courtyard
        .iter()
        .map(|point| format!("{},{}", number(point.x), number(-point.y)))
        .collect::<Vec<_>>()
        .join(" ");
    let pads = geometry.pads.iter().map(|pad| {
        let x = pad.at.x - pad.size.x / 2.0;
        let y = -pad.at.y - pad.size.y / 2.0;
        let radius = match &pad.shape {
            PadShape::Circle => pad.size.x / 2.0,
            PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
            PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
            PadShape::Rect => 0.0,
        };
        format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"#bd8e52\"/><title>{}</title>",
            number(x), number(y), number(pad.size.x), number(pad.size.y), number(radius), escape_xml(&pad.number)
        )
    }).collect::<String>();
    let traces = geometry.traces.iter().map(|trace| format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#bd8e52\" stroke-width=\"{}\"/>",
        number(trace.start.x), number(-trace.start.y), number(trace.end.x), number(-trace.end.y), number(trace.width)
    )).collect::<String>();
    let vias = geometry.vias.iter().map(|via| format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"#bd8e52\" stroke-width=\"0.2\"/>",
        number(via.at.x), number(-via.at.y), number(via.size / 2.0)
    )).collect::<String>();
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" role=\"img\" aria-label=\"Footprint preview\"><polygon points=\"{}\" fill=\"none\" stroke=\"#527d76\" stroke-width=\"0.2\"/>{traces}{vias}{pads}</svg>",
        number(min_x),
        number(-max_y),
        number(max_x - min_x),
        number(max_y - min_y),
        courtyard
    ))
}

fn validate_geometry_for_preview(geometry: &FootprintGeometry) -> Result<(), String> {
    if geometry.courtyard.len() < 3 || geometry.courtyard.iter().any(|point| !finite_point(*point))
    {
        return Err("Footprint preview needs a finite courtyard polygon".into());
    }
    if geometry.pads.iter().any(|pad| {
        !finite_point(pad.at) || !finite_point(pad.size) || pad.size.x <= 0.0 || pad.size.y <= 0.0
    }) || geometry.traces.iter().any(|trace| {
        !finite_point(trace.start)
            || !finite_point(trace.end)
            || !trace.width.is_finite()
            || trace.width <= 0.0
    }) || geometry
        .vias
        .iter()
        .any(|via| !finite_point(via.at) || !via.size.is_finite() || via.size <= 0.0)
    {
        return Err("Footprint preview contains invalid geometry".into());
    }
    Ok(())
}

fn finite_point(point: Vec2) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}

fn escape_xml(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '"' => "&quot;".to_owned(),
            '\'' => "&apos;".to_owned(),
            _ => character.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PartKind, Vec2};

    fn definition() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"authored-diode", "name":"Authored diode", "kind":"passive",
            "courtyard":[{"x":-3,"y":-1.5},{"x":3,"y":-1.5},{"x":3,"y":1.5},{"x":-3,"y":1.5}],
            "pads":[{"id":"anode","number":"1","at":{"x":-1.5,"y":0},"size":{"x":1,"y":1},"shape":"rect"}]
        })).unwrap()
    }

    #[test]
    fn retired_generators_cannot_fall_back_to_saved_pad_geometry() {
        let mut definition = definition();
        definition.generator = Some(crate::model::PartGenerator {
            source: "builtin:rgb-led".into(),
            version: "builtin-1".into(),
            parameters: Default::default(),
        });
        assert!(
            compile_authored(&definition, Side::Front)
                .unwrap_err()
                .contains("Unsupported")
        );
    }

    #[test]
    fn preview_svg_matches_expected_orientation_and_escapes_pad_numbers() {
        let mut definition = definition();
        definition.generator = None;
        definition.pads[0].number = "1<&".into();
        let output = compile_authored(&definition, Side::Back).unwrap();
        let svg = output.preview_svg.unwrap();
        assert!(svg.contains("viewBox=\"-4 -2.5 8 5\""));
        assert!(svg.contains("<polygon points=\"-3,1.5 3,1.5 3,-1.5 -3,-1.5\""));
        assert!(svg.contains("<title>1&lt;&amp;</title>"));
    }

    #[test]
    fn net_label_changes_do_not_change_compiled_geometry() {
        let mut definition = definition();
        let without_net = compile_authored(&definition, Side::Front).unwrap().geometry;
        definition.pads[0].net_id = Some("row".into());
        let with_net = compile_authored(&definition, Side::Front).unwrap().geometry;
        assert_eq!(without_net, with_net);
    }

    #[test]
    fn arbitrary_authored_geometry_can_be_compiled_without_generator() {
        let mut definition = definition();
        definition.generator = None;
        definition.courtyard = vec![
            Vec2 { x: -4.0, y: -4.0 },
            Vec2 { x: 4.0, y: -4.0 },
            Vec2 { x: 4.0, y: 4.0 },
        ];
        assert!(compile_authored(&definition, Side::Back).is_ok());
        assert_eq!(PartKind::Passive, definition.kind);
    }
}
