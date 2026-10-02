// Native-only seam for running the included pure tests. `load_bundled` is not
// called; this stub exists only because the production module contains its URL
// call in the same source file as the pure catalogue functions.
mod runtime {
    #[allow(dead_code)]
    pub fn resource_url(_path: &str) -> Result<String, String> {
        Err("native contract harness does not load browser resources".into())
    }
}

mod presentation {
    pub mod parts {
        #[path = "/tmp/frontend-run/parts-contract-tests/catalogue.rs.source-4b05d451"]
        mod catalogue;
    }
}
