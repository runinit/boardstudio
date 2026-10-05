//! Definition normalization and the generated catalogue: pads and envelope from
//! the generator, terminal pad groups from net markers, and the rules that keep
//! saved pad IDs, nets and authored envelopes.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{GeneratorError, Result};
use crate::geometry::geometry;
use crate::nets::{ConstantNet, NoNets};
use crate::params::ParamKind;
use crate::registry::{GENERATOR_VERSION, GeneratorSpec, Registry};
use crate::sexpr::{Expr, child, children, scalar_error};
use crate::types::{
    EnvelopeOrigin, EnvelopeSource, GeneratorRef, MatrixTerminals, Pad, PartKind, Vec2,
};

/// Prefix of generated definition IDs. Step 5 of the plan renames it to
/// `generator:`; this constant is the single place that changes.
pub const DEFINITION_ID_PREFIX: &str = "ergogen:";

const NO_CLOSED_COURTYARD: &str =
    "No closed courtyard is available; the outline uses physical graphics and pad extents.";

/// The definition fields normalization reads.
#[derive(Clone, Debug, Deserialize)]
pub struct DefinitionInput {
    pub id: String,
    pub kind: PartKind,
    #[serde(default)]
    pub keycap: Option<Vec2>,
    #[serde(rename = "envelopeSource", default)]
    pub envelope_source: Option<EnvelopeSource>,
    #[serde(default)]
    pub courtyard: Vec<Vec2>,
    #[serde(default)]
    pub pads: Vec<Pad>,
    #[serde(default)]
    pub terminals: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub generator: Option<GeneratorRef>,
}

/// What normalization changes on a definition.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Normalized {
    pub pads: Vec<Pad>,
    pub courtyard: Vec<Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keycap: Option<Vec2>,
    #[serde(rename = "envelopeSource")]
    pub envelope_source: EnvelopeSource,
    pub terminals: BTreeMap<String, Vec<String>>,
    #[serde(rename = "envelopeNotice", skip_serializing_if = "Option::is_none")]
    pub envelope_notice: Option<String>,
}

/// A built-in generator as a part definition, before any user edits.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CatalogueEntry {
    pub id: String,
    pub name: String,
    pub kind: PartKind,
    pub pads: Vec<Pad>,
    pub courtyard: Vec<Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keycap: Option<Vec2>,
    #[serde(rename = "matrixTerminals", skip_serializing_if = "Option::is_none")]
    pub matrix_terminals: Option<MatrixTerminals>,
    #[serde(rename = "envelopeSource")]
    pub envelope_source: EnvelopeSource,
    pub terminals: BTreeMap<String, Vec<String>>,
    pub generator: GeneratorRef,
    #[serde(rename = "envelopeNotice", skip_serializing_if = "Option::is_none")]
    pub envelope_notice: Option<String>,
}

fn valid_polygon(points: &[Vec2]) -> bool {
    points.len() >= 3 && points.iter().all(|p| p.x.is_finite() && p.y.is_finite())
}

impl Registry {
    /// Regenerate a definition's pads, envelope and terminals from its generator.
    ///
    /// `Ok(None)` for a definition with no generator, which is not ours to
    /// change. A generator source that is not registered is an error. Pads keep
    /// the saved ID and net of the pad at the same index, so a generator change
    /// must never alter pad count or order without a migration.
    pub fn normalize(&self, input: &DefinitionInput) -> Result<Option<Normalized>> {
        let Some(generator) = &input.generator else {
            return Ok(None);
        };
        let spec = self
            .get(&generator.source)
            .ok_or_else(|| GeneratorError::UnknownGenerator {
                source: generator.source.clone(),
            })?;
        let generated = geometry(&self.render(&input.id, generator, None, &mut NoNets)?)?;
        let pads: Vec<Pad> = generated
            .pads
            .iter()
            .enumerate()
            .map(|(index, pad)| match input.pads.get(index) {
                Some(previous) => Pad {
                    id: previous.id.clone(),
                    net_id: previous.net_id.clone().filter(|net| !net.is_empty()),
                    ..pad.clone()
                },
                None => pad.clone(),
            })
            .collect();
        let old = input.envelope_source.clone().unwrap_or_default();
        let courtyard_authored =
            old.courtyard == Some(EnvelopeOrigin::Authored) && valid_polygon(&input.courtyard);
        let keycap_authored = old.keycap == Some(EnvelopeOrigin::Authored)
            && input
                .keycap
                .is_some_and(|keycap| keycap.x > 0.0 && keycap.y > 0.0);
        let dims = keycap_dimensions(spec, generator)?;
        let keycap = if keycap_authored {
            input.keycap
        } else {
            dims.or(input.keycap)
        };
        let discovered = self.terminal_pads(&input.id, spec, generator)?;
        let terminals: BTreeMap<String, Vec<String>> = discovered
            .into_iter()
            .map(|(name, indices)| {
                let ids = indices
                    .into_iter()
                    .map(|index| {
                        pads.get(index)
                            .map_or_else(|| format!("pad-{index}"), |pad| pad.id.clone())
                    })
                    .collect();
                (name, ids)
            })
            .collect();
        Ok(Some(Normalized {
            courtyard: if courtyard_authored {
                input.courtyard.clone()
            } else {
                generated.courtyard
            },
            keycap,
            envelope_source: EnvelopeSource {
                courtyard: Some(if courtyard_authored {
                    EnvelopeOrigin::Authored
                } else {
                    EnvelopeOrigin::Generated
                }),
                keycap: keycap.map(|_| {
                    if keycap_authored {
                        EnvelopeOrigin::Authored
                    } else {
                        EnvelopeOrigin::Generated
                    }
                }),
            },
            terminals: if terminals.is_empty() {
                input.terminals.clone()
            } else {
                terminals
            },
            envelope_notice: (!courtyard_authored
                && generated.courtyard_fallback
                && input.kind != PartKind::Utility)
                .then(|| NO_CLOSED_COURTYARD.to_owned()),
            pads,
        }))
    }

