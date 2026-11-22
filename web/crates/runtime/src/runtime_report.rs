use std::cell::RefCell;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeReport {
    pub(crate) message: String,
    pub(crate) is_alert: bool,
    pub(crate) notification_error: Option<bool>,
}

/// A one-shot result for the shell's single notification consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeNotification {
    pub message: String,
    pub is_error: bool,
}

impl RuntimeReport {
    pub(crate) fn status(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Self::default()
        }
    }

    pub(crate) fn alert(message: impl Into<String>) -> Self {
        Self {
            is_alert: true,
            ..Self::status(message)
        }
    }

    pub(crate) fn transient(mut self) -> Self {
        self.notification_error = Some(self.is_alert);
        self
    }

    pub(crate) fn transient_error(mut self) -> Self {
        self.notification_error = Some(true);
        self
    }

    pub(crate) fn clear_alert(&mut self) {
        if self.is_alert {
            self.message.clear();
            self.is_alert = false;
        }
    }
}

pub(crate) fn publish(
    status: &RefCell<RuntimeReport>,
    notifications: &RefCell<Vec<RuntimeNotification>>,
    report: RuntimeReport,
    changed: impl FnOnce(),
) {
    if let Some(is_error) = report.notification_error {
        notifications.borrow_mut().push(RuntimeNotification {
            message: report.message.clone(),
            is_error,
        });
    }
    *status.borrow_mut() = report;
    changed();
}

pub(crate) fn take_notifications(
    notifications: &RefCell<Vec<RuntimeNotification>>,
) -> Vec<RuntimeNotification> {
    std::mem::take(&mut *notifications.borrow_mut())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn notification_queue_keeps_repeated_events_and_drains_once_without_notifying() {
        let status = RefCell::new(RuntimeReport::status("Ready"));
        let notifications = RefCell::new(Vec::new());
        let changes = Cell::new(0);
        for report in [
            RuntimeReport::status("Same result").transient(),
            RuntimeReport::status("Same result").transient(),
            RuntimeReport::alert("Action failed").transient(),
            RuntimeReport::status("Saved locally."),
        ] {
            publish(&status, &notifications, report, || {
                changes.set(changes.get() + 1)
            });
        }
        assert!(
            status.borrow().notification_error.is_none(),
            "classification never matches message text"
        );
        assert_eq!(changes.get(), 4);
        let pending = take_notifications(&notifications);
        assert_eq!(
            pending
                .iter()
                .map(|n| (n.message.as_str(), n.is_error))
                .collect::<Vec<_>>(),
            vec![
                ("Same result", false),
                ("Same result", false),
                ("Action failed", true)
            ]
        );
        assert!(take_notifications(&notifications).is_empty());
        assert_eq!(changes.get(), 4, "draining cannot schedule another render");
    }
}
