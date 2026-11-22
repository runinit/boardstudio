//! Matrix assembly presets: the preset and orientation choices, applying one to a matrix, and
//! duplicating the whole design as a variant that carries a different preset.
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Event, Lifecycle, Resolution, Scope, TerminalOutcome};
use boardstudio_model::model::{EditOperation, Matrix, PartDefinition, ProjectDoc, Side, Vec2};
use boardstudio_web_catalogue::matrix_input::matrix_input_available;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixPreset {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

impl MatrixPreset {
    pub const ALL: [Self; 8] = [
        Self::MxSolder,
        Self::MxHotswap,
        Self::ChocSolder,
        Self::ChocHotswap,
        Self::MxRgb,
        Self::ChocRgb,
        Self::MxHotswapRgb,
        Self::ChocHotswapRgb,
    ];

    pub fn as_str(self) -> &'static str {
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

    pub fn label(self) -> &'static str {
        match self {
            Self::MxSolder => "MX Solder",
            Self::MxHotswap => "MX Hotswap",
            Self::ChocSolder => "Choc V1 Solder",
            Self::ChocHotswap => "Choc V1 Hotswap",
            Self::MxRgb => "MX RGB",
            Self::ChocRgb => "Choc V1 RGB",
            Self::MxHotswapRgb => "MX Hotswap RGB",
            Self::ChocHotswapRgb => "Choc V1 Hotswap RGB",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.as_str() == value)
    }

    fn setup(self) -> crate::matrix_setup_operation::MatrixSetupPreset {
        use crate::matrix_setup_operation::MatrixSetupPreset as Setup;
        match self {
            Self::MxSolder => Setup::MxSolder,
            Self::MxHotswap => Setup::MxHotswap,
            Self::ChocSolder => Setup::ChocSolder,
            Self::ChocHotswap => Setup::ChocHotswap,
            Self::MxRgb => Setup::MxRgb,
            Self::ChocRgb => Setup::ChocRgb,
            Self::MxHotswapRgb => Setup::MxHotswapRgb,
            Self::ChocHotswapRgb => Setup::ChocHotswapRgb,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchOrientation {
    South,
    North,
}

impl SwitchOrientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::South => "south",
            Self::North => "north",
        }
    }
}

/// The preset and orientation a matrix carries, read from its switch cells' variant tag.
pub fn matrix_preset(matrix: &Matrix) -> (Option<MatrixPreset>, Option<SwitchOrientation>) {
    let variant = matrix
        .cells
        .iter()
        .find(|cell| cell.definition_id.as_deref() == Some(matrix.definition_id.as_str()))
        .and_then(|cell| cell.variant.as_deref())
        .and_then(|value| value.strip_prefix("preset/"));
    let Some(variant) = variant else { return (None, None) };
    let preset = variant.split('/').next().and_then(MatrixPreset::parse);
    let orientation = match variant.rsplit('/').next() {
        Some("north") => Some(SwitchOrientation::North),
        Some("south") => Some(SwitchOrientation::South),
        _ => None,
    };
    (preset, orientation)
}

pub fn is_reversible(document: &ProjectDoc) -> bool {
    document.parameters.get("reversibleLayout").and_then(serde_json::Value::as_bool).unwrap_or(false)
}

fn preset_variant(preset: MatrixPreset, orientation: SwitchOrientation, reversible: bool) -> String {
    let reversible = if reversible { "reversible/" } else { "" };
    format!("preset/{}/{reversible}{}", preset.as_str(), orientation.as_str())
}

