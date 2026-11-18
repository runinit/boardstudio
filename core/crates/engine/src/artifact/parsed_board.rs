//! One parse and `kicad_pcb` root selection of an original board, shared by its projections.
//!
//! Each operation maps `BoardParseError` to its own error category and message.
use super::sexpr;
use kiutils_sexpr::{CstDocument, Node};

pub(super) enum BoardParseError {
    Syntax(String),
    NotBoard,
}

pub(super) struct ParsedBoard<'a> {
    source: &'a str,
    document: CstDocument,
}

impl<'a> ParsedBoard<'a> {
    pub(super) fn parse(source: &'a str) -> Result<Self, BoardParseError> {
        let document = kiutils_sexpr::parse_one(source)
            .map_err(|error| BoardParseError::Syntax(error.to_string()))?;
        if document
            .nodes
            .first()
            .is_none_or(|node| sexpr::head(node) != Some("kicad_pcb"))
        {
            return Err(BoardParseError::NotBoard);
        }
        Ok(Self { source, document })
    }

    /// The original text that the root's spans index into.
    pub(super) fn source(&self) -> &'a str {
        self.source
    }

    pub(super) fn root(&self) -> &Node {
        &self.document.nodes[0]
    }
}
