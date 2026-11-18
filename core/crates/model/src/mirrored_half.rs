/// Imported MX stabilisers: ADR-0010 keeps their world heading in every companion mode. Their
/// position takes the X reflection, and they never raise an uncertain-heading finding.
pub const SYMMETRIC_DEFINITIONS: [&str; 2] = [
    "kicad:marbastlib/STAB_MX_2u",
    "kicad:marbastlib/STAB_MX_6.25u",
];
