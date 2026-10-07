//! The generator registry and the provider operations over it.
use std::collections::BTreeMap;

use serde_json::Value;

use crate::context::{NetRef, RenderContext, Resolved};
use crate::error::{GeneratorError, Result};
use crate::nets::NetIndexer;
use crate::params::{ParamKind, ParamSpec, ParameterSchema};
use crate::sexpr::{Expr, parse_forms};
use crate::types::{GeneratorRef, MatrixTerminals, PartKind, PartRef, Side};

/// The one generator version this build understands.
pub const GENERATOR_VERSION: &str = "bundled-1";

pub type Body = fn(&RenderContext<'_>) -> Result<String>;

/// Source licence and attribution, kept from the ported file's header.
#[derive(Clone, Copy, Debug)]
pub struct License {
    pub spdx: &'static str,
    pub author: &'static str,
}

/// Which parameters hold the keycap envelope (width, height).
#[derive(Clone, Copy, Debug)]
pub struct KeycapParameters {
    pub width: &'static str,
    pub height: &'static str,
}

/// Everything about one generator, declared where the generator is written.
pub struct GeneratorSpec {
    /// Stable source ID such as `ceoloide/switch_mx`.
    pub source: &'static str,
    pub display_name: &'static str,
    pub kind: PartKind,
    /// Net parameters that carry the matrix row and column.
    pub matrix_terminals: Option<(&'static str, &'static str)>,
    pub keycap_parameters: Option<KeycapParameters>,
    pub parameters: &'static [ParamSpec],
    pub license: License,
    pub body: Body,
}

#[derive(Debug, PartialEq)]
pub struct RegistryError(pub String);

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RegistryError {}

pub struct Registry {
    specs: Vec<&'static GeneratorSpec>,
}

impl Registry {
    /// Validates every declaration: unique sources, a `side` string parameter with
    /// default `F`, defaults that match their types, and consistent terminals.
    pub fn new(specs: &[&'static GeneratorSpec]) -> std::result::Result<Self, RegistryError> {
        let mut sorted = specs.to_vec();
        sorted.sort_by_key(|spec| spec.source);
        for pair in sorted.windows(2) {
            if pair[0].source == pair[1].source {
                return Err(RegistryError(format!(
                    "duplicate generator {}",
                    pair[0].source
                )));
            }
        }
        for spec in &sorted {
            validate(spec).map_err(|reason| RegistryError(format!("{}: {reason}", spec.source)))?;
        }
        Ok(Self { specs: sorted })
    }

    pub fn get(&self, source: &str) -> Option<&'static GeneratorSpec> {
        self.specs
            .binary_search_by_key(&source, |spec| spec.source)
            .ok()
            .map(|index| self.specs[index])
    }

    pub fn contains(&self, source: &str) -> bool {
        self.get(source).is_some()
    }

    /// Specs in source order.
    pub fn specs(&self) -> &[&'static GeneratorSpec] {
        &self.specs
    }

    fn spec(&self, source: &str) -> Result<&'static GeneratorSpec> {
        self.get(source)
            .ok_or_else(|| GeneratorError::UnknownGenerator {
                source: source.to_owned(),
            })
    }

    /// The parameter schema in declaration order.
    pub fn parameters(&self, source: &str) -> Result<Vec<ParameterSchema>> {
        Ok(self
            .spec(source)?
            .parameters
            .iter()
            .map(|param| ParameterSchema {
                name: param.name.to_owned(),
                kind: param.kind,
                value: param.default.to_value(),
            })
            .collect())
    }

    /// Render a definition (and optionally a placed part) to KiCad forms.
    ///
    /// Parameters merge definition then part; `null` and missing values fall back
    /// to the declared default; a value of the wrong type is an error. Net
    /// parameters resolve eagerly in declaration order so net indices are
    /// allocated in a stable order.
    pub fn render(
        &self,
        definition_id: &str,
        generator: &GeneratorRef,
        part: Option<&PartRef>,
        nets: &mut dyn NetIndexer,
    ) -> Result<Vec<Expr>> {
        let spec = self.spec(&generator.source)?;
        if generator.version != GENERATOR_VERSION {
            return Err(GeneratorError::UnsupportedVersion {
                version: generator.version.clone(),
            });
        }
        let mut inputs: BTreeMap<&str, &Value> = generator
            .parameters
            .iter()
            .map(|(key, value)| (key.as_str(), value))
            .collect();
        if let Some(saved) = part.and_then(|part| part.generator_parameters.as_ref()) {
            inputs.extend(saved.iter().map(|(key, value)| (key.as_str(), value)));
        }
        let mut values = Vec::with_capacity(spec.parameters.len());
        for param in spec.parameters {
            let supplied = inputs
                .get(param.name)
                .copied()
                .filter(|value| !value.is_null());
            let value = match supplied {
                Some(value) if param.kind.accepts(value) => Some(value.clone()),
                Some(_) => {
                    return Err(GeneratorError::InvalidParameter {
                        generator: spec.source.to_owned(),
                        parameter: param.name.to_owned(),
                        expected: param.kind.describe(),
                    });
                }
                None => param.default.to_value(),
            };
            let resolved = if param.kind == ParamKind::Net {
                let name = match value {
                    Some(Value::String(name)) => name,
                    _ => String::new(),
                };
                let index = if name.is_empty() {
                    0
                } else {
                    nets.index(&name)?
                };
                Resolved::Net(NetRef { name, index })
            } else {
                Resolved::Value(value.unwrap_or(Value::Null))
            };
            values.push((param.name, resolved));
        }
        let (reference, at, rotation, mirrored) = match part {
            Some(part) => (
                part.reference.clone(),
                part.pose.at,
                part.pose.rotation,
                part.side == Side::Back,
            ),
            None => (
                "REF**".to_owned(),
                crate::types::Vec2 { x: 0.0, y: 0.0 },
                0.0,
                false,
            ),
        };
        let context = RenderContext {
            generator: spec.source,
            values,
            nets: std::cell::RefCell::new(nets),
            local_id: part
                .map_or(definition_id, |part| part.id.as_str())
                .to_owned(),
            reference,
            at,
            rotation,
            mirrored,
            misuse: std::cell::RefCell::new(None),
        };
        let text = (spec.body)(&context)?;
        context.take_misuse()?;
        parse_forms(&text)
    }

    pub fn matrix_terminals(&self, source: &str) -> Result<Option<MatrixTerminals>> {
        Ok(self
            .spec(source)?
            .matrix_terminals
            .map(|(row, column)| MatrixTerminals {
                row: row.to_owned(),
                column: column.to_owned(),
            }))
    }
}

