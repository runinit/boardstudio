use boardstudio_application::OperationId;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

#[cfg(target_arch = "wasm32")]
use std::{future::Future, pin::Pin};

#[cfg(target_arch = "wasm32")]
pub(crate) type ArchiveWorkFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + 'a>>;

#[derive(Default)]
pub(crate) struct ArchiveOptionCaptures(BTreeMap<OperationId, bool>);

impl ArchiveOptionCaptures {
    pub(crate) fn capture(&mut self, operation: OperationId, embed_used_models: bool) {
        self.0.insert(operation, embed_used_models);
    }

    pub(crate) fn take(&mut self, operation: OperationId) -> Option<bool> {
        self.0.remove(&operation)
    }

    pub(crate) fn remove(&mut self, operation: OperationId) {
        self.0.remove(&operation);
    }
}

/// Owns the browser-session preference and the per-operation choice used by
/// the production RunExport dispatcher. Keeping dispatch here makes archive,
/// cancellation, and STEP classification testable without browser handles.
pub(crate) struct ArchiveExportOptions {
    embed_used_models: Cell<bool>,
    captures: RefCell<ArchiveOptionCaptures>,
}

impl Default for ArchiveExportOptions {
    fn default() -> Self {
        Self {
            embed_used_models: Cell::new(true),
            captures: RefCell::new(ArchiveOptionCaptures::default()),
        }
    }
}

impl ArchiveExportOptions {
    pub(crate) fn embed_used_models(&self) -> bool {
        self.embed_used_models.get()
    }

    pub(crate) fn set_embed_used_models(&self, value: bool) -> bool {
        self.embed_used_models.replace(value) != value
    }

    pub(crate) fn begin_archive(&self, operation: OperationId) {
        self.captures
            .borrow_mut()
            .capture(operation, self.embed_used_models.get());
    }

    pub(crate) fn settle(&self, operation: OperationId) {
        self.captures.borrow_mut().remove(operation);
    }

    pub(crate) fn cancel(&self, operation: OperationId) {
        self.captures.borrow_mut().remove(operation);
    }

    /// Select the exact operation path used by Runtime::run for RunExport.
    /// The returned value can be a future; no preference is read after dispatch.
    pub(crate) fn dispatch<T>(
        &self,
        operation: OperationId,
        is_step_export: bool,
        step: impl FnOnce() -> T,
        archive: impl FnOnce(bool) -> T,
    ) -> Result<T, String> {
        if is_step_export {
            return Ok(step());
        }
        let embed_used_models = self.captures.borrow_mut().take(operation).ok_or_else(|| {
            "Archive option was not captured for this export; try again.".to_owned()
        })?;
        Ok(archive(embed_used_models))
    }
}

pub(crate) fn archive_filename(accepted_project_name: &str) -> String {
    format!("{accepted_project_name}.boardstudio")
}

#[cfg(test)]
mod tests {
    use super::ArchiveExportOptions;
    use boardstudio_application::OperationId;

    #[test]
    fn session_option_defaults_on_and_notifies_only_when_changed() {
        let options = ArchiveExportOptions::default();
        assert!(options.embed_used_models());
        assert!(!options.set_embed_used_models(true));
        assert!(options.set_embed_used_models(false));
        assert!(!options.embed_used_models());
    }

    #[test]
    fn production_dispatch_uses_the_option_captured_when_archive_started() {
        let options = ArchiveExportOptions::default();
        let operation = OperationId(11);
        options.begin_archive(operation);
        options.set_embed_used_models(false);

        let decision = options.dispatch(
            operation,
            false,
            || "STEP path".to_owned(),
            |embed| format!("archive path, embed={embed}"),
        );

        assert_eq!(decision.unwrap(), "archive path, embed=true");
    }

    #[test]
    fn production_dispatch_fails_closed_after_archive_capture_is_cancelled() {
        let options = ArchiveExportOptions::default();
        let operation = OperationId(21);
        options.begin_archive(operation);
        options.cancel(operation);
        let mut archive_called = false;

        let decision = options.dispatch(
            operation,
            false,
            || "STEP path".to_owned(),
            |_| {
                archive_called = true;
                "archive path".to_owned()
            },
        );

        assert_eq!(
            decision.unwrap_err(),
            "Archive option was not captured for this export; try again."
        );
        assert!(!archive_called);
    }

    #[test]
    fn settled_operation_releases_its_unused_archive_choice() {
        let options = ArchiveExportOptions::default();
        let operation = OperationId(25);
        options.begin_archive(operation);
        options.settle(operation);

        let decision = options.dispatch(operation, false, || "STEP", |_| "archive");

        assert!(decision.is_err());
    }

    #[test]
    fn production_dispatch_keeps_step_separate_from_archive_capture() {
        let options = ArchiveExportOptions::default();
        let operation = OperationId(31);
        options.begin_archive(operation);
        let mut archive_called = false;

        let decision = options.dispatch(
            operation,
            true,
            || "STEP path".to_owned(),
            |_| {
                archive_called = true;
                "archive path".to_owned()
            },
        );

        assert_eq!(decision.unwrap(), "STEP path");
        assert!(!archive_called);
    }

    #[test]
    fn production_dispatch_consumes_each_operation_capture_once() {
        let options = ArchiveExportOptions::default();
        let first = OperationId(41);
        let second = OperationId(42);
        options.begin_archive(first);
        options.set_embed_used_models(false);
        options.begin_archive(second);

        let first_decision = options.dispatch(first, false, || false, |embed| embed);
        let second_decision = options.dispatch(second, false, || false, |embed| embed);
        let repeated = options.dispatch(first, false, || false, |embed| embed);

        assert!(first_decision.unwrap());
        assert!(!second_decision.unwrap());
        assert_eq!(
            repeated.unwrap_err(),
            "Archive option was not captured for this export; try again."
        );
    }

    #[test]
    fn archive_filename_uses_the_accepted_project_name() {
        assert_eq!(super::archive_filename("Sofle v2"), "Sofle v2.boardstudio");
    }
}
