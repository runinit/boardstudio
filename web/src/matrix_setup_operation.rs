use boardstudio_core::model::{Matrix, MatrixAssembly, MatrixCell, PartDefinition, Side, Vec2};
use serde_json::Value;
use std::collections::BTreeMap;

const PITCH_MM: f64 = 19.05;
const MAX_MATRIX_CELLS: u32 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MatrixSetupPreset {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatrixSetupRequest {
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
}

impl MatrixSetupPreset {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MxSolder => "mx-solder",
            Self::MxHotswap => "mx-hotswap",
            Self::ChocSolder => "choc-solder",
            Self::ChocHotswap => "choc-hotswap",
            Self::MxRgb => "mx-rgb",
            Self::ChocRgb => "choc-rgb",
            Self::MxHotswapRgb => "mx-hotswap-rgb",
            Self::ChocHotswapRgb => "choc-hotswap-rgb",
        }
    }

    fn config(self) -> PresetConfig {
        match self {
            Self::MxSolder => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_mx",
                hotswap: false,
                led: false,
                choc: false,
            },
            Self::MxHotswap => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_mx",
                hotswap: true,
                led: false,
                choc: false,
            },
            Self::ChocSolder => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_choc_v1_v2",
                hotswap: false,
                led: false,
                choc: true,
            },
            Self::ChocHotswap => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_choc_v1_v2",
                hotswap: true,
                led: false,
                choc: true,
            },
            Self::MxRgb => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_mx",
                hotswap: false,
                led: true,
                choc: false,
            },
            Self::ChocRgb => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_choc_v1_v2",
                hotswap: false,
                led: true,
                choc: true,
            },
            Self::MxHotswapRgb => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_mx",
                hotswap: true,
                led: true,
                choc: false,
            },
            Self::ChocHotswapRgb => PresetConfig {
                switch_id: "ergogen:ceoloide/switch_choc_v1_v2",
                hotswap: true,
                led: true,
                choc: true,
            },
        }
    }
}

struct PresetConfig {
    switch_id: &'static str,
    hotswap: bool,
    led: bool,
    choc: bool,
}

pub(crate) struct PreparedMatrix {
    pub matrix: Matrix,
    pub definitions: Vec<PartDefinition>,
}

