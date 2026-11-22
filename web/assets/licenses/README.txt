Third-party notices for the CAD WebAssembly module

The CAD module is built from Rust sources only. The following components are
distributed under the Apache License 2.0, whose text is in Apache-2.0.txt:

- cadmpeg 0.6.12 crates (cadmpeg-core, cadmpeg-ir, cadmpeg-codec-step,
  cadmpeg-tessellate), with cadmpeg-tessellate vendored with local patches
  under cad/vendor/cadmpeg-tessellate. Source: https://github.com/cadmpeg/cadmpeg
- parry3d-f64 0.31.1. Source: https://github.com/dimforge/parry
- manifold-rust 0.16.0. Source: https://crates.io/crates/manifold-rust/0.16.0

Inflector 0.11.4 (Copyright 2017 Josh Teeter) is used by step-p21 under
BSD-2-Clause. See Inflector-BSD-2-Clause.txt.

clipper2-rust 1.2.0 (Clipper2 by Angus Johnson, Rust port by Lars Brubaker
and contributors) is used by manifold-rust under the Boost Software License
1.0. See clipper2-rust-BSL-1.0.txt.

Keycap lettering embeds DejaVu Sans outlines. The Bitstream Vera and Arev
notices, including the public-domain DejaVu changes, are in
fonts/dejavu-LICENSE.txt. Source: https://dejavu-fonts.github.io/

ttf-parser 0.25.1 is used under its MIT license, reproduced in
ttf-parser-MIT.txt. Source: https://github.com/harfbuzz/ttf-parser/tree/v0.25.1

Components under web/crates/ui-shared/src/ui/ are copied from rust-ui (https://github.com/rust-ui/ui,
MIT, Copyright (c) 2026 Max Wells), reproduced in rust-ui-MIT.txt. Each file names its upstream path and commit.

The workbench fonts are under the SIL Open Font License 1.1: Source Sans 3 and Source Code Pro
(fonts/source-sans-LICENSE.md, fonts/source-code-pro-LICENSE.md) and Inter (fonts/inter-LICENSE.txt,
Copyright 2016 The Inter Project Authors, https://github.com/rsms/inter). See web/assets/fonts/README.md.

Lucide icons are vendored from https://github.com/lucide-icons/lucide at commit 20aef31db93fe4d9f398dbb15b120d99c48eef95 under ISC; its license and Feather-derived icon MIT notice are in lucide-LICENSE.txt.