    /// For each net parameter, the indices of the pads that carry it, found by
    /// rendering with a marker as that net's name.
    fn terminal_pads(
        &self,
        definition_id: &str,
        spec: &GeneratorSpec,
        generator: &GeneratorRef,
    ) -> Result<BTreeMap<String, Vec<usize>>> {
        let mut result: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for param in spec
            .parameters
            .iter()
            .filter(|param| param.kind == ParamKind::Net)
        {
            let marker = format!("__boardstudio_terminal_{}__", param.name);
            let mut draft = generator.clone();
            draft
                .parameters
                .insert(param.name.to_owned(), Value::String(marker.clone()));
            for form in self.render(definition_id, &draft, None, &mut ConstantNet(1))? {
                let Some(items) = form.as_list() else {
                    continue;
                };
                if !matches!(items.first(), Some(Expr::Atom(head)) if head == "footprint" || head == "module")
                {
                    continue;
                }
                for (index, pad) in children(items, "pad").enumerate() {
                    if let Some(net) = child(pad, "net")
                        && net.get(2).ok_or_else(scalar_error)?.value()? == marker
                    {
                        result.entry(param.name.to_owned()).or_default().push(index);
                    }
                }
            }
        }
        Ok(result)
    }

    /// The built-in generators as part definitions, in source order.
    pub fn catalogue(&self) -> Result<Vec<CatalogueEntry>> {
        self.specs()
            .iter()
            .map(|spec| self.catalogue_entry(spec))
            .collect()
    }

    fn catalogue_entry(&self, spec: &GeneratorSpec) -> Result<CatalogueEntry> {
        let generator = GeneratorRef {
            source: spec.source.to_owned(),
            version: GENERATOR_VERSION.to_owned(),
            parameters: BTreeMap::new(),
        };
        let id = format!("{DEFINITION_ID_PREFIX}{}", spec.source);
        let input = DefinitionInput {
            id: id.clone(),
            kind: spec.kind,
            keycap: None,
            envelope_source: None,
            courtyard: Vec::new(),
            pads: Vec::new(),
            terminals: BTreeMap::new(),
            generator: Some(generator.clone()),
        };
        let normalized = self
            .normalize(&input)
            .map_err(|error| {
                GeneratorError::rejected(
                    spec.source,
                    None,
                    format!("Cannot catalog {}: {}", spec.source, error.message()),
                )
            })?
            .expect("catalogue definitions have a generator");
        Ok(CatalogueEntry {
            id,
            name: spec.display_name.to_owned(),
            kind: spec.kind,
            pads: normalized.pads,
            courtyard: normalized.courtyard,
            keycap: normalized.keycap,
            matrix_terminals: spec.matrix_terminals.map(|(row, column)| MatrixTerminals {
                row: row.to_owned(),
                column: column.to_owned(),
            }),
            envelope_source: normalized.envelope_source,
            terminals: normalized.terminals,
            generator,
            envelope_notice: normalized.envelope_notice,
        })
    }
}

/// The keycap envelope a generator's parameters describe, if it declares one.
fn keycap_dimensions(spec: &GeneratorSpec, generator: &GeneratorRef) -> Result<Option<Vec2>> {
    let Some(keycap) = spec.keycap_parameters else {
        return Ok(None);
    };
    let read = |name: &str| -> Result<f64> {
        let param = spec
            .parameters
            .iter()
            .find(|param| param.name == name)
            .expect("registry validated keycap parameters");
        match generator
            .parameters
            .get(name)
            .filter(|value| !value.is_null())
        {
            Some(value) => value
                .as_f64()
                .ok_or_else(|| GeneratorError::InvalidParameter {
                    generator: spec.source.to_owned(),
                    parameter: name.to_owned(),
                    expected: param.kind.describe(),
                }),
            None => Ok(param
                .default
                .to_value()
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0)),
        }
    };
    let (width, height) = (read(keycap.width)?, read(keycap.height)?);
    for (name, value) in [(keycap.width, width), (keycap.height, height)] {
        if !(value.is_finite() && value > 0.0) {
            return Err(GeneratorError::rejected(
                spec.source,
                Some(name),
                "Keycap envelope dimensions must be greater than zero.",
            ));
        }
    }
    Ok(Some(Vec2 {
        x: width,
        y: height,
    }))
}
