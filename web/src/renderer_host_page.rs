// Binary-only private renderer module. `renderer_host_page_base.rs` is a
// source snapshot of renderer_host.rs at baseline 261ef5bfe1dc984f24be44c95873a684c4940968
// (SHA-256 377f45e0ad51f5979dcba152d6b14dec71ea035991ee031b370829b5ac5b06e6).
// Its only normalization changes the three leading `//!` crate-doc prefixes
// to `//`; Rust rejects those inner docs when the file is textually included.
// Keep the snapshot synchronized with the byte comparison test below.
include!("renderer_host_page_base.rs");
include!("renderer_host_page_extensions.rs");

#[cfg(test)]
mod source_sync_tests {
    #[test]
    fn private_page_host_matches_normalized_library_host() {
        let normalized = include_str!("renderer_host.rs").replacen("//!", "//", 3);
        assert_eq!(
            include_str!("renderer_host_page_base.rs"),
            normalized,
            "refresh the private page snapshot only from the byte-exact library host"
        );
    }
}
