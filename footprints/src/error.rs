//! Typed generator errors. The user-facing text of each variant is the baseline
//! wording recorded from the JavaScript provider where one existed.
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum GeneratorError {
    /// The definition's generator source is not in the registry.
    UnknownGenerator { source: String },
    /// The definition requests a generator version other than `bundled-1`.
    UnsupportedVersion { version: String },
    /// Rendering was requested for a definition that has no generator.
    MissingGenerator,
    /// A saved parameter value does not have its declared type.
    InvalidParameter {
        generator: String,
        parameter: String,
        expected: &'static str,
    },
    /// A generator rejected its parameters. `message` is the user-facing text.
    Rejected {
        generator: String,
        parameter: Option<String>,
        message: String,
    },
    /// Generator output could not be read as KiCad forms.
    Parse(String),
    /// Generated pads or outlines could not be measured.
    Geometry(String),
    /// A model path is not attached, bundled or exportable.
    Model(String),
    /// A form that exports cannot represent.
    Export(String),
    /// Net indices ran out.
    Net(String),
    /// A generator body asked for a parameter it did not declare.
    Internal(String),
}

impl GeneratorError {
    pub fn rejected(generator: &str, parameter: Option<&str>, message: impl Into<String>) -> Self {
        Self::Rejected {
            generator: generator.to_owned(),
            parameter: parameter.map(str::to_owned),
            message: message.into(),
        }
    }

    /// The text shown to the user.
    pub fn message(&self) -> String {
        match self {
            Self::UnknownGenerator { source } => format!("Unknown Ergogen generator: {source}"),
            Self::UnsupportedVersion { version } => {
                format!("Unsupported Ergogen generator version: {version}")
            }
            Self::MissingGenerator => "Unknown Ergogen generator: undefined".to_owned(),
            Self::InvalidParameter {
                generator,
                parameter,
                expected,
            } => format!("{generator}: parameter {parameter} must be {expected}."),
            Self::Rejected { message, .. }
            | Self::Parse(message)
            | Self::Geometry(message)
            | Self::Model(message)
            | Self::Export(message)
            | Self::Net(message)
            | Self::Internal(message) => message.clone(),
        }
    }
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for GeneratorError {}

pub type Result<T> = std::result::Result<T, GeneratorError>;