fn validate(spec: &GeneratorSpec) -> std::result::Result<(), String> {
    let mut seen = Vec::new();
    for param in spec.parameters {
        if seen.contains(&param.name) {
            return Err(format!("parameter {} is declared twice", param.name));
        }
        seen.push(param.name);
        match (param.kind, param.default.to_value()) {
            (ParamKind::Net, None) => {}
            (kind, Some(value)) if kind.accepts(&value) => {}
            (ParamKind::Net, Some(_)) => {
                return Err(format!("net {} needs a string default", param.name));
            }
            (_, None) => return Err(format!("parameter {} needs a default", param.name)),
            (_, Some(_)) => {
                return Err(format!("default of {} does not match its type", param.name));
            }
        }
    }
    let side = spec.parameters.iter().find(|param| param.name == "side");
    match side.map(|param| (param.kind, param.default.to_value())) {
        Some((ParamKind::String, Some(Value::String(value)))) if value == "F" => {}
        _ => {
            return Err("every generator declares a string `side` parameter with default F".into());
        }
    }
    let kind_of = |name: &str| {
        spec.parameters
            .iter()
            .find(|param| param.name == name)
            .map(|param| param.kind)
    };
    if let Some((row, column)) = spec.matrix_terminals {
        for name in [row, column] {
            if kind_of(name) != Some(ParamKind::Net) {
                return Err(format!("matrix terminal {name} is not a net parameter"));
            }
        }
    }
    if let Some(keycap) = spec.keycap_parameters {
        for name in [keycap.width, keycap.height] {
            if kind_of(name) != Some(ParamKind::Number) {
                return Err(format!("keycap parameter {name} is not a number parameter"));
            }
        }
    }
    Ok(())
}
