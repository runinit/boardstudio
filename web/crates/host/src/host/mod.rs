mod offline;
mod storage;

pub use offline::register_offline;
pub use storage::{
    AssetBytes, BrowserStore, CONFLICT, PersistError, ProjectListing, SaveError, SavedProject,
    StorageSession, StorageToken, UnreadableProject,
};

// Test seam for the page suite's storage lifetime tests (test-support only).
#[cfg(feature = "test-support")]
pub use storage::test_hooks;
