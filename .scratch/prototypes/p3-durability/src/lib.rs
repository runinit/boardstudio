#[cfg(feature = "page")]
mod archive_probe;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod browser;

#[cfg(all(target_arch = "wasm32", feature = "service-worker"))]
mod service_worker;

#[cfg(feature = "page")]
pub use archive_probe::{ArchiveError, UnpackedProject, pack_project, unpack_project};

#[cfg(any(feature = "service-worker", test))]
const CACHE_PREFIX: &str = "boardstudio-p3-durability-path-";

#[cfg(any(feature = "service-worker", test))]
fn scope_cache_prefix(scope_url: &str) -> String {
    let scope_path = scope_url
        .split_once("://")
        .and_then(|(_, authority_and_path)| {
            authority_and_path
                .find('/')
                .map(|index| &authority_and_path[index..])
        })
        .unwrap_or("/");
    let mut encoded = String::with_capacity(scope_path.len() * 2);
    for byte in scope_path.bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    format!("{CACHE_PREFIX}{encoded}-")
}

#[cfg(test)]
mod cache_prefix_tests {
    use super::scope_cache_prefix;

    #[test]
    fn path_hex_encoding_distinguishes_slashes_from_underscores() {
        assert_ne!(
            scope_cache_prefix("https://example.test/a/b/"),
            scope_cache_prefix("https://example.test/a_b/")
        );
        assert_ne!(
            scope_cache_prefix("https://example.test/"),
            scope_cache_prefix("https://example.test/_/")
        );
        assert_eq!(
            scope_cache_prefix("https://example.test/"),
            "boardstudio-p3-durability-path-2f-"
        );
    }
}