/// Mirrors the pinned `matrixWithPreset` recipe while retaining matrix-only fields and each
/// pre-existing cell's enabled state, local pose and non-diode/non-LED assembly membership.
pub fn matrix_with_preset(matrix: &Matrix, prepared: &Matrix, variant: &str, orientation: SwitchOrientation) -> Matrix {
    let old_cells: std::collections::HashMap<_, _> = matrix.cells.iter().map(|cell| ((cell.row, cell.column), cell)).collect();
    let orientation_rotation = if orientation == SwitchOrientation::North { 180.0 } else { 0.0 };
    let cells = prepared
        .cells
        .iter()
        .map(|template| {
            let old = old_cells.get(&(template.row, template.column)).copied();
            let old_rotation = old.and_then(|cell| cell.rotation).unwrap_or(0.0);
            let old_orientation = old
                .and_then(|cell| cell.variant.as_deref())
                .filter(|variant| variant.starts_with("preset/") && variant.ends_with("/north"))
                .map_or(0.0, |_| 180.0);
            let next_rotation = old_rotation - old_orientation + orientation_rotation;
            let delta = (next_rotation - old_rotation).to_radians();
            let (sin, cos) = delta.sin_cos();
            let mut cell = template.clone();
            cell.variant = Some(variant.to_owned());
            cell.enabled = old.is_none_or(|cell| cell.enabled);
            cell.offset = old.and_then(|cell| cell.offset);
            cell.rotation = Some(next_rotation);
            cell.assemblies_local = Some(true);
            if let Some(old) = old {
                cell.assemblies.extend(old.assemblies.iter().filter(|a| a.id != "diode" && a.id != "led").map(|assembly| {
                    let mut assembly = assembly.clone();
                    assembly.offset = Vec2 { x: assembly.offset.x * cos + assembly.offset.y * sin, y: -assembly.offset.x * sin + assembly.offset.y * cos };
                    assembly.rotation = Some(assembly.rotation.unwrap_or(0.0) - delta.to_degrees());
                    assembly.side.get_or_insert(Side::Front);
                    assembly
                }));
            }
            cell
        })
        .collect();
    let mut replacement = matrix.clone();
    replacement.definition_id = prepared.definition_id.clone();
    replacement.cells = cells;
    replacement
}

fn is_assembly_snapshot(id: &str, has_kicad_source: bool) -> bool {
    if has_kicad_source {
        return false;
    }
    let Some((prefix, _)) = id.split_once("/definition/") else { return false };
    prefix.starts_with("assembly-")
        || (prefix.len() == 36
            && prefix.bytes().enumerate().all(|(index, byte)| if matches!(index, 8 | 13 | 18 | 23) { byte == b'-' } else { byte.is_ascii_hexdigit() }))
}

/// The switch footprints a matrix can use: catalogue templates overlaid by the document's own
/// definitions, always including the current one.
pub fn switch_choices(document: &ProjectDoc, templates: &[PartDefinition], current_id: &str) -> Vec<(String, String)> {
    let mut definitions = Vec::<PartDefinition>::new();
    for definition in templates.iter().chain(&document.definitions) {
        match definitions.iter_mut().find(|existing| existing.id == definition.id) {
            Some(existing) => *existing = definition.clone(),
            None => definitions.push(definition.clone()),
        }
    }
    definitions
        .into_iter()
        .filter(|definition| {
            definition.id == current_id || (!is_assembly_snapshot(&definition.id, definition.kicad_source.is_some()) && matrix_input_available(definition))
        })
        .map(|definition| {
            let label = match definition.id.as_str() {
                "generator:ceoloide/switch_mx" => "MX switch".into(),
                "generator:ceoloide/switch_choc_v1_v2" => "Choc V1 / V2 switch".into(),
                "generator:ceoloide/switch_gateron_ks27_ks33" => "Gateron KS27 / KS33 switch".into(),
                _ => definition.name.clone(),
            };
            (definition.id, label)
        })
        .chain((!document.definitions.iter().chain(templates).any(|d| d.id == current_id)).then(|| (current_id.to_owned(), current_id.to_owned())))
        .collect()
}

/// The switch definitions an edit may need: only those the accepted document lacks.
pub fn missing_definitions(document: &ProjectDoc, catalogue: &[PartDefinition], ids: &[&str]) -> Vec<PartDefinition> {
    ids.iter()
        .filter(|id| !document.definitions.iter().any(|definition| definition.id == **id))
        .filter_map(|id| catalogue.iter().find(|definition| definition.id == **id).cloned())
        .collect()
}

struct Prepared {
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
    variant: String,
    orientation: SwitchOrientation,
}

async fn prepare(document: &ProjectDoc, matrix: &Matrix, board_id: &str, preset: MatrixPreset, orientation: SwitchOrientation) -> Result<Prepared, String> {
    let reversible = is_reversible(document);
    let templates = crate::presentation::parts::load_matrix_templates(reversible).await?;
    let mut prepared = crate::matrix_setup_operation::prepare_matrix(
        matrix.id.clone(),
        board_id.to_owned(),
        crate::matrix_setup_operation::MatrixSetupRequest { rows: matrix.rows, columns: matrix.columns, preset: preset.setup() },
        reversible,
        &templates,
    )?;
    for definition in &mut prepared.definitions {
        *definition = crate::presentation::parts::normalize_matrix_definition(definition.clone()).await?;
        if document.definitions.iter().any(|existing| existing.id == definition.id && existing != definition) {
            return Err(format!("The prepared matrix definition {} conflicts with an existing definition.", definition.id));
        }
    }
    Ok(Prepared { matrix: prepared.matrix, definitions: prepared.definitions, variant: preset_variant(preset, orientation, reversible), orientation })
}

