//! CAD capture response fields shared with the existing events.log format.

use serde::Serialize;

#[derive(Serialize)]
pub struct Resource {
    url: String,
    status: i64,
    #[serde(rename = "serviceWorker")]
    service_worker: bool,
}

pub fn resource(url: String, status: i64, service_worker: Option<bool>) -> Resource {
    Resource {
        url,
        status,
        service_worker: service_worker.unwrap_or(false),
    }
}

pub fn check_observed_core_failures(
    resources: &[Resource],
) -> boardstudio_browser_harness::Result<()> {
    for response in resources {
        let path = response
            .url
            .split(['?', '#'])
            .next()
            .unwrap_or(&response.url);
        if path.contains("/assets/core-worker/") && response.status >= 400 {
            return Err(boardstudio_browser_harness::Error(format!(
                "observed core worker response failed: {} status {}",
                response.url, response.status
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn observed_core_worker_failure_is_fatal_without_requiring_an_inventory() {
        let core = super::resource(
            "http://example/boardstudio/assets/core-worker/a.wasm".into(),
            404,
            Some(false),
        );
        let unrelated = super::resource("http://example/favicon.ico".into(), 404, Some(false));
        let redirect = super::resource(
            "http://example/boardstudio/assets/core-worker/a.wasm".into(),
            302,
            Some(false),
        );
        assert!(super::check_observed_core_failures(&[]).is_ok());
        assert!(super::check_observed_core_failures(&[unrelated, redirect]).is_ok());
        assert!(super::check_observed_core_failures(&[core])
            .unwrap_err()
            .to_string()
            .contains("core worker"));
    }

    #[test]
    fn response_preserves_status_and_worker_origin() {
        let ordinary = super::resource("http://example/".into(), 404, None);
        let worker = super::resource("http://example/worker".into(), 201, Some(true));
        assert_eq!(
            serde_json::to_value(ordinary).unwrap(),
            serde_json::json!({
                "url": "http://example/", "status": 404, "serviceWorker": false
            })
        );
        assert_eq!(
            serde_json::to_value(worker).unwrap(),
            serde_json::json!({
                "url": "http://example/worker", "status": 201, "serviceWorker": true
            })
        );
    }
}
