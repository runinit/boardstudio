# Mounted Runtime alert regression: expected red

The first run of `mounted_export_panel_dispatches_failure_alert_and_successful_retry_from_same_row` failed after the injected production Runtime generation failure. The Runtime held the expected typed error, but the mounted report element remained `role="status"` and the test could not find `role="alert"`. This showed the extracted shared `RuntimeReportBanner` was not tracking the update signal when tested in the mounted ExportPanel composition.

After the banner consumed the App's shared update signal, the same mounted assertion passed and observed the error through `role="alert"`; a successful retry through that same Export row cleared the alert and rendered `role="status"`. The green run is retained in [row-tests.log](row-tests.log).