/// Apply `preset` to the matrix, keeping its size, enabled keys, local poses and extra components.
pub async fn apply_preset(runtime: &Rc<Runtime>, matrix_id: &str, board_id: &str, preset: MatrixPreset, orientation: SwitchOrientation) -> Result<(), String> {
    let snapshot = runtime.model().accepted.ok_or("No accepted keyboard.")?;
    let matrix = snapshot.document.matrices.iter().find(|matrix| matrix.id == matrix_id).ok_or("The selected matrix no longer exists.")?.clone();
    let prepared = prepare(&snapshot.document, &matrix, board_id, preset, orientation).await?;
    let id = matrix_id.to_owned();
    runtime
        .commit(
            "layout-matrix-preset",
            EditResolver::new("layout-matrix-preset", move |accepted: &AcceptedSnapshot| {
                let Some(matrix) = accepted.document.matrices.iter().find(|matrix| matrix.id == id) else {
                    return Resolution::Retire("The selected matrix no longer exists.".into());
                };
                if (matrix.rows, matrix.columns) != (prepared.matrix.rows, prepared.matrix.columns) {
                    return Resolution::Retire("The matrix size changed before the preset was applied. Apply the preset again.".into());
                }
                let replacement = matrix_with_preset(matrix, &prepared.matrix, &prepared.variant, prepared.orientation);
                if &replacement == matrix {
                    return Resolution::Unchanged;
                }
                Resolution::submit(vec![id.clone()], EditOperation::SetMatrix { matrix: replacement, definitions: Some(prepared.definitions.clone()) })
            }),
        )
        .map(|_| ())
}

/// Poll an observed operation until it settles `Completed` with a saved accepted document `id`.
async fn wait_saved(runtime: &Rc<Runtime>, outcome: crate::operation_outcomes::OutcomeSlot, id: &str) -> Result<AcceptedSnapshot, String> {
    for _ in 0..1_200 {
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!("The project operation did not complete: {outcome:?}"));
            }
            let model = runtime.model();
            let accepted = model.accepted.ok_or("The project operation was not accepted.")?;
            if accepted.document.id != id || model.lifecycle != Lifecycle::Ready {
                return Err("The project operation was superseded by another open action.".into());
            }
            return Ok(accepted);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The project operation did not settle.".into())
}

/// Clone the whole accepted project into a new document, open it, and apply `preset` to the copy.
/// A failure after the copy opened reopens the saved original so the user returns to their design.
pub async fn duplicate_variant(runtime: Rc<Runtime>, source: &Scope, matrix_id: &str, preset: MatrixPreset, orientation: SwitchOrientation) -> Result<(), String> {
    let mut document = (*runtime.model().accepted.ok_or("No accepted keyboard.")?.document).clone();
    let source_id = document.id.clone();
    document.id = crate::runtime::new_project_id()?;
    let name = if document.name.trim().is_empty() { "Untitled keyboard" } else { document.name.trim() }.to_owned();
    document.name = format!("{name} (variant)");
    let variant_id = document.id.clone();
    let open_id = runtime.operation();
    let opened = runtime.observe_operation(open_id);
    runtime.submit(if runtime.model().lifecycle == Lifecycle::RecoveryRequired {
        Event::RecoverWithDocument { operation_id: open_id, document }
    } else {
        Event::Open { operation_id: open_id, document }
    });
    let work = async {
        wait_saved(&runtime, opened, &variant_id).await?;
        let navigate_id = runtime.operation();
        let navigated = runtime.observe_operation(navigate_id);
        runtime.submit(Event::Navigate { operation_id: navigate_id, board_id: source.board_id.clone(), instance_id: source.instance_id.clone() });
        wait_saved(&runtime, navigated, &variant_id).await?;
        apply_preset(&runtime, matrix_id, &source.board_id, preset, orientation).await
    };
    let Err(error) = work.await else { return Ok(()) };
    let model = runtime.model();
    let restored = match (model.accepted, runtime.scope()) {
        (Some(current), Some(scope)) => runtime.reopen_saved_if_current(source_id, &current, model.lifecycle, &scope).await.is_ok(),
        _ => false,
    };
    Err(if restored { format!("Could not create the matrix design variant; reopened the saved original. {error}") } else { format!("Could not create the matrix design variant. {error}") })
}
