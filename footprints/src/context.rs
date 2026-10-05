//! The render context a generator body receives: resolved parameters, the part's
//! pose, and the coordinate helpers (`isxy`, `iaxy`, `esxy`, `eaxy`).
use std::cell::RefCell;
use std::fmt;

use serde_json::Value;

use crate::error::{GeneratorError, Result};
use crate::nets::NetIndexer;
use crate::number::{cos, encode_uri_component, js_number, radians, sin};
use crate::types::Vec2;

/// A resolved net parameter. `Display` is KiCad's `(net <index> "<name>")`.
#[derive(Clone, Debug, PartialEq)]
pub struct NetRef {
    pub name: String,
    pub index: u32,
}

impl fmt::Display for NetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = serde_json::to_string(&self.name).map_err(|_| fmt::Error)?;
        write!(f, "(net {} {})", self.index, name)
    }
}

pub(crate) enum Resolved {
    Value(Value),
    Net(NetRef),
}

/// The part's placement, and where it mirrors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub mirrored: bool,
}

pub struct RenderContext<'a> {
    pub(crate) generator: &'static str,
    pub(crate) values: Vec<(&'static str, Resolved)>,
    pub(crate) nets: RefCell<&'a mut dyn NetIndexer>,
    pub(crate) local_id: String,
    pub(crate) reference: String,
    pub(crate) at: Vec2,
    pub(crate) rotation: f64,
    pub(crate) mirrored: bool,
    pub(crate) misuse: RefCell<Option<String>>,
}

impl RenderContext<'_> {
    fn lookup(&self, name: &str) -> Option<&Resolved> {
        self.values
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(value))
    }

    fn misuse(&self, name: &str, wanted: &str) {
        self.misuse.borrow_mut().get_or_insert_with(|| {
            format!(
                "{}: body read {wanted} parameter {name}, which it does not declare as such",
                self.generator
            )
        });
    }

    /// A boolean parameter.
    pub fn flag(&self, name: &str) -> bool {
        match self.lookup(name) {
            Some(Resolved::Value(Value::Bool(value))) => *value,
            _ => {
                self.misuse(name, "boolean");
                false
            }
        }
    }

    /// A number parameter.
    pub fn number(&self, name: &str) -> f64 {
        match self.lookup(name) {
            Some(Resolved::Value(Value::Number(value))) => value.as_f64().unwrap_or(0.0),
            _ => {
                self.misuse(name, "number");
                0.0
            }
        }
    }

    /// A string parameter.
    pub fn text(&self, name: &str) -> &str {
        match self.lookup(name) {
            Some(Resolved::Value(Value::String(value))) => value,
            _ => {
                self.misuse(name, "string");
                ""
            }
        }
    }

    /// An array parameter.
    pub fn list(&self, name: &str) -> &[Value] {
        match self.lookup(name) {
            Some(Resolved::Value(Value::Array(value))) => value,
            _ => {
                self.misuse(name, "array");
                &[]
            }
        }
    }

    /// A net parameter.
    pub fn net(&self, name: &str) -> &NetRef {
        static NONE: NetRef = NetRef {
            name: String::new(),
            index: 0,
        };
        match self.lookup(name) {
            Some(Resolved::Net(net)) => net,
            _ => {
                self.misuse(name, "net");
                &NONE
            }
        }
    }

    /// An optional vector parameter: a list of three numbers, or an empty list
    /// (or nothing) for "automatic".
    pub fn vec3(&self, name: &str) -> Result<Option<[f64; 3]>> {
        let items = self.list(name);
        if items.is_empty() {
            return Ok(None);
        }
        let numbers: Option<Vec<f64>> = items.iter().map(Value::as_f64).collect();
        match numbers.as_deref() {
            Some(&[x, y, z]) => Ok(Some([x, y, z])),
            _ => Err(GeneratorError::InvalidParameter {
                generator: self.generator.to_owned(),
                parameter: name.to_owned(),
                expected: "a list of three numbers",
            }),
        }
    }

    /// The layer the body renders on (`F` or `B`).
    pub fn side(&self) -> &str {
        self.text("side")
    }

    /// The reference designator (`REF**` when unplaced).
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Always empty; kept so ported bodies read like their sources.
    pub fn ref_hide(&self) -> &str {
        ""
    }

    pub fn mirrored(&self) -> bool {
        self.mirrored
    }

    pub fn point(&self) -> Point {
        Point {
            x: self.at.x,
            y: self.at.y,
            r: self.rotation,
            mirrored: self.mirrored,
        }
    }

    pub fn x(&self) -> f64 {
        self.at.x
    }

    /// The part's Y in KiCad's downward axis.
    pub fn y(&self) -> f64 {
        -self.at.y
    }

    pub fn rotation(&self) -> f64 {
        self.rotation
    }

    pub fn xy(&self) -> String {
        xy(self.at.x, -self.at.y)
    }

    /// `(at <x> <y> <rotation>)`.
    pub fn at(&self) -> String {
        format!("(at {} {})", self.xy(), js_number(self.rotation))
    }

    /// A local offset for graphics that follow the part's mirroring.
    pub fn isxy(&self, x: f64, y: f64) -> String {
        xy(if self.mirrored { -x } else { x }, y)
    }

    /// A local offset that never mirrors.
    pub fn iaxy(&self, x: f64, y: f64) -> String {
        xy(x, y)
    }

    /// An absolute position for a local offset, mirrored with the part.
    pub fn esxy(&self, x: f64, y: f64) -> String {
        let at = self.transform(x, y, false);
        xy(at.x, at.y)
    }

    /// An absolute position for a local offset, never mirrored.
    pub fn eaxy(&self, x: f64, y: f64) -> String {
        let at = self.transform(x, y, true);
        xy(at.x, at.y)
    }

    fn transform(&self, x: f64, y: f64, resist: bool) -> Vec2 {
        let angle = radians(self.rotation);
        let sx = if resist || !self.mirrored { x } else { -x };
        Vec2 {
            x: self.at.x + sx * cos(angle) + y * sin(angle),
            y: -self.at.y - sx * sin(angle) + y * cos(angle),
        }
    }

    /// A net private to this part, named from its stable identity (never its
    /// editable reference) so renaming a part keeps its local nets.
    pub fn local_net(&self, suffix: &str) -> Result<NetRef> {
        let name = format!(
            "__boardstudio_local_{}_{}",
            encode_uri_component(&self.local_id),
            encode_uri_component(suffix)
        );
        let index = self.nets.borrow_mut().index(&name)?;
        Ok(NetRef { name, index })
    }

    pub(crate) fn take_misuse(&self) -> Result<()> {
        match self.misuse.borrow_mut().take() {
            Some(message) => Err(GeneratorError::Internal(message)),
            None => Ok(()),
        }
    }
}

fn xy(x: f64, y: f64) -> String {
    format!("{} {}", js_number(x), js_number(y))
}
