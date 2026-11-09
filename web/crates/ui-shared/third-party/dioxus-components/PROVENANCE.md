# Dioxus Components provenance

BoardStudio uses the `dioxus-primitives` package from the official DioxusLabs
repository at the exact Git revision
`9a758255ea26e2b20c8cecf4c1c946feb9e71da7`.

- Repository: <https://github.com/DioxusLabs/dioxus-components>
- Pinned source: <https://github.com/DioxusLabs/dioxus-components/tree/9a758255ea26e2b20c8cecf4c1c946feb9e71da7>
- Package manifest: `primitives/Cargo.toml`
- License declared by upstream package: `MIT OR Apache-2.0`
- Local license notices: [`LICENSE`](LICENSE)

The workspace dependency disables default features and enables only the `web`
feature for its WASM target. The exact revision was resolved and compiled in
the isolated compatibility harness before integrating it into the workspace.
