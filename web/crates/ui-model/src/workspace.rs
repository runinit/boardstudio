use std::{fmt, str::FromStr};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Workspace {
    Layout,
    Pcb,
    Keymap,
    Keycaps,
    Case,
    Parts,
    Export,
}

impl Workspace {
    pub const TABS: [Self; 6] = [
        Self::Layout,
        Self::Pcb,
        Self::Keymap,
        Self::Keycaps,
        Self::Case,
        Self::Parts,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Layout => "Layout",
            Self::Pcb => "PCB",
            Self::Keymap => "Keymap",
            Self::Keycaps => "Keycaps",
            Self::Case => "Case",
            Self::Parts => "Parts",
            Self::Export => "Export",
        }
    }
}

impl fmt::Display for Workspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Workspace {
    type Err = &'static str;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::TABS
            .into_iter()
            .chain([Self::Export])
            .find(|workspace| workspace.as_str() == name)
            .ok_or("unknown workspace")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_names_round_trip_with_the_existing_spelling_and_tab_order() {
        let names = ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"];
        assert_eq!(Workspace::TABS.map(Workspace::as_str), names);

        for (workspace, name) in Workspace::TABS
            .into_iter()
            .chain([Workspace::Export])
            .zip(names.into_iter().chain(["Export"]))
        {
            assert_eq!(workspace.to_string(), name);
            assert_eq!(name.parse(), Ok(workspace));
        }
        assert!("pcb".parse::<Workspace>().is_err());
    }
}
