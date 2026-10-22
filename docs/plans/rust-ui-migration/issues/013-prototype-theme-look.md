# 013: Prototype the Original and shadcn Themes on copied components

Status: open
Type: prototype
Blocked by: None
Parent: [Map](../map.md)
Related: [005](005-theme-structure.md), [001](001-compile-rust-ui-against-our-build.md)

## Question

Do the Original (Graphite values in rust-ui names, 10px radius, Source Sans 3) and shadcn (rust-ui's neutral palette, Inter)
Themes look right, light and dark, on the copied Button, Input, Field, Select, Alert, Card, Badge, Toolbar and Sidenav? Build both
token blocks from [theme-contract.md](../theme-contract.md) in the scratch crate with a Theme and Appearance switch, view the
result with the user, and record the adjustments (radius, the authored success / warning / info fills, shared control sizes) that
the contract needs. Throwaway code; the findings are the output.
