//! The Layout selection kinds the command pill switches between. Plain logic: compiled
//! natively so the transform tool activation rules can be unit tested.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutSelectionKind {
    Matrix,
    Row,
    Column,
    #[default]
    Key,
    Part,
}

impl LayoutSelectionKind {
    pub(crate) const ALL: [Self; 5] =
        [Self::Matrix, Self::Row, Self::Column, Self::Key, Self::Part];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Matrix => "Matrix",
            Self::Row => "Row",
            Self::Column => "Column",
            Self::Key => "Key",
            Self::Part => "Part",
        }
    }
}
