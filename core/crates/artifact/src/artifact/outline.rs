use super::{ArtifactError, ArtifactErrorCode};
use crate::model::{ArtifactFile, Contour, OutlineExportFormat, OutlineExportRequest, Vec2};

fn invalid(message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::Validation, message)
}

fn validate(contours: &[Contour]) -> Result<(), ArtifactError> {
    if contours.is_empty() {
        return Err(invalid("No resolved outline"));
    }
    if contours.iter().any(|contour| {
        contour.points.len() < 3
            || contour
                .points
                .iter()
                .any(|point| !point.x.is_finite() || !point.y.is_finite())
    }) {
        return Err(invalid("Invalid resolved contour"));
    }
    Ok(())
}

fn bounds(contours: &[Contour]) -> (Vec2, Vec2) {
    let mut min = Vec2 {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };
    let mut max = Vec2 {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };
    for point in contours.iter().flat_map(|contour| &contour.points) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    (min, max)
}

fn svg(contours: &[Contour]) -> String {
    let (min, max) = bounds(contours);
    let width = max.x - min.x;
    let height = max.y - min.y;
    let path = contours
        .iter()
        .map(|contour| {
            let first = contour.points[0];
            let mut path = format!("M {} {}", number(first.x), number(-first.y));
            for point in contour.points.iter().skip(1) {
                path.push_str(&format!(" L {} {}", number(point.x), number(-point.y)));
            }
            path.push_str(" Z");
            path
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"{} {} {} {}\"><path d=\"{}\" fill=\"none\" stroke=\"#111\" stroke-width=\"0.1\" fill-rule=\"evenodd\"/></svg>\n",
        number(width),
        number(height),
        number(min.x),
        number(-max.y),
        number(width),
        number(height),
        path
    )
}

fn dxf(contours: &[Contour]) -> String {
    let mut rows = vec![
        "0",
        "SECTION",
        "2",
        "HEADER",
        "9",
        "$ACADVER",
        "1",
        "AC1015",
        "9",
        "$INSUNITS",
        "70",
        "4",
        "9",
        "$MEASUREMENT",
        "70",
        "1",
        "0",
        "ENDSEC",
        "0",
        "SECTION",
        "2",
        "ENTITIES",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for contour in contours {
        rows.extend(
            [
                "0",
                "LWPOLYLINE",
                "8",
                if contour.hole { "HOLE" } else { "OUTLINE" },
                "90",
                "",
                "70",
                "1",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        let count_index = rows.len() - 3;
        rows[count_index] = contour.points.len().to_string();
        for point in &contour.points {
            rows.push("10".into());
            rows.push(number(point.x));
            rows.push("20".into());
            rows.push(number(point.y));
        }
    }
    rows.extend(["0", "ENDSEC", "0", "EOF"].into_iter().map(str::to_owned));
    format!("{}\n", rows.join("\n"))
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}

pub fn export_outline(request: OutlineExportRequest) -> Result<ArtifactFile, ArtifactError> {
    validate(&request.contours)?;
    let content = match request.format {
        OutlineExportFormat::Svg => svg(&request.contours),
        OutlineExportFormat::Dxf => dxf(&request.contours),
    };
    Ok(ArtifactFile {
        filename: request.filename,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline() -> Vec<Contour> {
        vec![Contour {
            points: vec![
                Vec2 { x: 2.0, y: 3.0 },
                Vec2 { x: 7.0, y: 3.0 },
                Vec2 { x: 7.0, y: 9.0 },
            ],
            hole: false,
        }]
    }

    fn board() -> crate::model::Board {
        crate::model::Board {
            id: "b".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    #[test]
    fn svg_and_dxf_preserve_millimetre_coordinates_and_hole_layers() {
        let svg = export_outline(OutlineExportRequest {
            filename: "shape.svg".into(),
            board: board(),
            contours: outline(),
            format: OutlineExportFormat::Svg,
        })
        .unwrap();
        assert!(
            svg.content
                .contains("width=\"5mm\" height=\"6mm\" viewBox=\"2 -9 5 6\"")
        );
        assert!(svg.content.contains("M 2 -3 L 7 -3 L 7 -9 Z"));

        let mut contours = outline();
        contours[0].hole = true;
        let dxf = export_outline(OutlineExportRequest {
            filename: "shape.dxf".into(),
            board: board(),
            contours,
            format: OutlineExportFormat::Dxf,
        })
        .unwrap();
        assert!(dxf.content.contains("$INSUNITS\n70\n4"));
        assert!(dxf.content.contains("$MEASUREMENT\n70\n1"));
        assert!(dxf.content.contains("HOLE"));
        assert!(dxf.content.contains("\n70\n1\n10\n2\n20\n3\n"));
    }

    #[test]
    fn outline_export_rejects_open_or_nonfinite_geometry() {
        let mut contours = outline();
        contours[0].points.pop();
        assert_eq!(
            export_outline(OutlineExportRequest {
                filename: "shape.svg".into(),
                board: board(),
                contours,
                format: OutlineExportFormat::Svg
            })
            .unwrap_err()
            .code,
            ArtifactErrorCode::Validation
        );
        let mut contours = outline();
        contours[0].points[0].x = f64::NAN;
        assert!(
            export_outline(OutlineExportRequest {
                filename: "shape.dxf".into(),
                board: board(),
                contours,
                format: OutlineExportFormat::Dxf
            })
            .is_err()
        );
    }
}
