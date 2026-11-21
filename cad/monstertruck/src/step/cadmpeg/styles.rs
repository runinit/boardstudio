//! Recover direct surface fills suppressed by a STEP style override.
use super::*;

pub(super) fn suppressed_fills(records: &SourceRecords<'_>) -> HashMap<u64, [f32; 4]> {
    let mut overridden_targets = HashSet::new();
    let mut assignments = HashMap::<u64, Vec<&Parameter>>::new();
    for parts in records.values() {
        let [record] = parts.as_slice() else { continue };
        let count = match record.name.as_str() {
            "STYLED_ITEM" => 3,
            "OVER_RIDING_STYLED_ITEM" => 4,
            _ => continue,
        };
        let Ok(args) = parameters(record, count) else {
            continue;
        };
        let Ok(target) = reference(&args[2]) else {
            continue;
        };
        assignments.entry(target).or_default().push(&args[1]);
        if let Some(base) = args.get(3).and_then(|p| reference(p).ok())
            && let Ok(base) = named_record(records, base, "STYLED_ITEM")
            && let Ok(params) = parameters(base, 3)
            && let Ok(target) = reference(&params[2])
        {
            overridden_targets.insert(target);
        }
    }
    let mut fills = HashMap::new();
    for target in overridden_targets {
        let Some(styles) = assignments.get(&target) else {
            continue;
        };
        // An overriding context may target the same solid. Restore a scalar
        // only when every applicable assignment resolves to the same fill.
        let mut colors = Vec::new();
        let complete = styles.iter().all(|list| {
            let before = colors.len();
            for style in super::topology::references(list).unwrap_or_default() {
                collect(records, style, 0, &mut colors);
            }
            colors.len() > before
        });
        if complete
            && let Some(&color) = colors
                .first()
                .filter(|c| colors.iter().all(|other| other == *c))
        {
            fills.insert(target, color);
        }
    }
    fills
}

fn collect(records: &SourceRecords<'_>, id: u64, depth: usize, colors: &mut Vec<[f32; 4]>) {
    let Ok(record) = simple_record(records, id) else {
        return;
    };
    let Parameter::List(args) = &record.parameter else {
        return;
    };
    let next = match (depth, record.name.as_str(), args.as_slice()) {
        (0, "PRESENTATION_STYLE_ASSIGNMENT", [styles])
        | (0, "PRESENTATION_STYLE_BY_CONTEXT", [styles, _])
        | (2, "SURFACE_SIDE_STYLE", [_, styles])
        | (4, "FILL_AREA_STYLE", [_, styles]) => super::topology::references(styles).ok(),
        (1, "SURFACE_STYLE_USAGE", [Parameter::Enumeration(side), style]) if side == "BOTH" => {
            reference(style).ok().map(|id| vec![id])
        }
        (3, "SURFACE_STYLE_FILL_AREA", [style]) | (5, "FILL_AREA_STYLE_COLOUR", [_, style]) => {
            reference(style).ok().map(|id| vec![id])
        }
        (6, "COLOUR_RGB", [_, r, g, b]) => {
            if let [Ok(r), Ok(g), Ok(b)] = [r, g, b].map(scene::real) {
                if [r, g, b].iter().all(|x| (0.0..=1.0).contains(x)) {
                    colors.push([r as f32, g as f32, b as f32, 1.0]);
                }
            }
            None
        }
        (6, "DRAUGHTING_PRE_DEFINED_COLOUR", [Parameter::String(s)]) => {
            let rgb = match s.to_ascii_lowercase().as_str() {
                "black" => [0., 0., 0.],
                "white" => [1., 1., 1.],
                "red" => [1., 0., 0.],
                "green" => [0., 1., 0.],
                "blue" => [0., 0., 1.],
                "cyan" => [0., 1., 1.],
                "magenta" => [1., 0., 1.],
                "yellow" => [1., 1., 0.],
                _ => return,
            };
            colors.push([rgb[0], rgb[1], rgb[2], 1.]);
            None
        }
        _ => None,
    };
    for child in next.into_iter().flatten() {
        collect(records, child, depth + 1, colors);
    }
}