pub(crate) fn next_matrix_id(
    matrices: &[Matrix],
    definitions: &[PartDefinition],
    parts: &[boardstudio_core::model::Part],
    preset: &str,
    reversible: bool,
) -> String {
    let used = matrices
        .iter()
        .map(|matrix| matrix.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut next = 1u64;
    loop {
        let candidate = format!("matrix-{next}");
        let definition_prefix = format!(
            "assembly-preset-{preset}-south{}-{candidate}-0/definition/",
            if reversible { "-reversible" } else { "" }
        );
        let generated_part_prefix = format!("matrix/{candidate}/");
        if !used.contains(candidate.as_str())
            && !parts
                .iter()
                .any(|part| part.id.starts_with(&generated_part_prefix))
            && !definitions
                .iter()
                .any(|definition| definition.id.starts_with(&definition_prefix))
        {
            return candidate;
        }
        next = next.checked_add(1).expect("matrix identity exhausted");
    }
}

/// Prepare the reference matrix seed; Core remains responsible for validating it and deriving
/// live Parts, matrix membership, board ownership, scene and history from SetMatrix.
pub(crate) fn prepare_matrix(
    matrix_id: String,
    board_id: String,
    request: MatrixSetupRequest,
    reversible: bool,
    catalogue: &[PartDefinition],
) -> Result<PreparedMatrix, String> {
    if matrix_id.is_empty()
        || !matrix_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("Matrix ID must use letters, digits, _ or -".into());
    }
    if board_id.is_empty() {
        return Err("Select a board before creating a matrix.".into());
    }
    if request.rows == 0
        || request.columns == 0
        || request
            .rows
            .checked_mul(request.columns)
            .is_none_or(|cells| cells > MAX_MATRIX_CELLS)
    {
        return Err("Matrix dimensions must be positive and no larger than 4096 cells.".into());
    }

    let preset = request.preset.config();
    let assembly_name = format!(
        "preset-{}-south{}",
        request.preset.as_str(),
        if reversible { "-reversible" } else { "" }
    );
    let definition_prefix = format!("assembly-{assembly_name}-{matrix_id}-0/definition");
    let switch_source = required_catalogue_definition(catalogue, preset.switch_id)?;
    let diode_source =
        required_catalogue_definition(catalogue, "ergogen:ceoloide/diode_tht_sod123")?;
    let led_source = if preset.led {
        Some(required_catalogue_definition(
            catalogue,
            "ergogen:ceoloide/led_sk6812mini-e",
        )?)
    } else {
        None
    };

    let switch_id = format!("{definition_prefix}/switch");
    let diode_id = format!("{definition_prefix}/diode");
    let switch = snapshot_definition(
        switch_source,
        switch_id.clone(),
        [
            ("hotswap", Value::Bool(preset.hotswap)),
            ("solder", Value::Bool(!preset.hotswap)),
            ("reversible", Value::Bool(reversible)),
            ("side", Value::String("B".into())),
            ("include_keycap", Value::Bool(true)),
        ]
        .into_iter()
        .chain(
            preset
                .choc
                .then_some(("choc_v1_support", Value::Bool(true))),
        )
        .chain(
            preset
                .choc
                .then_some(("choc_v2_support", Value::Bool(false))),
        )
        .chain(
            preset
                .choc
                .then_some(("include_choc_v1_led_cutout_marks", Value::Bool(true))),
        )
        .collect(),
    )?;
    let diode = snapshot_definition(
        diode_source,
        diode_id.clone(),
        [
            ("side", Value::String("B".into())),
            ("reversible", Value::Bool(reversible)),
            ("include_tht", Value::Bool(false)),
        ]
        .into_iter()
        .collect(),
    )?;
    let led = led_source
        .map(|source| {
            snapshot_definition(
                source,
                format!("{definition_prefix}/led"),
                [
                    ("side", Value::String("B".into())),
                    ("reverse_mount", Value::Bool(true)),
                    ("reversible", Value::Bool(reversible)),
                ]
                .into_iter()
                .collect(),
            )
        })
        .transpose()?;

    let variant = format!(
        "preset/{}/{}south",
        request.preset.as_str(),
        if reversible { "reversible/" } else { "" }
    );
    let mut cells = Vec::with_capacity((request.rows * request.columns) as usize);
    for row in 0..request.rows {
        for column in 0..request.columns {
            let mut assemblies = vec![MatrixAssembly {
                id: "diode".into(),
                definition_id: diode_id.clone(),
                offset: Vec2 { x: 7.4, y: -1.5 },
                rotation: Some(90.0),
                side: Some(Side::Back),
            }];
            if let Some(led) = &led {
                assemblies.push(MatrixAssembly {
                    id: "led".into(),
                    definition_id: led.id.clone(),
                    offset: Vec2 {
                        x: 0.0,
                        y: if preset.choc { -4.7 } else { -4.75 },
                    },
                    rotation: Some(180.0),
                    side: Some(Side::Back),
                });
            }
            cells.push(MatrixCell {
                row,
                column,
                enabled: true,
                definition_id: Some(switch_id.clone()),
                variant: Some(variant.clone()),
                offset: None,
                rotation: Some(0.0),
                assemblies,
                assemblies_local: Some(true),
            });
        }
    }

    let matrix = Matrix {
        id: matrix_id,
        name: None,
        rows: request.rows,
        columns: request.columns,
        pitch: Vec2 {
            x: PITCH_MM,
            y: PITCH_MM,
        },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: switch_id,
        part_ids: Vec::new(),
        board_id: Some(board_id),
        mirror: None,
        rotation: None,
        edge_gap: Some(Vec2 { x: 1.0, y: 1.0 }),
        diode_direction: None,
        row_offsets: Vec::new(),
        column_offsets: Vec::new(),
        column_staggers: Vec::new(),
        column_splays: Vec::new(),
        column_origins: Vec::new(),
        cells,
    };
    let mut definitions = vec![switch, diode];
    if let Some(led) = led {
        definitions.push(led);
    }
    Ok(PreparedMatrix {
        matrix,
        definitions,
    })
}

fn required_catalogue_definition<'a>(
    catalogue: &'a [PartDefinition],
    id: &str,
) -> Result<&'a PartDefinition, String> {
    let mut matches = catalogue.iter().filter(|definition| definition.id == id);
    let definition = matches
        .next()
        .ok_or_else(|| format!("Required bundled matrix component {id} is unavailable."))?;
    if matches.next().is_some() {
        return Err(format!("Bundled matrix component {id} is ambiguous."));
    }
    if definition.generator.is_none() {
        return Err(format!("Bundled matrix component {id} has no generator."));
    }
    Ok(definition)
}

fn snapshot_definition(
    source: &PartDefinition,
    id: String,
    parameters: BTreeMap<&str, Value>,
) -> Result<PartDefinition, String> {
    let mut definition = source.clone();
    let generator = definition
        .generator
        .as_mut()
        .ok_or_else(|| format!("Bundled matrix component {} has no generator.", source.id))?;
    generator.parameters.extend(
        parameters
            .into_iter()
            .map(|(name, value)| (name.into(), value)),
    );
    definition.id = id;
    Ok(definition)
}
