use boardstudio_application::OperationId;
use std::collections::BTreeMap;

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

pub(crate) fn archive_filename(accepted_project_name: &str) -> String {
    format!("{accepted_project_name}.boardstudio")
}

#[cfg(test)]
mod tests {
    use super::ArchiveOptionCaptures;
    use boardstudio_application::OperationId;

    #[test]
    fn each_export_uses_the_option_captured_when_it_started() {
        let mut captured = ArchiveOptionCaptures::default();
        let first = OperationId(11);
        let second = OperationId(12);

        captured.capture(first, true);
        captured.capture(second, false);

        assert_eq!(captured.take(first), Some(true));
        assert_eq!(captured.take(second), Some(false));
        assert_eq!(captured.take(first), None);
    }

    #[test]
    fn cancelled_operation_does_not_leave_an_option_for_a_later_export() {
        let mut captured = ArchiveOptionCaptures::default();
        let cancelled = OperationId(21);
        captured.capture(cancelled, true);
        captured.remove(cancelled);
        assert_eq!(captured.take(cancelled), None);
    }

    #[test]
    fn archive_filename_uses_the_accepted_project_name() {
        assert_eq!(super::archive_filename("Sofle v2"), "Sofle v2.boardstudio");
    }
}
