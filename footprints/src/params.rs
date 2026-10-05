//! Declared generator parameters. Types are explicit; nothing is inferred from a
//! default value, and a saved value of the wrong type is rejected.
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamKind {
    Number,
    Boolean,
    String,
    /// A net name; an empty or missing name means no net.
    Net,
    Array,
}

impl ParamKind {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Number => "a number",
            Self::Boolean => "true or false",
            Self::String => "text",
            Self::Net => "a net name",
            Self::Array => "a list",
        }
    }

    pub fn accepts(self, value: &Value) -> bool {
        match self {
            Self::Number => value.is_number(),
            Self::Boolean => value.is_boolean(),
            Self::String | Self::Net => value.is_string(),
            Self::Array => value.is_array(),
        }
    }
}

/// A parameter's default, written where the generator is declared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefaultValue {
    /// Net parameters only: no net.
    None,
    Bool(bool),
    Num(f64),
    Str(&'static str),
    /// Array defaults, as JSON text.
    Json(&'static str),
}

impl DefaultValue {
    pub fn to_value(self) -> Option<Value> {
        match self {
            Self::None => None,
            Self::Bool(value) => Some(Value::Bool(value)),
            Self::Num(value) => serde_json::Number::from_f64(value).map(Value::Number),
            Self::Str(value) => Some(Value::String(value.to_owned())),
            Self::Json(text) => serde_json::from_str(text).ok(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParamSpec {
    pub name: &'static str,
    pub kind: ParamKind,
    pub default: DefaultValue,
}

impl ParamSpec {
    pub const fn number(name: &'static str, default: f64) -> Self {
        Self {
            name,
            kind: ParamKind::Number,
            default: DefaultValue::Num(default),
        }
    }
    pub const fn boolean(name: &'static str, default: bool) -> Self {
        Self {
            name,
            kind: ParamKind::Boolean,
            default: DefaultValue::Bool(default),
        }
    }
    pub const fn string(name: &'static str, default: &'static str) -> Self {
        Self {
            name,
            kind: ParamKind::String,
            default: DefaultValue::Str(default),
        }
    }
    /// A net parameter with no default net.
    pub const fn net(name: &'static str) -> Self {
        Self {
            name,
            kind: ParamKind::Net,
            default: DefaultValue::None,
        }
    }
    pub const fn net_default(name: &'static str, default: &'static str) -> Self {
        Self {
            name,
            kind: ParamKind::Net,
            default: DefaultValue::Str(default),
        }
    }
    pub const fn array(name: &'static str, json: &'static str) -> Self {
        Self {
            name,
            kind: ParamKind::Array,
            default: DefaultValue::Json(json),
        }
    }
    /// The layer parameter every generator declares. DefaultValue `F`.
    pub const SIDE: ParamSpec = ParamSpec::string("side", "F");
}

/// One entry of a generator's parameter schema, as the page shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParameterSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ParamKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}
